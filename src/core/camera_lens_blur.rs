//! Camera Lens Blur Effect Engine (AE Parity - Blur & Sharpen > Camera Lens Blur).
//!
//! Simulates physical camera lens aperture defocus with polygonal iris shapes
//! (3 to 16 blades: Triangle, Quad, Pentagonal, Hexagonal, Octagonal, etc.),
//! blade curvature/roundness, iris rotation, and specular highlight boost.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraLensBlurParams {
    pub blur_radius: f32,       // 0.0..100.0
    pub iris_blades: u32,       // 3..16
    pub iris_rotation_deg: f32, // 0.0..360.0
    pub iris_roundness: f32,    // 0.0..100.0%
    pub highlight_gain: f32,    // 0.0..5.0
    pub highlight_threshold: f32,// 0.0..1.0
}

impl Default for CameraLensBlurParams {
    fn default() -> Self {
        Self {
            blur_radius: 15.0,
            iris_blades: 6, // Hexagon bokeh
            iris_rotation_deg: 0.0,
            iris_roundness: 0.0,
            highlight_gain: 1.5,
            highlight_threshold: 0.8,
        }
    }
}

/// Generates a normalized 2D polygonal iris aperture convolution kernel.
pub fn generate_iris_kernel(
    radius: f32,
    blades: u32,
    rotation_deg: f32,
    roundness_pct: f32,
) -> (Vec<f32>, i32) {
    let r = radius.max(1.0);
    let k_size = (r.ceil() as i32) * 2 + 1;
    let half = k_size / 2;
    let mut kernel = vec![0.0f32; (k_size * k_size) as usize];

    let n = blades.clamp(3, 16) as f32;
    let rot = rotation_deg.to_radians();
    let roundness = (roundness_pct / 100.0).clamp(0.0, 1.0);

    let mut sum = 0.0f32;

    for ky in -half..=half {
        for kx in -half..=half {
            let x = kx as f32;
            let y = ky as f32;
            let dist = (x * x + y * y).sqrt();

            if dist > r + 0.5 {
                continue;
            }

            // Angle in rotated aperture space
            let angle = y.atan2(x) - rot;
            // Angle within sector
            let sector_angle = std::f32::consts::PI * 2.0 / n;
            let mut rel_angle = angle.rem_euclid(sector_angle) - sector_angle * 0.5;
            if rel_angle < 0.0 {
                rel_angle = -rel_angle;
            }

            // Distance to polygon edge
            let poly_dist = dist * (rel_angle.cos() / (sector_angle * 0.5).cos());

            // Blend between pure polygon and circle based on roundness
            let effective_dist = poly_dist * (1.0 - roundness) + dist * roundness;

            if effective_dist <= r {
                let weight = (1.0 - (effective_dist - (r - 0.75)).max(0.0)).clamp(0.0, 1.0);
                let idx = ((ky + half) * k_size + (kx + half)) as usize;
                kernel[idx] = weight;
                sum += weight;
            }
        }
    }

    if sum > 0.0 {
        for val in kernel.iter_mut() {
            *val /= sum;
        }
    }

    (kernel, half)
}

/// Applies Camera Lens Blur with iris bokeh shape and highlight blooming.
pub fn apply_camera_lens_blur(
    src: &[u8],
    width: u32,
    height: u32,
    params: &CameraLensBlurParams,
) -> Vec<u8> {
    if src.len() != (width * height * 4) as usize || width == 0 || height == 0 || params.blur_radius < 0.5 {
        return src.to_vec();
    }

    let (kernel, half) = generate_iris_kernel(
        params.blur_radius,
        params.iris_blades,
        params.iris_rotation_deg,
        params.iris_roundness,
    );

    let k_size = half * 2 + 1;
    let mut dst = vec![0u8; src.len()];

    let gain = params.highlight_gain;
    let thresh = params.highlight_threshold * 255.0;

    for y in 0..height {
        let iy = y as i32;
        for x in 0..width {
            let ix = x as i32;

            let mut acc_r = 0.0f32;
            let mut acc_g = 0.0f32;
            let mut acc_b = 0.0f32;
            let mut acc_a = 0.0f32;

            for ky in -half..=half {
                let sy = (iy + ky).clamp(0, height as i32 - 1) as usize;
                let k_row = (ky + half) * k_size;

                for kx in -half..=half {
                    let sx = (ix + kx).clamp(0, width as i32 - 1) as usize;
                    let k_idx = (k_row + (kx + half)) as usize;
                    let kw = kernel[k_idx];

                    if kw <= 0.0 {
                        continue;
                    }

                    let s_idx = (sy * width as usize + sx) * 4;
                    let r = src[s_idx] as f32;
                    let g = src[s_idx + 1] as f32;
                    let b = src[s_idx + 2] as f32;
                    let a = src[s_idx + 3] as f32;

                    // Specular highlight boost
                    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                    let boost = if luma > thresh && gain > 1.0 {
                        1.0 + (luma - thresh) / (255.0 - thresh + 0.001) * (gain - 1.0)
                    } else {
                        1.0
                    };

                    acc_r += r * boost * kw;
                    acc_g += g * boost * kw;
                    acc_b += b * boost * kw;
                    acc_a += a * kw;
                }
            }

            let d_idx = ((y * width + x) * 4) as usize;
            dst[d_idx] = acc_r.clamp(0.0, 255.0).round() as u8;
            dst[d_idx + 1] = acc_g.clamp(0.0, 255.0).round() as u8;
            dst[d_idx + 2] = acc_b.clamp(0.0, 255.0).round() as u8;
            dst[d_idx + 3] = acc_a.clamp(0.0, 255.0).round() as u8;
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iris_kernel_generation() {
        let (kernel, half) = generate_iris_kernel(5.0, 6, 0.0, 0.0);
        assert_eq!(half, 5);
        let sum: f32 = kernel.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_camera_lens_blur_uniform() {
        let w = 8u32;
        let h = 8u32;
        let src = vec![100u8; (w * h * 4) as usize];
        let params = CameraLensBlurParams {
            blur_radius: 2.0,
            iris_blades: 6,
            iris_rotation_deg: 0.0,
            iris_roundness: 0.0,
            highlight_gain: 1.0,
            highlight_threshold: 0.8,
        };

        let dst = apply_camera_lens_blur(&src, w, h, &params);
        assert_eq!(dst.len(), src.len());
        assert!((dst[0] as i32 - 100).abs() <= 1);
    }
}
