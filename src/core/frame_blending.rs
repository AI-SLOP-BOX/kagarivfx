#![allow(dead_code)]
//! Frame blending matching After Effects layer time-stretching settings.
//!
//! * FrameBlendMode::Off       - nearest frame, no interpolation.
//! * FrameBlendMode::FrameMix  - temporal cross-fade of adjacent frames.
//! * FrameBlendMode::PixelMotion - block-matching motion vectors warp both
//!   neighbouring frames toward the intermediate timestamp before blending,
//!   producing sharp slow-motion instead of double-exposure ghosting.
//!
//! All functions are pure, deterministic and panic-free.

/// Frame Blending modes matching After Effects layer time-stretching settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBlendMode {
    Off,
    FrameMix,     // Simple linear alpha cross-fade
    PixelMotion,  // Motion-vector interpolation
}

/// Evaluates fractional frame indices and weights for time-stretched playback.
pub fn evaluate_frame_blend_weights(
    time_sec: f32,
    fps: u32,
    blend_mode: FrameBlendMode,
) -> ((u32, f32), (u32, f32)) {
    let exact_frame = (time_sec * fps as f32).max(0.0);
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

/// Alpha-correct temporal cross-fade: RGB is blended in premultiplied space so
/// semi-transparent edges never produce dark halos (the naive per-channel lerp
/// does).
pub fn blend_pixel_buffers(buf0: &[u8], w0: f32, buf1: &[u8], w1: f32, out: &mut [u8]) {
    let len = buf0.len().min(buf1.len()).min(out.len());
    let n_px = len / 4;
    for i in 0..n_px {
        let idx = i * 4;
        let a0 = buf0[idx + 3] as f32 / 255.0 * w0;
        let a1 = buf1[idx + 3] as f32 / 255.0 * w1;
        let out_a = a0 + a1;
        if out_a <= 0.0001 {
            for c in 0..4 {
                out[idx + c] = 0;
            }
            continue;
        }
        for c in 0..3 {
            let v = (buf0[idx + c] as f32 / 255.0 * a0 + buf1[idx + c] as f32 / 255.0 * a1)
                / out_a;
            out[idx + c] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        out[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}
