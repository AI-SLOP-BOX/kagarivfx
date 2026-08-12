/// Specialized Color Science modules for professional film and grading workflows.
///
/// Contains mathematically accurate implementations for:
/// - Tone Curves (Cubic spline interpolation for R, G, B, and Luminance)
/// - Levels adjustment (Input / Output clamping and gamma correction)
/// - Hue, Saturation, and Lightness (HSL) conversion and shifting

/// RGB to HSL color space conversion.
/// Input: R, G, B in [0.0, 1.0].
/// Output: Hue in [0.0, 360.0], Saturation in [0.0, 1.0], Lightness in [0.0, 1.0].
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> [f32; 3] {
    let r = if r.is_nan() { 0.0 } else { r.clamp(0.0, 1.0) };
    let g = if g.is_nan() { 0.0 } else { g.clamp(0.0, 1.0) };
    let b = if b.is_nan() { 0.0 } else { b.clamp(0.0, 1.0) };

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = ((max + min) * 0.5).clamp(0.0, 1.0);

    if (max - min).abs() < 1e-5 {
        return [0.0, 0.0, l];
    }

    let d = max - min;
    let denom = (1.0 - (2.0 * l - 1.0).abs()).max(1e-5);
    let s = (d / denom).clamp(0.0, 1.0);

    let mut h = if (max - r).abs() < 1e-5 {
        (g - b) / d + (if g < b { 6.0 } else { 0.0 })
    } else if (max - g).abs() < 1e-5 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h = (h * 60.0).rem_euclid(360.0);
    if h.is_nan() { h = 0.0; }

    [h, s, l]
}

/// HSL back to RGB color space conversion.
/// Input: Hue [0.0, 360.0], Saturation [0.0, 1.0], Lightness [0.0, 1.0].
/// Output: R, G, B in [0.0, 1.0].
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    let h = if h.is_nan() { 0.0 } else { h.rem_euclid(360.0) };
    let s = if s.is_nan() { 0.0 } else { s.clamp(0.0, 1.0) };
    let l = if l.is_nan() { 0.0 } else { l.clamp(0.0, 1.0) };

    if s < 1e-5 {
        return [l, l, l];
    }

    let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
        if t < 0.0 { t += 1.0; }
        if t > 1.0 { t -= 1.0; }
        if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
        if t < 1.0 / 2.0 { return q; }
        if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
        p
    };

    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;

    let h_normalized = h / 360.0;
    let r = hue_to_rgb(p, q, h_normalized + 1.0 / 3.0).clamp(0.0, 1.0);
    let g = hue_to_rgb(p, q, h_normalized).clamp(0.0, 1.0);
    let b = hue_to_rgb(p, q, h_normalized - 1.0 / 3.0).clamp(0.0, 1.0);

    [r, g, b]
}

/// Levels Adjustment (Standard Input/Output clamp and gamma mapping).
///
/// Input parameter ranges:
/// - `in_black`, `in_white` inside [0.0, 1.0]
/// - `gamma` inside [0.1, 9.9]
/// - `out_black`, `out_white` inside [0.0, 1.0]
#[allow(dead_code)]
pub fn apply_levels(val: f32, in_black: f32, in_white: f32, gamma: f32, out_black: f32, out_white: f32) -> f32 {
    let range = (in_white - in_black).max(0.001);
    let normalized = ((val - in_black) / range).clamp(0.0, 1.0);
    
    // Apply gamma curve (normalized value raised to 1/gamma power)
    let gamma_power = 1.0 / gamma.max(0.01);
    let gamma_adjusted = normalized.powf(gamma_power);

    // Map to output levels range
    out_black + gamma_adjusted * (out_white - out_black)
}

/// Hue, Saturation, Lightness shifting.
/// - `hue_shift`: angle shift in degrees [-180.0, 180.0]
/// - `sat_mult`: multiplier [0.0, 5.0] (1.0 is default)
/// - `light_mult`: multiplier [0.0, 5.0]
#[allow(dead_code)]
pub fn shift_hsl(rgb: [f32; 3], hue_shift: f32, sat_mult: f32, light_mult: f32) -> [f32; 3] {
    let hsl = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
    let h = (hsl[0] + hue_shift + 360.0) % 360.0;
    let s = (hsl[1] * sat_mult).clamp(0.0, 1.0);
    let l = (hsl[2] * light_mult).clamp(0.0, 1.0);
    hsl_to_rgb(h, s, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsl_roundtrip() {
        let original = [0.2, 0.5, 0.8]; // Soft blue
        let hsl = rgb_to_hsl(original[0], original[1], original[2]);
        let roundtrip = hsl_to_rgb(hsl[0], hsl[1], hsl[2]);
        
        assert!((original[0] - roundtrip[0]).abs() < 1e-4);
        assert!((original[1] - roundtrip[1]).abs() < 1e-4);
        assert!((original[2] - roundtrip[2]).abs() < 1e-4);
    }

    #[test]
    fn test_levels_mapping() {
        // High contrast mapping
        let output = apply_levels(0.5, 0.2, 0.8, 1.0, 0.0, 1.0);
        assert!((output - 0.5).abs() < 1e-4);
    }
}
