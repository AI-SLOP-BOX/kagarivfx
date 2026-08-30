//! Find Edges Effect Engine (AE Parity).
//!
//! Identifies pixels in an image that have significant color transitions and emphasizes edges.
//! By default, displays edges as dark lines on a white background; with `invert = true`,
//! displays edges as neon/bright lines on a dark background.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FindEdgesParams {
    pub invert: bool,
}

impl Default for FindEdgesParams {
    fn default() -> Self {
        Self { invert: false }
    }
}

/// Applies Find Edges filter across an RGBA image buffer.
pub fn apply_find_edges(src: &[u8], width: u32, height: u32, params: &FindEdgesParams) -> Vec<u8> {
    let Some(pixel_count) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return src.to_vec();
    };
    if src.len() != pixel_count * 4 || width < 3 || height < 3 {
        return src.to_vec();
    }

    let mut dst = vec![0u8; src.len()];

    let get_pixel = |x: i32, y: i32| -> [f32; 3] {
        let cx = x.clamp(0, width as i32 - 1) as usize;
        let cy = y.clamp(0, height as i32 - 1) as usize;
        let idx = (cy * width as usize + cx) * 4;
        [src[idx] as f32, src[idx + 1] as f32, src[idx + 2] as f32]
    };

    for y in 0..height {
        let iy = y as i32;
        for x in 0..width {
            let ix = x as i32;

            // 3x3 neighborhood
            let p00 = get_pixel(ix - 1, iy - 1);
            let p10 = get_pixel(ix, iy - 1);
            let p20 = get_pixel(ix + 1, iy - 1);

            let p01 = get_pixel(ix - 1, iy);
            let p21 = get_pixel(ix + 1, iy);

            let p02 = get_pixel(ix - 1, iy + 1);
            let p12 = get_pixel(ix, iy + 1);
            let p22 = get_pixel(ix + 1, iy + 1);

            let d_idx = ((y * width + x) * 4) as usize;

            for c in 0..3 {
                // Sobel kernels
                let gx = -p00[c] + p20[c] - 2.0 * p01[c] + 2.0 * p21[c] - p02[c] + p22[c];
                let gy = -p00[c] - 2.0 * p10[c] - p20[c] + p02[c] + 2.0 * p12[c] + p22[c];

                let mag = (gx * gx + gy * gy).sqrt();

                let val = if params.invert {
                    mag.clamp(0.0, 255.0)
                } else {
                    (255.0 - mag).clamp(0.0, 255.0)
                };

                dst[d_idx + c] = val.round() as u8;
            }

            dst[d_idx + 3] = src[d_idx + 3]; // Preserve original alpha
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_edges_uniform_image() {
        let w = 8u32;
        let h = 8u32;
        let src = vec![128u8; (w * h * 4) as usize];
        let params = FindEdgesParams { invert: false };

        let dst = apply_find_edges(&src, w, h, &params);
        assert_eq!(dst.len(), src.len());
        // Uniform image has 0 gradient magnitude -> 255 (white) when not inverted
        assert_eq!(dst[0], 255);
        assert_eq!(dst[1], 255);
        assert_eq!(dst[2], 255);
    }

    #[test]
    fn test_find_edges_inverted_uniform_image() {
        let w = 8u32;
        let h = 8u32;
        let src = vec![128u8; (w * h * 4) as usize];
        let params = FindEdgesParams { invert: true };

        let dst = apply_find_edges(&src, w, h, &params);
        // Uniform image has 0 gradient magnitude -> 0 (black) when inverted
        assert_eq!(dst[0], 0);
    }

    #[test]
    fn test_find_edges_rejects_dimension_overflow() {
        let src = vec![128u8; 4];
        let dst = apply_find_edges(&src, u32::MAX, u32::MAX, &FindEdgesParams::default());
        assert_eq!(dst, src);
    }
}
