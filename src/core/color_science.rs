/// Specialized Color Science modules for professional film and grading workflows.
///
/// Contains mathematically accurate implementations for:
/// - Tone Curves (Cubic spline interpolation for R, G, B, and Luminance)
/// - Levels adjustment (Input / Output clamping and gamma correction)
/// - Hue, Saturation, and Lightness (HSL) conversion and shifting
///
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

/// 3D LUT Color Grade Table with Tetrahedral Interpolation from NextVFX Engine.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct Lut3D {
    pub size: usize,
    pub data: Vec<f32>, // Flat RGB array of length size * size * size * 3
}

#[allow(dead_code)]
impl Lut3D {
    /// Create a flat identity 3D LUT of dimension `size`.
    pub fn identity(size: usize) -> Self {
        let mut data = Vec::with_capacity(size * size * size * 3);
        let s = (size - 1) as f32;
        for r in 0..size {
            for g in 0..size {
                for b in 0..size {
                    data.push(r as f32 / s);
                    data.push(g as f32 / s);
                    data.push(b as f32 / s);
                }
            }
        }
        Self { size, data }
    }

    /// Tetrahedral 3D LUT interpolation algorithm from NextVFX.
    /// Performs exact tetrahedral simplex subdivision to avoid color banding.
    pub fn apply(&self, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        if self.size == 0 || self.data.is_empty() {
            return (r, g, b);
        }
        let r_safe = if r.is_nan() { 0.0 } else { r };
        let g_safe = if g.is_nan() { 0.0 } else { g };
        let b_safe = if b.is_nan() { 0.0 } else { b };

        let s = (self.size - 1) as f32;
        let fr = (r_safe * s).clamp(0.0, s - 0.001);
        let fg = (g_safe * s).clamp(0.0, s - 0.001);
        let fb = (b_safe * s).clamp(0.0, s - 0.001);

        let ir = fr as usize;
        let ig = fg as usize;
        let ib = fb as usize;

        let dr = fr - ir as f32;
        let dg = fg - ig as f32;
        let db = fb - ib as f32;

        let get = |x: usize, y: usize, z: usize| {
            let idx = (x * self.size * self.size + y * self.size + z) * 3;
            if idx + 2 < self.data.len() {
                (self.data[idx], self.data[idx + 1], self.data[idx + 2])
            } else {
                (r, g, b)
            }
        };

        let c000 = get(ir, ig, ib);
        let c111 = get(ir + 1, ig + 1, ib + 1);

        if dr > dg {
            if dg > db { // R > G > B
                let c100 = get(ir + 1, ig, ib);
                let c110 = get(ir + 1, ig + 1, ib);
                (
                    c000.0 + (c100.0 - c000.0) * dr + (c110.0 - c100.0) * dg + (c111.0 - c110.0) * db,
                    c000.1 + (c100.1 - c000.1) * dr + (c110.1 - c100.1) * dg + (c111.1 - c110.1) * db,
                    c000.2 + (c100.2 - c000.2) * dr + (c110.2 - c100.2) * dg + (c111.2 - c110.2) * db,
                )
            } else if dr > db { // R > B > G
                let c100 = get(ir + 1, ig, ib);
                let c101 = get(ir + 1, ig, ib + 1);
                (
                    c000.0 + (c100.0 - c000.0) * dr + (c101.0 - c100.0) * db + (c111.0 - c101.0) * dg,
                    c000.1 + (c100.1 - c000.1) * dr + (c101.1 - c100.1) * db + (c111.1 - c101.1) * dg,
                    c000.2 + (c100.2 - c000.2) * dr + (c101.2 - c100.2) * db + (c111.2 - c101.2) * dg,
                )
            } else { // B > R > G
                let c001 = get(ir, ig, ib + 1);
                let c101 = get(ir + 1, ig, ib + 1);
                (
                    c000.0 + (c001.0 - c000.0) * db + (c101.0 - c001.0) * dr + (c111.0 - c101.0) * dg,
                    c000.1 + (c001.1 - c000.1) * db + (c101.1 - c001.1) * dr + (c111.1 - c101.1) * dg,
                    c000.2 + (c001.2 - c000.2) * db + (c101.2 - c001.2) * dr + (c111.2 - c101.2) * dg,
                )
            }
        } else if db > dg { // B > G > R
            let c001 = get(ir, ig, ib + 1);
            let c011 = get(ir, ig + 1, ib + 1);
            (
                c000.0 + (c001.0 - c000.0) * db + (c011.0 - c001.0) * dg + (c111.0 - c011.0) * dr,
                c000.1 + (c001.1 - c000.1) * db + (c011.1 - c001.1) * dg + (c111.1 - c011.1) * dr,
                c000.2 + (c001.2 - c000.2) * db + (c011.2 - c001.2) * dg + (c111.2 - c011.2) * dr,
            )
        } else if db > dr { // G > B > R
            let c010 = get(ir, ig + 1, ib);
            let c011 = get(ir, ig + 1, ib + 1);
            (
                c000.0 + (c010.0 - c000.0) * dg + (c011.0 - c010.0) * db + (c111.0 - c011.0) * dr,
                c000.1 + (c010.1 - c000.1) * dg + (c011.1 - c010.1) * db + (c111.1 - c011.1) * dr,
                c000.2 + (c010.2 - c000.2) * dg + (c011.2 - c010.2) * db + (c111.2 - c011.2) * dr,
            )
        } else { // G > R > B
            let c010 = get(ir, ig + 1, ib);
            let c110 = get(ir + 1, ig + 1, ib);
            (
                c000.0 + (c010.0 - c000.0) * dg + (c110.0 - c010.0) * dr + (c111.0 - c110.0) * db,
                c000.1 + (c010.1 - c000.1) * dg + (c110.1 - c010.1) * dr + (c111.1 - c110.1) * db,
                c000.2 + (c010.2 - c000.2) * dg + (c110.2 - c010.2) * dr + (c111.2 - c110.2) * db,
            )
        }
    }
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

    #[test]
    fn test_lut3d_tetrahedral_identity() {
        let lut = Lut3D::identity(17);
        let mapped = lut.apply(0.5, 0.25, 0.75);
        assert!((mapped.0 - 0.5).abs() < 1e-3);
        assert!((mapped.1 - 0.25).abs() < 1e-3);
        assert!((mapped.2 - 0.75).abs() < 1e-3);
    }

    #[test]
    fn test_lut3d_nan_input_sanitization() {
        let lut = Lut3D::identity(4);
        let (r, g, b) = lut.apply(f32::NAN, f32::NAN, f32::NAN);
        // NaN inputs must be sanitized to 0.0 and produce valid (non-NaN) outputs
        assert!(!r.is_nan(), "Lut3D::apply returned NaN for NaN r input");
        assert!(!g.is_nan(), "Lut3D::apply returned NaN for NaN g input");
        assert!(!b.is_nan(), "Lut3D::apply returned NaN for NaN b input");
        // Identity LUT with 0.0 input → should return near 0.0
        assert!(r.abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
    }
}
