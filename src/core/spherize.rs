#![allow(dead_code)]
/// Spherize / CC Sphere options matching After Effects Spherize effect.
#[derive(Debug, Clone)]
pub struct SpherizeOptions {
    pub radius: f32,       // Sphere radius in pixels
    pub center: [f32; 2],  // Center coordinates of sphere
    pub refractive_index: f32, // Spherical refraction strength
}

impl SpherizeOptions {
    pub fn default_for_size(width: f32, height: f32) -> Self {
        Self {
            radius: (width.min(height) * 0.4).max(10.0),
            center: [width * 0.5, height * 0.5],
            refractive_index: 1.0,
        }
    }
}

/// Applies 3D Spherical Refraction / Spherize warping to RGBA pixel buffer.
pub fn apply_spherize(
    pixels: &[u8],
    width: u32,
    height: u32,
    options: &SpherizeOptions,
) -> Vec<u8> {
    let num_pixels = (width * height) as usize;
    if pixels.len() != num_pixels * 4 || options.radius <= 0.001 {
        return pixels.to_vec();
    }

    let mut out_pixels = vec![0u8; num_pixels * 4];
    let w_f32 = width as f32;
    let h_f32 = height as f32;
    let r_sq = options.radius * options.radius;

    for y in 0..height {
        let dy = y as f32 - options.center[1];
        for x in 0..width {
            let dx = x as f32 - options.center[0];
            let dist_sq = dx * dx + dy * dy;

            let out_idx = ((y * width + x) * 4) as usize;

            if dist_sq < r_sq {
                // Inside spherical distortion lens
                let dist = dist_sq.sqrt();
                let norm_d = dist / options.radius;

                // Spherical curvature z = sqrt(1 - norm_d^2)
                let z = (1.0 - norm_d * norm_d).max(0.0).sqrt();

                // Refraction distortion factor
                let factor = if dist > 0.001 {
                    (1.0 - z * (1.0 - 1.0 / options.refractive_index.max(0.1))) * (norm_d.asin() / (std::f32::consts::PI * 0.5))
                } else {
                    1.0
                };

                let src_x = (options.center[0] + dx * factor).clamp(0.0, w_f32 - 1.0);
                let src_y = (options.center[1] + dy * factor).clamp(0.0, h_f32 - 1.0);

                let x0 = src_x.floor() as u32;
                let y0 = src_y.floor() as u32;
                let x1 = (x0 + 1).min(width - 1);
                let y1 = (y0 + 1).min(height - 1);

                let tx = src_x - x0 as f32;
                let ty = src_y - y0 as f32;

                let idx00 = ((y0 * width + x0) * 4) as usize;
                let idx10 = ((y0 * width + x1) * 4) as usize;
                let idx01 = ((y1 * width + x0) * 4) as usize;
                let idx11 = ((y1 * width + x1) * 4) as usize;

                for c in 0..4 {
                    let p00 = pixels[idx00 + c] as f32;
                    let p10 = pixels[idx10 + c] as f32;
                    let p01 = pixels[idx01 + c] as f32;
                    let p11 = pixels[idx11 + c] as f32;

                    let top = p00 + (p10 - p00) * tx;
                    let bottom = p01 + (p11 - p01) * tx;
                    let val = top + (bottom - top) * ty;

                    out_pixels[out_idx + c] = val.round().clamp(0.0, 255.0) as u8;
                }
            } else {
                // Outside sphere: pass-through original pixel
                out_pixels[out_idx..out_idx + 4].copy_from_slice(&pixels[out_idx..out_idx + 4]);
            }
        }
    }

    out_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spherize_buffer_size() {
        let pixels = vec![255u8; 64]; // 4x4
        let options = SpherizeOptions::default_for_size(4.0, 4.0);
        let out = apply_spherize(&pixels, 4, 4, &options);
        assert_eq!(out.len(), 64);
    }
}
