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
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            x1_l: 0.0, x2_l: 0.0, y1_l: 0.0, y2_l: 0.0,
            x1_r: 0.0, x2_r: 0.0, y1_r: 0.0, y2_r: 0.0,
        }
    }

    fn process_stereo(&mut self, buf: &mut [f32]) {
        for sample in buf.chunks_exact_mut(2) {
            let x_l = sample[0] as f64;
            let x_r = sample[1] as f64;

            let y_l = self.b0 * x_l + self.b1 * self.x1_l + self.b2 * self.x2_l
                       - self.a1 * self.y1_l - self.a2 * self.y2_l;
            let y_r = self.b0 * x_r + self.b1 * self.x1_r + self.b2 * self.x2_r
                       - self.a1 * self.y1_r - self.a2 * self.y2_r;

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
    if bands.is_empty() || buf.is_empty() {
        return;
    }
    let fs = sample_rate as f64;
    let nyquist = fs * 0.5;
    let filters: Vec<Biquad> = bands.iter()
        .filter(|b| {
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
pub fn apply_compressor(buf: &mut [f32], params: &CompressorParams, state: &mut CompressorState, sample_rate: u32) {
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
pub fn apply_limiter(buf: &mut [f32], ceiling_db: f32, release_ms: f32, state: &mut CompressorState, sample_rate: u32) {
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
        let rms_before: f64 = buf.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / (len as f64);
        apply_eq(&mut buf, &bands, sr);
        let rms_after: f64 = buf.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / (len as f64);
        // 12dB boost should increase RMS significantly
        assert!(rms_after > rms_before * 1.5, "Bell boost did not increase level: before={:.4} after={:.4}", rms_before, rms_after);
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
        let rms_after: f64 = buf_loud.iter().map(|s| (*s as f64).powi(2)).sum::<f64>().sqrt() / 2048.0;
        // Compressor with 4:1 ratio above -20dB threshold should reduce level
        assert!(rms_after < rms_before, "Compressor did not reduce loud signal: before={:.4} after={:.4}", rms_before, rms_after);
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
}
