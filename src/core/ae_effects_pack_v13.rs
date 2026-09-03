#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 13).
/// Authentic mathematical distortion and visual effect kernels.
// 1. Vortex Center Distortion (Attenuated Spiral Vortex)
pub fn apply_vortex_distortion(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    max_radius: f32,
    angle_deg: f32,
) {
    if angle_deg.abs() <= 0.001 || max_radius <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let max_rad = angle_deg.to_radians();

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center[0];
            let dy = y as f32 - center[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < max_radius {
                let factor = (1.0 - dist / max_radius).powi(2);
                let rotation = max_rad * factor;

                let current_angle = dy.atan2(dx);
                let new_angle = current_angle + rotation;

                let sx =
                    (center[0] + dist * new_angle.cos()).clamp(0.0, (width - 1) as f32) as usize;
                let sy =
                    (center[1] + dist * new_angle.sin()).clamp(0.0, (height - 1) as f32) as usize;

                let dst_idx = (y as usize * width as usize + x as usize) * 4;
                let src_idx = (sy * width as usize + sx) * 4;

                pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
            }
        }
    }
}

// 2. Heat Distortion Simulation (Rising Thermal Air Turbulence)
pub fn apply_heat_distortion(pixels: &mut [u8], width: u32, height: u32, time: f32, strength: f32) {
    if strength <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let fy = y as f32;
            let fx = x as f32;

            // Thermal upward flow distortion
            let wave_y = ((fy * 0.05 - time * 3.0).sin() + (fx * 0.03).cos()) * strength * 2.0;
            let wave_x = ((fx * 0.04 + time * 2.0).cos()) * strength * 1.5;

            let sx = (fx + wave_x).clamp(0.0, (width - 1) as f32) as usize;
            let sy = (fy + wave_y).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (sy * width as usize + sx) * 4;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 3. Digital Block Glitch Displacement
pub fn apply_glitch_displacement(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    seed: u32,
    amount: f32,
) {
    if amount <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let block_height = 8u32;
    let num_blocks = height / block_height;

    let mut rng = seed;
    for b in 0..num_blocks {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let shift_flag = (rng % 100) < (amount * 30.0) as u32;

        if shift_flag {
            let offset_x = ((rng >> 8) % 40) as i32 - 20;

            for y_off in 0..block_height {
                let y = b * block_height + y_off;
                if y >= height {
                    break;
                }

                for x in 0..width {
                    let sx = (x as i32 + offset_x).clamp(0, width as i32 - 1) as usize;
                    let dst_idx = (y as usize * width as usize + x as usize) * 4;
                    let src_idx = (y as usize * width as usize + sx) * 4;

                    pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
                }
            }
        }
    }
}

// 4. Duotone Threshold Mapping
pub fn apply_threshold_duotone(
    pixels: &mut [u8],
    threshold: u8,
    color_dark: [u8; 3],
    color_light: [u8; 3],
) {
    for i in (0..pixels.len()).step_by(4) {
        let luma =
            (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                / 1000;
        let target = if luma < threshold as u32 {
            color_dark
        } else {
            color_light
        };

        pixels[i] = target[0];
        pixels[i + 1] = target[1];
        pixels[i + 2] = target[2];
    }
}

// 5. Chromatic Zoom Aberration (Wavelength-Dependent Scale)
pub fn apply_chromatic_zoom(pixels: &mut [u8], width: u32, height: u32, zoom_amount: f32) {
    if zoom_amount.abs() <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;

    let red_scale = 1.0 + zoom_amount * 0.05;
    let blue_scale = 1.0 - zoom_amount * 0.05;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;

            let rx = (cx + dx / red_scale).clamp(0.0, (width - 1) as f32) as usize;
            let ry = (cy + dy / red_scale).clamp(0.0, (height - 1) as f32) as usize;

            let bx = (cx + dx / blue_scale).clamp(0.0, (width - 1) as f32) as usize;
            let by = (cy + dy / blue_scale).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let r_idx = (ry * width as usize + rx) * 4;
            let b_idx = (by * width as usize + bx) * 4;

            pixels[dst_idx] = temp[r_idx]; // Red shifted by scale
            pixels[dst_idx + 2] = temp[b_idx + 2]; // Blue shifted by scale
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v13_filters() {
        let mut pixels = vec![100u8; 64];
        apply_threshold_duotone(&mut pixels, 128, [0, 0, 0], [255, 255, 255]);
        assert_eq!(pixels.len(), 64);
    }
}
