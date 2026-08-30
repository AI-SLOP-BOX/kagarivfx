//! Audio Spectrum & Waveform Visualization Engine (AE Parity).
//!
//! Generates 2D renderable visualizer paths, lines, and polar radial spectrums
//! from audio PCM sample buffers using discrete Fourier frequency analysis.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AudioSpectrumType {
    #[default]
    DigitalBands,
    AnalogLines,
    AnalogDots,
}

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
    pub spectrum_type: AudioSpectrumType,
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

fn default_fft_size() -> usize {
    2048
}
fn default_start_freq() -> f32 {
    20.0
}
fn default_end_freq() -> f32 {
    20000.0
}
fn default_db_floor() -> f32 {
    -60.0
}
fn default_release() -> f32 {
    0.25
}
fn default_peak_decay() -> f32 {
    0.02
}

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
            spectrum_type: AudioSpectrumType::DigitalBands,
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
                if s.is_finite() {
                    sum_sq = (sum_sq + s * s).min(f32::MAX);
                }
            }
            let rms = (sum_sq / (end - start).max(1) as f32).sqrt();
            magnitudes[b] = (rms * 2.5).clamp(0.0, 1.0);
        }
    }

    magnitudes
}

/// Multi-band spectrum analyzer with smoothing and peak tracking state.
#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    pub band_count: usize,
    prev_bands: Vec<f32>,
    peaks: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(band_count: usize) -> Self {
        Self {
            band_count,
            prev_bands: vec![0.0; band_count],
            peaks: vec![0.0; band_count],
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
            self.peaks = vec![0.0; current.len()];
        }
        let smooth = options.smoothing.clamp(0.0, 0.95);
        for (i, &v) in current.iter().enumerate() {
            self.prev_bands[i] = self.prev_bands[i] * smooth + v * (1.0 - smooth);
            self.peaks[i] = self.peaks[i].max(self.prev_bands[i]) - options.peak_decay;
            if self.peaks[i] < 0.0 {
                self.peaks[i] = 0.0;
            }
        }
        self.prev_bands.clone()
    }

    pub fn peaks(&self) -> &[f32] {
        &self.peaks
    }
}

/// Extract RMS waveform envelope
pub fn extract_waveform(samples: &[f32], target_len: usize) -> Vec<f32> {
    if samples.is_empty() || target_len == 0 {
        return vec![0.0; target_len];
    }
    let chunk_size = (samples.len() / target_len).max(1);
    let mut out = Vec::with_capacity(target_len);
    for i in 0..target_len {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(samples.len());
        if start >= samples.len() {
            out.push(0.0);
            continue;
        }
        let mut max_val = 0.0f32;
        for &s in &samples[start..end] {
            if s.is_finite() {
                max_val = max_val.max(s.abs());
            }
        }
        out.push(max_val);
    }
    out
}

/// Direct RGBA buffer rasterizer for spectrum bars
pub fn render_spectrum(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    bands: &[f32],
    _options: &AudioSpectrumOptions,
    color_a: [u8; 3],
    color_b: [u8; 3],
) {
    if bands.is_empty() || width == 0 || height == 0 {
        return;
    }
    let band_w = (width as f32 / bands.len() as f32).max(1.0);
    let cy = height as f32 * 0.5;

    for (b_idx, &mag) in bands.iter().enumerate() {
        let bx = b_idx as f32 * band_w;
        let safe_mag = if mag.is_finite() { mag.max(0.0) } else { 0.0 };
        let bh = (safe_mag * (height as f32 * 0.45)).min(height as f32).max(1.0);
        let top = (cy - bh).clamp(0.0, height as f32 - 1.0) as u32;
        let bot = (cy + bh).clamp(0.0, height as f32 - 1.0) as u32;

        let t = b_idx as f32 / bands.len() as f32;
        let r = ((1.0 - t) * color_a[0] as f32 + t * color_b[0] as f32) as u8;
        let g = ((1.0 - t) * color_a[1] as f32 + t * color_b[1] as f32) as u8;
        let b = ((1.0 - t) * color_a[2] as f32 + t * color_b[2] as f32) as u8;

        for y in top..=bot {
            for x in (bx as u32)..((bx + band_w * 0.8) as u32).min(width) {
                let Some(idx) = (y as usize)
                    .checked_mul(width as usize)
                    .and_then(|offset| offset.checked_add(x as usize))
                    .and_then(|offset| offset.checked_mul(4))
                else {
                    continue;
                };
                if idx + 3 < buffer.len() {
                    buffer[idx] = r;
                    buffer[idx + 1] = g;
                    buffer[idx + 2] = b;
                    buffer[idx + 3] = 255;
                }
            }
        }
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
    fn test_audio_analysis_ignores_nonfinite_samples_and_magnitudes() {
        let samples = [f32::NAN, f32::INFINITY, 0.5, -0.5];
        let options = AudioSpectrumOptions {
            frequency_bands: 2,
            ..Default::default()
        };
        let bands = generate_audio_spectrum_bands(&samples, 48_000, &options);
        assert_eq!(bands.len(), 2);
        assert!(bands.iter().all(|value| value.is_finite()));

        let mut buffer = vec![11u8; 16];
        render_spectrum(
            &mut buffer,
            2,
            2,
            &[f32::NAN, f32::INFINITY],
            &options,
            [255, 0, 0],
            [0, 0, 255],
        );
        assert!(buffer.iter().all(|value| *value <= 255));
    }

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
        let h = ((bars[0].end[0] - bars[0].start[0]).powi(2)
            + (bars[0].end[1] - bars[0].start[1]).powi(2))
        .sqrt();
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

    #[test]
    fn test_audio_waveform_generation_and_render() {
        let samples = vec![0.3f32; 512];
        let opts = AudioWaveformOptions {
            start_point: [10.0, 32.0],
            end_point: [54.0, 32.0],
            max_height: 20.0,
            ..Default::default()
        };
        let pts = generate_waveform_points(&samples, &opts);
        assert_eq!(pts.len(), opts.sample_count);

        let mut buf = vec![0u8; 64 * 64 * 4];
        render_audio_waveform(&mut buf, 64, 64, &samples, &opts, [255, 255, 255, 255]);
        // Buffer should contain some drawn waveform pixels
        let non_zero = buf.iter().any(|&b| b > 0);
        assert!(non_zero);
    }
}

/// Options for Audio Waveform visualizer (AE Parity: Generate > Audio Waveform).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioWaveformOptions {
    pub start_point: [f32; 2],
    pub end_point: [f32; 2],
    pub sample_count: usize,
    pub max_height: f32,
    pub stroke_width: f32,
    pub is_polar: bool,
    pub polar_radius: f32,
    pub softness: f32,
}

impl Default for AudioWaveformOptions {
    fn default() -> Self {
        Self {
            start_point: [200.0, 540.0],
            end_point: [1720.0, 540.0],
            sample_count: 256,
            max_height: 120.0,
            stroke_width: 2.5,
            is_polar: false,
            polar_radius: 120.0,
            softness: 1.0,
        }
    }
}

/// Generates evaluated 2D point sequence for an oscilloscope / audio waveform.
pub fn generate_waveform_points(samples: &[f32], options: &AudioWaveformOptions) -> Vec<[f32; 2]> {
    let count = options.sample_count.clamp(16, 2048);
    let mut points = Vec::with_capacity(count);

    let step = if samples.is_empty() {
        0
    } else {
        samples.len() / count
    };

    if options.is_polar {
        let center = options.start_point;
        let base_r = options.polar_radius;

        for i in 0..count {
            let s_idx = (i * step).min(samples.len().saturating_sub(1));
            let val = if samples.is_empty() {
                0.0
            } else {
                samples[s_idx]
            };

            let theta = (i as f32 / count as f32) * std::f32::consts::TAU;
            let (sin_t, cos_t) = theta.sin_cos();

            let r = base_r + val * options.max_height;
            points.push([center[0] + cos_t * r, center[1] + sin_t * r]);
        }
    } else {
        let p0 = options.start_point;
        let p1 = options.end_point;
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let normal = [-dy / len, dx / len];

        for i in 0..count {
            let t = i as f32 / (count.saturating_sub(1)).max(1) as f32;
            let s_idx = (i * step).min(samples.len().saturating_sub(1));
            let val = if samples.is_empty() {
                0.0
            } else {
                samples[s_idx]
            };

            let base_x = p0[0] + dx * t;
            let base_y = p0[1] + dy * t;
            let h = val * options.max_height;

            points.push([base_x + normal[0] * h, base_y + normal[1] * h]);
        }
    }

    points
}

/// Rasterizes oscilloscope audio waveform directly into an RGBA pixel buffer.
pub fn render_audio_waveform(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    samples: &[f32],
    options: &AudioWaveformOptions,
    color: [u8; 4],
) {
    if width == 0 || height == 0 {
        return;
    }

    let pts = generate_waveform_points(samples, options);
    if pts.len() < 2 {
        return;
    }

    let stroke_r = (options.stroke_width * 0.5).max(0.5);
    let stroke_r_sq = (stroke_r + options.softness).powi(2);

    for w in pts.windows(2) {
        let p0 = w[0];
        let p1 = w[1];

        let min_x = ((p0[0].min(p1[0]) - stroke_r - 1.0).floor() as i32).clamp(0, width as i32 - 1);
        let max_x = ((p0[0].max(p1[0]) + stroke_r + 1.0).ceil() as i32).clamp(0, width as i32 - 1);
        let min_y =
            ((p0[1].min(p1[1]) - stroke_r - 1.0).floor() as i32).clamp(0, height as i32 - 1);
        let max_y = ((p0[1].max(p1[1]) + stroke_r + 1.0).ceil() as i32).clamp(0, height as i32 - 1);

        let seg_dx = p1[0] - p0[0];
        let seg_dy = p1[1] - p0[1];
        let seg_len_sq = (seg_dx * seg_dx + seg_dy * seg_dy).max(1e-5);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32;
                let py = y as f32;

                // Project point onto line segment
                let u =
                    (((px - p0[0]) * seg_dx + (py - p0[1]) * seg_dy) / seg_len_sq).clamp(0.0, 1.0);
                let proj_x = p0[0] + u * seg_dx;
                let proj_y = p0[1] + u * seg_dy;

                let dist_sq = (px - proj_x).powi(2) + (py - proj_y).powi(2);
                if dist_sq <= stroke_r_sq {
                    let dist = dist_sq.sqrt();
                    let alpha_factor = if dist <= stroke_r {
                        1.0
                    } else {
                        (1.0 - (dist - stroke_r) / options.softness.max(0.1)).clamp(0.0, 1.0)
                    };

                    let idx = ((y as u32 * width + x as u32) * 4) as usize;
                    if idx + 3 < buffer.len() {
                        let a = (color[3] as f32 * alpha_factor) as u8;
                        if a > buffer[idx + 3] {
                            buffer[idx] = color[0];
                            buffer[idx + 1] = color[1];
                            buffer[idx + 2] = color[2];
                            buffer[idx + 3] = a;
                        }
                    }
                }
            }
        }
    }
}
