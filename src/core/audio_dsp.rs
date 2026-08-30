/// Audio DSP effects: Parametric EQ, Compressor, Limiter, and High/Low-pass filters.
/// All processing is done on f32 stereo interleaved buffers.
/// Parametric EQ band: bell, high shelf, low shelf, or pass filter.
#[derive(Debug, Clone)]
pub struct EqBand {
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
    pub band_type: EqBandType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EqBandType {
    Bell,
    HighShelf,
    LowShelf,
    HighPass,
    LowPass,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            freq: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            band_type: EqBandType::Bell,
        }
    }
}

/// Biquad filter state (one second-order section).
#[derive(Debug, Clone)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1_l: f64,
    x2_l: f64,
    y1_l: f64,
    y2_l: f64,
    x1_r: f64,
    x2_r: f64,
    y1_r: f64,
    y2_r: f64,
}

impl Biquad {
    fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1_l: 0.0,
            x2_l: 0.0,
            y1_l: 0.0,
            y2_l: 0.0,
            x1_r: 0.0,
            x2_r: 0.0,
            y1_r: 0.0,
            y2_r: 0.0,
        }
    }

    fn process_stereo(&mut self, buf: &mut [f32]) {
        for sample in buf.chunks_exact_mut(2) {
            let x_l = sample[0] as f64;
            let x_r = sample[1] as f64;

            let y_l = self.b0 * x_l + self.b1 * self.x1_l + self.b2 * self.x2_l
                - self.a1 * self.y1_l
                - self.a2 * self.y2_l;
            let y_r = self.b0 * x_r + self.b1 * self.x1_r + self.b2 * self.x2_r
                - self.a1 * self.y1_r
                - self.a2 * self.y2_r;

            self.x2_l = self.x1_l;
            self.x1_l = x_l;
            self.y2_l = self.y1_l;
            self.y1_l = y_l;

            self.x2_r = self.x1_r;
            self.x1_r = x_r;
            self.y2_r = self.y1_r;
            self.y1_r = y_r;

            sample[0] = y_l as f32;
            sample[1] = y_r as f32;
        }
    }
}

/// Design a biquad filter for the given band type.
fn design_biquad(band: &EqBand, sample_rate: f64) -> Biquad {
    let mut bq = Biquad::new();
    let f0 = band.freq as f64;
    let gain = band.gain_db as f64;
    let q = band.q as f64;
    let fs = sample_rate;

    let a = if gain.abs() > 0.001 {
        10.0_f64.powf(gain / 40.0)
    } else {
        1.0
    };
    let w0 = 2.0 * std::f64::consts::PI * f0 / fs;
    let sin_w0 = w0.sin();
    let cos_w0 = w0.cos();
    let alpha = sin_w0 / (2.0 * q);

    match band.band_type {
        EqBandType::Bell => {
            let a_sqrt = a.sqrt();
            let b0 = 1.0 + alpha * a_sqrt;
            let b1 = -2.0 * cos_w0;
            let b2 = 1.0 - alpha * a_sqrt;
            let a0 = 1.0 + alpha / a_sqrt;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha / a_sqrt;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::HighShelf => {
            let a_sqrt = a.sqrt();
            let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha);
            let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha);
            let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha;
            let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::LowShelf => {
            let a_sqrt = a.sqrt();
            let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha);
            let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha);
            let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha;
            let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::HighPass => {
            let b0 = (1.0 + cos_w0) / 2.0;
            let b1 = -(1.0 + cos_w0);
            let b2 = (1.0 + cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::LowPass => {
            let b0 = (1.0 - cos_w0) / 2.0;
            let b1 = 1.0 - cos_w0;
            let b2 = (1.0 - cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
    }
    bq
}

/// Apply parametric EQ with multiple bands to a stereo interleaved buffer.
pub fn apply_eq(buf: &mut [f32], bands: &[EqBand], sample_rate: u32) {
    if bands.is_empty() || buf.is_empty() || sample_rate == 0 {
        return;
    }
    let fs = sample_rate as f64;
    let nyquist = fs * 0.5;
    let filters: Vec<Biquad> = bands
        .iter()
        .filter(|b| {
            if !b.freq.is_finite() || !b.gain_db.is_finite() || !b.q.is_finite() || b.q <= 0.0 {
                return false;
            }
            // Skip pass filters at extreme frequencies (bypass mode)
            match b.band_type {
                EqBandType::HighPass => b.freq > 1.0 && (b.freq as f64) < nyquist,
                EqBandType::LowPass => b.freq > 100.0 && (b.freq as f64) < nyquist,
                _ => b.gain_db.abs() > 0.01,
            }
        })
        .map(|b| design_biquad(b, fs))
        .collect();

    for mut bq in filters {
        bq.process_stereo(buf);
    }
}

/// Compressor state (keeps running envelope for smooth gain reduction).
#[derive(Debug, Clone)]
pub struct CompressorState {
    envelope: f64,
}

impl Default for CompressorState {
    fn default() -> Self {
        Self { envelope: 0.0 }
    }
}

/// Compressor parameters.
#[derive(Debug, Clone)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_db: f32,
    pub makeup_gain_db: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        }
    }
}

/// Apply compressor to a stereo interleaved buffer.
pub fn apply_compressor(
    buf: &mut [f32],
    params: &CompressorParams,
    state: &mut CompressorState,
    sample_rate: u32,
) {
    if buf.is_empty() {
        return;
    }
    let fs = sample_rate as f64;
    let attack_coeff = (-1.0 / (params.attack_ms as f64 * 0.001 * fs)).exp();
    let release_coeff = (-1.0 / (params.release_ms as f64 * 0.001 * fs)).exp();
    let threshold = params.threshold_db as f64;
    let ratio = params.ratio as f64;
    let knee = params.knee_db as f64;
    let makeup = 10.0_f64.powf(params.makeup_gain_db as f64 / 20.0);

    for sample in buf.chunks_exact_mut(2) {
        let peak_l = sample[0] as f64;
        let peak_r = sample[1] as f64;
        let peak = peak_l.abs().max(peak_r.abs());

        // dB level
        let level_db = if peak > 1e-10 {
            20.0 * peak.log10()
        } else {
            -120.0
        };

        // Gain reduction with soft knee
        let gr_db = if level_db < threshold - knee / 2.0 {
            0.0
        } else if level_db > threshold + knee / 2.0 {
            threshold + (level_db - threshold) / ratio - level_db
        } else {
            // Soft knee region
            let x = level_db - threshold + knee / 2.0;
            (1.0 / ratio - 1.0) * x * x / (2.0 * knee)
        };

        // Smooth envelope
        let coeff = if gr_db < state.envelope {
            attack_coeff
        } else {
            release_coeff
        };
        state.envelope = coeff * state.envelope + (1.0 - coeff) * gr_db;

        let gain = 10.0_f64.powf(state.envelope / 20.0) * makeup;
        sample[0] = (peak_l * gain) as f32;
        sample[1] = (peak_r * gain) as f32;
    }
}

/// Apply a simple limiter (hard-knee compressor with very high ratio).
pub fn apply_limiter(
    buf: &mut [f32],
    ceiling_db: f32,
    release_ms: f32,
    state: &mut CompressorState,
    sample_rate: u32,
) {
    let params = CompressorParams {
        threshold_db: ceiling_db,
        ratio: 20.0,
        attack_ms: 0.1,
        release_ms,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    apply_compressor(buf, &params, state, sample_rate);

    // Hard clip as safety net
    for sample in buf.iter_mut() {
        *sample = sample.clamp(-1.0, 1.0);
    }
}

// ──────────────── Multi-band Audio Keyframe Extraction ────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioExtractBand {
    Master,
    Bass,
    Mid,
    Treble,
}

#[derive(Debug, Clone)]
pub struct AudioKeyframeOptions {
    pub band: AudioExtractBand,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub min_db: f32,
    pub max_db: f32,
    pub multiplier: f32,
}

impl Default for AudioKeyframeOptions {
    fn default() -> Self {
        Self {
            band: AudioExtractBand::Master,
            attack_ms: 10.0,
            release_ms: 100.0,
            min_db: -48.0,
            max_db: 0.0,
            multiplier: 100.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MultiBandAudioKeyframes {
    pub master: Vec<crate::core::keyframe::Keyframe<f32>>,
    pub bass: Vec<crate::core::keyframe::Keyframe<f32>>,
    pub mid: Vec<crate::core::keyframe::Keyframe<f32>>,
    pub treble: Vec<crate::core::keyframe::Keyframe<f32>>,
}

/// Convert an audio buffer into multi-band animated keyframes per frame (AE Parity).
pub fn extract_multiband_audio_keyframes(
    pcm_stereo: &[f32],
    sample_rate: u32,
    fps: u32,
    total_frames: u32,
    options: &AudioKeyframeOptions,
) -> MultiBandAudioKeyframes {
    if pcm_stereo.is_empty() || sample_rate == 0 || fps == 0 || total_frames == 0 {
        return MultiBandAudioKeyframes::default();
    }

    let total_stereo_samples = pcm_stereo.len() / 2;

    // Filter banks for frequency crossover:
    // Bass: LowPass 250Hz
    let mut bq_bass = design_biquad(
        &EqBand {
            freq: 250.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::LowPass,
        },
        sample_rate as f64,
    );
    // Treble: HighPass 4000Hz
    let mut bq_treble = design_biquad(
        &EqBand {
            freq: 4000.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::HighPass,
        },
        sample_rate as f64,
    );
    // Mid: BandPass (LowPass 4000Hz + HighPass 250Hz)
    let mut bq_mid_lp = design_biquad(
        &EqBand {
            freq: 4000.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::LowPass,
        },
        sample_rate as f64,
    );
    let mut bq_mid_hp = design_biquad(
        &EqBand {
            freq: 250.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::HighPass,
        },
        sample_rate as f64,
    );

    // Envelope followers per band
    let mut env_master = 0.0f32;
    let mut env_bass = 0.0f32;
    let mut env_mid = 0.0f32;
    let mut env_treble = 0.0f32;

    let attack_ms = if options.attack_ms.is_finite() {
        options.attack_ms.max(1.0)
    } else {
        10.0
    };
    let release_ms = if options.release_ms.is_finite() {
        options.release_ms.max(1.0)
    } else {
        100.0
    };
    let min_db = if options.min_db.is_finite() {
        options.min_db
    } else {
        -48.0
    };
    let max_db = if options.max_db.is_finite() && options.max_db > min_db {
        options.max_db
    } else {
        0.0
    };
    let multiplier = if options.multiplier.is_finite() {
        options.multiplier.max(0.0)
    } else {
        100.0
    };
    let dt = 1.0f32 / fps.max(1) as f32;
    let att_coef = (-dt / (attack_ms * 0.001)).exp();
    let rel_coef = (-dt / (release_ms * 0.001)).exp();

    let mut res = MultiBandAudioKeyframes::default();
    let db_to_value = |amplitude: f32| {
        let db = 20.0 * amplitude.max(1.0e-8).log10();
        let span = (max_db - min_db).max(f32::EPSILON);
        ((db - min_db) / span).clamp(0.0, 1.0) * multiplier
    };

    for f in 0..total_frames {
        let start_sample_idx =
            ((f as f64 * sample_rate as f64) / fps.max(1) as f64).floor() as usize;
        let mut end_sample_idx =
            (((f as u64 + 1) as f64 * sample_rate as f64) / fps.max(1) as f64).ceil() as usize;
        if end_sample_idx == start_sample_idx {
            end_sample_idx = start_sample_idx.saturating_add(1);
        }
        let start_sample = (start_sample_idx.saturating_mul(2)).min(pcm_stereo.len());
        let end_sample = (end_sample_idx.saturating_mul(2)).min(pcm_stereo.len());

        let frame_buf = if start_sample < pcm_stereo.len() && start_sample < end_sample {
            pcm_stereo[start_sample..end_sample]
                .iter()
                .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
                .collect()
        } else {
            // Beyond audio stream end -> silence (zero-filled, never loops)
            Vec::new()
        };

        if frame_buf.is_empty() {
            let kf = |val: f32| {
                crate::core::keyframe::Keyframe::new(
                    f,
                    val,
                    crate::core::keyframe::InterpolationType::Linear,
                )
            };
            res.master.push(kf(0.0));
            res.bass.push(kf(0.0));
            res.mid.push(kf(0.0));
            res.treble.push(kf(0.0));
            continue;
        }

        // Master RMS
        let mut master_sq = 0.0f64;
        for s in frame_buf.chunks_exact(2) {
            master_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_master = (master_sq / (frame_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Bass filtered RMS
        let mut bass_buf = frame_buf.clone();
        bq_bass.process_stereo(&mut bass_buf);
        let mut bass_sq = 0.0f64;
        for s in bass_buf.chunks_exact(2) {
            bass_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_bass = (bass_sq / (bass_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Mid filtered RMS
        let mut mid_buf = frame_buf.clone();
        bq_mid_lp.process_stereo(&mut mid_buf);
        bq_mid_hp.process_stereo(&mut mid_buf);
        let mut mid_sq = 0.0f64;
        for s in mid_buf.chunks_exact(2) {
            mid_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_mid = (mid_sq / (mid_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Treble filtered RMS
        let mut treble_buf = frame_buf.clone();
        bq_treble.process_stereo(&mut treble_buf);
        let mut treble_sq = 0.0f64;
        for s in treble_buf.chunks_exact(2) {
            treble_sq += (s[0] as f64).powi(2) + (s[1] as f64).powi(2);
        }
        let peak_treble = (treble_sq / (treble_buf.len() as f64 * 0.5).max(1.0)).sqrt() as f32;

        // Update envelope followers
        let update_env = |env: &mut f32, target: f32| {
            if target > *env {
                *env = target + att_coef * (*env - target);
            } else {
                *env = target + rel_coef * (*env - target);
            }
            (*env * multiplier).clamp(0.0, multiplier)
        };

        let v_master =
            db_to_value(update_env(&mut env_master, peak_master) / multiplier.max(1.0));
        let v_bass =
            db_to_value(update_env(&mut env_bass, peak_bass) / multiplier.max(1.0));
        let v_mid = db_to_value(update_env(&mut env_mid, peak_mid) / multiplier.max(1.0));
        let v_treble =
            db_to_value(update_env(&mut env_treble, peak_treble) / multiplier.max(1.0));

        let kf = |val: f32| {
            crate::core::keyframe::Keyframe::new(
                f,
                val,
                crate::core::keyframe::InterpolationType::Linear,
            )
        };

        res.master.push(kf(v_master));
        res.bass.push(kf(v_bass));
        res.mid.push(kf(v_mid));
        res.treble.push(kf(v_treble));
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_lowpass_stability() {
        let bands = vec![EqBand {
            freq: 1000.0,
            gain_db: 0.0,
            q: 0.707,
            band_type: EqBandType::LowPass,
        }];
        let mut buf = vec![0.0f32; 1024];
        // Impulse
        buf[0] = 1.0;
        buf[1] = 1.0;
        apply_eq(&mut buf, &bands, 44100);
        // Should not blow up
        for s in &buf {
            assert!(s.abs() < 10.0, "Low-pass filter went unstable: {}", s);
        }
    }

    #[test]
    fn test_eq_with_invalid_sample_rate_is_a_safe_bypass() {
        let mut buf = vec![0.25f32, -0.5, 0.75, -1.0];
        let original = buf.clone();
        apply_eq(&mut buf, &[EqBand::default()], 0);
        assert_eq!(buf, original);
    }

    #[test]
    fn test_eq_skips_invalid_band_parameters() {
        let mut buf = vec![0.25f32, -0.5, 0.75, -1.0];
        let original = buf.clone();
        apply_eq(
            &mut buf,
            &[
                EqBand {
                    q: 0.0,
                    ..EqBand::default()
                },
                EqBand {
                    freq: f32::NAN,
                    ..EqBand::default()
                },
            ],
            48_000,
        );
        assert_eq!(buf, original);
    }

    #[test]
    fn test_eq_bell_boost() {
        let bands = vec![EqBand {
            freq: 1000.0,
            gain_db: 12.0,
            q: 1.0,
            band_type: EqBandType::Bell,
        }];
        // Sine at 1kHz should be boosted
        let sr = 44100u32;
        let len = 1024usize;
        let mut buf = Vec::with_capacity(len * 2);
        for i in 0..len {
            let t = i as f32 / sr as f32;
            let v = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.5;
            buf.push(v);
            buf.push(v);
        }
        let rms_before: f64 =
            buf.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / (len as f64);
        apply_eq(&mut buf, &bands, sr);
        let rms_after: f64 =
            buf.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / (len as f64);
        // 12dB boost should increase RMS significantly
        assert!(
            rms_after > rms_before * 1.5,
            "Bell boost did not increase level: before={:.4} after={:.4}",
            rms_before,
            rms_after
        );
    }

    #[test]
    fn test_compressor_reduces_dynamic_range() {
        let params = CompressorParams {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        };
        // Test with loud signal: should be compressed below threshold
        let mut state = CompressorState::default();
        let mut buf_loud = vec![0.0f32; 4096];
        for s in buf_loud.chunks_exact_mut(2) {
            s[0] = 0.8;
            s[1] = 0.8;
        }
        let rms_before: f64 = 0.8;
        apply_compressor(&mut buf_loud, &params, &mut state, 44100);
        let rms_after: f64 = buf_loud
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum::<f64>()
            .sqrt()
            / 2048.0;
        // Compressor with 4:1 ratio above -20dB threshold should reduce level
        assert!(
            rms_after < rms_before,
            "Compressor did not reduce loud signal: before={:.4} after={:.4}",
            rms_before,
            rms_after
        );
    }

    #[test]
    fn test_limiter_clips() {
        let mut state = CompressorState::default();
        let mut buf = vec![0.0f32; 1024];
        for s in buf.chunks_exact_mut(2) {
            s[0] = 1.5;
            s[1] = 1.5;
        }
        apply_limiter(&mut buf, -0.1, 50.0, &mut state, 44100);
        for s in &buf {
            assert!(s.abs() <= 1.001, "Limiter failed to clip: {}", s);
        }
    }

    #[test]
    fn test_extract_multiband_audio_keyframes() {
        let sr = 44100u32;
        let fps = 30u32;
        let total_frames = 10u32;
        // Generate a 100Hz bass tone
        let num_samples = (sr as usize / fps as usize) * (total_frames as usize);
        let mut pcm = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sr as f32;
            let v = (2.0 * std::f32::consts::PI * 100.0 * t).sin() * 0.8;
            pcm.push(v);
            pcm.push(v);
        }

        let options = AudioKeyframeOptions::default();
        let kfs = extract_multiband_audio_keyframes(&pcm, sr, fps, total_frames, &options);

        assert_eq!(kfs.master.len(), total_frames as usize);
        assert_eq!(kfs.bass.len(), total_frames as usize);
        assert_eq!(kfs.treble.len(), total_frames as usize);

        // Bass level should be significantly higher than treble level for a 100Hz sine wave
        let mid_frame = (total_frames / 2) as usize;
        let bass_val = kfs.bass[mid_frame].value;
        let treble_val = kfs.treble[mid_frame].value;
        assert!(
            bass_val > treble_val * 3.0,
            "Bass {} must be much higher than Treble {}",
            bass_val,
            treble_val
        );
    }

    #[test]
    fn test_extract_multiband_audio_keyframes_low_sample_rate() {
        // sample_rate (20 Hz) < fps (60 fps)
        let sr = 20u32;
        let fps = 60u32;
        let total_frames = 30u32;
        let pcm = vec![0.8f32; 100]; // 50 stereo samples
        let options = AudioKeyframeOptions::default();
        let kfs = extract_multiband_audio_keyframes(&pcm, sr, fps, total_frames, &options);

        assert_eq!(kfs.master.len(), total_frames as usize);
        // Master values must not be zero/empty
        assert!(kfs.master.iter().any(|kf| kf.value > 0.0));
    }

    #[test]
    fn test_extract_multiband_sanitizes_nonfinite_options_and_pcm() {
        let options = AudioKeyframeOptions {
            attack_ms: f32::NAN,
            release_ms: f32::INFINITY,
            min_db: f32::NAN,
            max_db: f32::NEG_INFINITY,
            multiplier: f32::NAN,
            ..AudioKeyframeOptions::default()
        };
        let pcm = vec![f32::NAN, f32::INFINITY, 0.5, -0.5];
        let result = extract_multiband_audio_keyframes(&pcm, 48_000, 30, 4, &options);
        assert_eq!(result.master.len(), 4);
        assert!(result
            .master
            .iter()
            .chain(&result.bass)
            .chain(&result.mid)
            .chain(&result.treble)
            .all(|kf| kf.value.is_finite()));
    }

    #[test]
    fn test_extract_multiband_audio_is_silent_after_eof() {
        let options = AudioKeyframeOptions {
            min_db: -60.0,
            max_db: 0.0,
            multiplier: 100.0,
            ..Default::default()
        };
        let pcm = vec![1.0f32, 1.0f32];
        let kfs = extract_multiband_audio_keyframes(&pcm, 48_000, 24, 3, &options);

        assert!(kfs.master[0].value > 0.0);
        assert_eq!(kfs.master[1].value, 0.0);
        assert_eq!(kfs.master[2].value, 0.0);
    }
}
