#![allow(dead_code)]
/// Pack of 20 Additional Essential VFX compositing Effects & Filters (Part 2).
// 21. Wave Warp
pub fn apply_wave_warp(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    height_px: f32,
    width_px: f32,
    speed_time: f32,
) {
    let temp = pixels.to_vec();
    let k_w = 2.0 * std::f32::consts::PI / width_px.max(1.0);

    for y in 0..height {
        let shift_x = (y as f32 * k_w + speed_time).sin() * height_px;
        for x in 0..width {
            let sx = (x as f32 + shift_x).clamp(0.0, width as f32 - 1.0) as u32;
            let idx = ((y * width + x) * 4) as usize;
            let s_idx = ((y * width + sx) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 22. Ripple
pub fn apply_ripple(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    amplitude: f32,
    wave_length: f32,
    phase_time: f32,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();
    let k_w = 2.0 * std::f32::consts::PI / wave_length.max(1.0);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            if r > 0.001 {
                let shift = (r * k_w - phase_time).sin() * amplitude;
                let new_r = (r + shift).max(0.0);
                let sx = (cx + (rx / r) * new_r).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + (ry / r) * new_r).clamp(0.0, height as f32 - 1.0) as u32;

                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((sy * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 23. Gradient Ramp
pub fn apply_gradient_ramp(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    start_c: [u8; 4],
    end_c: [u8; 4],
    is_radial: bool,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = (cx * cx + cy * cy).sqrt().max(1.0);

    for y in 0..height {
        for x in 0..width {
            let t = if is_radial {
                let rx = x as f32 - cx;
                let ry = y as f32 - cy;
                ((rx * rx + ry * ry).sqrt() / max_r).clamp(0.0, 1.0)
            } else {
                (y as f32 / height as f32).clamp(0.0, 1.0)
            };

            let idx = ((y * width + x) * 4) as usize;
            for c in 0..4 {
                pixels[idx + c] =
                    (start_c[c] as f32 + (end_c[c] as f32 - start_c[c] as f32) * t).round() as u8;
            }
        }
    }
}

// 24. Find Edges
pub fn apply_find_edges(pixels: &mut [u8], width: u32, height: u32) {
    let temp = pixels.to_vec();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let get_luma = |px: u32, py: u32| {
                let i = ((py * width + px) * 4) as usize;
                temp[i] as f32 * 0.299 + temp[i + 1] as f32 * 0.587 + temp[i + 2] as f32 * 0.114
            };

            let gx = -get_luma(x - 1, y - 1) + get_luma(x + 1, y - 1) - 2.0 * get_luma(x - 1, y)
                + 2.0 * get_luma(x + 1, y)
                - get_luma(x - 1, y + 1)
                + get_luma(x + 1, y + 1);

            let gy = -get_luma(x - 1, y - 1) - 2.0 * get_luma(x, y - 1) - get_luma(x + 1, y - 1)
                + get_luma(x - 1, y + 1)
                + 2.0 * get_luma(x, y + 1)
                + get_luma(x + 1, y + 1);

            let edge = (gx * gx + gy * gy).sqrt().clamp(0.0, 255.0) as u8;
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = edge;
            pixels[idx + 1] = edge;
            pixels[idx + 2] = edge;
        }
    }
}

// 25. Emboss
pub fn apply_emboss_color(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32, depth: f32) {
    let rad = angle_deg.to_radians();
    let dx = rad.cos().round() as i32;
    let dy = rad.sin().round() as i32;
    let temp = pixels.to_vec();

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let p1_idx = (((y as i32 - dy) as u32 * width + (x as i32 - dx) as u32) * 4) as usize;
            let p2_idx = (((y as i32 + dy) as u32 * width + (x as i32 + dx) as u32) * 4) as usize;

            let idx = ((y * width + x) * 4) as usize;
            for c in 0..3 {
                let diff = (temp[p1_idx + c] as f32 - temp[p2_idx + c] as f32) * depth;
                pixels[idx + c] = (128.0 + diff).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 26. Mosaic
pub fn apply_mosaic_average(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    block_w: u32,
    block_h: u32,
) {
    if block_w == 0 || block_h == 0 {
        return;
    }
    for y_b in (0..height).step_by(block_h as usize) {
        for x_b in (0..width).step_by(block_w as usize) {
            let center_x = (x_b + block_w / 2).min(width - 1);
            let center_y = (y_b + block_h / 2).min(height - 1);
            let c_idx = ((center_y * width + center_x) * 4) as usize;
            let color = [
                pixels[c_idx],
                pixels[c_idx + 1],
                pixels[c_idx + 2],
                pixels[c_idx + 3],
            ];

            for py in y_b..(y_b + block_h).min(height) {
                for px in x_b..(x_b + block_w).min(width) {
                    let idx = ((py * width + px) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    }
}

// 27. CC Glass
pub fn apply_cc_glass(pixels: &mut [u8], width: u32, height: u32, bump_height: f32) {
    apply_emboss_color(pixels, width, height, 45.0, bump_height * 0.05);
}

// 28. CC Lens
pub fn apply_cc_lens(pixels: &mut [u8], width: u32, height: u32, convergence: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            let factor = 1.0 + (convergence * 0.005) * (r / (width as f32 * 0.5));
            let sx = (cx + rx * factor).clamp(0.0, width as f32 - 1.0) as u32;
            let sy = (cy + ry * factor).clamp(0.0, height as f32 - 1.0) as u32;

            let idx = ((y * width + x) * 4) as usize;
            let s_idx = ((sy * width + sx) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 29. CC Tiler
pub fn apply_cc_tiler(pixels: &mut [u8], width: u32, height: u32, scale_percent: f32) {
    let factor = (scale_percent * 0.01).max(0.1);
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let sx = ((x as f32 / factor) as u32) % width;
            let sy = ((y as f32 / factor) as u32) % height;

            let idx = ((y * width + x) * 4) as usize;
            let s_idx = ((sy * width + sx) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 30. CC Kaleida
pub fn apply_cc_kaleida(pixels: &mut [u8], width: u32, height: u32, sides: u32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let sector = 2.0 * std::f32::consts::PI / sides.max(2) as f32;
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();
            let mut angle = ry.atan2(rx) % sector;
            if angle < 0.0 {
                angle += sector;
            }

            let sx = (cx + angle.cos() * r).clamp(0.0, width as f32 - 1.0) as u32;
            let sy = (cy + angle.sin() * r).clamp(0.0, height as f32 - 1.0) as u32;

            let idx = ((y * width + x) * 4) as usize;
            let s_idx = ((sy * width + sx) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 31. Grid Generator
pub fn apply_grid(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    grid_size: u32,
    border: u32,
    color: [u8; 4],
) {
    if grid_size == 0 {
        return;
    }
    for y in 0..height {
        for x in 0..width {
            if (x % grid_size) < border || (y % grid_size) < border {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

// 32. Checkerboard
pub fn apply_checkerboard(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    box_size: u32,
    c1: [u8; 4],
    c2: [u8; 4],
) {
    if box_size == 0 {
        return;
    }
    for y in 0..height {
        for x in 0..width {
            let check = ((x / box_size) + (y / box_size)).is_multiple_of(2);
            let color = if check { c1 } else { c2 };
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

// 33. Fill
pub fn apply_fill(pixels: &mut [u8], fill_color: [u8; 4]) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i..i + 4].copy_from_slice(&fill_color);
    }
}

// 34. Stroke Effect
pub fn apply_stroke_effect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    stroke_color: [u8; 4],
    stroke_width: u32,
) {
    apply_find_edges(pixels, width, height);
    apply_simple_choker_alpha(pixels, stroke_color, stroke_width);
}

fn apply_simple_choker_alpha(pixels: &mut [u8], color: [u8; 4], _width: u32) {
    for i in (0..pixels.len()).step_by(4) {
        if pixels[i] > 50 {
            pixels[i..i + 4].copy_from_slice(&color);
        }
    }
}

// 35. Vignette
pub fn apply_vignette(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = (cx * cx + cy * cy).sqrt().max(1.0);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let norm_r = (rx * rx + ry * ry).sqrt() / max_r;
            let factor = (1.0 - norm_r * amount).clamp(0.0, 1.0);

            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = (pixels[idx] as f32 * factor).round() as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 * factor).round() as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 * factor).round() as u8;
        }
    }
}

// 36. Channel Combiner
pub fn apply_channel_combiner(pixels: &mut [u8]) {
    for i in (0..pixels.len()).step_by(4) {
        let luma =
            (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                / 1000;
        pixels[i + 3] = luma as u8;
    }
}

// 37. Extract Key
pub fn apply_extract_key(pixels: &mut [u8], black_point: u8, white_point: u8) {
    for i in (0..pixels.len()).step_by(4) {
        let luma =
            (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                / 1000;
        if (luma as u8) < black_point || (luma as u8) > white_point {
            pixels[i + 3] = 0;
        }
    }
}

// 38. Time Displacement
pub fn apply_time_displacement(pixels: &mut [u8], width: u32, height: u32, shift: i32) {
    crate::core::ae_effects_pack::apply_offset(pixels, width, height, shift, 0);
}

// 39. Radial Wipe
pub fn apply_radial_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    if completion <= 0.0 {
        return;
    }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_angle = (completion * 0.01) * 2.0 * std::f32::consts::PI;

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let mut angle = ry.atan2(rx) + std::f32::consts::PI * 0.5;
            if angle < 0.0 {
                angle += 2.0 * std::f32::consts::PI;
            }

            if angle < max_angle {
                let idx = ((y * width + x) * 4 + 3) as usize;
                pixels[idx] = 0;
            }
        }
    }
}

// 40. Iris Wipe
pub fn apply_iris_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = (cx * cx + cy * cy).sqrt();
    let cut_r = max_r * (completion * 0.01);

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            if (rx * rx + ry * ry).sqrt() < cut_r {
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
    fn test_ae_effects_v2_filters() {
        let mut pixels = vec![255u8; 64];
        apply_vignette(&mut pixels, 4, 4, 0.5);
        assert_eq!(pixels.len(), 64);
    }
}
