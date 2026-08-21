#![allow(dead_code)]
/// After Effects VFX Kernels Part 24 — Advanced Color Science & Remapping
// 1. White Balance Correction (Temperature + Tint Kelvin Model)
pub fn apply_white_balance(pixels: &mut [u8], temperature: f32, tint: f32) {
    // temperature > 0 = warmer (orange), < 0 = cooler (blue)
    // tint > 0 = green, < 0 = magenta
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32;
        let g = pixels[i + 1] as f32;
        let b = pixels[i + 2] as f32;

        let new_r = (r + temperature * 0.3).clamp(0.0, 255.0);
        let new_g = (g + tint * 0.3).clamp(0.0, 255.0);
        let new_b = (b - temperature * 0.3).clamp(0.0, 255.0);

        pixels[i] = new_r as u8;
        pixels[i + 1] = new_g as u8;
        pixels[i + 2] = new_b as u8;
    }
}

// 2. HSL Selective Color Correction (Target Hue Range Only)
pub fn apply_hsl_selective(pixels: &mut [u8], target_hue: f32, hue_range: f32, hue_shift: f32, sat_shift: f32, lum_shift: f32) {
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32 / 255.0;
        let g = pixels[i + 1] as f32 / 255.0;
        let b = pixels[i + 2] as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        if delta < 0.001 { continue; }

        let hue = if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        let hue_diff = (hue - target_hue + 180.0).rem_euclid(360.0) - 180.0;
        let weight = (1.0 - (hue_diff / hue_range.max(1.0)).abs()).clamp(0.0, 1.0);
        if weight <= 0.001 { continue; }

        let sat = delta / max;
        let lum = (max + min) * 0.5;

        let new_hue = (hue + hue_shift * weight).rem_euclid(360.0);
        let new_sat = (sat + sat_shift * weight).clamp(0.0, 1.0);
        let new_lum = (lum + lum_shift * weight * 255.0 / 255.0).clamp(0.0, 1.0);

        // Convert back HSL -> RGB (simplified)
        let c = (1.0 - (2.0 * new_lum - 1.0).abs()) * new_sat;
        let x = c * (1.0 - ((new_hue / 60.0).rem_euclid(2.0) - 1.0).abs());
        let m = new_lum - c * 0.5;
        let (r2, g2, b2) = match (new_hue / 60.0) as u32 {
            0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
            3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
        };

        pixels[i] = ((r2 + m) * 255.0).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = ((g2 + m) * 255.0).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = ((b2 + m) * 255.0).clamp(0.0, 255.0) as u8;
    }
}

// 3. Tone Curve with Cubic Spline (3-Point Custom Curve)
pub fn apply_tone_curve(pixels: &mut [u8], black: f32, gamma_point: f32, white: f32) {
    let lut: Vec<u8> = (0..256).map(|i| {
        let t = i as f32 / 255.0;
        // Piecewise cubic: shadow -> gamma -> highlight
        let v = if t < 0.5 {
            (black + (gamma_point - black) * (t / 0.5).powf(0.6)).clamp(0.0, 1.0)
        } else {
            (gamma_point + (white - gamma_point) * ((t - 0.5) / 0.5).powf(0.6)).clamp(0.0, 1.0)
        };
        (v * 255.0) as u8
    }).collect();

    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = lut[pixels[i] as usize];
        pixels[i + 1] = lut[pixels[i + 1] as usize];
        pixels[i + 2] = lut[pixels[i + 2] as usize];
    }
}

// 4. Posterize (Step Quantize Color Levels)
pub fn apply_posterize(pixels: &mut [u8], levels: u8) {
    if levels < 2 { return; }
    let step = 255.0 / (levels - 1) as f32;

    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            pixels[i + c] = ((pixels[i + c] as f32 / step).round() * step).clamp(0.0, 255.0) as u8;
        }
    }
}

// 5. Tritone Effect (3-Zone Luminance Color Map)
pub fn apply_tritone(pixels: &mut [u8], shadow: [u8; 3], midtone: [u8; 3], highlight: [u8; 3]) {
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299 + pixels[i + 1] as f32 * 0.587 + pixels[i + 2] as f32 * 0.114) / 255.0;

        let (ca, cb, t) = if luma < 0.5 {
            (shadow, midtone, luma * 2.0)
        } else {
            (midtone, highlight, (luma - 0.5) * 2.0)
        };

        pixels[i] = (ca[0] as f32 * (1.0 - t) + cb[0] as f32 * t) as u8;
        pixels[i + 1] = (ca[1] as f32 * (1.0 - t) + cb[1] as f32 * t) as u8;
        pixels[i + 2] = (ca[2] as f32 * (1.0 - t) + cb[2] as f32 * t) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v24_filters() {
        let mut pixels = vec![128u8; 8 * 8 * 4];
        apply_white_balance(&mut pixels, 20.0, -10.0);
        apply_posterize(&mut pixels, 4);
        apply_tritone(&mut pixels, [0, 0, 50], [100, 100, 100], [255, 240, 200]);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
