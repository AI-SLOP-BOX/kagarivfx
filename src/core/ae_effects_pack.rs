#![allow(dead_code)]
use rayon::prelude::*;
/// Pack of 20 Essential Adobe After Effects Effects & Filters.
// 1. Fast Box Blur (Separable 2-pass with O(n) sliding window per pass)
pub fn apply_fast_box_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 || width == 0 || height == 0 { return; }
    let r = radius as i32;
    let w = width as usize;
    let h = height as usize;
    let stride = w * 4;

    // --- Horizontal pass: pixels -> temp_h (sliding window) ---
    let mut temp_h = vec![0u8; pixels.len()];
    for y in 0..h {
        let row = y * stride;
        // Initialize accumulator for x=0
        let mut acc = [0f32; 4];
        let mut count = 0f32;
        for dx in -r..=r {
            let px = dx.clamp(0, w as i32 - 1) as usize;
            let idx = row + px * 4;
            for c in 0..4 { acc[c] += pixels[idx + c] as f32; }
            count += 1.0;
        }
        for c in 0..4 { temp_h[row + c] = (acc[c] / count).round() as u8; }

        // Slide the window across the row
        for x in 1..w {
            // Add new right edge
            let add_px = (x as i32 + r).min(w as i32 - 1) as usize;
            let add_idx = row + add_px * 4;
            for c in 0..4 { acc[c] += pixels[add_idx + c] as f32; }

            // Remove old left edge
            let rem_px = (x as i32 - r - 1).max(0) as usize;
            let rem_idx = row + rem_px * 4;
            for c in 0..4 { acc[c] -= pixels[rem_idx + c] as f32; }

            let out_idx = row + x * 4;
            for c in 0..4 { temp_h[out_idx + c] = (acc[c] / count).round() as u8; }
        }
    }

    // --- Vertical pass: temp_h -> pixels (sliding window) ---
    for x in 0..w {
        let col = x * 4;
        // Initialize accumulator for y=0
        let mut acc = [0f32; 4];
        let mut count = 0f32;
        for dy in -r..=r {
            let py = dy.clamp(0, h as i32 - 1) as usize;
            let idx = py * stride + col;
            for c in 0..4 { acc[c] += temp_h[idx + c] as f32; }
            count += 1.0;
        }
        let out_idx = col;
        for c in 0..4 { pixels[out_idx + c] = (acc[c] / count).round() as u8; }

        // Slide the window down the column
        for y in 1..h {
            // Add new bottom edge
            let add_py = (y as i32 + r).min(h as i32 - 1) as usize;
            let add_idx = add_py * stride + col;
            for c in 0..4 { acc[c] += temp_h[add_idx + c] as f32; }

            // Remove old top edge
            let rem_py = (y as i32 - r - 1).max(0) as usize;
            let rem_idx = rem_py * stride + col;
            for c in 0..4 { acc[c] -= temp_h[rem_idx + c] as f32; }

            let out_idx = y * stride + col;
            for c in 0..4 { pixels[out_idx + c] = (acc[c] / count).round() as u8; }
        }
    }
}


// 2. Directional Blur (rayon-parallelized)
pub fn apply_directional_blur(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32, length: f32) {
    if length <= 0.01 || width == 0 || height == 0 { return; }
    let rad = angle_deg.to_radians();
    let dx = rad.sin();
    let dy = -rad.cos();

    let temp = pixels.to_vec();
    let samples = ((length * 2.0) as usize).max(4).next_power_of_two();

    let sigma = (samples as f32 * 0.5 / 3.0).max(1.0);
    let mut weights = vec![0f32; samples];
    let mut w_sum = 0f32;
    for (s, w) in weights.iter_mut().enumerate() {
        let t = (s as f32 - (samples as f32 - 1.0) * 0.5) / sigma;
        *w = (-t * t * 0.5).exp();
        w_sum += *w;
    }
    for w in weights.iter_mut() { *w /= w_sum; }

    pixels.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, px)| {
            let x = (i % width as usize) as u32;
            let y = (i / width as usize) as u32;
            let mut acc = [0f32; 4];
            for (s, &wt) in weights.iter().enumerate() {
                let offset = (s as f32) - (samples as f32 * 0.5);
                let sx = (x as f32 + dx * offset).round().clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (y as f32 + dy * offset).round().clamp(0.0, height as f32 - 1.0) as u32;
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 { acc[c] += temp[idx + c] as f32 * wt; }
            }
            for c in 0..4 { px[c] = acc[c].round().clamp(0.0, 255.0) as u8; }
        });
}

// 3. Radial Blur (Spin) (rayon-parallelized)
pub fn apply_radial_blur(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    if amount.abs() < 0.01 || width == 0 || height == 0 { return; }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    let steps = 16;
    let mut weights = [0f32; 16];
    let sigma = (steps as f32 * 0.5 / 3.0).max(1.0);
    let mut w_sum = 0f32;
    for (s, w) in weights.iter_mut().enumerate() {
        let t = (s as f32 - (steps as f32 - 1.0) * 0.5) / sigma;
        *w = (-t * t * 0.5).exp();
        w_sum += *w;
    }
    for w in weights.iter_mut() { *w /= w_sum; }

    pixels.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, px)| {
            let x = (i % width as usize) as u32;
            let y = (i / width as usize) as u32;
            let mut acc = [0f32; 4];
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let base_angle = ry.atan2(rx);
            let dist = (rx * rx + ry * ry).sqrt();

            for (s, &wt) in weights.iter().enumerate() {
                let t = (s as f32 / steps as f32 - 0.5) * amount * 0.05;
                let a = base_angle + t;
                let sx = (cx + a.cos() * dist).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + a.sin() * dist).clamp(0.0, height as f32 - 1.0) as u32;
                let idx = ((sy * width + sx) * 4) as usize;
                for c in 0..4 { acc[c] += temp[idx + c] as f32 * wt; }
            }
            for c in 0..4 { px[c] = acc[c].round().clamp(0.0, 255.0) as u8; }
        });
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

    // Soft threshold: pixels near the cutoff get partial weight, which removes
    // the harsh banding edge a hard binary threshold produces.
    let soft_width = (radius.max(2) as f32 * 8.0).min(64.0);
    for i in (0..num_bytes).step_by(4) {
        let luma = pixels[i] as f32 * 0.299 + pixels[i + 1] as f32 * 0.587 + pixels[i + 2] as f32 * 0.114;
        let t = ((luma - threshold * 255.0) / soft_width + 0.5).clamp(0.0, 1.0);
        // Smoothstep the transition for a filmic rolloff
        let w = t * t * (3.0 - 2.0 * t);
        glow_map[i] = (pixels[i] as f32 * w) as u8;
        glow_map[i + 1] = (pixels[i + 1] as f32 * w) as u8;
        glow_map[i + 2] = (pixels[i + 2] as f32 * w) as u8;
        glow_map[i + 3] = pixels[i + 3];
    }

    // True Gaussian on the glow map: bloom quality depends directly on this blur
    apply_gaussian_blur(&mut glow_map, width, height, radius);

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
    if width == 0 || height == 0 { return; }
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

    // True Gaussian blur on the shadow alpha gives soft, smooth edges
    // (a single box blur produces visible hard banding rings).
    apply_gaussian_blur(&mut shadow_buf, width, height, softness);

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

    // Per-pixel independent — parallelize with rayon
    pixels.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = i % width as usize;
            let y = i / width as usize;
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

                let s_idx = ((sy * width + sx) * 4) as usize;
                out.copy_from_slice(&temp[s_idx..s_idx + 4]);
            } else {
                out.copy_from_slice(&temp[i * 4..i * 4 + 4]);
            }
        });
}

// 17. Bulge
pub fn apply_bulge(pixels: &mut [u8], width: u32, height: u32, amount: f32, radius: f32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let temp = pixels.to_vec();

    pixels.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = i % width as usize;
            let y = i / width as usize;
            let rx = x as f32 - cx;
            let ry = y as f32 - cy;
            let r = (rx * rx + ry * ry).sqrt();

            if r < radius && r > 0.001 {
                let norm_r = r / radius;
                let factor = 1.0 + amount * (1.0 - norm_r * norm_r);
                let sx = (cx + rx * factor).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (cy + ry * factor).clamp(0.0, height as f32 - 1.0) as u32;

                let s_idx = ((sy * width + sx) * 4) as usize;
                out.copy_from_slice(&temp[s_idx..s_idx + 4]);
            } else {
                out.copy_from_slice(&temp[i * 4..i * 4 + 4]);
            }
        });
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
    if completion <= 0.0 || width_px == 0 || width == 0 || height == 0 { return; }
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

/// Computes box-blur radii that approximate a true Gaussian of the given sigma
/// (W3C/Featherstone standard: three successive box blurs ≈ Gaussian).
fn boxes_for_gauss(sigma: f32, n: u32) -> [f32; 3] {
    let n = n as f32;
    let w_ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = w_ideal.floor();
    if wl % 2.0 == 0.0 {
        wl -= 1.0;
    }
    let wu = wl + 2.0;
    let m_ideal = (12.0 * sigma * sigma - n * wl * wl - 4.0 * n * wl - 3.0 * n) / (-4.0 * wl - 4.0);
    let m = m_ideal.round();
    let mut out = [0.0f32; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = if (i as f32) < m { wl } else { wu };
    }
    out
}

/// True Gaussian blur via three successive box blurs with optimized radii.
/// Visually indistinguishable from an exact Gaussian while staying O(n) per pass.
pub fn apply_gaussian_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    // The UI's "radius" maps to a Gaussian sigma; 3-box approx covers it well.
    let sigma = radius as f32 / 2.0;
    let boxes = boxes_for_gauss(sigma, 3);
    let mut buf_a = pixels.to_vec();
    let mut buf_b = vec![0u8; pixels.len()];
    let mut scratch = vec![0u8; pixels.len()];

    for &box_w in &boxes {
        let r = ((box_w - 1.0) / 2.0).round().max(1.0) as u32;
        box_blur_separable(&buf_a, &mut buf_b, &mut scratch, width, height, r);
        std::mem::swap(&mut buf_a, &mut buf_b);
    }
    pixels.copy_from_slice(&buf_a);
}

/// One separable box blur: horizontal src -> scratch, vertical scratch -> dst.
fn box_blur_separable(src: &[u8], dst: &mut [u8], scratch: &mut [u8], width: u32, height: u32, radius: u32) {
    let r = radius as i32;

    // Horizontal: src -> scratch
    for y in 0..height {
        for x in 0..width as i32 {
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for dx in -r..=r {
                let px = (x + dx).clamp(0, width as i32 - 1) as u32;
                let idx = ((y * width + px) * 4) as usize;
                for ch in 0..4 {
                    acc[ch] += src[idx + ch] as f32;
                }
                count += 1.0;
            }
            let oidx = ((y * width + x as u32) * 4) as usize;
            for ch in 0..4 {
                scratch[oidx + ch] = (acc[ch] / count).round() as u8;
            }
        }
    }

    // Vertical: scratch -> dst
    for y in 0..height as i32 {
        for x in 0..width {
            let mut acc = [0f32; 4];
            let mut count = 0f32;
            for dy in -r..=r {
                let py = (y + dy).clamp(0, height as i32 - 1) as u32;
                let idx = ((py * width + x) * 4) as usize;
                for ch in 0..4 {
                    acc[ch] += scratch[idx + ch] as f32;
                }
                count += 1.0;
            }
            let oidx = ((y as u32 * width + x) * 4) as usize;
            for ch in 0..4 {
                dst[oidx + ch] = (acc[ch] / count).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod gaussian_tests {
    use super::*;

    #[test]
    fn test_boxes_for_gauss_sane() {
        let [w1, w2, w3] = boxes_for_gauss(4.0, 3);
        // All widths must be positive and odd (box convention)
        for w in [w1, w2, w3] {
            assert!(w >= 1.0);
            assert_eq!(w % 2.0, 1.0);
        }
        // Widths must be roughly increasing then equal (standard property)
        assert!(w1 <= w2 + 2.0);
    }

    #[test]
    fn test_gaussian_blur_smooths_uniform_image_to_itself() {
        // A uniform image must be unchanged by any blur (edge clamping)
        let w = 32u32;
        let h = 32u32;
        let mut px = vec![128u8; (w * h * 4) as usize];
        let original = px.clone();
        apply_gaussian_blur(&mut px, w, h, 6);
        assert_eq!(px, original);
    }

    #[test]
    fn test_gaussian_blur_spreads_energy() {
        // A single bright pixel spreads into neighbors and loses peak intensity
        let w = 64u32;
        let h = 64u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        let center = ((32 * w + 32) * 4) as usize;
        px[center] = 255;
        px[center + 1] = 255;
        px[center + 2] = 255;
        px[center + 3] = 255;
        apply_gaussian_blur(&mut px, w, h, 8);
        let peak = px[center];
        assert!(peak < 255, "peak must drop, got {}", peak);
        let neighbor = ((32 * w + 33) * 4) as usize;
        assert!(px[neighbor] > 0, "energy must spread to neighbors");
        // Symmetry check: left and right neighbors receive equal energy
        let left = ((32 * w + 31) * 4) as usize;
        assert!((px[left] as i32 - px[neighbor] as i32).abs() <= 2, "blur must be symmetric");
    }

    #[test]
    fn test_gaussian_vs_single_box_quality() {
        // The 3-box Gaussian must produce a smoother falloff than a single box:
        // at 1.5x radius from center, a single box is already 0 while Gaussian
        // still has measurable energy.
        let w = 128u32;
        let h = 9u32;
        let mut g = vec![0u8; (w * h * 4) as usize];
        let mut b = g.clone();
        let cy = 4 * w;
        for x in 60..68 {
            let idx = ((cy + x) * 4) as usize;
            g[idx] = 255;
            b[idx] = 255;
        }
        apply_gaussian_blur(&mut g, w, h, 10);
        apply_fast_box_blur(&mut b, w, h, 10);
        // Quality assertions:
        // 1. Gaussian concentrates energy at the center (higher peak than box)
        let center_g = g[((cy + 63) * 4) as usize];
        let center_b = b[((cy + 63) * 4) as usize];
        assert!(center_g > center_b, "gaussian peak ({}) must exceed box ({})", center_g, center_b);

        // 2. Smooth monotone falloff (no abrupt steps like a box kernel)
        let mut prev = u32::MAX;
        for x in 64..80 {
            let v = g[((cy + x) * 4) as usize] as u32;
            assert!(v <= prev.saturating_add(1), "falloff must be monotone at x={}: {} -> {}", x, prev, v);
            prev = v;
        }

        // 3. Symmetry around the source center (x=63.5)
        for d in 1..8u32 {
            let l = g[((cy + (64 - d)) * 4) as usize] as i32;
            let r = g[((cy + (63 + d)) * 4) as usize] as i32;
            assert!((l - r).abs() <= 1, "blur must be symmetric at d={}: {} vs {}", d, l, r);
        }
    }
}

#[cfg(test)]
mod glow_tests {
    use super::*;

    #[test]
    fn test_glow_adds_energy_near_bright_areas() {
        let w = 64u32;
        let h = 64u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        // Bright white square in the center
        for y in 28..36 {
            for x in 28..36 {
                let idx = ((y * w + x) * 4) as usize;
                px[idx] = 255;
                px[idx + 1] = 255;
                px[idx + 2] = 255;
                px[idx + 3] = 255;
            }
        }
        let before = px.clone();
        apply_glow(&mut px, w, h, 0.5, 8, 50.0);

        // A pixel just outside the square must receive bloom energy
        let outside = ((30 * w + 40) * 4) as usize;
        assert!(
            px[outside] > before[outside],
            "glow must add energy outside bright regions, got {}",
            px[outside]
        );
        // Dark corners far away stay dark
        let corner = ((2 * w + 2) * 4) as usize;
        assert!(px[corner] < 20, "far dark areas must stay dark, got {}", px[corner]);
    }

    #[test]
    fn test_soft_threshold_no_hard_band() {
        // With a hard threshold, a gradient produces a binary mask with a sharp
        // edge; soft thresholding must produce a gradual ramp.
        let w = 64u32;
        let h = 8u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        for x in 0..w {
            let v = ((x as f32 / w as f32) * 255.0) as u8;
            for y in 0..h {
                let idx = ((y * w + x) * 4) as usize;
                px[idx] = v;
                px[idx + 1] = v;
                px[idx + 2] = v;
                px[idx + 3] = 255;
            }
        }
        // Dark gradient (max 60/255) with threshold 0.15 and modest intensity:
        // nothing saturates, so any hard banding in the mask would show as a
        // large single-step jump across the threshold boundary.
        for i in (0..px.len()).step_by(4) {
            let src_x = (i / 4 % w as usize) as f32 / w as f32;
            let v = (src_x * 60.0) as u8;
            px[i] = v;
            px[i + 1] = v;
            px[i + 2] = v;
        }
        let mut out = px.clone();
        apply_glow(&mut out, w, h, 0.15, 6, 30.0);
        let row = 4 * w;
        let mut max_delta = 0i32;
        for x in 1..w {
            let d = (out[((row + x) * 4) as usize] as i32 - out[((row + x - 1) * 4) as usize] as i32).abs();
            max_delta = max_delta.max(d);
        }
        // With blur radius 6 the steepest step should be gentle (< 60/px)
        assert!(
            max_delta < 60,
            "soft threshold should avoid hard banding, max step was {}",
            max_delta
        );
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;

    fn solid_square(w: u32, h: u32, x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u8> {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in y0..y1 {
            for x in x0..x1 {
                let idx = (y * w as usize + x) * 4;
                px[idx] = 255;
                px[idx + 1] = 255;
                px[idx + 2] = 255;
                px[idx + 3] = 255;
            }
        }
        px
    }

    #[test]
    fn test_drop_shadow_places_shadow_beyond_shape() {
        // Square on the left half; shadow offset to the right must appear
        // in the empty right region.
        let w = 64u32;
        let h = 64u32;
        let mut px = solid_square(w, h, 8, 24, 24, 40);
        apply_drop_shadow(&mut px, w, h, 16.0, 90.0, 4, [0, 0, 0, 255]);
        let right = ((32 * w + 30) * 4) as usize;
        assert!(px[right + 3] > 0, "shadow alpha must extend past shape edge");
        assert_eq!(px[right], 0, "shadow must be black");
    }

    #[test]
    fn test_drop_shadow_soft_edges_are_smooth() {
        // With Gaussian softness, the shadow falloff across the edge must be
        // monotone and gradual (no hard box-blur steps).
        let w = 96u32;
        let h = 64u32;
        let mut px = solid_square(w, h, 8, 24, 24, 40);
        apply_drop_shadow(&mut px, w, h, 20.0, 90.0, 10, [0, 0, 0, 255]);
        // Sample a horizontal run of shadow-only pixels right of the shape.
        // The profile must rise then fall smoothly (no hard box-blur steps).
        let row = 32usize;
        let vals: Vec<i32> = (28..90)
            .map(|x| px[(row * w as usize + x) * 4 + 3] as i32)
            .collect();
        let peak = vals.iter().enumerate().max_by_key(|(_, v)| **v).unwrap().0;
        assert!(peak > 0 && peak < vals.len() - 1, "shadow must have an interior peak");
        let mut prev = i32::MAX;
        for (i, &v) in vals.iter().enumerate().skip(peak) {
            assert!(
                v <= prev.saturating_add(1),
                "shadow falloff must be monotone at x={}: {} -> {}",
                peak + i, prev, v
            );
            prev = v;
        }
    }

    #[test]
    fn test_drop_shadow_preserves_foreground() {
        let w = 32u32;
        let h = 32u32;
        let mut px = solid_square(w, h, 12, 20, 12, 20);
        apply_drop_shadow(&mut px, w, h, 5.0, 45.0, 3, [0, 0, 0, 200]);
        // Foreground center stays fully opaque white
        let c = ((16 * w + 16) * 4) as usize;
        assert_eq!(px[c], 255);
        assert_eq!(px[c + 1], 255);
        assert_eq!(px[c + 2], 255);
        assert_eq!(px[c + 3], 255);
    }
}

#[cfg(test)]
mod directional_radial_tests {
    use super::*;

    #[test]
    fn test_directional_blur_spreads_along_angle_only() {
        // A single bright pixel smears along the direction vector; the
        // perpendicular neighbor must stay dark.
        let w = 64u32;
        let h = 64u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        let c = ((32 * w + 32) * 4) as usize;
        px[c] = 255;
        px[c + 1] = 255;
        px[c + 2] = 255;
        px[c + 3] = 255;
        apply_directional_blur(&mut px, w, h, 90.0, 20.0);
        // 90deg => horizontal smear
        let right = ((32 * w + 38) * 4) as usize;
        let below = ((36 * w + 32) * 4) as usize;
        assert!(px[right] > 0, "energy must spread horizontally");
        assert!(px[below] < 10, "perpendicular axis must stay dark, got {}", px[below]);
    }

    #[test]
    fn test_directional_blur_zero_length_is_noop() {
        let mut px = vec![128u8; 16 * 16 * 4];
        let orig = px.clone();
        apply_directional_blur(&mut px, 16, 16, 45.0, 0.0);
        assert_eq!(px, orig);
    }

    #[test]
    fn test_radial_blur_smears_angularly() {
        // An off-center bright pixel spreads angularly around center.
        let w = 64u32;
        let h = 64u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        let p = ((32 * w + 44) * 4) as usize;
        px[p] = 255;
        px[p + 1] = 255;
        px[p + 2] = 255;
        px[p + 3] = 255;
        apply_radial_blur(&mut px, w, h, 100.0);
        // Energy must spread to multiple pixels around the original radius
        let nonzero = px.chunks_exact(4).filter(|p| p[0] > 0).count();
        assert!(nonzero > 1, "radial blur must spread angularly, got {} lit pixels", nonzero);
        let peak = px.chunks_exact(4).map(|p| p[0]).max().unwrap();
        assert!(peak < 255, "peak must drop as energy spreads, got {}", peak);
    }

    #[test]
    fn test_radial_blur_uniform_image_is_stable() {
        // A uniform image has no angular structure: blur must be a no-op.
        let w = 33u32;
        let h = 33u32;
        let mut px = vec![200u8; (w * h * 4) as usize];
        for i in (3..px.len()).step_by(4) { px[i] = 255; }
        let orig = px.clone();
        apply_radial_blur(&mut px, w, h, 50.0);
        assert_eq!(px, orig);
    }
}

// ── Inner Shadow / Inner Glow (layer styles) ──
/// Inner Shadow: darkens the inside edges of the layer's alpha, offset in
/// `angle_deg` (compass convention matching apply_drop_shadow) by `distance`,
/// softened by `softness`. Composites `shadow_color` under existing pixels.
pub fn apply_inner_shadow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    distance: f32,
    angle_deg: f32,
    softness: u32,
    shadow_color: [u8; 4],
) {
    if width == 0 || height == 0 {
        return;
    }
    let rad = angle_deg.to_radians();
    let dx = (rad.sin() * distance).round() as i32;
    let dy = (-rad.cos() * distance).round() as i32;

    // Mask M = A(x,y) × (1 − A(x−dx, y−dy)): the band of opaque pixels whose
    // back-shifted sample falls outside the shape.
    let mut mask = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let a = pixels[idx + 3] as f32 / 255.0;
            if a <= 0.003 {
                continue;
            }
            let sx = (x as i32 - dx).clamp(0, width as i32 - 1) as u32;
            let sy = (y as i32 - dy).clamp(0, height as i32 - 1) as u32;
            let s_a = pixels[((sy * width + sx) * 4) as usize + 3] as f32 / 255.0;
            mask[(y * width + x) as usize] = (a * (1.0 - s_a) * 255.0).round() as u8;
        }
    }

    // Soften the band into an RGBA buffer so gaussian blur can run on alpha.
    let mut soft = vec![shadow_color[0]; (width * height * 4) as usize];
    for i in (0..soft.len()).step_by(4) {
        soft[i] = shadow_color[0];
        soft[i + 1] = shadow_color[1];
        soft[i + 2] = shadow_color[2];
        soft[i + 3] = mask[i / 4];
    }
    if softness > 0 {
        apply_gaussian_blur(&mut soft, width, height, softness);
    }

    // Under-composite: shadow sits beneath the original pixels but only where
    // the original is opaque (inner region), scaled by its own alpha.
    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        let sh_a = (soft[i + 3] as f32 / 255.0) * (shadow_color[3] as f32 / 255.0);
        let m = sh_a.min(1.0);
        if m <= 0.003 || base_a <= 0.003 {
            continue;
        }
        for c in 0..3 {
            let base = pixels[i + c] as f32 / 255.0;
            let sc = soft[i + c] as f32 / 255.0;
            // Multiply-ish darkening toward shadow color, preserving base alpha
            let blended = base * (1.0 - m) + sc * m;
            pixels[i + c] = (blended * 255.0).round() as u8;
        }
    }
}

/// Inner Glow: brightens the interior edges of the layer's alpha with
/// `glow_color`, falloff controlled by `size`, strength by `opacity` (0..100).
pub fn apply_inner_glow(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    size: u32,
    glow_color: [u8; 4],
    opacity: f32,
) {
    if width == 0 || height == 0 || opacity <= 0.0 {
        return;
    }
    let strength = (opacity / 100.0).clamp(0.0, 1.0);

    // Invert alpha, blur it, then re-mask by original alpha: brightest just
    // inside the edge, fading toward the interior.
    let mut inv = vec![0u8; (width * height * 4) as usize];
    for i in (0..inv.len()).step_by(4) {
        inv[i + 3] = 255 - pixels[i + 3];
    }
    if size > 0 {
        apply_gaussian_blur(&mut inv, width, height, size);
    }

    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        if base_a <= 0.003 {
            continue;
        }
        let edge = (inv[i + 3] as f32 / 255.0).min(1.0);
        let g = edge * strength * (glow_color[3] as f32 / 255.0);
        if g <= 0.003 {
            continue;
        }
        for c in 0..3 {
            let base = pixels[i + c] as f32;
            let gc = glow_color[c] as f32;
            // Additive-leaning screen blend keeps interiors luminous
            let screened = 255.0 - ((255.0 - base) * (255.0 - gc) / 255.0);
            pixels[i + c] = (base + (screened - base) * g).round() as u8;
        }
    }
}

// ── Satin (layer style) ──
/// Satin parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct SatinParams {
    /// Offset of the sheen source (px)
    pub distance: f32,
    /// Sheen direction in degrees (compass convention: 0 = up, 90 = right)
    pub angle_deg: f32,
    /// Softness of the band (gaussian radius px)
    pub size: u32,
    /// Tint color RGBA
    pub color: [u8; 4],
    /// Blend strength (0..100)
    pub opacity: f32,
}

/// Satin: interior sheen band. Shifts the inverted alpha by the compass
/// offset, intersects it with the original shape, blurs, then tints.
pub fn apply_satin(pixels: &mut [u8], width: u32, height: u32, p: &SatinParams) {
    if width == 0 || height == 0 || p.opacity <= 0.0 {
        return;
    }
    let strength = (p.opacity / 100.0).clamp(0.0, 1.0);
    let rad = p.angle_deg.to_radians();
    let dx = (rad.sin() * p.distance).round() as i32;
    let dy = (-rad.cos() * p.distance).round() as i32;

    // Band M(x,y) = A(x,y) × Ainv_shifted(x,y): opaque pixels whose
    // forward-shifted sample falls outside the shape.
    let mut mask = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let a = pixels[idx + 3] as f32 / 255.0;
            if a <= 0.003 {
                continue;
            }
            let sx = (x as i32 - dx).clamp(0, width as i32 - 1) as u32;
            let sy = (y as i32 - dy).clamp(0, height as i32 - 1) as u32;
            let s_a = pixels[((sy * width + sx) * 4) as usize + 3] as f32 / 255.0;
            // Inverted shifted alpha intersected with the shape
            mask[(y * width + x) as usize] = (a * (1.0 - s_a) * 255.0).round() as u8;
        }
    }

    let mut soft = vec![0u8; (width * height * 4) as usize];
    for i in (0..soft.len()).step_by(4) {
        soft[i] = p.color[0];
        soft[i + 1] = p.color[1];
        soft[i + 2] = p.color[2];
        soft[i + 3] = mask[i / 4];
    }
    if p.size > 0 {
        apply_gaussian_blur(&mut soft, width, height, p.size);
    }

    for i in (0..pixels.len()).step_by(4) {
        let base_a = pixels[i + 3] as f32 / 255.0;
        let m = ((soft[i + 3] as f32 / 255.0) * (p.color[3] as f32 / 255.0)).min(1.0) * strength;
        if m <= 0.003 || base_a <= 0.003 {
            continue;
        }
        for c in 0..3 {
            let base = pixels[i + c] as f32 / 255.0;
            let sc = soft[i + c] as f32 / 255.0;
            pixels[i + c] = ((base * (1.0 - m) + sc * m) * 255.0).round() as u8;
        }
    }
}

// ── Layer Style Overlays ──
/// Solid Color Overlay: blends every pixel toward `color` by `opacity` (0..100),
/// weighted by existing alpha so transparent areas stay transparent.
pub fn apply_color_overlay(
    pixels: &mut [u8],
    _width: u32,
    _height: u32,
    color: [u8; 4],
    opacity: f32,
) {
    if opacity <= 0.0 {
        return;
    }
    let mix = (opacity / 100.0).clamp(0.0, 1.0);
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a < 0.003 {
            continue;
        }
        px[0] = (px[0] as f32 + (color[0] as f32 - px[0] as f32) * mix) as u8;
        px[1] = (px[1] as f32 + (color[1] as f32 - px[1] as f32) * mix) as u8;
        px[2] = (px[2] as f32 + (color[2] as f32 - px[2] as f32) * mix) as u8;
    }
}

/// Parameters for [`apply_gradient_overlay`].
#[derive(Debug, Clone, PartialEq)]
pub struct GradientOverlayParams {
    /// Gradient direction in degrees (0 = left→right, 90 = bottom→top)
    pub angle_deg: f32,
    /// Gradient length as % of layer bounding extent
    pub scale_pct: f32,
    /// Gradient start / end colors
    pub start: [u8; 4],
    pub end: [u8; 4],
    /// Blend strength (0..100)
    pub opacity: f32,
}

/// Linear Gradient Overlay: interpolates start→end along `angle_deg`
/// (0 = left→right, 90 = bottom→top), spanning `scale_pct` of the layer
/// extent, blended at `opacity` over existing alpha.
pub fn apply_gradient_overlay(pixels: &mut [u8], width: u32, height: u32, p: &GradientOverlayParams) {
    if width == 0 || height == 0 || p.opacity <= 0.0 {
        return;
    }
    let mix_max = (p.opacity / 100.0).clamp(0.0, 1.0);
    let rad = p.angle_deg.to_radians();
    // Screen-space direction: 0° → +x, 90° → up (-y)
    let dx = rad.cos();
    let dy = -rad.sin();
    let w = width as f32;
    let h = height as f32;

    // Project the four corners to find the gradient extent
    let mut t_min = f32::INFINITY;
    let mut t_max = f32::NEG_INFINITY;
    for (cx, cy) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
        let t = cx * dx + cy * dy;
        t_min = t_min.min(t);
        t_max = t_max.max(t);
    }
    let span = ((t_max - t_min) * (p.scale_pct / 100.0).clamp(0.05, 4.0)).max(1e-3);
    let mid = (t_min + t_max) * 0.5;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let a = pixels[idx + 3] as f32 / 255.0;
            if a < 0.003 {
                continue;
            }
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = (((px * dx + py * dy) - mid) / span + 0.5).clamp(0.0, 1.0);
            let or = p.start[0] as f32 + (p.end[0] as f32 - p.start[0] as f32) * t;
            let og = p.start[1] as f32 + (p.end[1] as f32 - p.start[1] as f32) * t;
            let ob = p.start[2] as f32 + (p.end[2] as f32 - p.start[2] as f32) * t;
            let oa = p.start[3] as f32 + (p.end[3] as f32 - p.start[3] as f32) * t;
            // Blend overlay color at (mix × overlay alpha)
            let m = mix_max * (oa / 255.0);
            pixels[idx] = (pixels[idx] as f32 + (or - pixels[idx] as f32) * m) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 + (og - pixels[idx + 1] as f32) * m) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 + (ob - pixels[idx + 2] as f32) * m) as u8;
        }
    }
}

// ── Lens Flare (CPU mirror of the WGSL screen-space flare) ──
/// Screen-space optical flare parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct LensFlareParams {
    /// Normalized source position (0..1)
    pub pos_x: f32,
    pub pos_y: f32,
    /// Overall brightness multiplier
    pub intensity: f32,
    /// Output gain applied after element summation
    pub threshold: f32,
    /// Tint color (0-255 RGB)
    pub color: [u8; 3],
}

/// Screen-space optical flare: Gaussian core + concentric ring artifacts +
/// 4-point star streaks, additively tinted onto the buffer.
/// Mirrors `shader.wgsl` fs_main flare pass for GPU/CPU export parity.
pub fn apply_lens_flare(pixels: &mut [u8], width: u32, height: u32, p: &LensFlareParams) {
    if width == 0 || height == 0 || p.intensity <= 0.0 {
        return;
    }
    let w = width as f32;
    let h = height as f32;
    let cx = p.pos_x.clamp(0.0, 1.0);
    let cy = p.pos_y.clamp(0.0, 1.0);
    let intensity = p.intensity;
    let threshold = p.threshold;
    let cr = p.color[0] as f32 / 255.0;
    let cg = p.color[1] as f32 / 255.0;
    let cb = p.color[2] as f32 / 255.0;

    for y in 0..height {
        let ny = (y as f32 + 0.5) / h;
        for x in 0..width {
            let nx = (x as f32 + 0.5) / w;

            let dx = nx - cx;
            let dy = ny - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let d_norm = d * 2.0;

            // Core glow
            let core_radius = 0.02;
            let core = (-d_norm * d_norm / (core_radius * core_radius)).exp() * intensity;

            // Concentric rings fading with distance
            let ring_phase = d_norm * 12.0;
            let ring = (ring_phase.sin() * 0.3) + 0.5;
            let ring_mask = (-d_norm * 3.0).exp();
            let rings = ring * ring_mask * intensity * 0.3;

            // Horizontal + vertical star streaks
            let streak_h = (-dy.abs() * 80.0).exp() * (-dx.abs() * 8.0).exp();
            let streak_v = (-dx.abs() * 80.0).exp() * (-dy.abs() * 8.0).exp();
            let streaks = (streak_h + streak_v) * intensity * 0.4;

            let flare = ((core + rings + streaks) * threshold).min(4.0);
            if flare < 0.002 {
                continue;
            }

            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let r = pixels[idx] as f32 + cr * flare * 255.0;
            let g = pixels[idx + 1] as f32 + cg * flare * 255.0;
            let b = pixels[idx + 2] as f32 + cb * flare * 255.0;
            pixels[idx] = r.min(255.0) as u8;
            pixels[idx + 1] = g.min(255.0) as u8;
            pixels[idx + 2] = b.min(255.0) as u8;
        }
    }
}

// ── Camera Shake ──
/// Applies procedural camera shake to a position value.
/// Returns offset [dx, dy] for the given time.
pub fn camera_shake(time_sec: f32, intensity: f32, speed_hz: f32, seed: u64) -> [f32; 2] {    let t = time_sec * speed_hz;
    let s = seed as f32;
    let dx = (t * 1.7 + s).sin() * 0.6 + (t * 3.3 + s * 1.7).sin() * 0.4;
    let dy = (t * 2.1 + s * 2.3).sin() * 0.5 + (t * 4.7 + s * 0.9).sin() * 0.5;
    [dx * intensity, dy * intensity]
}

#[cfg(test)]
mod camera_shake_tests {
    #[test]
    fn test_camera_shake_is_deterministic() {
        let a1 = super::camera_shake(5.0, 10.0, 8.0, 42);
        let a2 = super::camera_shake(5.0, 10.0, 8.0, 42);
        assert_eq!(a1[0], a2[0]);
        assert_eq!(a1[1], a2[1]);
    }

    #[test]
    fn test_camera_shake_varies_with_seed() {
        let a = super::camera_shake(5.0, 10.0, 8.0, 42);
        let b = super::camera_shake(5.0, 10.0, 8.0, 99);
        assert!(a != b, "different seeds must produce different shake");
    }

    #[test]
    fn test_camera_shake_zero_intensity() {
        let a = super::camera_shake(5.0, 0.0, 8.0, 42);
        assert_eq!(a[0], 0.0);
        assert_eq!(a[1], 0.0);
    }

    #[test]
    fn test_lens_flare_brightens_center() {
        let w = 64u32;
        let h = 64u32;
        let mut px = vec![10u8; (w * h * 4) as usize];
        super::apply_lens_flare(
            &mut px,
            w,
            h,
            &super::LensFlareParams { pos_x: 0.5, pos_y: 0.5, intensity: 1.0, threshold: 1.0, color: [255, 255, 255] },
        );
        let center = ((32 * w + 32) * 4) as usize;
        assert!(px[center] > 200, "flare core must brighten center, got {}", px[center]);
        // Far corner should stay near original
        let corner = ((2 * w + 2) * 4) as usize;
        assert!(px[corner] < 60, "corner must be barely affected, got {}", px[corner]);
    }

    #[test]
    fn test_lens_flare_zero_intensity_noop() {
        let mut px = vec![40u8; 16 * 16 * 4];
        let before = px.clone();
        super::apply_lens_flare(
            &mut px,
            16,
            16,
            &super::LensFlareParams { pos_x: 0.5, pos_y: 0.5, intensity: 0.0, threshold: 1.0, color: [255, 255, 255] },
        );
        assert_eq!(px, before);
    }

    #[test]
    fn test_lens_flare_clamps_to_white() {
        let w = 32u32;
        let h = 32u32;
        let mut px = vec![250u8; (w * h * 4) as usize];
        super::apply_lens_flare(
            &mut px,
            w,
            h,
            &super::LensFlareParams { pos_x: 0.5, pos_y: 0.5, intensity: 5.0, threshold: 2.0, color: [255, 255, 255] },
        );
        // Center region must clamp to pure white
        for y in 12..20u32 {
            for x in 12..20u32 {
                let i = ((y * w + x) * 4) as usize;
                assert_eq!(px[i], 255, "pixel ({x},{y}) must clamp to white");
            }
        }
    }

    #[test]
    fn test_lens_flare_deterministic() {
        let run = || {
            let mut px = vec![0u8; 32 * 32 * 4];
            super::apply_lens_flare(
                &mut px,
                32,
                32,
                &super::LensFlareParams { pos_x: 0.3, pos_y: 0.7, intensity: 1.0, threshold: 1.0, color: [200, 150, 100] },
            );
            px
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_color_overlay_blends_opaque_pixels() {
        let mut px = vec![0u8; 16];
        // One opaque red pixel
        px[0] = 255; px[1] = 0; px[2] = 0; px[3] = 255;
        // One transparent pixel (must stay untouched)
        px[4] = 200; px[5] = 100; px[6] = 50; px[7] = 0;
        let before = px.clone();
        super::apply_color_overlay(&mut px, 2, 2, [0, 0, 255, 255], 50.0);
        assert!(px[0] >= 127 && px[0] <= 128, "red blended halfway toward blue, got {}", px[0]);
        assert!(px[2] >= 127 && px[2] <= 128);
        assert_eq!(&px[4..8], &before[4..8], "transparent pixel untouched");
    }

    #[test]
    fn test_gradient_overlay_direction() {
        // 8x1 layer, gradient left→right (angle 0), white→black
        let mut px = vec![0u8; 8 * 4];
        for i in 0..8 {
            px[i * 4 + 3] = 255;
        }
        super::apply_gradient_overlay(
            &mut px,
            8,
            1,
            &super::GradientOverlayParams {
                angle_deg: 0.0,
                scale_pct: 100.0,
                start: [255, 255, 255, 255],
                end: [0, 0, 0, 255],
                opacity: 100.0,
            },
        );
        let left = px[0];
        let right = px[(7 * 4) as usize];
        assert!(left > 200, "left edge near start color, got {left}");
        assert!(right < 55, "right edge near end color, got {right}");
        assert!(left > right + 100, "strong horizontal ramp expected");
    }

    #[test]
    fn test_gradient_overlay_zero_opacity_noop() {
        let mut px = vec![80u8; 64];
        let before = px.clone();
        super::apply_gradient_overlay(
            &mut px,
            4,
            4,
            &super::GradientOverlayParams {
                angle_deg: 90.0,
                scale_pct: 100.0,
                start: [255, 255, 255, 255],
                end: [0, 0, 0, 255],
                opacity: 0.0,
            },
        );
        assert_eq!(px, before);
    }

    #[test]
    fn test_inner_shadow_darkens_offset_edge() {
        // Opaque rectangle x∈[6..26) inside a 32x8 buffer; angle 90°
        // (light from left) must darken only the LEFT inner band.
        let (w, h) = (32u32, 8u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 6..26u32 {
                let i = ((y * w + x) * 4) as usize;
                px[i] = 200;
                px[i + 1] = 200;
                px[i + 2] = 200;
                px[i + 3] = 255;
            }
        }
        super::apply_inner_shadow(&mut px, w, h, 6.0, 90.0, 1, [0, 0, 0, 255]);
        let left = px[(2 * w as usize + 7) * 4];
        let right = px[(2 * w as usize + 22) * 4];
        assert!(left < right, "left band must darken: left={left} right={right}");
        assert!(right >= 195, "far side stays bright: {right}");
    }

    #[test]
    fn test_inner_glow_brightens_edges_not_center() {
        // Opaque rectangle with transparent margins so alpha edges exist
        let (w, h) = (16u32, 16u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 3..13usize {
            for x in 3..13usize {
                let i = (y * w as usize + x) * 4;
                px[i] = 40;
                px[i + 1] = 40;
                px[i + 2] = 40;
                px[i + 3] = 255;
            }
        }
        super::apply_inner_glow(&mut px, w, h, 2, [255, 255, 0, 255], 100.0);
        let edge = ((4 * w as usize + 4) * 4) as usize;
        let center = ((8 * w as usize + 8) * 4) as usize;
        assert!(px[edge] > 60, "inner edge must glow, got {}", px[edge]);
        assert_eq!(px[center], 40, "deep interior untouched");
    }

    fn opaque_rect(w: u32, h: u32, x0: u32, x1: u32, y0: u32, y1: u32, gray: u8) -> Vec<u8> {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 4) as usize;
                px[i] = gray;
                px[i + 1] = gray;
                px[i + 2] = gray;
                px[i + 3] = 255;
            }
        }
        px
    }

    #[test]
    fn test_satin_bands_one_side() {
        // Wide rect; angle 90° (dx=+distance): sheen band near LEFT interior.
        let (w, h) = (48u32, 16u32);
        let mut px = opaque_rect(w, h, 6, 42, 2, 14, 120);
        super::apply_satin(
            &mut px,
            w,
            h,
            &super::SatinParams { distance: 10.0, angle_deg: 90.0, size: 4, color: [30, 30, 60, 255], opacity: 100.0 },
        );
        let band = px[(7 * w as usize + 9) * 4];
        let far = px[(7 * w as usize + 38) * 4];
        assert!(band < far, "left band tinted: band={band} far={far}");
        assert!(far <= 120 && far >= 110, "far side ~original: {far}");
    }

    #[test]
    fn test_satin_zero_opacity_noop() {
        let (w, h) = (16u32, 16u32);
        let mut px = opaque_rect(w, h, 2, 14, 2, 14, 90);
        let before = px.clone();
        super::apply_satin(
            &mut px,
            w,
            h,
            &super::SatinParams { distance: 5.0, angle_deg: 45.0, size: 4, color: [200, 0, 0, 255], opacity: 0.0 },
        );
        assert_eq!(px, before);
    }
}
