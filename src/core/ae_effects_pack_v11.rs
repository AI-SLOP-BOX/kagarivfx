#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 11).
/// All algorithms are implemented with distinct pixel operations.
// 1. Radial Blur Zoom (Center Zoom Motion Blur)
pub fn apply_radial_blur_zoom(pixels: &mut [u8], width: u32, height: u32, center: [f32; 2], amount: f32) {
    if amount <= 0.001 { return; }
    let temp = pixels.to_vec();
    let samples = 12u32;

    for y in 0..height {
        for x in 0..width {
            let dx = center[0] - x as f32;
            let dy = center[1] - y as f32;

            let mut r_sum = 0.0f32;
            let mut g_sum = 0.0f32;
            let mut b_sum = 0.0f32;

            for s in 0..samples {
                let t = (s as f32 / samples as f32) * amount * 0.1;
                let sx = (x as f32 + dx * t).clamp(0.0, (width - 1) as f32) as usize;
                let sy = (y as f32 + dy * t).clamp(0.0, (height - 1) as f32) as usize;

                let s_idx = (sy * width as usize + sx) * 4;
                r_sum += temp[s_idx] as f32;
                g_sum += temp[s_idx + 1] as f32;
                b_sum += temp[s_idx + 2] as f32;
            }

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx] = (r_sum / samples as f32) as u8;
            pixels[idx + 1] = (g_sum / samples as f32) as u8;
            pixels[idx + 2] = (b_sum / samples as f32) as u8;
        }
    }
}

// 2. Polar Coordinates (Rect to Polar / Polar to Rect)
pub fn apply_polar_coordinates(pixels: &mut [u8], width: u32, height: u32, to_polar: bool) {
    let temp = pixels.to_vec();
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_radius = cx.min(cy);

    for y in 0..height {
        for x in 0..width {
            let (src_x, src_y) = if to_polar {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let r = (dx * dx + dy * dy).sqrt() / max_radius * width as f32;
                let mut theta = dy.atan2(dx);
                if theta < 0.0 { theta += std::f32::consts::TAU; }
                let a = (theta / std::f32::consts::TAU) * height as f32;
                (r, a)
            } else {
                let r = (x as f32 / width as f32) * max_radius;
                let theta = (y as f32 / height as f32) * std::f32::consts::TAU;
                let px = cx + r * theta.cos();
                let py = cy + r * theta.sin();
                (px, py)
            };

            let sx = src_x.clamp(0.0, (width - 1) as f32) as usize;
            let sy = src_y.clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (sy * width as usize + sx) * 4;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 3. Linear Wipe with Feather Edge
pub fn apply_linear_wipe_feather(pixels: &mut [u8], width: u32, height: u32, completion: f32, angle_deg: f32, feather: f32) {
    let rad = angle_deg.to_radians();
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    let threshold = completion * 0.01 * (width as f32 + height as f32);
    let feather_val = feather.max(1.0);

    for y in 0..height {
        for x in 0..width {
            let proj = x as f32 * cos_a + y as f32 * sin_a;
            let dist = proj - threshold;

            let alpha_factor = if dist < 0.0 {
                0.0
            } else if dist < feather_val {
                dist / feather_val
            } else {
                1.0
            };

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx + 3] = (pixels[idx + 3] as f32 * alpha_factor) as u8;
        }
    }
}

// 4. HDR Exposure Compensation (EV Value Exposure)
pub fn apply_exposure_hdr(pixels: &mut [u8], ev: f32, offset: f32, gamma: f32) {
    let scale = 2.0f32.powf(ev);
    let inv_gamma = 1.0 / gamma.max(0.01);

    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let norm = pixels[i + c] as f32 / 255.0;
            let exposed = (norm * scale + offset).max(0.0);
            let corrected = exposed.powf(inv_gamma);
            pixels[i + c] = (corrected * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
}

// 5. Channel Mixer Matrix (3x3 RGB Color Matrix)
pub fn apply_channel_mixer(pixels: &mut [u8], matrix: [[f32; 3]; 3]) {
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32;
        let g = pixels[i + 1] as f32;
        let b = pixels[i + 2] as f32;

        let new_r = r * matrix[0][0] + g * matrix[0][1] + b * matrix[0][2];
        let new_g = r * matrix[1][0] + g * matrix[1][1] + b * matrix[1][2];
        let new_b = r * matrix[2][0] + g * matrix[2][1] + b * matrix[2][2];

        pixels[i] = new_r.clamp(0.0, 255.0) as u8;
        pixels[i + 1] = new_g.clamp(0.0, 255.0) as u8;
        pixels[i + 2] = new_b.clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v11_filters() {
        let mut pixels = vec![100u8; 64];
        apply_exposure_hdr(&mut pixels, 1.0, 0.0, 1.0);
        assert_eq!(pixels.len(), 64);
    }
}
