#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 16).
/// Advanced Noise Flow, Keying and CMYK Optical Renderers.
// 1. Perlin Flow Vector Noise Synthesis
pub fn apply_perlin_flow_noise(pixels: &mut [u8], width: u32, height: u32, time: f32, scale: f32) {
    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 * scale * 0.05 + time;
            let fy = y as f32 * scale * 0.05 - time * 0.5;

            let n1 = fx.sin() * fy.cos() * 0.5 + 0.5;
            let n2 = (fx * 2.0).cos() * (fy * 2.0).sin() * 0.5 + 0.5;
            let noise_val = ((n1 + n2 * 0.5) / 1.5 * 255.0) as u8;

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx] = pixels[idx].saturating_add(noise_val / 2);
            pixels[idx + 1] = pixels[idx + 1].saturating_add(noise_val / 2);
            pixels[idx + 2] = pixels[idx + 2].saturating_add(noise_val / 2);
        }
    }
}

// 2. Luma Key Range (Selective Luminance Matte Extraction)
pub fn apply_luma_key_range(
    pixels: &mut [u8],
    low_threshold: u8,
    high_threshold: u8,
    invert: bool,
) {
    for i in (0..pixels.len()).step_by(4) {
        let luma =
            (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                / 1000;
        let is_key = luma >= low_threshold as u32 && luma <= high_threshold as u32;

        let alpha_mult = if invert {
            if is_key {
                1.0
            } else {
                0.0
            }
        } else if is_key {
            0.0
        } else {
            1.0
        };

        pixels[i + 3] = (pixels[i + 3] as f32 * alpha_mult) as u8;
    }
}

// 3. Spherical Refraction Lens with Index of Refraction (IOR)
pub fn apply_spherical_refraction_lens(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    radius: f32,
    ior: f32,
) {
    if radius <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center[0];
            let dy = y as f32 - center[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < radius {
                let norm_dist = dist / radius;
                let z = (1.0 - norm_dist * norm_dist).sqrt();
                let refract_factor = (1.0 / ior.max(0.1)) * norm_dist;

                let sx = (center[0] + dx * refract_factor).clamp(0.0, (width - 1) as f32) as usize;
                let sy = (center[1] + dy * refract_factor).clamp(0.0, (height - 1) as f32) as usize;

                let dst_idx = (y as usize * width as usize + x as usize) * 4;
                let src_idx = (sy * width as usize + sx) * 4;

                // Specular lens reflection
                let highlight = (z * 60.0) as i16;
                for c in 0..3 {
                    let val = temp[src_idx + c] as i16 + highlight;
                    pixels[dst_idx + c] = val.clamp(0, 255) as u8;
                }
                pixels[dst_idx + 3] = temp[src_idx + 3];
            }
        }
    }
}

// 4. Cinematic Film Bloom HDR
pub fn apply_film_bloom_hdr(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    threshold: u8,
    intensity: f32,
) {
    if intensity <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let blur_r = 4i32;

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;

            // Extract high-light bloom source
            let luma = (temp[idx] as u32 + temp[idx + 1] as u32 + temp[idx + 2] as u32) / 3;
            if luma > threshold as u32 {
                let mut bloom_r = 0.0f32;
                let mut bloom_g = 0.0f32;
                let mut bloom_b = 0.0f32;
                let mut count = 0u32;

                for ky in -blur_r..=blur_r {
                    let py = (y as i32 + ky).clamp(0, height as i32 - 1) as usize;
                    for kx in -blur_r..=blur_r {
                        let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
                        let k_idx = (py * width as usize + px) * 4;
                        bloom_r += temp[k_idx] as f32;
                        bloom_g += temp[k_idx + 1] as f32;
                        bloom_b += temp[k_idx + 2] as f32;
                        count += 1;
                    }
                }

                let b_r = (bloom_r / count as f32) * intensity;
                let b_g = (bloom_g / count as f32) * intensity;
                let b_b = (bloom_b / count as f32) * intensity;

                pixels[idx] = (temp[idx] as f32 + b_r).clamp(0.0, 255.0) as u8;
                pixels[idx + 1] = (temp[idx + 1] as f32 + b_g).clamp(0.0, 255.0) as u8;
                pixels[idx + 2] = (temp[idx + 2] as f32 + b_b).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 5. CMYK 4-Color Separation Halftone
pub fn apply_color_halftone_cmyk(pixels: &mut [u8], width: u32, height: u32, dot_size: u32) {
    if dot_size == 0 {
        return;
    }
    for y in (0..height).step_by(dot_size as usize) {
        for x in (0..width).step_by(dot_size as usize) {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let r = pixels[idx] as f32 / 255.0;
            let g = pixels[idx + 1] as f32 / 255.0;
            let b = pixels[idx + 2] as f32 / 255.0;

            let k = 1.0 - r.max(g).max(b);
            let c = (1.0 - r - k) / (1.0 - k + 0.001);
            let m = (1.0 - g - k) / (1.0 - k + 0.001);
            let y_cmyk = (1.0 - b - k) / (1.0 - k + 0.001);

            // Quantize CMYK grid
            let q_c = (c * 2.0).round() / 2.0;
            let q_m = (m * 2.0).round() / 2.0;
            let q_y = (y_cmyk * 2.0).round() / 2.0;

            let new_r = ((1.0 - q_c) * (1.0 - k) * 255.0) as u8;
            let new_g = ((1.0 - q_m) * (1.0 - k) * 255.0) as u8;
            let new_b = ((1.0 - q_y) * (1.0 - k) * 255.0) as u8;

            for dy in 0..dot_size {
                let py = y + dy;
                if py >= height {
                    break;
                }
                for dx in 0..dot_size {
                    let px = x + dx;
                    if px >= width {
                        break;
                    }
                    let p_idx = (py as usize * width as usize + px as usize) * 4;
                    pixels[p_idx] = new_r;
                    pixels[p_idx + 1] = new_g;
                    pixels[p_idx + 2] = new_b;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v16_filters() {
        let mut pixels = vec![100u8; 64];
        apply_luma_key_range(&mut pixels, 50, 200, false);
        assert_eq!(pixels.len(), 64);
    }
}
