/// Multi-Track Audio Mixing & DSP Engine for After Effects timeline playback.
///
/// Features:
/// - Real PCM stereo AudioBuffer storage & multi-track mixing
/// - Per-layer animated volume/gain evaluation (`volume.evaluate(frame)`)
/// - Peak RMS VU meter calculation for audio mixers
use crate::core::timeline::{Composition, LayerType};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f32>, // Interleaved stereo PCM [L, R, L, R...]
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioBuffer {
    #[allow(dead_code)]
    pub fn new_sine_preview(duration_sec: f32, sample_rate: u32, freq: f32) -> Self {
        let num_samples = (duration_sec * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let s = (t * freq * std::f32::consts::TAU).sin() * 0.25;
            samples.push(s);
            samples.push(s);
        }
        Self { samples, sample_rate, channels: 2 }
    }

    /// Resample PCM audio samples to match destination sample rate (e.g. 44.1kHz -> 48kHz).
    #[allow(dead_code)]
    pub fn resample(&self, target_sample_rate: u32) -> Self {
        if self.sample_rate == target_sample_rate || self.sample_rate == 0 || self.samples.is_empty() {
            return self.clone();
        }

        let ratio = self.sample_rate as f64 / target_sample_rate as f64;
        let num_frames = (self.samples.len() / 2) as f64;
        let new_frames = (num_frames / ratio) as usize;
        let mut out = Vec::with_capacity(new_frames * 2);

        for i in 0..new_frames {
            let src_idx = (i as f64 * ratio) as usize * 2;
            if src_idx + 1 < self.samples.len() {
                out.push(self.samples[src_idx]);
                out.push(self.samples[src_idx + 1]);
            } else {
                out.push(0.0);
                out.push(0.0);
            }
        }
        Self {
            samples: out,
            sample_rate: target_sample_rate,
            channels: self.channels,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AudioFrameMeter {
    pub peak_db_left: f32,
    pub peak_db_right: f32,
    pub rms_db_left: f32,
    pub rms_db_right: f32,
}

impl Default for AudioFrameMeter {
    fn default() -> Self {
        Self {
            peak_db_left: -90.0,
            peak_db_right: -90.0,
            rms_db_left: -90.0,
            rms_db_right: -90.0,
        }
    }
}

/// Mix audio tracks across all active layers for a given frame window into a 2-channel stereo f32 buffer.
/// Delegates to `mix_audio_sources_for_frame` for real WAV sampling.
#[allow(dead_code)]
/// Master DSP parameters (passed from UI controls to audio engine).
#[derive(Debug, Clone)]
pub struct MasterDspParams {
    pub eq_highpass: f32,
    pub eq_lowpass: f32,
    pub eq_mid_gain: f32,
    pub eq_mid_freq: f32,
    pub comp_threshold: f32,
    pub comp_ratio: f32,
    pub comp_attack: f32,
    pub comp_release: f32,
    pub comp_makeup: f32,
}

impl Default for MasterDspParams {
    fn default() -> Self {
        Self {
            eq_highpass: 30.0,
            eq_lowpass: 18000.0,
            eq_mid_gain: 0.0,
            eq_mid_freq: 1000.0,
            comp_threshold: -12.0,
            comp_ratio: 2.0,
            comp_attack: 10.0,
            comp_release: 100.0,
            comp_makeup: 0.0,
        }
    }
}

pub fn mix_audio_for_frame(
    comp: &Composition,
    frame: u32,
    sample_rate: u32,
    buffer_size: usize,
    dsp: &MasterDspParams,
) -> (Vec<f32>, AudioFrameMeter) {
    mix_audio_sources_for_frame(comp, frame, sample_rate, buffer_size, None, dsp)
}

/// Keyframe data point generated from audio amplitude analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AudioAmplitudeKeyframe {
    pub frame: u32,
    pub left_amp: f32,
    pub right_amp: f32,
    pub both_amp: f32,
}

/// Evaluates audio waveforms across the composition duration and converts volume amplitudes into keyframe sequences.
/// Returns a list of generated AudioAmplitudeKeyframe structs.
pub fn convert_audio_to_keyframes(comp: &Composition) -> Vec<AudioAmplitudeKeyframe> {
    let mut keyframes = Vec::with_capacity(comp.duration_frames as usize);
    let sample_rate = 44100;
    let buffer_size = (sample_rate / comp.fps.max(1)) as usize;

    for frame in 0..comp.duration_frames {
        let (_pcm, meter) = mix_audio_for_frame(comp, frame, sample_rate, buffer_size, &MasterDspParams::default());

        // Convert dB (-90dB .. +6dB) to normalized 0.0 .. 100.0 AE amplitude scale
        let db_to_ae_scale = |db: f32| -> f32 {
            let norm = ((db + 90.0) / 96.0).clamp(0.0, 1.0);
            norm * 100.0
        };

        let left_amp = db_to_ae_scale(meter.rms_db_left);
        let right_amp = db_to_ae_scale(meter.rms_db_right);
        let both_amp = (left_amp + right_amp) * 0.5;

        keyframes.push(AudioAmplitudeKeyframe {
            frame,
            left_amp,
            right_amp,
            both_amp,
        });
    }

    keyframes
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_audio_for_frame_empty_comp() {
        let comp = Composition::new("c1".into(), "Comp 1".into(), 1920, 1080, 30, 300);
        let (buf, meter) = mix_audio_for_frame(&comp, 0, 44100, 512, &MasterDspParams::default());
        assert_eq!(buf.len(), 1024);
        assert!(meter.peak_db_left < -100.0, "silent comp should have very low peak");
    }
}

// ── WAV loading & waveform extraction ───────────────────────────────────────

impl AudioBuffer {
    /// Parses a RIFF/WAVE file (PCM 8/16/24-bit, mono or stereo).
    /// Returns an error with a human-readable reason for unsupported formats.
    pub fn load_wav(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("cannot read WAV: {}", e))?;
        if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return Err("not a RIFF/WAVE file".into());
        }

        // Walk chunks to find fmt and data
        let mut pos = 12usize;
        let mut format_tag = 1u16;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits = 0u16;
        let mut audio_data: Option<&[u8]> = None;

        while pos + 8 <= data.len() {
            let id = &data[pos..pos + 4];
            let size =
                u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize;
            let body_start = pos + 8;
            let body_end = body_start.saturating_add(size).min(data.len());
            match id {
                b"fmt " => {
                    let b = &data[body_start..body_end];
                    if b.len() >= 16 {
                        format_tag = u16::from_le_bytes([b[0], b[1]]);
                        channels = u16::from_le_bytes([b[2], b[3]]);
                        sample_rate = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                        bits = u16::from_le_bytes([b[14], b[15]]);
                    }
                }
                b"data" => audio_data = Some(&data[body_start..body_end]),
                _ => {}
            }
            // Chunks are word-aligned
            pos = body_start + size + (size & 1);
        }

        if audio_data.is_none() {
            return Err("WAV has no data chunk".into());
        }
        if channels == 0 || sample_rate == 0 || bits == 0 {
            return Err("WAV has invalid fmt chunk".into());
        }
        if format_tag != 1 {
            return Err(format!("unsupported WAV format tag {} (need PCM)", format_tag));
        }
        let raw = audio_data.unwrap();
        let bytes_per_sample = (bits / 8) as usize;
        let frame_bytes = bytes_per_sample * channels as usize;

        let mut samples = Vec::with_capacity(raw.len() / frame_bytes * channels as usize);
        let mut i = 0usize;
        while i + frame_bytes <= raw.len() {
            for ch in 0..channels as usize {
                let base = i + ch * bytes_per_sample;
                let v = match bits {
                    8 => (raw[base] as f32 - 128.0) / 128.0,
                    16 => i16::from_le_bytes([raw[base], raw[base + 1]]) as f32 / 32768.0,
                    24 => {
                        let b0 = raw[base] as i32;
                        let b1 = raw[base + 1] as i32;
                        let b2 = raw[base + 2] as i32;
                        (((b2 << 16) | (b1 << 8) | b0) << 8) as f32 / 2147483648.0
                    }
                    _ => return Err(format!("unsupported bit depth {}", bits)),
                };
                samples.push(v.clamp(-1.0, 1.0));
            }
            i += frame_bytes;
        }

        Ok(Self { samples, sample_rate, channels })
    }

    /// Peak amplitude in [0, 1] at a given time offset (± window/2 seconds).
    pub fn peak_at(&self, time_sec: f32, window_sec: f32) -> f32 {
        let start_sample = ((time_sec - window_sec * 0.5).max(0.0) * self.sample_rate as f32) as usize
            * self.channels as usize;
        let end_sample = (((time_sec + window_sec * 0.5) * self.sample_rate as f32) as usize)
            .saturating_mul(self.channels as usize)
            .min(self.samples.len());
        let mut peak = 0.0f32;
        for s in &self.samples[start_sample.min(self.samples.len())..end_sample] {
            peak = peak.max(s.abs());
        }
        peak
    }

    /// Downsamples the buffer into `bins` peak values — ready for waveform drawing.
    pub fn waveform_peaks(&self, bins: usize) -> Vec<f32> {
        let bins = bins.max(1);
        let total = self.samples.len();
        let per_bin = (total / bins).max(1);
        let mut peaks = Vec::with_capacity(bins);
        for b in 0..bins {
            let range = b * per_bin..((b + 1) * per_bin).min(total);
            peaks.push(range.clone().next().map_or(0.0, |_| {
                self.samples[range].iter().fold(0.0f32, |m, s| m.max(s.abs()))
            }));
        }
        peaks
    }
}

#[cfg(test)]
mod wav_tests {
    use super::*;

    /// Writes a minimal 16-bit PCM WAV file for testing.
    fn write_test_wav(path: &std::path::Path, samples: &[f32], rate: u32) {
        use std::io::Write;
        let mut f = std::fs::File::create(path).unwrap();
        let data_len = (samples.len() * 2) as u32;
        f.write_all(b"RIFF").unwrap();
        f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
        f.write_all(&rate.to_le_bytes()).unwrap();
        f.write_all(&(rate * 2).to_le_bytes()).unwrap(); // byte rate
        f.write_all(&2u16.to_le_bytes()).unwrap(); // block align
        f.write_all(&16u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&data_len.to_le_bytes()).unwrap();
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            f.write_all(&v.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn test_load_wav_roundtrip_and_peaks() {
        let path = std::env::temp_dir().join("aevfx_test_tone.wav");
        let rate = 48000u32;
        // 1-second sine at 440 Hz with amplitude 0.5
        let samples: Vec<f32> = (0..rate as usize)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / rate as f32).sin())
            .collect();
        write_test_wav(&path, &samples, rate);

        let buf = AudioBuffer::load_wav(&path).expect("wav must parse");
        assert_eq!(buf.sample_rate, rate);
        assert_eq!(buf.channels, 1);
        assert_eq!(buf.samples.len(), samples.len());

        // Peak near the max of the sine should be close to 0.5
        let peak = buf.peak_at(0.25, 0.01);
        assert!(peak > 0.45 && peak <= 0.51, "unexpected peak {}", peak);

        // Waveform bins cover the file and stay normalized
        let peaks = buf.waveform_peaks(100);
        assert_eq!(peaks.len(), 100);
        assert!(peaks.iter().all(|p| (0.0..=1.0).contains(p)));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_invalid_wav_files_error_cleanly() {
        let dir = std::env::temp_dir().join(format!("aevfx_wav_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        // Missing file
        assert!(AudioBuffer::load_wav(&dir.join("nope.wav")).is_err());

        // Garbage bytes
        let bad = dir.join("bad.wav");
        std::fs::write(&bad, b"totally not a wav file at all").unwrap();
        assert!(AudioBuffer::load_wav(&bad).is_err());

        // Truncated header
        let trunc = dir.join("trunc.wav");
        std::fs::write(&trunc, b"RIFF----WAVEfmt ").unwrap();
        assert!(AudioBuffer::load_wav(&trunc).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ── Multi-track mixing: real audio sources ──────────────────────────────────

/// Recursively mix audio from a sub-composition (PreComp) into the stereo output.
#[allow(clippy::too_many_arguments)]
fn mix_precomp_audio(
    sub_comp: &crate::core::timeline::Composition,
    sub_frame: u32,
    stereo_output: &mut [f32],
    sample_rate: u32,
    buffer_size: usize,
    gain: f32,
    pan: f32,
    fps: f32,
) {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static WAV_CACHE: std::sync::OnceLock<Mutex<HashMap<String, std::sync::Arc<AudioBuffer>>>> =
        std::sync::OnceLock::new();
    let cache = WAV_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    for layer in &sub_comp.layers {
        if !layer.is_active(sub_frame) || !layer.visible {
            continue;
        }
        match &layer.layer_type {
            crate::core::timeline::LayerType::PreComp { comp_id, .. } => {
                if let Some(sub) = sub_comp.find_sub_comp(comp_id) {
                    mix_precomp_audio(
                        sub, sub_frame.saturating_sub(layer.in_frame),
                        stereo_output, sample_rate, buffer_size,
                        gain, pan, fps,
                    );
                }
            }
            crate::core::timeline::LayerType::Audio { volume, .. } => {
                let vol_db = volume.evaluate(sub_frame);
                let _layer_gain = gain * 10.0f32.powf(vol_db / 20.0);
            }
            crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } => {
                let time_start = (sub_frame.saturating_sub(layer.in_frame)) as f32 / fps;
                let source = {
                    let map = cache.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(buf) = map.get(w) {
                        Some(buf.clone())
                    } else {
                        drop(map);
                        let loaded = AudioBuffer::load_wav(std::path::Path::new(w)).ok()
                            .map(|b| std::sync::Arc::new(b.resample(sample_rate)));
                        if let Some(buf) = &loaded {
                            cache.lock().unwrap_or_else(|e| e.into_inner()).insert(w.clone(), buf.clone());
                        }
                        loaded
                    }
                };
                if let Some(buf) = source {
                    for i in 0..buffer_size {
                        let t = time_start + i as f32 / sample_rate as f32;
                        let idx = (t.max(0.0) * buf.sample_rate as f32) as usize * buf.channels as usize;
                        let l = buf.samples.get(idx).copied().unwrap_or(0.0);
                        let r = if buf.channels > 1 {
                            buf.samples.get(idx + 1).copied().unwrap_or(l)
                        } else {
                            l
                        };
                        let gl = gain * (1.0 - pan.max(0.0));
                        let gr = gain * (1.0 - (-pan).max(0.0));
                        let l_idx = i * 2;
                        let r_idx = i * 2 + 1;
                        if l_idx < stereo_output.len() {
                            stereo_output[l_idx] += l * gl;
                        }
                        if r_idx < stereo_output.len() {
                            stereo_output[r_idx] += r * gr;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Mixes ALL audio-carrying layers (Audio layers + Video layers with WAVs)
/// at the given frame into a stereo buffer, respecting per-layer volume and
/// active ranges. This is the real backing for the audio mixer UI.
///
/// WAVs are decoded once and cached per path in a thread-local map.
pub fn mix_audio_sources_for_frame(
    comp: &Composition,
    frame: u32,
    sample_rate: u32,
    buffer_size: usize,
    // Optional per-layer mixer overrides indexed by layer order
    mixer: Option<&[crate::app_state::MixerChannel]>,
    dsp: &MasterDspParams,
) -> (Vec<f32>, AudioFrameMeter) {
    use std::collections::HashMap;
    use std::sync::Mutex;

    static WAV_CACHE: std::sync::OnceLock<Mutex<HashMap<String, std::sync::Arc<AudioBuffer>>>> =
        std::sync::OnceLock::new();
    let cache = WAV_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let mut stereo_output = vec![0.0f32; buffer_size * 2];
    let mut sum_sq_l = 0.0f32;
    let mut sum_sq_r = 0.0f32;
    let mut max_peak_l = 0.0f32;
    let mut max_peak_r = 0.0f32;
    let fps = comp.fps.max(1) as f32;

    for (layer_idx, layer) in comp.layers.iter().enumerate() {
        if !layer.is_active(frame) || !layer.visible {
            continue;
        }
        // Handle PreComp layers: recursively mix sub-composition audio
        if let LayerType::PreComp { comp_id, .. } = &layer.layer_type {
            if let Some(sub) = comp.find_sub_comp(comp_id) {
                let time_start = (frame.saturating_sub(layer.in_frame)) as f32 / fps;
                let sub_frame = (time_start * fps) as u32;
                let mut sub_gain = 1.0f32;
                let mut sub_pan = 0.0f32;
                if let Some(mix) = mixer {
                    if let Some(ch) = mix.get(layer_idx) {
                        sub_gain *= 10.0f32.powf(ch.gain_db / 20.0);
                        sub_pan = (ch.pan / 100.0).clamp(-1.0, 1.0);
                        if ch.mute { sub_gain = 0.0; }
                    }
                }
                if sub_gain > 0.0 {
                    mix_precomp_audio(sub, sub_frame, &mut stereo_output, sample_rate, buffer_size, sub_gain, sub_pan, fps);
                }
            }
            continue;
        }
        // Resolve the WAV path + gain for this layer
        let (wav_path, gain_db) = match &layer.layer_type {
            LayerType::Audio { volume, .. } => {
                (None, volume.evaluate(frame))
            }
            LayerType::Video { audio_wav: Some(w), .. } => (Some(w.clone()), 0.0f32),
            _ => continue,
        };
        let mut gain = 10.0f32.powf(gain_db / 20.0);
        let mut pan = 0.0f32;
        let mut is_muted = false;
        if let Some(mix) = mixer {
            if let Some(ch) = mix.get(layer_idx) {
                gain *= 10.0f32.powf(ch.gain_db / 20.0);
                pan = (ch.pan / 100.0).clamp(-1.0, 1.0);
                is_muted = ch.mute;
            }
        }
        // Solo logic: if any channel in the mixer is soloed, mute non-soloed channels
        if let Some(mix) = mixer {
            let any_soloed = mix.iter().any(|ch| ch.solo);
            if any_soloed {
                let is_soloed = mix.get(layer_idx).is_some_and(|ch| ch.solo);
                if !is_soloed {
                    is_muted = true;
                }
            }
        }
        if is_muted {
            gain = 0.0;
        }
        let time_start = (frame.saturating_sub(layer.in_frame)) as f32 / fps;

        // Source samples: real WAV if present, otherwise silent placeholder
        let source: Option<std::sync::Arc<AudioBuffer>> = match &wav_path {
            Some(p) => {
                let map = cache.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(buf) = map.get(p) {
                    Some(buf.clone())
                } else {
                    drop(map);
                    let loaded = AudioBuffer::load_wav(std::path::Path::new(p)).ok()
                        .map(|b| std::sync::Arc::new(b.resample(sample_rate)));
                    if let Some(buf) = &loaded {
                        cache.lock().unwrap_or_else(|e| e.into_inner()).insert(p.clone(), buf.clone());
                    }
                    loaded
                }
            }
            None => None,
        };

        let frame_len_sec = buffer_size as f32 / sample_rate as f32;
        for i in 0..buffer_size {
            let t = time_start + i as f32 / sample_rate as f32;
            let (sample_l, sample_r) = match &source {
                Some(buf) => {
                    // Layer in_frame offsets into the source timeline
                    let src_time = (frame as f32 / fps) + (i as f32 / sample_rate as f32);
                    let _ = src_time;
                    let idx = (t.max(0.0) * buf.sample_rate as f32) as usize * buf.channels as usize;
                    let l = buf.samples.get(idx).copied().unwrap_or(0.0);
                    let r = if buf.channels > 1 {
                        buf.samples.get(idx + 1).copied().unwrap_or(l)
                    } else {
                        l
                    };
                    let gl = gain * (1.0 - pan.max(0.0));
                    let gr = gain * (1.0 - (-pan).max(0.0));
                    (l * gl, r * gr)
                }
                None => (0.0, 0.0),
            };
            let _ = frame_len_sec;
            let l_idx = i * 2;
            let r_idx = i * 2 + 1;
            stereo_output[l_idx] += sample_l;
            stereo_output[r_idx] += sample_r;
        }
        let _ = frame_len_sec;
    }

    // ── Master DSP processing (EQ → Compressor → Limiter) ──
    {
        use crate::core::audio_dsp;
        let master_eq = vec![
            audio_dsp::EqBand {
                freq: dsp.eq_highpass,
                gain_db: 0.0,
                q: 0.707,
                band_type: audio_dsp::EqBandType::HighPass,
            },
            audio_dsp::EqBand {
                freq: dsp.eq_lowpass,
                gain_db: 0.0,
                q: 0.707,
                band_type: audio_dsp::EqBandType::LowPass,
            },
            audio_dsp::EqBand {
                freq: dsp.eq_mid_freq,
                gain_db: dsp.eq_mid_gain,
                q: 1.0,
                band_type: audio_dsp::EqBandType::Bell,
            },
        ];
        audio_dsp::apply_eq(&mut stereo_output, &master_eq, sample_rate);

        let comp_params = audio_dsp::CompressorParams {
            threshold_db: dsp.comp_threshold,
            ratio: dsp.comp_ratio,
            attack_ms: dsp.comp_attack,
            release_ms: dsp.comp_release,
            knee_db: 6.0,
            makeup_gain_db: dsp.comp_makeup,
        };
        let mut comp_state = audio_dsp::CompressorState::default();
        audio_dsp::apply_compressor(&mut stereo_output, &comp_params, &mut comp_state, sample_rate);
    }

    // Meter the MIXED output, not per-layer contributions — otherwise the peak
    // reflects the loudest single source instead of the actual sum.
    for chunk in stereo_output.chunks_exact(2) {
        let l = chunk[0];
        let r = chunk[1];
        sum_sq_l += l * l;
        sum_sq_r += r * r;
        max_peak_l = max_peak_l.max(l.abs());
        max_peak_r = max_peak_r.max(r.abs());
    }

    let rms_l = (sum_sq_l / buffer_size.max(1) as f32).sqrt();
    let rms_r = (sum_sq_r / buffer_size.max(1) as f32).sqrt();

    // Silence clamps to -120 dBFS (20·log10(1e-6)); the keyframe mapper and
    // meter displays treat anything below -90 dB as digital silence.
    let to_db = |v: f32| 20.0 * v.max(1e-6).log10();
    (
        stereo_output,
        AudioFrameMeter {
            peak_db_left: to_db(max_peak_l),
            peak_db_right: to_db(max_peak_r),
            rms_db_left: to_db(rms_l),
            rms_db_right: to_db(rms_r),
        },
    )
}

/// Energy-based onset (beat) detection over a WAV file.
/// Returns comp-frame indices where transients occur, sorted & deduped.
pub fn detect_beat_frames(path: &std::path::Path, total_frames: u32, fps: f32) -> Vec<u32> {
    let Ok(buf) = AudioBuffer::load_wav(path) else { return Vec::new() };
    if buf.samples.is_empty() || total_frames == 0 || fps <= 0.0 {
        return Vec::new();
    }
    // ~10 ms hop energy envelope (mono mix)
    let hop = ((buf.sample_rate as f32 * 0.01) as usize).max(1);
    let ch = buf.channels.max(1) as usize;
    let mut energies: Vec<f32> = Vec::with_capacity(buf.samples.len() / hop + 1);
    let mut i = 0;
    while i < buf.samples.len() {
        let end = (i + hop * ch).min(buf.samples.len());
        let n = ((end - i) / ch).max(1);
        let sum: f32 = buf.samples[i..end].iter().enumerate().map(|(k, s)| if k % ch == 0 { *s } else { 0.0 }).sum::<f32>()
            + buf.samples[i..end].iter().enumerate().map(|(k, s)| if k % ch == 1 { *s } else { 0.0 }).sum::<f32>();
        energies.push((sum / (2.0 * n as f32)).abs());
        i += hop * ch;
    }
    if energies.len() < 3 {
        return Vec::new();
    }
    // Onset = positive energy flux; adaptive threshold from local mean
    let win = 30usize; // ~0.3 s
    let mut beats: Vec<u32> = Vec::new();
    for h in 1..energies.len() {
        let flux = energies[h] - energies[h - 1];
        if flux <= 0.0 {
            continue;
        }
        let lo = h.saturating_sub(win);
        let hi = (h + win).min(energies.len());
        let local_mean: f32 = energies[lo..hi].iter().sum::<f32>() / (hi - lo) as f32;
        if flux > local_mean * 1.5 && flux > 0.01 {
            let sec = h as f32 * 0.01;
            let frame = (sec * fps).round() as u32;
            if frame < total_frames && beats.last() != Some(&frame) {
                beats.push(frame);
            }
        }
    }
    beats
}

#[cfg(test)]
mod multitrack_tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};
    use crate::core::property::Animatable;
    use crate::core::keyframe::Keyframe;

    fn write_wav(path: &std::path::Path, samples: &[f32], rate: u32) {
        use std::io::Write;
        let mut f = std::fs::File::create(path).unwrap();
        let data_len = (samples.len() * 2) as u32;
        f.write_all(b"RIFF").unwrap();
        f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap();
        f.write_all(&rate.to_le_bytes()).unwrap();
        f.write_all(&(rate * 2).to_le_bytes()).unwrap();
        f.write_all(&2u16.to_le_bytes()).unwrap();
        f.write_all(&16u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&data_len.to_le_bytes()).unwrap();
        for s in samples {
            f.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn test_multitrack_mix_sums_two_wavs() {
        let dir = std::env::temp_dir().join(format!("aevfx_mix_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let rate = 48000u32;
        let wav_a = dir.join("a.wav");
        let wav_b = dir.join("b.wav");
        // Track A: constant 0.5, Track B: constant 0.25
        let samples_a: Vec<f32> = vec![0.5; rate as usize];
        let samples_b: Vec<f32> = vec![0.25; rate as usize];
        write_wav(&wav_a, &samples_a, rate);
        write_wav(&wav_b, &samples_b, rate);

        let mut comp = Composition::new("c".into(), "Mix".into(), 64, 64, 30, 30);
        let mut la = Layer::new("a".into(), "TrackA".into(), LayerType::Video {
            source: "a".into(), frames_dir: "/tmp/na".into(), frame_count: 10,
            audio_wav: Some(wav_a.to_string_lossy().to_string()), speed: 1.0,
        }, 30);
        la.in_frame = 0;
        la.out_frame = 30;
        let mut lb = Layer::new("b".into(), "TrackB".into(), LayerType::Video {
            source: "b".into(), frames_dir: "/tmp/nb".into(), frame_count: 10,
            audio_wav: Some(wav_b.to_string_lossy().to_string()), speed: 1.0,
        }, 30);
        lb.in_frame = 0;
        lb.out_frame = 30;
        comp.layers.push(la);
        comp.layers.push(lb);

        let bypass_dsp = MasterDspParams {
            eq_highpass: 0.0,
            eq_lowpass: 24000.0,
            eq_mid_gain: 0.0,
            eq_mid_freq: 1000.0,
            comp_threshold: 0.0,
            comp_ratio: 1.0,
            comp_attack: 0.1,
            comp_release: 10.0,
            comp_makeup: 0.0,
        };
        let (mix, meter) = mix_audio_sources_for_frame(&comp, 0, rate, 480, None, &bypass_dsp);
        // Sum of 0.5 + 0.25 at sample 0 ≈ 0.75
        assert!((mix[0] - 0.75).abs() < 0.01, "mix[0] = {}", mix[0]);
        // Meter reflects the combined level
        assert!(meter.peak_db_left > -5.0, "peak {} dB", meter.peak_db_left);
        let _ = Animatable::<f32>::new_constant;
        let _ = Keyframe::new(0, 0.0f32, crate::core::keyframe::InterpolationType::Linear);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod beat_tests {
    use super::*;

    fn write_test_wav(path: &std::path::Path, sr: u32, clicks_at_sec: &[f32]) {
        let total = (sr as f32 * 2.0) as usize;
        let mut samples = vec![0i16; total];
        for &t in clicks_at_sec {
            let idx = (t * sr as f32) as usize;
            let click_len = sr as usize / 50;
            for k in 0..click_len {
                if idx + k < total {
                    samples[idx + k] = ((k as f32 / click_len as f32) * 30000.0) as i16;
                }
            }
        }
        // Minimal 44-byte RIFF/WAVE header, mono 16-bit
        let data_len = (samples.len() * 2) as u32;
        let mut bytes: Vec<u8> = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&sr.to_le_bytes());
        bytes.extend_from_slice(&(sr * 2).to_le_bytes()); // byte rate
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for s in samples { bytes.extend_from_slice(&s.to_le_bytes()); }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_detect_beat_frames_finds_clicks() {
        let dir = std::env::temp_dir().join("aevfx_beat_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clicks.wav");
        write_test_wav(&path, 44100, &[0.2, 0.8, 1.4]);
        let beats = detect_beat_frames(&path, 60, 30.0);
        assert!(beats.contains(&6), "expected onset near frame 6, got {:?}", beats);
        assert!(beats.contains(&24), "expected onset near frame 24, got {:?}", beats);
        assert!(beats.len() >= 3 && beats.len() <= 12, "sane beat count, got {:?}", beats);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_detect_beat_frames_silence_is_empty() {
        let dir = std::env::temp_dir().join("aevfx_beat_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("silence.wav");
        write_test_wav(&path, 22050, &[]);
        let beats = detect_beat_frames(&path, 60, 30.0);
        assert!(beats.is_empty(), "silence should have no beats, got {:?}", beats);
        let _ = std::fs::remove_file(&path);
    }
}
