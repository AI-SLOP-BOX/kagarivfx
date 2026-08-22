#![allow(dead_code)]
//! Real FFT-driven Audio Spectrum / Audio Waveform analysis matching the
//! After Effects "Audio Spectrum" and "Audio Waveform" effects.
//!
//! Pipeline: PCM → Hann window → radix-2 FFT → log-spaced band energies →
//! dB normalization → temporal smoothing + peak hold → RGBA renderer.

use crate::core::fft::magnitude_spectrum;

/// Display mode of the spectrum generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSpectrumType {
    DigitalBands,
    AnalogLines,
    AnalogDots,
}

/// Options for Audio Spectrum generation.
#[derive(Debug, Clone)]
pub struct AudioSpectrumOptions {
    pub start_frequency: f32, // Hz (e.g. 20.0)
    pub end_frequency: f32,   // Hz (e.g. 15000.0)
    pub frequency_bands: u32, // Number of bands (e.g. 64)
    pub max_height: f32,      // Max bar height in pixels
    pub spectrum_type: AudioSpectrumType,
    /// FFT window size in samples (power of two; clamped 256..8192).
    pub fft_size: usize,
    /// dB level mapped to zero amplitude (e.g. -60).
    pub db_floor: f32,
    /// Release smoothing 0..1 per analysis frame (higher = snappier fall).
    pub release: f32,
    /// Peak-hold decay per analysis frame (0..1 fraction of remaining hold).
    pub peak_decay: f32,
}

impl Default for AudioSpectrumOptions {
    fn default() -> Self {
        Self {
            start_frequency: 20.0,
            end_frequency: 15000.0,
            frequency_bands: 64,
            max_height: 100.0,
            spectrum_type: AudioSpectrumType::DigitalBands,
            fft_size: 2048,
            db_floor: -60.0,
            release: 0.25,
            peak_decay: 0.02,
        }
    }
}

/// Stateful spectrum analyzer with smoothing and peak hold.
#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    bands: Vec<f32>,
    peaks: Vec<f32>,
}

impl Default for SpectrumAnalyzer {
    fn default() -> Self {
        Self::new(64)
    }
}

impl SpectrumAnalyzer {
    pub fn new(num_bands: u32) -> Self {
        let n = num_bands.max(1) as usize;
        Self { bands: vec![0.0; n], peaks: vec![0.0; n] }
    }

    /// Analyze a window of mono PCM and return normalized band amplitudes
    /// in 0..1 (already smoothed against previous state).
    pub fn analyze(&mut self, pcm: &[f32], sample_rate: u32, options: &AudioSpectrumOptions) -> Vec<f32> {
        let num_bands = options.frequency_bands.max(1) as usize;
        if self.bands.len() != num_bands {
            self.bands = vec![0.0; num_bands];
            self.peaks = vec![0.0; num_bands];
        }
        if pcm.is_empty() || sample_rate == 0 {
            return self.bands.clone();
        }

        let win = options.fft_size.clamp(256, 8192);
        let take = pcm.len().min(win);
        let window = &pcm[pcm.len() - take..]; // most recent audio

        // Hann windowing is applied here (the fft kernel itself is window-free).
        let wlen = window.len();
        let windowed: Vec<f32> = window
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / wlen as f32).cos());
                s * w
            })
            .collect();

        // Returns `win` bins; the first win/2 carry the unique real spectrum.
        let mags = magnitude_spectrum(&windowed, win);
        if mags.is_empty() {
            return self.bands.clone();
        }
        let usable_bins = (win / 2).max(1);
        let bin_hz = sample_rate as f32 / win as f32;
        if bin_hz <= 0.0 {
            return self.bands.clone();
        }

        let lo = options.start_frequency.max(1.0);
        let hi = options.end_frequency.max(lo + 1.0);
        let floor_db = options.db_floor.min(-1.0);

        for i in 0..num_bands {
            // Log-spaced band edges.
            let t0 = i as f32 / num_bands as f32;
            let t1 = (i + 1) as f32 / num_bands as f32;
            let f0 = lo * (hi / lo).powf(t0);
            let f1 = lo * (hi / lo).powf(t1);
            let b0 = (f0 / bin_hz).floor() as usize;
            let b1 = ((f1 / bin_hz).ceil() as usize).clamp(b0 + 1, usable_bins);
            let energy: f32 = mags[b0..b1].iter().copied().fold(0.0f32, f32::max);

            // dB normalize into 0..1.
            let db = 20.0 * energy.max(1e-6).log10();
            let norm = ((db - floor_db) / -floor_db).clamp(0.0, 1.0);

            // Attack instant, release smoothed.
            let prev = self.bands[i];
            let rel = options.release.clamp(0.0, 1.0);
            let value = if norm > prev { norm } else { prev + (norm - prev) * rel };
            self.bands[i] = value;

            // Peak hold with decay.
            let decay = options.peak_decay.clamp(0.0, 1.0);
            self.peaks[i] = (self.peaks[i] * (1.0 - decay)).max(value);
        }
        self.bands.clone()
    }

    /// Current held peak amplitudes (0..1), same length as bands.
    pub fn peaks(&self) -> &[f32] {
        &self.peaks
    }
}

/// Compute frequency band amplitudes from raw PCM (stateless convenience
/// wrapper kept for backward compatibility with earlier call sites).
pub fn generate_audio_spectrum_bands(
    pcm_samples: &[f32],
    sample_rate: u32,
    options: &AudioSpectrumOptions,
) -> Vec<f32> {
    let mut analyzer = SpectrumAnalyzer::new(options.frequency_bands);
    analyzer
        .analyze(pcm_samples, sample_rate, options)
        .into_iter()
        .map(|v| v * options.max_height)
        .collect()
}

/// Peak-envelope waveform points for the "Audio Waveform" display.
/// Each point is the max absolute amplitude of its chunk (0..1).
pub fn extract_waveform(pcm: &[f32], num_points: u32) -> Vec<f32> {
    let n = num_points.max(1) as usize;
    if pcm.is_empty() {
        return vec![0.0; n];
    }
    let chunk = pcm.len().div_ceil(n);
    (0..n)
        .map(|i| {
            let start = i * chunk;
            if start >= pcm.len() {
                return 0.0;
            }
            let end = (start + chunk).min(pcm.len());
            pcm[start..end].iter().fold(0.0f32, |a, &s| a.max(s.abs())).min(1.0)
        })
        .collect()
}

/// Render band amplitudes (0..1) into an RGBA buffer using the selected
/// display style. Colors interpolate from `color_low` to `color_high` by
/// amplitude.
pub fn render_spectrum(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    bands: &[f32],
    options: &AudioSpectrumOptions,
    color_low: [u8; 3],
    color_high: [u8; 3],
) {
    if width == 0 || height == 0 || buffer.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let n = bands.len().max(1);
    let baseline = height as f32 - 2.0;
    let max_h = options.max_height.min(height as f32 - 4.0).max(1.0);

    let color_at = |t: f32| -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        [
            (color_low[0] as f32 + (color_high[0] - color_low[0]) as f32 * t) as u8,
            (color_low[1] as f32 + (color_high[1] - color_low[1]) as f32 * t) as u8,
            (color_low[2] as f32 + (color_high[2] - color_low[2]) as f32 * t) as u8,
        ]
    };

    let put_px = |buffer: &mut [u8], x: u32, y: u32, c: [u8; 3]| {
        if x >= width || y >= height {
            return;
        }
        let idx = ((y * width + x) * 4) as usize;
        if idx + 3 < buffer.len() {
            buffer[idx] = c[0];
            buffer[idx + 1] = c[1];
            buffer[idx + 2] = c[2];
            buffer[idx + 3] = 255;
        }
    };

    match options.spectrum_type {
        AudioSpectrumType::DigitalBands => {
            let slot = width as f32 / n as f32;
            let bar_w = (slot * 0.7).max(1.0).floor() as u32;
            for (i, &b) in bands.iter().enumerate() {
                let h = (b * max_h).round();
                let x0 = (i as f32 * slot + slot * 0.15) as u32;
                let c = color_at(b);
                for dy in 0..h as u32 {
                    let y = (baseline - dy as f32) as u32;
                    for dx in 0..bar_w {
                        put_px(buffer, x0 + dx, y, c);
                    }
                }
            }
        }
        AudioSpectrumType::AnalogLines => {
            // Connected polyline across band tops.
            let step_x = (width as f32 - 1.0) / (n - 1).max(1) as f32;
            for i in 0..n.saturating_sub(1) {
                let h0 = bands[i] * max_h;
                let h1 = bands[i + 1] * max_h;
                let x0 = i as f32 * step_x;
                let x1 = (i + 1) as f32 * step_x;
                let steps = (x1 - x0).ceil().max(1.0) as u32;
                for s in 0..=steps {
                    let t = s as f32 / steps as f32;
                    let x = (x0 + (x1 - x0) * t) as u32;
                    let y = (baseline - h0 + (h0 - h1) * t) as u32;
                    put_px(buffer, x, y, color_at((h0 + (h1 - h0) * t) / max_h));
                    put_px(buffer, x, y + 1, color_at((h0 + (h1 - h0) * t) / max_h));
                }
            }
        }
        AudioSpectrumType::AnalogDots => {
            let slot = width as f32 / n as f32;
            for (i, &b) in bands.iter().enumerate() {
                let cx = (i as f32 * slot + slot * 0.5) as u32;
                let cy = (baseline - b * max_h) as u32;
                let r = (slot * 0.35).clamp(1.0, 6.0);
                let ri = r.ceil() as i32;
                let c = color_at(b);
                for dy in -ri..=ri {
                    for dx in -ri..=ri {
                        if (dx * dx + dy * dy) as f32 <= r * r {
                            put_px(
                                buffer,
                                (cx as i32 + dx).max(0) as u32,
                                (cy as i32 + dy).max(0) as u32,
                                c,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
        let n = (seconds * sr as f32) as usize;
        (0..n)
            .map(|i| (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin() * amp)
            .collect()
    }

    #[test]
    fn test_analyzer_responds_to_tone_energy() {
        let opts = AudioSpectrumOptions::default();
        let mut analyzer = SpectrumAnalyzer::new(opts.frequency_bands);
        let loud = analyzer.analyze(&sine(440.0, 0.05, 44100, 0.9), 44100, &opts);
        assert_eq!(loud.len(), 64);
        let loud_sum: f32 = loud.iter().sum();

        let mut quiet_analyzer = SpectrumAnalyzer::new(opts.frequency_bands);
        let quiet = quiet_analyzer.analyze(&sine(440.0, 0.05, 44100, 0.05), 44100, &opts);
        let quiet_sum: f32 = quiet.iter().sum();
        assert!(loud_sum > quiet_sum * 2.0, "louder tone must yield larger bands");
    }

    #[test]
    fn test_analyzer_silence_is_zero() {
        let opts = AudioSpectrumOptions::default();
        let mut analyzer = SpectrumAnalyzer::new(32);
        let bands = analyzer.analyze(&vec![0.0f32; 2048], 44100, &opts);
        assert!(bands.iter().all(|&b| b.abs() < 1e-5));
        assert!(analyzer.peaks().iter().all(|&p| p.abs() < 1e-5));
    }

    #[test]
    fn test_peak_hold_exceeds_current_band_after_decay_input() {
        let opts = AudioSpectrumOptions::default();
        let mut analyzer = SpectrumAnalyzer::new(16);
        analyzer.analyze(&sine(1000.0, 0.05, 44100, 1.0), 44100, &opts);
        let peak_after_loud = *analyzer.peaks().iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&0.0);
        // Feed silence: bands fall, peaks hold above them.
        for _ in 0..10 {
            analyzer.analyze(&vec![0.0f32; 2048], 44100, &opts);
        }
        let band_max = analyzer.bands.iter().cloned().fold(0.0f32, f32::max);
        let peak_max = analyzer.peaks.iter().cloned().fold(0.0f32, f32::max);
        assert!(peak_after_loud > 0.5);
        assert!(peak_max >= band_max - 1e-4, "peaks must hold above falling bands");
    }

    #[test]
    fn test_legacy_wrapper_returns_pixel_heights() {
        let opts = AudioSpectrumOptions::default();
        let bands = generate_audio_spectrum_bands(&sine(220.0, 0.05, 44100, 0.8), 44100, &opts);
        assert_eq!(bands.len(), 64);
        assert!(bands.iter().all(|&b| b >= 0.0 && b <= opts.max_height + 1e-3));
    }

    #[test]
    fn test_waveform_extraction() {
        let pcm = sine(50.0, 0.1, 8000, 0.5);
        let wf = extract_waveform(&pcm, 20);
        assert_eq!(wf.len(), 20);
        assert!(wf.iter().all(|&v| (0.0..=1.0).contains(&v)));
        assert!(wf.iter().any(|&v| v > 0.3), "tone chunks should carry energy");
        assert_eq!(extract_waveform(&[], 8), vec![0.0; 8]);
    }

    #[test]
    fn test_render_all_modes_draw_pixels() {
        let opts = AudioSpectrumOptions {
            spectrum_type: AudioSpectrumType::DigitalBands,
            ..Default::default()
        };
        let bands = generate_audio_spectrum_bands(&sine(300.0, 0.05, 44100, 0.9), 44100, &opts);
        for mode in [AudioSpectrumType::DigitalBands, AudioSpectrumType::AnalogLines, AudioSpectrumType::AnalogDots] {
            let mut o = opts.clone();
            o.spectrum_type = mode;
            let mut buf = vec![0u8; 200 * 120 * 4];
            render_spectrum(&mut buf, 200, 120, &bands, &o, [0, 40, 90], [90, 220, 255]);
            assert!(
                buf.chunks(4).any(|px| px[3] == 255),
                "{mode:?} drew nothing"
            );
        }
    }

    #[test]
    fn test_render_degenerate_inputs_safe() {
        let opts = AudioSpectrumOptions::default();
        let mut empty: Vec<u8> = vec![];
        render_spectrum(&mut empty, 0, 0, &[], &opts, [0, 0, 0], [255, 255, 255]);
        let mut tiny = vec![0u8; 16];
        render_spectrum(&mut tiny, 2, 2, &[1.0; 8], &opts, [0, 0, 0], [255, 255, 255]);
    }

    #[test]
    fn test_stateful_smoothing_falls_gradually() {
        let opts = AudioSpectrumOptions { release: 0.2, ..Default::default() };
        let mut analyzer = SpectrumAnalyzer::new(8);
        let loud = analyzer.analyze(&sine(500.0, 0.05, 44100, 1.0), 44100, &opts);
        let peak_val = loud.iter().cloned().fold(0.0f32, f32::max);
        let after = analyzer.analyze(&vec![0.0f32; 2048], 44100, &opts);
        let after_val = after.iter().cloned().fold(0.0f32, f32::max);
        assert!(peak_val > 0.5);
        assert!(after_val > 0.01, "release smoothing must not snap to zero instantly");
        assert!(after_val < peak_val, "value must be falling");
    }
}