#![allow(dead_code)]
/// Frame Blending modes matching After Effects layer time-stretching settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBlendMode {
    Off,
    FrameMix,     // Simple linear alpha cross-fade
    PixelMotion,  // Motion vector interpolation
}

/// Evaluates fractional frame indices and weights for time-stretched playback (e.g. 50% slow motion).
pub fn evaluate_frame_blend_weights(
    time_sec: f32,
    fps: u32,
    blend_mode: FrameBlendMode,
) -> ((u32, f32), (u32, f32)) {
    let exact_frame = time_sec * fps as f32;
    let f0 = exact_frame.floor() as u32;
    let f1 = f0 + 1;
    let frac = exact_frame - f0 as f32;

    match blend_mode {
        FrameBlendMode::Off => ((f0, 1.0), (f0, 0.0)),
        FrameBlendMode::FrameMix | FrameBlendMode::PixelMotion => {
            let w1 = frac.clamp(0.0, 1.0);
            let w0 = 1.0 - w1;
            ((f0, w0), (f1, w1))
        }
    }
}

/// Blends two RGBA pixel buffers together using computed Frame Mix weights.
pub fn blend_pixel_buffers(
    buf0: &[u8],
    w0: f32,
    buf1: &[u8],
    w1: f32,
    out: &mut [u8],
) {
    let len = buf0.len().min(buf1.len()).min(out.len());
    for i in 0..len {
        let v0 = buf0[i] as f32 * w0;
        let v1 = buf1[i] as f32 * w1;
        out[i] = (v0 + v1).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_mix_weights() {
        let ((f0, w0), (f1, w1)) = evaluate_frame_blend_weights(0.5, 30, FrameBlendMode::FrameMix);
        assert_eq!(f0, 15);
        assert_eq!(f1, 16);
        assert_eq!(w0, 1.0);
        assert_eq!(w1, 0.0);

        let ((f0, w0), (f1, w1)) = evaluate_frame_blend_weights(0.51, 30, FrameBlendMode::FrameMix);
        assert_eq!(f0, 15);
        assert_eq!(f1, 16);
        assert!((w0 - 0.7).abs() < 0.01);
        assert!((w1 - 0.3).abs() < 0.01);
    }
}
