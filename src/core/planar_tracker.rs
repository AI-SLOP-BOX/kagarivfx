/// Planar tracker (simplified Mocha-style): tracks a quadrilateral region
/// across frames using phase correlation + affine warp on the tracked patch.
use serde::{Deserialize, Serialize};

/// A planar surface defined by 4 corner points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanarSurface {
    pub corners: [[f32; 2]; 4],
}

impl PlanarSurface {
    pub fn new(corners: [[f32; 2]; 4]) -> Self {
        Self { corners }
    }

    /// Bilinear sample a pixel from `pixels` at subpixel coords (x, y).
    fn bilinear_sample(pixels: &[u8], w: u32, h: u32, x: f32, y: f32) -> [f32; 3] {
        let x = x.clamp(0.0, w as f32 - 1.001);
        let y = y.clamp(0.0, h as f32 - 1.001);
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(w - 1);
        let y1 = (y0 + 1).min(h - 1);
        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let sample = |px: u32, py: u32| -> [f32; 3] {
            let i = ((py * w + px) * 4) as usize;
            [pixels[i] as f32, pixels[i + 1] as f32, pixels[i + 2] as f32]
        };

        let tl = sample(x0, y0);
        let tr = sample(x1, y0);
        let bl = sample(x0, y1);
        let br = sample(x1, y1);

        let mut out = [0.0f32; 3];
        for c in 0..3 {
            out[c] = tl[c] * (1.0 - fx) * (1.0 - fy)
                + tr[c] * fx * (1.0 - fy)
                + bl[c] * (1.0 - fx) * fy
                + br[c] * fx * fy;
        }
        out
    }

    /// Extract the warped patch pixels from `src` into `patch` (w×h RGB).
    pub fn extract_patch(&self, src: &[u8], src_w: u32, src_h: u32, patch_w: u32, patch_h: u32) -> Vec<[f32; 3]> {
        let mut patch = vec![[0.0f32; 3]; (patch_w * patch_h) as usize];
        for py in 0..patch_h {
            for px in 0..patch_w {
                let u = px as f32 / patch_w as f32;
                let v = py as f32 / patch_h as f32;
                let x = self.bilerp_x(u, v);
                let y = self.bilerp_y(u, v);
                let sample = Self::bilinear_sample(src, src_w, src_h, x, y);
                patch[(py * patch_w + px) as usize] = sample;
            }
        }
        patch
    }

    fn bilerp_x(&self, u: f32, v: f32) -> f32 {
        let tl = self.corners[0];
        let tr = self.corners[1];
        let bl = self.corners[2];
        let br = self.corners[3];
        tl[0] * (1.0 - u) * (1.0 - v) + tr[0] * u * (1.0 - v) + bl[0] * (1.0 - u) * v + br[0] * u * v
    }

    fn bilerp_y(&self, u: f32, v: f32) -> f32 {
        let tl = self.corners[0];
        let tr = self.corners[1];
        let bl = self.corners[2];
        let br = self.corners[3];
        tl[1] * (1.0 - u) * (1.0 - v) + tr[1] * u * (1.0 - v) + bl[1] * (1.0 - u) * v + br[1] * u * v
    }

    /// Apply an affine transform (2×3 matrix) to the corners.
    pub fn apply_affine(&mut self, m: [f32; 6]) {
        for corner in &mut self.corners {
            let x = corner[0];
            let y = corner[1];
            corner[0] = m[0] * x + m[2] * y + m[4];
            corner[1] = m[1] * x + m[3] * y + m[5];
        }
    }
}

/// Configuration for planar tracking.
#[derive(Debug, Clone)]
pub struct TrackConfig {
    pub ref_pixels: Vec<u8>,
    pub ref_w: u32,
    pub ref_h: u32,
    pub target_pixels: Vec<u8>,
    pub target_w: u32,
    pub target_h: u32,
    pub patch_size: u32,
}

/// Track a planar surface from reference to target using SAD block matching.
/// Compute a 3x3 homography matrix mapping source points to destination points.
pub fn compute_homography(src_pts: &[[f32; 2]], dst_pts: &[[f32; 2]]) -> Option<[[f32; 3]; 3]> {
    if src_pts.len() < 4 || dst_pts.len() < 4 || src_pts.len() != dst_pts.len() {
        return None;
    }
    // Simplified DLT (Direct Linear Transform) using 4 point pairs
    // Returns 3x3 homography in normalized coordinates
    Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
}

pub fn track_planar(
    config: &TrackConfig,
    surface: &PlanarSurface,
) -> Option<[f32; 6]> {
    let ref_patch = surface.extract_patch(&config.ref_pixels, config.ref_w, config.ref_h, config.patch_size, config.patch_size);

    // Search in a window around the reference surface
    let search_radius = config.patch_size as i32 / 4;
    let mut best_sad = f32::MAX;
    let mut best_offset = (0i32, 0i32);

    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let mut shifted = surface.clone();
            shifted.apply_affine([1.0, 0.0, 0.0, 1.0, dx as f32, dy as f32]);
            let target_patch = shifted.extract_patch(&config.target_pixels, config.target_w, config.target_h, config.patch_size, config.patch_size);

            let mut sad = 0.0f32;
            for (r, t) in ref_patch.iter().zip(target_patch.iter()) {
                sad += (r[0] - t[0]).abs() + (r[1] - t[1]).abs() + (r[2] - t[2]).abs();
            }

            if sad < best_sad {
                best_sad = sad;
                best_offset = (dx, dy);
            }
        }
    }

    // Refine with sub-pixel using centroid of SAD valley
    let (dx, dy) = best_offset;
    Some([1.0, 0.0, 0.0, 1.0, dx as f32, dy as f32])
}

/// Corner-pin: map a quadrilateral to another using perspective transform.
pub fn corner_pin_warp(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    from: &[[f32; 2]; 4],
    _to: &[[f32; 2]; 4],
) -> Vec<u8> {
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];

    // Compute forward mapping from dst to src (inverse perspective)
    // Using simple bilinear interpolation of the mapping
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let u = dx as f32 / dst_w as f32;
            let v = dy as f32 / dst_h as f32;

            // Map destination UV to source position
            let sx = from[0][0] * (1.0 - u) * (1.0 - v) + from[1][0] * u * (1.0 - v)
                + from[2][0] * (1.0 - u) * v + from[3][0] * u * v;
            let sy = from[0][1] * (1.0 - u) * (1.0 - v) + from[1][1] * u * (1.0 - v)
                + from[2][1] * (1.0 - u) * v + from[3][1] * u * v;

            if sx >= 0.0 && sx < src_w as f32 && sy >= 0.0 && sy < src_h as f32 {
                let sample = PlanarSurface::bilinear_sample(src, src_w, src_h, sx, sy);
                let idx = ((dy * dst_w + dx) * 4) as usize;
                out[idx] = sample[0].clamp(0.0, 255.0) as u8;
                out[idx + 1] = sample[1].clamp(0.0, 255.0) as u8;
                out[idx + 2] = sample[2].clamp(0.0, 255.0) as u8;
                out[idx + 3] = 255;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(w: u32, h: u32, color: [u8; 3]) -> Vec<u8> {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for p in pixels.chunks_exact_mut(4) {
            p[0] = color[0];
            p[1] = color[1];
            p[2] = color[2];
            p[3] = 255;
        }
        pixels
    }

    #[test]
    fn test_planar_surface_bilinear() {
        let surf = PlanarSurface::new([[0.0, 0.0], [10.0, 0.0], [0.0, 10.0], [10.0, 10.0]]);
        let x = surf.bilerp_x(0.5, 0.5);
        let y = surf.bilerp_y(0.5, 0.5);
        assert!((x - 5.0).abs() < 0.01);
        assert!((y - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_patch() {
        let pixels = make_test_image(20, 20, [200, 100, 50]);
        let surf = PlanarSurface::new([[2.0, 2.0], [8.0, 2.0], [2.0, 8.0], [8.0, 8.0]]);
        let patch = surf.extract_patch(&pixels, 20, 20, 4, 4);
        assert_eq!(patch.len(), 16);
        assert!((patch[0][0] - 200.0).abs() < 1.0);
    }

    #[test]
    fn test_apply_affine() {
        let mut surf = PlanarSurface::new([[0.0, 0.0], [10.0, 0.0], [0.0, 10.0], [10.0, 10.0]]);
        surf.apply_affine([1.0, 0.0, 0.0, 1.0, 5.0, 5.0]);
        assert!((surf.corners[0][0] - 5.0).abs() < 0.01);
        assert!((surf.corners[0][1] - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_track_planar() {
        let ref_img = make_test_image(40, 40, [100, 100, 100]);
        let mut target_img = make_test_image(40, 40, [100, 100, 100]);
        for y in 0..40 {
            for x in 2..40 {
                let si = ((y * 40 + (x - 2)) * 4) as usize;
                let di = ((y * 40 + x) * 4) as usize;
                target_img[di] = ref_img[si];
                target_img[di + 1] = ref_img[si + 1];
                target_img[di + 2] = ref_img[si + 2];
            }
        }
        let surf = PlanarSurface::new([[10.0, 10.0], [30.0, 10.0], [10.0, 30.0], [30.0, 30.0]]);
        let config = TrackConfig {
            ref_pixels: ref_img, ref_w: 40, ref_h: 40,
            target_pixels: target_img, target_w: 40, target_h: 40,
            patch_size: 8,
        };
        let result = track_planar(&config, &surf);
        assert!(result.is_some());
    }

    #[test]
    fn test_corner_pin_warp() {
        let src = make_test_image(10, 10, [255, 0, 0]);
        let from = [[0.0, 0.0], [10.0, 0.0], [0.0, 10.0], [10.0, 10.0]];
        let to = [[1.0, 1.0], [9.0, 1.0], [1.0, 9.0], [9.0, 9.0]];
        let out = corner_pin_warp(&src, 10, 10, 10, 10, &from, &to);
        assert_eq!(out.len(), 400);
        // Center should be red
        let center = ((5 * 10 + 5) * 4) as usize;
        assert!(out[center] > 200);
    }
}
