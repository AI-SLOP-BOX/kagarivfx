#![allow(dead_code)]
/// Audio Spectrum / Audio Waveform display modes matching After Effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSpectrumType {
    DigitalBands,
    AnalogLines,
    AnalogDots,
}

/// Options for Audio Spectrum generator effect.
#[derive(Debug, Clone)]
pub struct AudioSpectrumOptions {
    pub start_frequency: f32, // Hz (e.g. 20.0)
    pub end_frequency: f32,   // Hz (e.g. 15000.0)
    pub frequency_bands: u32, // Number of bands (e.g. 64)
    pub max_height: f32,      // Max bar height in pixels
    pub spectrum_type: AudioSpectrumType,
}

impl Default for AudioSpectrumOptions {
    fn default() -> Self {
        Self {
            start_frequency: 20.0,
            end_frequency: 15000.0,
            frequency_bands: 64,
            max_height: 100.0,
            spectrum_type: AudioSpectrumType::DigitalBands,
        }
    }
}

/// Computes frequency band amplitudes from raw audio PCM samples for visual rendering.
pub fn generate_audio_spectrum_bands(
    pcm_samples: &[f32],
    sample_rate: u32,
    options: &AudioSpectrumOptions,
) -> Vec<f32> {
    let num_bands = options.frequency_bands.max(1) as usize;
    if pcm_samples.is_empty() {
        return vec![0.0; num_bands];
    }

    let n = pcm_samples.len();
    let mut bands = vec![0.0f32; num_bands];

    // Compute Band Energies using Discrete Fourier Band summation with Hann windowing
    for (i, band) in bands.iter_mut().enumerate() {
        let band_factor = i as f32 / num_bands as f32;
        let freq = options.start_frequency + (options.end_frequency - options.start_frequency) * band_factor;

        // Target sample stride / period corresponding to frequency
        let period_samples = (sample_rate as f32 / freq.max(1.0)).max(1.0) as usize;

        let mut energy = 0.0f32;
        let mut count = 0;

        for k in (0..n).step_by(period_samples.max(1)) {
            // Hann window: 0.5 * (1 - cos(2*pi*k / N))
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * k as f32 / n as f32).cos());
            energy += (pcm_samples[k] * window).abs();
            count += 1;
        }

        let avg_amp = if count > 0 { energy / count as f32 } else { 0.0 };
        *band = (avg_amp * options.max_height).min(options.max_height);
    }

    bands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_spectrum_generation() {
        let samples = vec![0.5f32; 1024]; // Constant test tone PCM
        let options = AudioSpectrumOptions::default();
        let bands = generate_audio_spectrum_bands(&samples, 44100, &options);

        assert_eq!(bands.len(), 64);
        assert!(bands[0] >= 0.0);
    }
}
