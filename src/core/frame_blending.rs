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

/// Resolves effective sample frame and fractional blend weight for Time Remapped layers.
pub fn evaluate_time_remap_seconds(
    remap_sec: f32,
    source_fps: u32,
    source_total_frames: u32,
    blend_mode: FrameBlendMode,
) -> ((u32, f32), (u32, f32)) {
    let exact_frame = (remap_sec * source_fps as f32).max(0.0);
    let max_frame = source_total_frames.saturating_sub(1);

    let f0 = (exact_frame.floor() as u32).min(max_frame);
    let f1 = (f0 + 1).min(max_frame);
    let frac = (exact_frame - f0 as f32).clamp(0.0, 1.0);

    match blend_mode {
        FrameBlendMode::Off => ((f0, 1.0), (f0, 0.0)),
        FrameBlendMode::FrameMix | FrameBlendMode::PixelMotion => {
            ((f0, 1.0 - frac), (f1, frac))
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

/// Options for the simplified Pixel Motion interpolator.
#[derive(Debug, Clone, Copy)]
pub struct PixelMotionOptions {
    /// Motion estimation block size in pixels (8..64).
    pub block_size: u32,
    /// Search radius in pixels around each block.
    pub search_radius: u32,
    /// SAD threshold above which a block is considered occluded and falls
    /// back to a plain cross-fade for that region (mean per-channel 0..255).
    pub occlusion_threshold: f32,
}

impl Default for PixelMotionOptions {
    fn default() -> Self {
        Self { block_size: 16, search_radius: 6, occlusion_threshold: 40.0 }
    }
}

/// Simplified Pixel Motion interpolation.
///
/// For every block of frame buf0, a translation vector into buf1 is estimated
/// by exhaustive SAD search. Both frames are then warped toward the
/// intermediate time t (0..1) using those vectors and alpha-correctly mixed.
/// Blocks whose best match exceeds the occlusion threshold degrade to a plain
/// cross-fade, matching the fallback behaviour used for disocclusions.
pub fn blend_pixel_motion(
    buf0: &[u8],
    buf1: &[u8],
    t: f32,
    width: u32,
    height: u32,
    options: &PixelMotionOptions,
) -> Vec<u8> {
    let n_bytes = (width as usize) * (height as usize) * 4;
    if buf0.len() != n_bytes || buf1.len() != n_bytes || n_bytes == 0 {
        return buf0.to_vec();
    }
    let t = t.clamp(0.0, 1.0);

    let bs = options.block_size.clamp(4, 128).max(1) as usize;
    let rad = options.search_radius.min(32) as i32;
    let mut out = vec![0u8; n_bytes];

    let sample = |buf: &[u8], fx: f32, fy: f32| -> [f32; 4] {
        let x = fx.clamp(0.0, width as f32 - 1.0);
        let y = fy.clamp(0.0, height as f32 - 1.0);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(width as usize - 1);
        let y1 = (y0 + 1).min(height as usize - 1);
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let g = |xx: usize, yy: usize, c: usize| buf[(yy * width as usize + xx) * 4 + c] as f32;
        let mut rgba = [0.0f32; 4];
        for (c, slot) in rgba.iter_mut().enumerate() {
            let top = g(x0, y0, c) + (g(x1, y0, c) - g(x0, y0, c)) * tx;
            let bot = g(x0, y1, c) + (g(x1, y1, c) - g(x0, y1, c)) * tx;
            *slot = top + (bot - top) * ty;
        }
        rgba
    };

    let mix_px = |s0: [f32; 4], s1: [f32; 4]| -> [f32; 4] {
        let a = s0[3] * (1.0 - t) + s1[3] * t;
        if a > 0.001 {
            [
                (s0[0] * s0[3] * (1.0 - t) + s1[0] * s1[3] * t) / a,
                (s0[1] * s0[3] * (1.0 - t) + s1[1] * s1[3] * t) / a,
                (s0[2] * s0[3] * (1.0 - t) + s1[2] * s1[3] * t) / a,
                a,
            ]
        } else {
            [0.0; 4]
        }
    };

    let mut by = 0usize;
    while by < height as usize {
        let mut bx = 0usize;
        while bx < width as usize {
            // Exhaustive block match: f0 block at (bx,by) to best offset in f1.
            let mut best_sad = f32::MAX;
            let mut best_dx = 0i32;
            let mut best_dy = 0i32;
            for dy in -rad..=rad {
                for dx in -rad..=rad {
                    let sx = bx as i32 + dx;
                    let sy = by as i32 + dy;
                    if sx < 0 || sy < 0 || sx + bs as i32 > width as i32 || sy + bs as i32 > height as i32 {
                        continue;
                    }
                    let mut sad = 0.0f64;
                    let mut count = 0u64;
                    for yy in 0..bs {
                        let ya = by + yy;
                        let yb = sy as usize + yy;
                        for xx in 0..bs {
                            let xa = bx + xx;
                            let xb = sx as usize + xx;
                            let ia = (ya * width as usize + xa) * 4;
                            let ib = (yb * width as usize + xb) * 4;
                            for c in 0..3 {
                                sad += (buf0[ia + c] as i32 - buf1[ib + c] as i32).abs() as f64;
                            }
                            count += 1;
                        }
                    }
                    let mean = if count > 0 { (sad / (count as f64 * 3.0)) as f32 } else { f32::MAX };
                    if mean < best_sad {
                        best_sad = mean;
                        best_dx = dx;
                        best_dy = dy;
                    }
                }
            }

            let occluded = best_sad > options.occlusion_threshold;

            for yy in 0..bs {
                let py = by + yy;
                if py >= height as usize { break; }
                for xx in 0..bs {
                    let px = bx + xx;
                    if px >= width as usize { break; }
                    let qx = px as f32;
                    let qy = py as f32;
                    let rgba = if occluded {
                        mix_px(sample(buf0, qx, qy), sample(buf1, qx, qy))
                    } else {
                        // Two-sided warp toward the intermediate timestamp.
                        // A f0 block matching f1 at offset d means its content
                        // sits at q - d*t (from f0) / q + d*(1-t) (from f1).
                        let s0 = sample(buf0, qx - best_dx as f32 * t, qy - best_dy as f32 * t);
                        let s1 = sample(buf1, qx + best_dx as f32 * (1.0 - t), qy + best_dy as f32 * (1.0 - t));
                        mix_px(s0, s1)
                    };
                    let idx = (py * width as usize + px) * 4;
                    for c in 0..4 {
                        out[idx + c] = rgba[c].round().clamp(0.0, 255.0) as u8;
                    }
                }
            }

            bx += bs;
        }
        by += bs;
    }

    out
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

        let ((f0, w0), (_, w1)) = evaluate_frame_blend_weights(0.51, 30, FrameBlendMode::FrameMix);
        assert_eq!(f0, 15);
        assert!((w0 - 0.7).abs() < 0.01);
        assert!((w1 - 0.3).abs() < 0.01);

        let ((f0, w0), (_, w1)) = evaluate_frame_blend_weights(0.99, 30, FrameBlendMode::Off);
        assert_eq!(f0, 29);
        assert_eq!(w0, 1.0);
        assert_eq!(w1, 0.0);
    }

    #[test]
    fn test_blend_is_alpha_correct_no_dark_fringe() {
        let buf0 = vec![255u8, 0, 0, 128];
        let buf1 = vec![0u8; 4];
        let mut out = vec![0u8; 4];
        blend_pixel_buffers(&buf0, 1.0, &buf1, 0.0, &mut out);
        assert_eq!(out[0], 255, "red must stay saturated");
        assert_eq!(out[3], 128);
    }

    #[test]
    fn test_blend_full_crossfade_midpoint() {
        let a = vec![200u8, 0, 0, 255];
        let b = vec![0u8, 200, 0, 255];
        let mut out = vec![0u8; 4];
        blend_pixel_buffers(&a, 0.5, &b, 0.5, &mut out);
        assert!((out[0] as i32 - 100).abs() <= 1);
        assert!((out[1] as i32 - 100).abs() <= 1);
    }

    fn moving_square(x: u32) -> Vec<u8> {
        let mut v = vec![0u8; 32 * 32 * 4];
        for yy in 12..20 {
            for xx in x..x + 8 {
                let i = ((yy * 32 + xx) * 4) as usize;
                v[i] = 255;
                v[i + 1] = 255;
                v[i + 2] = 255;
                v[i + 3] = 255;
            }
        }
        v
    }

    #[test]
    fn test_pixel_motion_tracks_moving_square() {
        let f0 = moving_square(6);
        let f1 = moving_square(14);
        let out = blend_pixel_motion(
            &f0,
            &f1,
            0.5,
            32,
            32,
            &PixelMotionOptions { block_size: 8, search_radius: 10, ..Default::default() },
        );

        // Motion compensation keeps the interpolated square much brighter than
        // a plain cross-fade would at its centre.
        let lum = |buf: &[u8], xx: usize, yy: usize| buf[(yy * 32 + xx) * 4] as i32;
        let centre = lum(&out, 13, 15);
        let crossfade_centre = (lum(&f0, 13, 15) + lum(&f1, 13, 15)) / 2;
        assert!(
            centre > crossfade_centre + 60,
            "motion compensation must beat ghosting: {} vs {}",
            centre,
            crossfade_centre
        );
    }

    #[test]
    fn test_pixel_motion_endpoints_recover_inputs() {
        let f0 = moving_square(6);
        let f1 = moving_square(14);
        let opts = PixelMotionOptions { block_size: 8, search_radius: 10, ..Default::default() };
        let at0 = blend_pixel_motion(&f0, &f1, 0.0, 32, 32, &opts);
        let at1 = blend_pixel_motion(&f0, &f1, 1.0, 32, 32, &opts);
        let diff0: u64 = at0
            .chunks(4)
            .zip(f0.chunks(4))
            .map(|(a, b)| (a[0] as i64 - b[0] as i64).unsigned_abs())
            .sum();
        assert!(diff0 < 500, "t=0 should reconstruct frame 0, diff={}", diff0);
        let diff1: u64 = at1
            .chunks(4)
            .zip(f1.chunks(4))
            .map(|(a, b)| (a[0] as i64 - b[0] as i64).unsigned_abs())
            .sum();
        assert!(diff1 < 500, "t=1 should reconstruct frame 1, diff={}", diff1);
    }

    #[test]
    fn test_pixel_motion_occlusion_falls_back_gracefully() {
        // Pure noise frames have no matches: every block degrades to fade.
        let mut state = 0x2545F4914F6CDD1Du64;
        let mut noise = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state & 0xFF) as u8
        };
        let f0: Vec<u8> = (0..32 * 32 * 4).map(|_| noise()).collect();
        let f1: Vec<u8> = (0..32 * 32 * 4).map(|_| noise()).collect();
        let out = blend_pixel_motion(&f0, &f1, 0.5, 32, 32, &PixelMotionOptions::default());
        assert_eq!(out.len(), f0.len());
    }

    #[test]
    fn test_pixel_motion_deterministic_and_safe_on_degenerate_input() {
        let f0 = moving_square(6);
        let f1 = moving_square(14);
        let opts = PixelMotionOptions::default();
        let a = blend_pixel_motion(&f0, &f1, 0.5, 32, 32, &opts);
        let b = blend_pixel_motion(&f0, &f1, 0.5, 32, 32, &opts);
        assert_eq!(a, b);
        let small = vec![1u8; 16];
        let fallback = blend_pixel_motion(&f0, &small, 0.5, 32, 32, &opts);
        assert_eq!(fallback, f0);
    }

    #[test]
    fn test_time_remap_seconds_evaluation() {
        // 30fps source, 100 frames total.
        // Remap to 1.5 seconds -> exact frame 45.0
        let ((f0, w0), (f1, w1)) = evaluate_time_remap_seconds(1.5, 30, 100, FrameBlendMode::FrameMix);
        assert_eq!(f0, 45);
        assert_eq!(f1, 46);
        assert_eq!(w0, 1.0);
        assert_eq!(w1, 0.0);

        // Remap to 1.55 seconds -> frame 46.5
        let ((f0, w0), (f1, w1)) = evaluate_time_remap_seconds(1.55, 30, 100, FrameBlendMode::FrameMix);
        assert_eq!(f0, 46);
        assert_eq!(f1, 47);
        assert!((w0 - 0.5).abs() < 1e-4);
        assert!((w1 - 0.5).abs() < 1e-4);
    }
}
