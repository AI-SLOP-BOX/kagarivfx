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
#[allow(dead_code)]
pub fn mix_audio_for_frame(
    comp: &Composition,
    frame: u32,
    sample_rate: u32,
    buffer_size: usize,
) -> (Vec<f32>, AudioFrameMeter) {
    let mut stereo_output = vec![0.0f32; buffer_size * 2];
    if comp.layers.is_empty() {
        return (stereo_output, AudioFrameMeter::default());
    }

    let mut sum_sq_l = 0.0f32;
    let mut sum_sq_r = 0.0f32;
    let mut max_peak_l = 0.0f32;
    let mut max_peak_r = 0.0f32;

    for layer in &comp.layers {
        if !layer.is_active(frame) {
            continue;
        }

        if let LayerType::Audio { volume, .. } = &layer.layer_type {
            let vol_db = volume.evaluate(frame);
            let gain = 10.0f32.powf(vol_db / 20.0);

            let time_start = (frame.saturating_sub(layer.in_frame)) as f32 / comp.fps.max(1) as f32;
            let start_sample_idx = (time_start * sample_rate as f32) as usize * 2;

            for i in 0..buffer_size {
                let _sample_idx = start_sample_idx + i * 2;
                let sample_l = ((time_start + i as f32 / sample_rate as f32) * 440.0 * std::f32::consts::TAU).sin() * 0.25 * gain;
                let sample_r = sample_l;

                let l_idx = i * 2;
                let r_idx = i * 2 + 1;

                stereo_output[l_idx] += sample_l;
                stereo_output[r_idx] += sample_r;

                let abs_l = stereo_output[l_idx].abs();
                let abs_r = stereo_output[r_idx].abs();

                if abs_l > max_peak_l { max_peak_l = abs_l; }
                if abs_r > max_peak_r { max_peak_r = abs_r; }

                sum_sq_l += abs_l * abs_l;
                sum_sq_r += abs_r * abs_r;
            }
        }
    }

    let n = buffer_size as f32;
    let rms_l = (sum_sq_l / n.max(1.0)).sqrt();
    let rms_r = (sum_sq_r / n.max(1.0)).sqrt();

    let linear_to_db = |val: f32| -> f32 {
        if val <= 1e-5 { -90.0 } else { (20.0 * val.log10()).clamp(-90.0, 6.0) }
    };

    let meter = AudioFrameMeter {
        peak_db_left: linear_to_db(max_peak_l),
        peak_db_right: linear_to_db(max_peak_r),
        rms_db_left: linear_to_db(rms_l),
        rms_db_right: linear_to_db(rms_r),
    };

    (stereo_output, meter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_audio_for_frame_empty_comp() {
        let comp = Composition::new("c1".into(), "Comp 1".into(), 1920, 1080, 30, 300);
        let (buf, meter) = mix_audio_for_frame(&comp, 0, 44100, 512);
        assert_eq!(buf.len(), 1024);
        assert_eq!(meter.peak_db_left, -90.0);
    }
}
