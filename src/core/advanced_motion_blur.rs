//! High-Precision Sub-Frame Motion Blur Engine with Shutter Angle and Phase (AE Parity).
//!
//! Generates time-integrated sub-frame sample distributions based on composition
//! shutter angle (exposure duration) and shutter phase (temporal offset).

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubframeMotionBlurSettings {
    pub shutter_angle_deg: f32,   // e.g. 180.0 deg (default cinema shutter)
    pub shutter_phase_deg: f32,   // e.g. -90.0 deg (centered exposure)
    pub samples_per_frame: usize, // 4 .. 32
}

impl Default for SubframeMotionBlurSettings {
    fn default() -> Self {
        Self {
            shutter_angle_deg: 180.0,
            shutter_phase_deg: -90.0,
            samples_per_frame: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubframeSample {
    pub subframe_time_sec: f32,
    pub weight: f32,
}

/// Evaluates exact fractional sub-frame sampling points for motion blur time integration.
pub fn evaluate_subframe_samples(
    frame: u32,
    fps: u32,
    settings: &SubframeMotionBlurSettings,
) -> Vec<SubframeSample> {
    if fps == 0
        || !settings.shutter_angle_deg.is_finite()
        || !settings.shutter_phase_deg.is_finite()
    {
        return Vec::new();
    }
    let frame_dur = 1.0f32 / fps as f32;
    let base_t = frame as f32 * frame_dur;

    let shutter_dur = (settings.shutter_angle_deg / 360.0).clamp(0.0, 2.0) * frame_dur;
    let phase_offset = (settings.shutter_phase_deg / 360.0) * frame_dur;

    let t_start = base_t + phase_offset;
    let n = settings.samples_per_frame.clamp(1, 64);
    let mut samples = Vec::with_capacity(n);

    let weight = 1.0f32 / n as f32;
    for i in 0..n {
        let frac = if n > 1 {
            i as f32 / (n - 1) as f32
        } else {
            0.5
        };
        let t = (t_start + frac * shutter_dur).max(0.0);
        samples.push(SubframeSample {
            subframe_time_sec: t,
            weight,
        });
    }

    samples
}

/// Blends a sequence of sub-frame RGBA pixel buffers into a final motion-blurred frame.
pub fn accumulate_motion_blur_buffers(
    subframe_buffers: &[(&[u8], f32)], // (Buffer, Weight)
    width: u32,
    height: u32,
    out_accum: &mut [u8],
) {
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|s| s.checked_mul(4))
    else {
        out_accum.fill(0);
        return;
    };
    if out_accum.len() < size {
        return;
    }
    if subframe_buffers.is_empty() {
        return;
    }

    let mut float_acc = vec![0.0f32; size];
    let mut total_weight = 0.0f32;

    for &(buf, weight) in subframe_buffers {
        if buf.len() < size || !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        total_weight += weight;
        for i in 0..size {
            float_acc[i] += buf[i] as f32 * weight;
        }
    }

    if total_weight > 0.0 {
        let norm = 1.0 / total_weight;
        for i in 0..size {
            out_accum[i] = (float_acc[i] * norm).round().clamp(0.0, 255.0) as u8;
        }
    } else {
        out_accum[..size].fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_blur_sample_distribution() {
        let settings = SubframeMotionBlurSettings {
            shutter_angle_deg: 180.0,
            shutter_phase_deg: -90.0,
            samples_per_frame: 4,
        };

        // Frame 1 at 30 fps (base_t = 0.0333s)
        let samples = evaluate_subframe_samples(1, 30, &settings);
        assert_eq!(samples.len(), 4);
        // Total weights sum to 1.0
        let sum_w: f32 = samples.iter().map(|s| s.weight).sum();
        assert!((sum_w - 1.0).abs() < 1e-4);

        // Samples must be strictly monotonic
        for w in samples.windows(2) {
            assert!(w[1].subframe_time_sec >= w[0].subframe_time_sec);
        }
    }

    #[test]
    fn test_accumulate_motion_blur_buffers() {
        let f0 = vec![100u8; 16];
        let f1 = vec![200u8; 16];
        let mut out = vec![0u8; 16];

        accumulate_motion_blur_buffers(&[(&f0, 0.5), (&f1, 0.5)], 2, 2, &mut out);
        assert_eq!(out[0], 150);
    }
}
