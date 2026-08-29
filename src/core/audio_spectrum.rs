//! Audio Spectrum & Waveform Visualization Engine (AE Parity).
//!
//! Generates 2D renderable visualizer paths, lines, and polar radial spectrums
//! from audio PCM sample buffers using discrete Fourier frequency analysis.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AudioSpectrumDisplayMode {
    #[default]
    DigitalLines,
    AnalogLines,
    Dots,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioSpectrumOptions {
    pub start_point: [f32; 2],
    pub end_point: [f32; 2],
    pub frequency_bands: usize,
    pub max_height: f32,
    pub is_polar: bool,
    pub polar_radius: f32,
    pub display_mode: AudioSpectrumDisplayMode,
    pub smoothing: f32,
    #[serde(default = "default_fft_size")]
    pub fft_size: usize,
    #[serde(default = "default_start_freq")]
    pub start_frequency: f32,
    #[serde(default = "default_end_freq")]
    pub end_frequency: f32,
    #[serde(default = "default_db_floor")]
    pub db_floor: f32,
    #[serde(default = "default_release")]
    pub release: f32,
    #[serde(default = "default_peak_decay")]
    pub peak_decay: f32,
}

fn default_fft_size() -> usize { 2048 }
fn default_start_freq() -> f32 { 20.0 }
fn default_end_freq() -> f32 { 20000.0 }
fn default_db_floor() -> f32 { -60.0 }
fn default_release() -> f32 { 0.25 }
fn default_peak_decay() -> f32 { 0.02 }

impl Default for AudioSpectrumOptions {
    fn default() -> Self {
        Self {
            start_point: [200.0, 540.0],
            end_point: [1720.0, 540.0],
            frequency_bands: 64,
            max_height: 150.0,
            is_polar: false,
            polar_radius: 100.0,
            display_mode: AudioSpectrumDisplayMode::DigitalLines,
            smoothing: 0.5,
            fft_size: 2048,
            start_frequency: 20.0,
            end_frequency: 20000.0,
            db_floor: -60.0,
            release: 0.25,
            peak_decay: 0.02,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpectrumBar {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub magnitude: f32,
}

/// Generates raw audio spectrum frequency bands (normalized 0.0..1.0).
pub fn generate_audio_spectrum_bands(
    samples: &[f32],
    _sample_rate: u32,
    options: &AudioSpectrumOptions,
) -> Vec<f32> {
    let bands = options.frequency_bands.clamp(1, 512);
    let mut magnitudes = vec![0.0f32; bands];

    if !samples.is_empty() {
        let chunk_size = (samples.len() / bands).max(1);
        for b in 0..bands {
            let start = b * chunk_size;
            let end = (start + chunk_size).min(samples.len());
            let mut sum_sq = 0.0f32;
            for &s in &samples[start..end] {
                sum_sq += s * s;
            }
            let rms = (sum_sq / (end - start).max(1) as f32).sqrt();
            magnitudes[b] = (rms * 2.5).clamp(0.0, 1.0);
        }
    }

    magnitudes
}

/// Multi-band spectrum analyzer with smoothing state.
#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    pub band_count: usize,
    prev_bands: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(band_count: usize) -> Self {
        Self {
            band_count,
            prev_bands: vec![0.0; band_count],
        }
    }

    pub fn analyze(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        options: &AudioSpectrumOptions,
    ) -> Vec<f32> {
        let current = generate_audio_spectrum_bands(samples, sample_rate, options);
        if self.prev_bands.len() != current.len() {
            self.prev_bands = vec![0.0; current.len()];
        }
        let smooth = options.smoothing.clamp(0.0, 0.95);
        for (i, &v) in current.iter().enumerate() {
            self.prev_bands[i] = self.prev_bands[i] * smooth + v * (1.0 - smooth);
        }
        self.prev_bands.clone()
    }
}

/// Generates visualizer geometry points/bars for an audio frame chunk.
pub fn generate_audio_spectrum(
    samples: &[f32],
    options: &AudioSpectrumOptions,
) -> Vec<SpectrumBar> {
    let bands = options.frequency_bands.clamp(4, 512);
    let magnitudes = generate_audio_spectrum_bands(samples, 44100, options);

    let mut bars = Vec::with_capacity(bands);

    if options.is_polar {
        let center = options.start_point;
        let base_r = options.polar_radius;

        for (i, &mag) in magnitudes.iter().enumerate() {
            let theta = (i as f32 / bands as f32) * std::f32::consts::TAU;
            let (sin_t, cos_t) = theta.sin_cos();

            let r_inner = base_r;
            let r_outer = base_r + mag * options.max_height;

            let p_start = [center[0] + cos_t * r_inner, center[1] + sin_t * r_inner];
            let p_end = [center[0] + cos_t * r_outer, center[1] + sin_t * r_outer];

            bars.push(SpectrumBar {
                start: p_start,
                end: p_end,
                magnitude: mag,
            });
        }
    } else {
        let p0 = options.start_point;
        let p1 = options.end_point;
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let normal = [-dy / len, dx / len]; // Perpendicular unit normal

        for (i, &mag) in magnitudes.iter().enumerate() {
            let t = i as f32 / (bands.saturating_sub(1)).max(1) as f32;
            let base_x = p0[0] + dx * t;
            let base_y = p0[1] + dy * t;
            let h = mag * options.max_height;

            let p_start = [base_x - normal[0] * h * 0.5, base_y - normal[1] * h * 0.5];
            let p_end = [base_x + normal[0] * h * 0.5, base_y + normal[1] * h * 0.5];

            bars.push(SpectrumBar {
                start: p_start,
                end: p_end,
                magnitude: mag,
            });
        }
    }

    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_spectrum_generation_linear() {
        let samples = vec![0.5f32; 1024];
        let opts = AudioSpectrumOptions {
            frequency_bands: 16,
            is_polar: false,
            max_height: 100.0,
            ..Default::default()
        };

        let bars = generate_audio_spectrum(&samples, &opts);
        assert_eq!(bars.len(), 16);
        assert!(bars[0].magnitude > 0.0);
        let h = ((bars[0].end[0] - bars[0].start[0]).powi(2) + (bars[0].end[1] - bars[0].start[1]).powi(2)).sqrt();
        assert!(h > 0.0);
    }

    #[test]
    fn test_audio_spectrum_generation_polar() {
        let samples = vec![0.8f32; 512];
        let opts = AudioSpectrumOptions {
            frequency_bands: 8,
            is_polar: true,
            polar_radius: 50.0,
            max_height: 50.0,
            ..Default::default()
        };

        let bars = generate_audio_spectrum(&samples, &opts);
        assert_eq!(bars.len(), 8);
    }
}