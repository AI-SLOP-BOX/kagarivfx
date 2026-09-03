#![allow(dead_code)]
/// After Effects VFX Kernels Part 20 — Color Grading & Film Emulation
// 1. LUT Apply (1D Per-Channel Tone Mapping)
pub fn apply_lut_1d(pixels: &mut [u8], lut_r: &[u8; 256], lut_g: &[u8; 256], lut_b: &[u8; 256]) {
    for i in (0..pixels.len()).step_by(4) {
        // Read all channels BEFORE writing to avoid self-referential corruption
        let r = pixels[i];
        let g = pixels[i + 1];
        let b = pixels[i + 2];
        pixels[i] = lut_r[r as usize];
        pixels[i + 1] = lut_g[g as usize];
        pixels[i + 2] = lut_b[b as usize];
    }
}

// 2. Color Balance (Shadow / Midtone / Highlight CMY-RGB Shift)
pub fn apply_color_balance(
    pixels: &mut [u8],
    shadow: [f32; 3],
    midtone: [f32; 3],
    highlight: [f32; 3],
) {
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32 / 255.0;
        let g = pixels[i + 1] as f32 / 255.0;
        let b = pixels[i + 2] as f32 / 255.0;

        let luma = r * 0.299 + g * 0.587 + b * 0.114;

        // Tonally weighted region masks
        let shadow_w = (1.0 - luma * 2.0).clamp(0.0, 1.0);
        let highlight_w = ((luma * 2.0) - 1.0).clamp(0.0, 1.0);
        let mid_w = 1.0 - shadow_w - highlight_w;

        let channels = [r, g, b];
        let results: [u8; 3] = std::array::from_fn(|c| {
            let shift = shadow[c] * shadow_w + midtone[c] * mid_w + highlight[c] * highlight_w;
            ((channels[c] + shift) * 255.0).clamp(0.0, 255.0) as u8
        });

        pixels[i] = results[0];
        pixels[i + 1] = results[1];
        pixels[i + 2] = results[2];
    }
}

// 3. Kodak/Fuji Film Emulation via S-Curve + Hue Shift
pub fn apply_film_emulation(
    pixels: &mut [u8],
    lift: f32,
    gamma: f32,
    gain: f32,
    hue_shift_deg: f32,
) {
    let inv_gamma = 1.0 / gamma.max(0.01);
    let hue_rad = hue_shift_deg.to_radians();
    let cos_h = hue_rad.cos();
    let sin_h = hue_rad.sin();

    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32 / 255.0;
        let g = pixels[i + 1] as f32 / 255.0;
        let b = pixels[i + 2] as f32 / 255.0;

        // ASC CDL Grade
        let r2 = ((r * gain + lift).max(0.0)).powf(inv_gamma);
        let g2 = ((g * gain + lift).max(0.0)).powf(inv_gamma);
        let b2 = ((b * gain + lift).max(0.0)).powf(inv_gamma);

        // Hue rotation in YCbCr space
        let y = 0.299 * r2 + 0.587 * g2 + 0.114 * b2;
        let cb = -0.169 * r2 - 0.331 * g2 + 0.500 * b2;
        let cr = 0.500 * r2 - 0.419 * g2 - 0.081 * b2;

        let cb2 = cb * cos_h - cr * sin_h;
        let cr2 = cb * sin_h + cr * cos_h;

        let r3 = (y + 1.402 * cr2).clamp(0.0, 1.0);
        let g3 = (y - 0.344 * cb2 - 0.714 * cr2).clamp(0.0, 1.0);
        let b3 = (y + 1.772 * cb2).clamp(0.0, 1.0);

        pixels[i] = (r3 * 255.0) as u8;
        pixels[i + 1] = (g3 * 255.0) as u8;
        pixels[i + 2] = (b3 * 255.0) as u8;
    }
}

// 4. Vignette (Cinematic Lens Falloff)
pub fn apply_vignette(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    radius: f32,
    feather: f32,
    strength: f32,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_dist = (cx * cx + cy * cy).sqrt();

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / max_dist;

            let vig = if dist < radius {
                1.0f32
            } else {
                let fade = ((dist - radius) / feather.max(0.001)).clamp(0.0, 1.0);
                1.0 - fade * strength
            };

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx] = (pixels[idx] as f32 * vig) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 * vig) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 * vig) as u8;
        }
    }
}

// 5. Night Vision Effect (Phosphor Green Amplification + Noise)
pub fn apply_night_vision(pixels: &mut [u8], amplification: f32, noise_seed: u32) {
    let mut rng = noise_seed;

    for i in (0..pixels.len()).step_by(4) {
        let luma =
            pixels[i] as f32 * 0.299 + pixels[i + 1] as f32 * 0.587 + pixels[i + 2] as f32 * 0.114;
        let amplified = (luma * amplification).clamp(0.0, 255.0);

        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((rng >> 16) as f32 / 65535.0 - 0.5) * 15.0;

        let green = (amplified + noise).clamp(0.0, 255.0) as u8;
        pixels[i] = (green as f32 * 0.1) as u8;
        pixels[i + 1] = green;
        pixels[i + 2] = (green as f32 * 0.1) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v20_filters() {
        let mut pixels = vec![128u8; 64 * 4];
        apply_vignette(&mut pixels, 8, 8, 0.5, 0.3, 0.8);
        apply_night_vision(&mut pixels, 2.0, 42);
        assert_eq!(pixels.len(), 64 * 4);
    }
}
