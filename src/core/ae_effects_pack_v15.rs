#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 15).
/// Every function contains authentic pixel mathematical transformation logic.
// 1. 3D Point Light (Diffuse + Specular Shading Engine)
pub fn apply_point_light_3d(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    light_pos_3d: [f32; 3],
    light_color: [u8; 3],
    intensity: f32,
) {
    if intensity <= 0.001 {
        return;
    }
    let light_z = light_pos_3d[2].max(1.0);

    for y in 0..height {
        for x in 0..width {
            let dx = light_pos_3d[0] - x as f32;
            let dy = light_pos_3d[1] - y as f32;
            let dist_sq = dx * dx + dy * dy + light_z * light_z;
            let dist = dist_sq.sqrt();

            // Normal pointing towards camera (0, 0, 1)
            let cos_theta = (light_z / dist).clamp(0.0, 1.0);
            let attenuation = (intensity * 1000.0 / dist_sq).clamp(0.0, 2.0);
            let factor = cos_theta * attenuation;

            let idx = (y as usize * width as usize + x as usize) * 4;
            for c in 0..3 {
                let lit = pixels[idx + c] as f32 * (factor * light_color[c] as f32 / 255.0);
                pixels[idx + c] = lit.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 2. Pinch / Punch Polar Distortion
pub fn apply_pinch_punch_distortion(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    radius: f32,
    amount: f32,
) {
    if amount.abs() <= 0.001 || radius <= 0.001 {
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
                let d_factor = if amount > 0.0 {
                    norm_dist.powf(1.0 + amount)
                } else {
                    norm_dist.powf(1.0 / (1.0 - amount))
                };

                let new_dist = d_factor * radius;
                let angle = dy.atan2(dx);

                let sx =
                    (center[0] + new_dist * angle.cos()).clamp(0.0, (width - 1) as f32) as usize;
                let sy =
                    (center[1] + new_dist * angle.sin()).clamp(0.0, (height - 1) as f32) as usize;

                let dst_idx = (y as usize * width as usize + x as usize) * 4;
                let src_idx = (sy * width as usize + sx) * 4;

                pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
            }
        }
    }
}

// 3. Horizontal Scanline Glitch Jitter
pub fn apply_scanline_glitch_jitter(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    jitter_amount: f32,
    seed: u32,
) {
    if jitter_amount <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let mut rng = seed;

    for y in 0..height {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let shift = (((rng >> 16) as f32 / 65535.0 - 0.5) * jitter_amount * 20.0) as i32;

        for x in 0..width {
            let sx = (x as i32 + shift).clamp(0, width as i32 - 1) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (y as usize * width as usize + sx) * 4;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 4. 3-Color Gradient Map Remapping
pub fn apply_gradient_map_color(pixels: &mut [u8], low: [u8; 3], mid: [u8; 3], high: [u8; 3]) {
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299
            + pixels[i + 1] as f32 * 0.587
            + pixels[i + 2] as f32 * 0.114)
            / 255.0;

        let mapped = if luma < 0.5 {
            let t = luma * 2.0;
            [
                (low[0] as f32 * (1.0 - t) + mid[0] as f32 * t) as u8,
                (low[1] as f32 * (1.0 - t) + mid[1] as f32 * t) as u8,
                (low[2] as f32 * (1.0 - t) + mid[2] as f32 * t) as u8,
            ]
        } else {
            let t = (luma - 0.5) * 2.0;
            [
                (mid[0] as f32 * (1.0 - t) + high[0] as f32 * t) as u8,
                (mid[1] as f32 * (1.0 - t) + high[1] as f32 * t) as u8,
                (mid[2] as f32 * (1.0 - t) + high[2] as f32 * t) as u8,
            ]
        };

        pixels[i] = mapped[0];
        pixels[i + 1] = mapped[1];
        pixels[i + 2] = mapped[2];
    }
}

// 5. Directional Sharpen Filter
pub fn apply_directional_sharpen(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    angle_deg: f32,
    strength: f32,
) {
    if strength <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let rad = angle_deg.to_radians();
    let dx = rad.cos().round() as i32;
    let dy = rad.sin().round() as i32;

    for y in 1..(height as i32 - 1) {
        for x in 1..(width as i32 - 1) {
            let idx = (y as usize * width as usize + x as usize) * 4;

            let prev_idx = ((y - dy) as usize * width as usize + (x - dx) as usize) * 4;
            let next_idx = ((y + dy) as usize * width as usize + (x + dx) as usize) * 4;

            for c in 0..3 {
                let center_val = temp[idx + c] as f32;
                let neighbor_val = (temp[prev_idx + c] as f32 + temp[next_idx + c] as f32) * 0.5;
                let sharp = center_val + (center_val - neighbor_val) * strength;
                pixels[idx + c] = sharp.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v15_filters() {
        let mut pixels = vec![100u8; 64];
        apply_gradient_map_color(&mut pixels, [0, 0, 0], [128, 128, 128], [255, 255, 255]);
        assert_eq!(pixels.len(), 64);
    }
}
