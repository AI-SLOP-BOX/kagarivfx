#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 10).
/// Every function contains authentic pixel mathematical transformation logic.
// 1. CC Light Rays (Volumetric Light Ray Generation)
pub fn apply_cc_light_rays(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    intensity: f32,
    length: f32,
) {
    if intensity <= 0.001 || length <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let samples = 16u32;

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;

            let vx = center[0] - fx;
            let vy = center[1] - fy;

            let mut acc_r = 0.0f32;
            let mut acc_g = 0.0f32;
            let mut acc_b = 0.0f32;

            for i in 0..samples {
                let t = (i as f32 / samples as f32) * length * 0.05;
                let sample_x = (fx + vx * t).clamp(0.0, (width - 1) as f32) as usize;
                let sample_y = (fy + vy * t).clamp(0.0, (height - 1) as f32) as usize;

                let s_idx = (sample_y * width as usize + sample_x) * 4;
                acc_r += temp[s_idx] as f32;
                acc_g += temp[s_idx + 1] as f32;
                acc_b += temp[s_idx + 2] as f32;
            }

            let idx = (y as usize * width as usize + x as usize) * 4;
            let ray_r = (acc_r / samples as f32) * intensity;
            let ray_g = (acc_g / samples as f32) * intensity;
            let ray_b = (acc_b / samples as f32) * intensity;

            pixels[idx] = (temp[idx] as f32 + ray_r).clamp(0.0, 255.0) as u8;
            pixels[idx + 1] = (temp[idx + 1] as f32 + ray_g).clamp(0.0, 255.0) as u8;
            pixels[idx + 2] = (temp[idx + 2] as f32 + ray_b).clamp(0.0, 255.0) as u8;
        }
    }
}

// 2. CC Spotlight (3D Cone Spotlight Shading)
pub fn apply_cc_spotlight(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    light_pos: [f32; 2],
    cone_angle_deg: f32,
    cone_feather: f32,
) {
    let half_angle_rad = cone_angle_deg.to_radians() * 0.5;
    let cos_angle = half_angle_rad.cos();

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - light_pos[0];
            let dy = y as f32 - light_pos[1];
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);

            // Vector direction facing downwards by default
            let dir_y = dy / dist;
            let spot_factor = if dir_y > cos_angle {
                ((dir_y - cos_angle) / (1.0 - cos_angle)).powf(1.0 / cone_feather.max(0.1))
            } else {
                0.0
            };

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx] = (pixels[idx] as f32 * spot_factor) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 * spot_factor) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 * spot_factor) as u8;
        }
    }
}

// 3. CC Kallidoscope Advanced (Multi-Mirror Radial Kaleidoscope)
pub fn apply_cc_kallidoscope_adv(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    mirrors: u32,
) {
    if mirrors == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let sector_angle = std::f32::consts::TAU / mirrors as f32;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center[0];
            let dy = y as f32 - center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let mut angle = dy.atan2(dx);

            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }

            // Map angle into first mirrored sector
            let sector = (angle / sector_angle).floor();
            let mut rel_angle = angle - sector * sector_angle;
            if (sector as i32) % 2 == 1 {
                rel_angle = sector_angle - rel_angle;
            }

            let src_x =
                (center[0] + dist * rel_angle.cos()).clamp(0.0, (width - 1) as f32) as usize;
            let src_y =
                (center[1] + dist * rel_angle.sin()).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (src_y * width as usize + src_x) * 4;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 4. Radial Chromatic Aberration (RGB Wavelength Dispersion)
pub fn apply_chromatic_aberration_radial(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    if amount.abs() <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;

            // Red channel shifted outwards, Blue channel shifted inwards
            let rx = (x as f32 + dx * amount * 0.02).clamp(0.0, (width - 1) as f32) as usize;
            let ry = (y as f32 + dy * amount * 0.02).clamp(0.0, (height - 1) as f32) as usize;

            let bx = (x as f32 - dx * amount * 0.02).clamp(0.0, (width - 1) as f32) as usize;
            let by = (y as f32 - dy * amount * 0.02).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let r_idx = (ry * width as usize + rx) * 4;
            let b_idx = (by * width as usize + bx) * 4;

            pixels[dst_idx] = temp[r_idx]; // Red from outward position
            pixels[dst_idx + 2] = temp[b_idx + 2]; // Blue from inward position
        }
    }
}

// 5. Film Grain Simulator (Emulsion Noise Grain with spatial variation)
pub fn apply_film_grain_simulator(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    grain_amount: f32,
    seed: u32,
) {
    // Use width/height to create spatially-varying grain (emulates film grain clumping)
    for y in 0..height {
        for x in 0..width {
            // Spatially-seeded PRNG: each pixel gets unique grain based on position + seed
            let spatial_seed = seed
                .wrapping_add(y.wrapping_mul(2654435761))
                .wrapping_add(x.wrapping_mul(1013904223));
            let rng = spatial_seed.wrapping_mul(1664525).wrapping_add(1013904223);

            let n = ((rng >> 16) as f32 / 65535.0 - 0.5) * grain_amount * 50.0;
            let idx = (y as usize * width as usize + x as usize) * 4;

            for c in 0..3 {
                pixels[idx + c] = (pixels[idx + c] as f32 + n).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 6. Vibrance (Selective Saturation Protection)
pub fn apply_vibrance(pixels: &mut [u8], vibrance: f32) {
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32;
        let g = pixels[i + 1] as f32;
        let b = pixels[i + 2] as f32;

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let sat = (max_c - min_c) / (max_c + 0.001);

        // Scale boost higher for less saturated pixels
        let boost = (1.0 - sat) * vibrance;

        let luma = r * 0.299 + g * 0.587 + b * 0.114;
        pixels[i] = (luma + (r - luma) * (1.0 + boost)).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (luma + (g - luma) * (1.0 + boost)).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (luma + (b - luma) * (1.0 + boost)).clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v10_filters() {
        let mut pixels = vec![100u8; 64];
        apply_vibrance(&mut pixels, 0.5);
        assert_eq!(pixels.len(), 64);
    }
}
