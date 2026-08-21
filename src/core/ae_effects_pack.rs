#![allow(dead_code)]
/// Pack of 20 Essential Adobe After Effects Effects & Filters.
// 1. Fast Box Blur (Separable 2-pass: Horizontal then Vertical)
pub fn apply_fast_box_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 || width == 0 || height == 0 { return; }
    let r = radius as i32;

    // --- Horizontal pass: pixels -> temp_h ---
    let mut temp_h = vec![0u8; pixels.len()];
    for y in 0..height {
        for x in 0..width as i32 {
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for dx in -r..=r {
                let px = (x + dx).clamp(0, width as i32 - 1) as u32;
                let idx = ((y * width + px) * 4) as usize;
                for c in 0..4 { acc[c] += pixels[idx + c] as f32; }
                count += 1.0;
            }
            let out_idx = ((y * width + x as u32) * 4) as usize;
            for c in 0..4 { temp_h[out_idx + c] = (acc[c] / count).round() as u8; }
        }
    }

    // --- Vertical pass: temp_h -> pixels ---
    for y in 0..height as i32 {
        for x in 0..width {
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for dy in -r..=r {
                let py = (y + dy).clamp(0, height as i32 - 1) as u32;
                let idx = ((py * width + x) * 4) as usize;
                for c in 0..4 { acc[c] += temp_h[idx + c] as f32; }
                count += 1.0;
            }
            let out_idx = ((y as u32 * width + x) * 4) as usize;
            for c in 0..4 { pixels[out_idx + c] = (acc[c] / count).round() as u8; }
        }
    }
}


// 2. Directional Blur
pub fn apply_directional_blur(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32, length: f32) {
    if length <= 0.01 { return; }
    let rad = angle_deg.to_radians();
    let dx = rad.sin();
    let dy = -rad.cos();

    let temp = pixels.to_vec();
    let samples = (length as usize).max(1);

    for y in 0..height {
        for x in 0..width {
            let mut acc = [0f32; 4];
            for s in 0..samples {
                let offset = (s as f32) - (samples as f32 * 0.5);
                let sx = (x as f32 + dx * offset).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (y as f32 + dy * offset).clamp(0.0, height as f32 - 1.0) as u32;
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 { acc[c] += temp[idx + c] as f32; }
            }
            let out_idx = ((y * width + x) * 4) as usize;
            for c in 0..4 { pixels[out_idx + c] = (acc[c] / samples as f32).round() as u8; }
        }
    }
}

// 3. Radial Blur (Spin)
pub fn apply_radial_blur(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    if amount.abs() < 0.01 { return; }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    let steps = 8;
    for y in 0..height {
        for x in 0..width {
            let mut acc = [0f32; 4];
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let base_angle = ry.atan2(rx);
            let dist = (rx * rx + ry * ry).sqrt();

            for s in 0..steps {
                let t = (s as f32 / steps as f32 - 0.5) * amount * 0.05;
                let a = base_angle + t;
                let sx = (cx + a.cos() * dist).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + a.sin() * dist).clamp(0.0, height as f32 - 1.0) as u32;
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 { acc[c] += temp[idx + c] as f32; }
            }
            let out_idx = ((y * width + x) * 4) as usize;
            for c in 0..4 { pixels[out_idx + c] = (acc[c] / steps as f32).round() as u8; }
        }
    }
}

// 4. Unsharp Mask
pub fn apply_unsharp_mask(pixels: &mut [u8], width: u32, height: u32, amount: f32, radius: u32) {
    let mut blurred = pixels.to_vec();
    apply_fast_box_blur(&mut blurred, width, height, radius);

    let k = amount * 0.01;
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let orig = pixels[i + c] as f32;
            let blur = blurred[i + c] as f32;
            pixels[i + c] = (orig + (orig - blur) * k).clamp(0.0, 255.0) as u8;
        }
    }
}

// 5. Sharpen
pub fn apply_sharpen(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    apply_unsharp_mask(pixels, width, height, amount, 1);
}

// 6. Glow
pub fn apply_glow(pixels: &mut [u8], width: u32, height: u32, threshold: f32, radius: u32, intensity: f32) {
    let num_bytes = pixels.len();
    let mut glow_map = vec![0u8; num_bytes];

    let thresh_u8 = (threshold * 255.0) as u8;
    for i in (0..num_bytes).step_by(4) {
        let luma = (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114) / 1000;
        if luma as u8 >= thresh_u8 {
            glow_map[i] = pixels[i];
            glow_map[i + 1] = pixels[i + 1];
            glow_map[i + 2] = pixels[i + 2];
            glow_map[i + 3] = pixels[i + 3];
        }
    }

    apply_fast_box_blur(&mut glow_map, width, height, radius);

    for i in (0..num_bytes).step_by(4) {
        for c in 0..3 {
            let base = pixels[i + c] as f32;
            let g = glow_map[i + c] as f32 * intensity;
            pixels[i + c] = (base + g).clamp(0.0, 255.0) as u8;
        }
    }
}

// 7. Drop Shadow
pub fn apply_drop_shadow(pixels: &mut [u8], width: u32, height: u32, distance: f32, angle_deg: f32, softness: u32, shadow_color: [u8; 4]) {
    let rad = angle_deg.to_radians();
    let dx = (rad.sin() * distance).round() as i32;
    let dy = (-rad.cos() * distance).round() as i32;

    let mut shadow_buf = vec![0u8; pixels.len()];

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let alpha = pixels[idx + 3];
            if alpha > 0 {
                let sx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                let s_idx = ((sy * width + sx) * 4) as usize;
                shadow_buf[s_idx] = shadow_color[0];
                shadow_buf[s_idx + 1] = shadow_color[1];
                shadow_buf[s_idx + 2] = shadow_color[2];
                shadow_buf[s_idx + 3] = ((alpha as f32 * (shadow_color[3] as f32 / 255.0)).round()) as u8;
            }
        }
    }

    apply_fast_box_blur(&mut shadow_buf, width, height, softness);

    // Composite original over shadow
    for i in (0..pixels.len()).step_by(4) {
        let fg_a = pixels[i + 3] as f32 / 255.0;
        let bg_a = shadow_buf[i + 3] as f32 / 255.0;
        let out_a = fg_a + bg_a * (1.0 - fg_a);

        if out_a > 0.0 {
            for c in 0..3 {
                let fg_c = pixels[i + c] as f32 / 255.0;
                let bg_c = shadow_buf[i + c] as f32 / 255.0;
                let out_c = (fg_c * fg_a + bg_c * bg_a * (1.0 - fg_a)) / out_a;
                pixels[i + c] = (out_c * 255.0).round() as u8;
            }
            pixels[i + 3] = (out_a * 255.0).round() as u8;
        }
    }
}

// 8. CC Radial Fast Blur
pub fn apply_radial_fast_blur(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    apply_radial_blur(pixels, width, height, amount * 1.5);
}

// 9. Simple Choker
pub fn apply_simple_choker(pixels: &mut [u8], choke_amount: f32) {
    let k = 1.0 - (choke_amount * 0.01).clamp(-1.0, 1.0);
    for i in (3..pixels.len()).step_by(4) {
        let a = pixels[i] as f32 / 255.0;
        pixels[i] = (a * k * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

// 10. Matte Choker
pub fn apply_matte_choker(pixels: &mut [u8], choke_amount: f32, gray_level: f32) {
    apply_simple_choker(pixels, choke_amount);
    let thresh = (gray_level * 255.0) as u8;
    for i in (3..pixels.len()).step_by(4) {
        if pixels[i] < thresh { pixels[i] = 0; }
    }
}

// 11. Tint
pub fn apply_tint(pixels: &mut [u8], black_to: [u8; 3], white_to: [u8; 3], amount: f32) {
    let k = amount.clamp(0.0, 1.0);
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299 + pixels[i + 1] as f32 * 0.587 + pixels[i + 2] as f32 * 0.114) / 255.0;
        for c in 0..3 {
            let tinted = black_to[c] as f32 + (white_to[c] as f32 - black_to[c] as f32) * luma;
            let orig = pixels[i + c] as f32;
            pixels[i + c] = (orig + (tinted - orig) * k).round() as u8;
        }
    }
}

// 12. Tritone
pub fn apply_tritone(pixels: &mut [u8], shadow_c: [u8; 3], mid_c: [u8; 3], high_c: [u8; 3]) {
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299 + pixels[i + 1] as f32 * 0.587 + pixels[i + 2] as f32 * 0.114) / 255.0;
        for c in 0..3 {
            let val = if luma < 0.5 {
                shadow_c[c] as f32 + (mid_c[c] as f32 - shadow_c[c] as f32) * (luma * 2.0)
            } else {
                mid_c[c] as f32 + (high_c[c] as f32 - mid_c[c] as f32) * ((luma - 0.5) * 2.0)
            };
            pixels[i + c] = val.round() as u8;
        }
    }
}

// 13. Posterize
pub fn apply_posterize(pixels: &mut [u8], levels: u32) {
    let l = levels.max(2) as f32;
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let norm = pixels[i + c] as f32 / 255.0;
            pixels[i + c] = ((norm * l).floor() / l * 255.0).round() as u8;
        }
    }
}

// 14. Invert
pub fn apply_invert(pixels: &mut [u8], invert_alpha: bool) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = 255 - pixels[i];
        pixels[i + 1] = 255 - pixels[i + 1];
        pixels[i + 2] = 255 - pixels[i + 2];
        if invert_alpha { pixels[i + 3] = 255 - pixels[i + 3]; }
    }
}

// 15. Threshold
pub fn apply_threshold(pixels: &mut [u8], threshold: u8) {
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114) / 1000;
        let val = if luma as u8 >= threshold { 255 } else { 0 };
        pixels[i] = val; pixels[i + 1] = val; pixels[i + 2] = val;
    }
}

// 16. Twirl
pub fn apply_twirl(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32, radius: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = radius.max(1.0);
    let max_angle = angle_deg.to_radians();

    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            if r < max_r {
                let factor = (1.0 - r / max_r).powi(2);
                let twirl_a = max_angle * factor;
                let curr_a = ry.atan2(rx);
                let new_a = curr_a + twirl_a;

                let sx = (cx + new_a.cos() * r).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + new_a.sin() * r).clamp(0.0, height as f32 - 1.0) as u32;

                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((sy * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 17. Bulge
pub fn apply_bulge(pixels: &mut [u8], width: u32, height: u32, amount: f32, radius: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            if r < radius && r > 0.001 {
                let norm_r = r / radius;
                let factor = 1.0 + amount * (1.0 - norm_r * norm_r);
                let sx = (cx + rx * factor).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + ry * factor).clamp(0.0, height as f32 - 1.0) as u32;

                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((sy * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 18. Offset
pub fn apply_offset(pixels: &mut [u8], width: u32, height: u32, shift_x: i32, shift_y: i32) {
    let temp = pixels.to_vec();
    let w = width as i32;
    let h = height as i32;

    for y in 0..h {
        for x in 0..w {
            let sx = ((x - shift_x) % w + w) % w;
            let sy = ((y - shift_y) % h + h) % h;

            let idx = ((y * w + x) * 4) as usize;
            let s_idx = ((sy * w + sx) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 19. Venetian Blinds
pub fn apply_venetian_blinds(pixels: &mut [u8], width: u32, height: u32, completion: f32, width_px: u32) {
    if completion <= 0.0 || width_px == 0 { return; }
    let blind_w = width_px as usize;
    let cut_w = (blind_w as f32 * (completion * 0.01)) as usize;

    for y in 0..height as usize {
        for x in 0..width as usize {
            if (x % blind_w) < cut_w {
                let idx = (y * width as usize + x) * 4;
                pixels[idx + 3] = 0;
            }
        }
    }
}

// 20. Linear Wipe
pub fn apply_linear_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32, angle_deg: f32) {
    if completion <= 0.0 { return; }
    if completion >= 100.0 {
        for i in (3..pixels.len()).step_by(4) { pixels[i] = 0; }
        return;
    }

    let rad = angle_deg.to_radians();
    let dir = [rad.sin(), -rad.cos()];
    let max_proj = width as f32 * dir[0].abs() + height as f32 * dir[1].abs();
    let threshold = max_proj * (completion * 0.01);

    for y in 0..height {
        for x in 0..width {
            let proj = x as f32 * dir[0] + y as f32 * dir[1];
            if proj < threshold {
                let idx = ((y * width + x) * 4 + 3) as usize;
                pixels[idx] = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_pack_filters() {
        let mut pixels = vec![100u8; 64]; // 4x4
        apply_fast_box_blur(&mut pixels, 4, 4, 1);
        assert_eq!(pixels.len(), 64);

        apply_invert(&mut pixels, false);
        assert_eq!(pixels[0], 155);
    }
}
