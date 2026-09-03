//! High-Precision 4-Point Corner Pin & Perspective Warp Engine (AE Parity).
//!
//! Computes exact 3x3 Projective Homography matrices using Direct Linear Transformation (DLT)
//! with sub-pixel bilinear backward sampling.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CornerPinQuad {
    pub top_left: [f32; 2],
    pub top_right: [f32; 2],
    pub bottom_right: [f32; 2],
    pub bottom_left: [f32; 2],
}

impl CornerPinQuad {
    pub fn from_rect(w: f32, h: f32) -> Self {
        Self {
            top_left: [0.0, 0.0],
            top_right: [w, 0.0],
            bottom_right: [w, h],
            bottom_left: [0.0, h],
        }
    }
}

/// Computes 3x3 Homography Matrix mapping source rectangle [0..w, 0..h] to target quad.
pub fn compute_homography_matrix(
    src_w: f32,
    src_h: f32,
    dst: &CornerPinQuad,
) -> Option<[[f64; 3]; 3]> {
    // Solve H mapping unit square to dst quad: x_i = (h00*u + h01*v + h02) / (h20*u + h21*v + 1)
    let x0 = dst.top_left[0] as f64;
    let y0 = dst.top_left[1] as f64;
    let x1 = dst.top_right[0] as f64;
    let y1 = dst.top_right[1] as f64;
    let x2 = dst.bottom_right[0] as f64;
    let y2 = dst.bottom_right[1] as f64;
    let x3 = dst.bottom_left[0] as f64;
    let y3 = dst.bottom_left[1] as f64;

    let dx1 = x1 - x2;
    let dx2 = x3 - x2;
    let dy1 = y1 - y2;
    let dy2 = y3 - y2;
    let sx = x0 - x1 + x2 - x3;
    let sy = y0 - y1 + y2 - y3;

    let det = dx1 * dy2 - dx2 * dy1;
    if det.abs() < 1e-8 {
        // Affine fallback
        let a = (x1 - x0) / src_w.max(1.0) as f64;
        let b = (x3 - x0) / src_h.max(1.0) as f64;
        let c = x0;
        let d = (y1 - y0) / src_w.max(1.0) as f64;
        let e = (y3 - y0) / src_h.max(1.0) as f64;
        let f = y0;
        return Some([[a, b, c], [d, e, f], [0.0, 0.0, 1.0]]);
    }

    let g = (sx * dy2 - sy * dx2) / det;
    let h = (dx1 * sy - dy1 * sx) / det;
    let a = x1 - x0 + g * x1;
    let b = x3 - x0 + h * x3;
    let c = x0;
    let d = y1 - y0 + g * y1;
    let e = y3 - y0 + h * y3;
    let f = y0;

    // Scale by source width and height
    let sw = src_w.max(1.0) as f64;
    let sh = src_h.max(1.0) as f64;

    Some([
        [a / sw, b / sh, c],
        [d / sw, e / sh, f],
        [g / sw, h / sh, 1.0],
    ])
}

/// Inverts a 3x3 matrix.
fn invert_3x3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-12 {
        return None;
    }

    let inv_det = 1.0 / det;
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ])
}

/// Applies high-precision Corner Pin perspective warp to an image buffer.
pub fn apply_corner_pin_warp(
    src_pixels: &[u8],
    src_w: u32,
    src_h: u32,
    dst_pixels: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    quad: &CornerPinQuad,
) {
    if src_pixels.len() != (src_w * src_h * 4) as usize
        || dst_pixels.len() != (dst_w * dst_h * 4) as usize
    {
        return;
    }

    let h_mat = match compute_homography_matrix(src_w as f32, src_h as f32, quad) {
        Some(m) => m,
        None => return,
    };

    let inv_h = match invert_3x3(&h_mat) {
        Some(m) => m,
        None => return,
    };

    let sw_i = src_w as f32;
    let sh_i = src_h as f32;

    for y in 0..dst_h {
        for x in 0..dst_w {
            let dx = x as f64;
            let dy = y as f64;

            let w_denom = inv_h[2][0] * dx + inv_h[2][1] * dy + inv_h[2][2];
            if w_denom.abs() < 1e-7 {
                continue;
            }

            let src_x = ((inv_h[0][0] * dx + inv_h[0][1] * dy + inv_h[0][2]) / w_denom) as f32;
            let src_y = ((inv_h[1][0] * dx + inv_h[1][1] * dy + inv_h[1][2]) / w_denom) as f32;

            if src_x >= 0.0 && src_x < (sw_i - 1.0) && src_y >= 0.0 && src_y < (sh_i - 1.0) {
                let x0 = src_x.floor() as usize;
                let y0 = src_y.floor() as usize;
                let x1 = x0 + 1;
                let y1 = y0 + 1;

                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;

                let idx00 = (y0 * src_w as usize + x0) * 4;
                let idx10 = (y0 * src_w as usize + x1) * 4;
                let idx01 = (y1 * src_w as usize + x0) * 4;
                let idx11 = (y1 * src_w as usize + x1) * 4;

                let d_idx = (y as usize * dst_w as usize + x as usize) * 4;

                for c in 0..4 {
                    let val = src_pixels[idx00 + c] as f32 * w00
                        + src_pixels[idx10 + c] as f32 * w10
                        + src_pixels[idx01 + c] as f32 * w01
                        + src_pixels[idx11 + c] as f32 * w11;
                    dst_pixels[d_idx + c] = val.round() as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_corner_pin_reconstructs_image() {
        let w = 32u32;
        let h = 32u32;
        let mut src = vec![0u8; (w * h * 4) as usize];
        // Fill center with green
        for y in 10..20 {
            for x in 10..20 {
                let idx = (y * 32 + x) * 4;
                src[idx + 1] = 255;
                src[idx + 3] = 255;
            }
        }

        let mut dst = vec![0u8; (w * h * 4) as usize];
        let quad = CornerPinQuad::from_rect(w as f32, h as f32);

        apply_corner_pin_warp(&src, w, h, &mut dst, w, h, &quad);

        // Center pixel (15, 15) must be preserved
        let center_idx = (15 * 32 + 15) * 4;
        assert_eq!(dst[center_idx + 1], 255);
        assert_eq!(dst[center_idx + 3], 255);
    }
}
