//! CC Bender / Bend It Non-linear Arc Geometric Warp Engine (AE Parity).
//!
//! Performs cylindrical and circular arc bending between two defined anchor pins
//! with backward sub-pixel bilinear sampling.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum BendWarpType {
    #[default]
    Bend,
    Pinch,
    Twist,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BendWarpConfig {
    pub start_point: [f32; 2],
    pub end_point: [f32; 2],
    pub bend_amount: f32, // -100.0 .. +100.0
    pub warp_type: BendWarpType,
}

impl Default for BendWarpConfig {
    fn default() -> Self {
        Self {
            start_point: [960.0, 800.0],
            end_point: [960.0, 200.0],
            bend_amount: 30.0,
            warp_type: BendWarpType::Bend,
        }
    }
}

/// Applies non-linear bend warp to an RGBA pixel buffer.
pub fn apply_bend_warp(
    src_pixels: &[u8],
    width: u32,
    height: u32,
    dst_pixels: &mut [u8],
    config: &BendWarpConfig,
) {
    let Some(expected_len) = (width as usize).checked_mul(height as usize).and_then(|s| s.checked_mul(4)) else {
        return;
    };
    if src_pixels.len() != expected_len || dst_pixels.len() != expected_len || width == 0 || height == 0 {
        return;
    }

    if config.bend_amount.abs() < 1e-5 || !config.bend_amount.is_finite() {
        dst_pixels.copy_from_slice(src_pixels);
        return;
    }

    let p0 = config.start_point;
    let p1 = config.end_point;
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let axis_len = (dx * dx + dy * dy).sqrt();
    if axis_len < 1e-4 || !axis_len.is_finite() {
        dst_pixels.copy_from_slice(src_pixels);
        return;
    }

    let dir = [dx / axis_len, dy / axis_len];
    let norm = [-dir[1], dir[0]];
    let bend = config.bend_amount * 0.01;
    let w = width as usize;

    for y in 0..height {
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;

            // Project onto start-end local coordinate system
            let vx = px - p0[0];
            let vy = py - p0[1];
            let u = vx * dir[0] + vy * dir[1];     // Along axis distance

            let norm_u = (u / axis_len).clamp(0.0, 1.0);

            // Calculate bend curvature offset
            let displacement = match config.warp_type {
                BendWarpType::Bend => {
                    // Parabolic arc curvature: 4 * norm_u * (1 - norm_u)
                    4.0 * norm_u * (1.0 - norm_u) * (bend * axis_len * 0.5)
                }
                BendWarpType::Pinch => {
                    let factor = (1.0 - (norm_u - 0.5).abs() * 2.0).max(0.0);
                    factor * bend * 100.0
                }
                BendWarpType::Twist => {
                    (norm_u * std::f32::consts::PI).sin() * (bend * 80.0)
                }
            };

            // Inverse sample coordinate
            let src_x = px - norm[0] * displacement;
            let src_y = py - norm[1] * displacement;

            let dst_idx = (y as usize * w + x as usize) * 4;

            if src_x >= 0.0 && src_x <= (width - 1) as f32 && src_y >= 0.0 && src_y <= (height - 1) as f32 {
                let x0 = (src_x.floor() as usize).min(width as usize - 1);
                let y0 = (src_y.floor() as usize).min(height as usize - 1);
                let x1 = (x0 + 1).min(width as usize - 1);
                let y1 = (y0 + 1).min(height as usize - 1);

                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;

                let idx00 = (y0 * w + x0) * 4;
                let idx10 = (y0 * w + x1) * 4;
                let idx01 = (y1 * w + x0) * 4;
                let idx11 = (y1 * w + x1) * 4;

                for c in 0..4 {
                    let val = src_pixels[idx00 + c] as f32 * w00
                        + src_pixels[idx10 + c] as f32 * w10
                        + src_pixels[idx01 + c] as f32 * w01
                        + src_pixels[idx11 + c] as f32 * w11;
                    dst_pixels[dst_idx + c] = val.round().clamp(0.0, 255.0) as u8;
                }
            } else {
                for c in 0..4 {
                    dst_pixels[dst_idx + c] = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bend_warp_zero_amount_is_identity() {
        let width = 32u32;
        let height = 32u32;
        let mut src = vec![0u8; (width * height * 4) as usize];
        // Center pixel red
        let c_idx = (16 * 32 + 16) * 4;
        src[c_idx] = 255;
        src[c_idx + 3] = 255;

        let mut dst = vec![0u8; (width * height * 4) as usize];
        let config = BendWarpConfig {
            start_point: [16.0, 30.0],
            end_point: [16.0, 2.0],
            bend_amount: 0.0,
            warp_type: BendWarpType::Bend,
        };

        apply_bend_warp(&src, width, height, &mut dst, &config);
        assert_eq!(dst[c_idx], 255);
        assert_eq!(dst[c_idx + 3], 255);
    }
}
