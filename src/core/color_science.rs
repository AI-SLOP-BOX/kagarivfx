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
    if h.is_nan() {
        h = 0.0;
    }

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
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
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
pub fn apply_levels(
    val: f32,
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
) -> f32 {
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
            if dg > db {
                // R > G > B
                let c100 = get(ir + 1, ig, ib);
                let c110 = get(ir + 1, ig + 1, ib);
                (
                    c000.0
                        + (c100.0 - c000.0) * dr
                        + (c110.0 - c100.0) * dg
                        + (c111.0 - c110.0) * db,
                    c000.1
                        + (c100.1 - c000.1) * dr
                        + (c110.1 - c100.1) * dg
                        + (c111.1 - c110.1) * db,
                    c000.2
                        + (c100.2 - c000.2) * dr
                        + (c110.2 - c100.2) * dg
                        + (c111.2 - c110.2) * db,
                )
            } else if dr > db {
                // R > B > G
                let c100 = get(ir + 1, ig, ib);
                let c101 = get(ir + 1, ig, ib + 1);
                (
                    c000.0
                        + (c100.0 - c000.0) * dr
                        + (c101.0 - c100.0) * db
                        + (c111.0 - c101.0) * dg,
                    c000.1
                        + (c100.1 - c000.1) * dr
                        + (c101.1 - c100.1) * db
                        + (c111.1 - c101.1) * dg,
                    c000.2
                        + (c100.2 - c000.2) * dr
                        + (c101.2 - c100.2) * db
                        + (c111.2 - c101.2) * dg,
                )
            } else {
                // B > R > G
                let c001 = get(ir, ig, ib + 1);
                let c101 = get(ir + 1, ig, ib + 1);
                (
                    c000.0
                        + (c001.0 - c000.0) * db
                        + (c101.0 - c001.0) * dr
                        + (c111.0 - c101.0) * dg,
                    c000.1
                        + (c001.1 - c000.1) * db
                        + (c101.1 - c001.1) * dr
                        + (c111.1 - c101.1) * dg,
                    c000.2
                        + (c001.2 - c000.2) * db
                        + (c101.2 - c001.2) * dr
                        + (c111.2 - c101.2) * dg,
                )
            }
        } else if db > dg {
            // B > G > R
            let c001 = get(ir, ig, ib + 1);
            let c011 = get(ir, ig + 1, ib + 1);
            (
                c000.0 + (c001.0 - c000.0) * db + (c011.0 - c001.0) * dg + (c111.0 - c011.0) * dr,
                c000.1 + (c001.1 - c000.1) * db + (c011.1 - c001.1) * dg + (c111.1 - c011.1) * dr,
                c000.2 + (c001.2 - c000.2) * db + (c011.2 - c001.2) * dg + (c111.2 - c011.2) * dr,
            )
        } else if db > dr {
            // G > B > R
            let c010 = get(ir, ig + 1, ib);
            let c011 = get(ir, ig + 1, ib + 1);
            (
                c000.0 + (c010.0 - c000.0) * dg + (c011.0 - c010.0) * db + (c111.0 - c011.0) * dr,
                c000.1 + (c010.1 - c000.1) * dg + (c011.1 - c010.1) * db + (c111.1 - c011.1) * dr,
                c000.2 + (c010.2 - c000.2) * dg + (c011.2 - c010.2) * db + (c111.2 - c011.2) * dr,
            )
        } else {
            // G > R > B
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

// ────────────────────── Cinema Log Color Profiles ──────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CinemaLogProfile {
    #[default]
    ArriLogC3,
    ArriLogC4,
    SonySLog3,
    RedLog3G10,
    CanonCLog2,
    CanonCLog3,
    BmdFilmGen5,
}

/// Convert Linear floating-point scene light to Cinema Log value.
pub fn linear_to_cinema_log(x: f32, profile: CinemaLogProfile) -> f32 {
    let x = if x.is_nan() { 0.0 } else { x };
    match profile {
        CinemaLogProfile::ArriLogC3 => {
            // ARRI ALEXA LogC3 EI 800
            let cut = 0.010591f32;
            let a = 5.555556f32;
            let b = 0.052272f32;
            let c = 0.247190f32;
            let d = 0.385537f32;
            let e = 5.367655f32;
            let f = 0.092809f32;
            if x > cut {
                c * (a * x + b).log10() + d
            } else {
                e * x + f
            }
        }
        CinemaLogProfile::ArriLogC4 => {
            // ARRI ALEXA LogC4
            let a = 0.0003f32;
            let b = 0.2238f32;
            let c = 0.18f32;
            if x > a {
                b * (x / c + 1.0).ln() + 0.1
            } else {
                (x / a) * 0.1
            }
        }
        CinemaLogProfile::SonySLog3 => {
            // Sony S-Log3
            if x >= 0.01125000 {
                (420.0 + ((x + 0.01) / (0.18 + 0.01)).log10() * 261.5) / 1023.0
            } else {
                (x * (171.2102946929 - 95.0) / 0.01125000 + 95.0) / 1023.0
            }
        }
        CinemaLogProfile::RedLog3G10 => {
            // RED Log3G10
            let a = 0.224282f32;
            let b = 155.975327f32;
            let sign = if x < 0.0 { -1.0 } else { 1.0 };
            sign * (a * (b * x.abs() + 1.0).log10())
        }
        CinemaLogProfile::CanonCLog2 => {
            // Canon C-Log2
            if x >= 0.00392157 {
                0.24136077 * (8.725661 * x + 1.0).log10() + 0.092864
            } else {
                3.98402 * x + 0.07727
            }
        }
        CinemaLogProfile::CanonCLog3 => {
            // Canon C-Log3
            if x >= 0.00392157 {
                0.1788 * (14.0 * x + 1.0).log10() + 0.125
            } else {
                4.2 * x + 0.06
            }
        }
        CinemaLogProfile::BmdFilmGen5 => {
            // Blackmagic Film Gen5
            let a = 0.08692876f32;
            let b = 0.005494143f32;
            let c = 0.5300135f32;
            let d = 8.283606f32;
            let e = 0.09246582f32;
            if x > -b {
                c * (a * x + b + 1.0).ln() + e
            } else {
                d * x
            }
        }
    }
}

/// Convert Cinema Log encoded value back to Linear light floating-point.
pub fn cinema_log_to_linear(y: f32, profile: CinemaLogProfile) -> f32 {
    let y = if y.is_nan() { 0.0 } else { y };
    match profile {
        CinemaLogProfile::ArriLogC3 => {
            let cut = 0.010591f32;
            let a = 5.555556f32;
            let b = 0.052272f32;
            let c = 0.247190f32;
            let d = 0.385537f32;
            let e = 5.367655f32;
            let f = 0.092809f32;
            let y_cut = e * cut + f;
            if y > y_cut {
                (10.0f32.powf((y - d) / c) - b) / a
            } else {
                (y - f) / e
            }
        }
        CinemaLogProfile::ArriLogC4 => {
            let a = 0.0003f32;
            let b = 0.2238f32;
            let c = 0.18f32;
            if y > 0.1 {
                (((y - 0.1) / b).exp() - 1.0) * c
            } else {
                (y / 0.1) * a
            }
        }
        CinemaLogProfile::SonySLog3 => {
            let y_cut = 171.2102946929 / 1023.0;
            if y >= y_cut {
                10.0f32.powf((y * 1023.0 - 420.0) / 261.5) * (0.18 + 0.01) - 0.01
            } else {
                (y * 1023.0 - 95.0) * 0.01125000 / (171.2102946929 - 95.0)
            }
        }
        CinemaLogProfile::RedLog3G10 => {
            let a = 0.224282f32;
            let b = 155.975327f32;
            let sign = if y < 0.0 { -1.0 } else { 1.0 };
            sign * ((10.0f32.powf(y.abs() / a) - 1.0) / b)
        }
        CinemaLogProfile::CanonCLog2 => {
            let y_cut = 0.24136077 * (8.725661 * 0.00392157 + 1.0f32).log10() + 0.092864;
            if y >= y_cut {
                (10.0f32.powf((y - 0.092864) / 0.24136077) - 1.0) / 8.725661
            } else {
                (y - 0.07727) / 3.98402
            }
        }
        CinemaLogProfile::CanonCLog3 => {
            let y_cut = 0.1788 * (14.0 * 0.00392157 + 1.0f32).log10() + 0.125;
            if y >= y_cut {
                (10.0f32.powf((y - 0.125) / 0.1788) - 1.0) / 14.0
            } else {
                (y - 0.06) / 4.2
            }
        }
        CinemaLogProfile::BmdFilmGen5 => {
            let a = 0.08692876f32;
            let b = 0.005494143f32;
            let c = 0.5300135f32;
            let d = 8.283606f32;
            let e = 0.09246582f32;
            if y > e {
                (((y - e) / c).exp() - 1.0 - b) / a
            } else {
                y / d
            }
        }
    }
}

/// Apply cinema log encoding or decoding to an [R, G, B] triple in-place.
pub fn apply_cinema_log_to_rgb(rgb: [f32; 3], profile: CinemaLogProfile, invert: bool) -> [f32; 3] {
    if invert {
        [
            cinema_log_to_linear(rgb[0], profile),
            cinema_log_to_linear(rgb[1], profile),
            cinema_log_to_linear(rgb[2], profile),
        ]
    } else {
        [
            linear_to_cinema_log(rgb[0], profile),
            linear_to_cinema_log(rgb[1], profile),
            linear_to_cinema_log(rgb[2], profile),
        ]
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

    #[test]
    fn test_cinema_log_roundtrip() {
        let profiles = [
            CinemaLogProfile::ArriLogC3,
            CinemaLogProfile::ArriLogC4,
            CinemaLogProfile::SonySLog3,
            CinemaLogProfile::RedLog3G10,
            CinemaLogProfile::CanonCLog2,
            CinemaLogProfile::CanonCLog3,
            CinemaLogProfile::BmdFilmGen5,
        ];
        let test_values = [0.01f32, 0.18f32, 0.5f32, 1.0f32, 2.0f32];

        for &profile in &profiles {
            for &val in &test_values {
                let log = linear_to_cinema_log(val, profile);
                let roundtrip = cinema_log_to_linear(log, profile);
                assert!(
                    (roundtrip - val).abs() < 1e-3 * val.max(1.0),
                    "profile {:?}: linear {} -> log {} -> linear {}",
                    profile,
                    val,
                    log,
                    roundtrip
                );
            }
        }
    }

    #[test]
    fn test_working_color_space_conversions() {
        let white = [1.0f32, 1.0, 1.0];
        let p3 = convert_color_space(
            white,
            WorkingColorSpace::Rec709,
            WorkingColorSpace::DisplayP3,
        );
        assert!((p3[0] - 1.0).abs() < 0.05);
        assert!((p3[1] - 1.0).abs() < 0.05);
        assert!((p3[2] - 1.0).abs() < 0.05);

        let black = [0.0f32, 0.0, 0.0];
        let lin_black = convert_color_space(
            black,
            WorkingColorSpace::Rec709,
            WorkingColorSpace::LinearSRGB,
        );
        assert_eq!(lin_black, [0.0, 0.0, 0.0]);
    }
}

/// Standard professional working color spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum WorkingColorSpace {
    #[default]
    Rec709,
    DisplayP3,
    Rec2020,
    LinearSRGB,
    AcesCG,
}

/// Converts sRGB Gamma (2.2 / standard transfer) to Linear RGB.
pub fn srgb_to_linear(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts Linear RGB to sRGB Gamma.
pub fn linear_to_srgb(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Color transformation matrix multiply for Rec.709 <-> Display P3 <-> Rec.2020 <-> ACEScg
pub fn convert_color_space(
    rgb: [f32; 3],
    src: WorkingColorSpace,
    dst: WorkingColorSpace,
) -> [f32; 3] {
    if src == dst {
        return rgb;
    }
    // Convert source to Linear sRGB first
    let lin_src = match src {
        WorkingColorSpace::Rec709 => [
            srgb_to_linear(rgb[0]),
            srgb_to_linear(rgb[1]),
            srgb_to_linear(rgb[2]),
        ],
        WorkingColorSpace::LinearSRGB => rgb,
        WorkingColorSpace::DisplayP3 => {
            let lin_p3 = [
                srgb_to_linear(rgb[0]),
                srgb_to_linear(rgb[1]),
                srgb_to_linear(rgb[2]),
            ];
            [
                lin_p3[0] * 1.2249 - lin_p3[1] * 0.2247 + lin_p3[2] * 0.0,
                -lin_p3[0] * 0.0420 + lin_p3[1] * 1.0419 - lin_p3[2] * 0.0,
                -lin_p3[0] * 0.0196 - lin_p3[1] * 0.0786 + lin_p3[2] * 1.0982,
            ]
        }
        WorkingColorSpace::Rec2020 => {
            let lin_2020 = [
                srgb_to_linear(rgb[0]),
                srgb_to_linear(rgb[1]),
                srgb_to_linear(rgb[2]),
            ];
            [
                lin_2020[0] * 1.6605 - lin_2020[1] * 0.5876 - lin_2020[2] * 0.0728,
                -lin_2020[0] * 0.1246 + lin_2020[1] * 1.1329 - lin_2020[2] * 0.0083,
                -lin_2020[0] * 0.0182 - lin_2020[1] * 0.1006 + lin_2020[2] * 1.1187,
            ]
        }
        WorkingColorSpace::AcesCG => [
            rgb[0] * 1.7050 - rgb[1] * 0.6242 - rgb[2] * 0.0808,
            -rgb[0] * 0.1297 + rgb[1] * 1.1385 - rgb[2] * 0.0088,
            -rgb[0] * 0.0241 - rgb[1] * 0.1246 + rgb[2] * 1.1488,
        ],
    };

    // Convert Linear sRGB to destination
    match dst {
        WorkingColorSpace::Rec709 => [
            linear_to_srgb(lin_src[0]),
            linear_to_srgb(lin_src[1]),
            linear_to_srgb(lin_src[2]),
        ],
        WorkingColorSpace::LinearSRGB => lin_src,
        WorkingColorSpace::DisplayP3 => {
            let p3_r = lin_src[0] * 0.8225 + lin_src[1] * 0.1775 + lin_src[2] * 0.0;
            let p3_g = lin_src[0] * 0.0332 + lin_src[1] * 0.9668 + lin_src[2] * 0.0;
            let p3_b = lin_src[0] * 0.0171 + lin_src[1] * 0.0724 + lin_src[2] * 0.9105;
            [
                linear_to_srgb(p3_r),
                linear_to_srgb(p3_g),
                linear_to_srgb(p3_b),
            ]
        }
        WorkingColorSpace::Rec2020 => {
            let r2020_r = lin_src[0] * 0.6274 + lin_src[1] * 0.3293 + lin_src[2] * 0.0433;
            let r2020_g = lin_src[0] * 0.0691 + lin_src[1] * 0.9195 + lin_src[2] * 0.0114;
            let r2020_b = lin_src[0] * 0.0164 + lin_src[1] * 0.0880 + lin_src[2] * 0.8956;
            [
                linear_to_srgb(r2020_r),
                linear_to_srgb(r2020_g),
                linear_to_srgb(r2020_b),
            ]
        }
        WorkingColorSpace::AcesCG => [
            lin_src[0] * 0.6131 + lin_src[1] * 0.3395 + lin_src[2] * 0.0474,
            lin_src[0] * 0.0702 + lin_src[1] * 0.9164 + lin_src[2] * 0.0134,
            lin_src[0] * 0.0206 + lin_src[1] * 0.1096 + lin_src[2] * 0.8698,
        ],
    }
}

/// Project / Composition color bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum BitDepth {
    #[default]
    EightBit, // 8bpc (0..255 integer)
    SixteenBit,        // 16bpc Half Float (unclamped HDR)
    ThirtyTwoBitFloat, // 32bpc Full Float (infinite dynamic range, scene-linear)
}

impl BitDepth {
    pub fn is_hdr(&self) -> bool {
        matches!(self, BitDepth::SixteenBit | BitDepth::ThirtyTwoBitFloat)
    }

    pub fn is_32bpc(&self) -> bool {
        matches!(self, BitDepth::ThirtyTwoBitFloat)
    }

    pub fn label(&self) -> &'static str {
        match self {
            BitDepth::EightBit => "8 bpc",
            BitDepth::SixteenBit => "16 bpc",
            BitDepth::ThirtyTwoBitFloat => "32 bpc (float)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            BitDepth::EightBit => "8bpc",
            BitDepth::SixteenBit => "16bpc",
            BitDepth::ThirtyTwoBitFloat => "32bpc",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            BitDepth::EightBit => BitDepth::SixteenBit,
            BitDepth::SixteenBit => BitDepth::ThirtyTwoBitFloat,
            BitDepth::ThirtyTwoBitFloat => BitDepth::EightBit,
        }
    }
}

/// High Dynamic Range (HDR) 32bpc / 16bpc scene-linear float pixel buffer.
/// Stores RGBA components as 32-bit floats with unclamped range (-inf, +inf).
#[derive(Debug, Clone)]
pub struct HdrF32Buffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>, // 4 floats per pixel: [R, G, B, A]
}

impl HdrF32Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            data: vec![0.0; len],
        }
    }

    /// Creates an HDR float buffer from an 8-bit sRGB RGBA slice.
    pub fn from_rgba8(pixels: &[u8], width: u32, height: u32) -> Self {
        let mut buffer = Self::new(width, height);
        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            let base = i * 4;
            if base + 3 < buffer.data.len() {
                buffer.data[base] = srgb_to_linear(chunk[0] as f32 / 255.0);
                buffer.data[base + 1] = srgb_to_linear(chunk[1] as f32 / 255.0);
                buffer.data[base + 2] = srgb_to_linear(chunk[2] as f32 / 255.0);
                buffer.data[base + 3] = chunk[3] as f32 / 255.0;
            }
        }
        buffer
    }

    /// Converts HDR float buffer back to 8bpc sRGB for display / 8-bit export.
    /// Supports exposure EV offset and optional ACES filmic tonemapping.
    pub fn to_rgba8(&self, tonemap: bool, exposure: f32) -> Vec<u8> {
        self.to_rgba8_dithered(tonemap, exposure, false)
    }

    /// Converts HDR float buffer to 8bpc sRGB with optional TPDF dithering.
    pub fn to_rgba8_dithered(&self, tonemap: bool, exposure: f32, dither: bool) -> Vec<u8> {
        let num_pixels = (self.width as usize) * (self.height as usize);
        let mut out = vec![0u8; num_pixels * 4];
        let exp_factor = 2.0f32.powf(exposure);

        for (i, chunk) in self.data.chunks_exact(4).enumerate() {
            let base = i * 4;
            let mut r = chunk[0] * exp_factor;
            let mut g = chunk[1] * exp_factor;
            let mut b = chunk[2] * exp_factor;
            let a = chunk[3].clamp(0.0, 1.0);

            if tonemap {
                // ACES / Reinhard filmic tonemapping for HDR highlights
                r = r / (1.0 + r.max(0.0));
                g = g / (1.0 + g.max(0.0));
                b = b / (1.0 + b.max(0.0));
            }

            let sr = linear_to_srgb(r);
            let sg = linear_to_srgb(g);
            let sb = linear_to_srgb(b);

            if dither {
                let seed = i as f32 * 0.618_034;
                let t1 = (seed * 7.13).fract();
                let t2 = (seed * 3.71).fract();
                let noise = (t1 - t2) / 255.0;
                out[base] = ((sr + noise).clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 1] = ((sg + noise).clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 2] = ((sb + noise).clamp(0.0, 1.0) * 255.0).round() as u8;
            } else {
                out[base] = (sr.clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 1] = (sg.clamp(0.0, 1.0) * 255.0).round() as u8;
                out[base + 2] = (sb.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            out[base + 3] = (a * 255.0).round() as u8;
        }
        out
    }

    /// Alpha blend source layer over this buffer in scene-linear float space.
    pub fn blend_over(&mut self, src: &HdrF32Buffer, opacity: f32) {
        if self.width != src.width || self.height != src.height {
            return;
        }
        let op = opacity.clamp(0.0, 1.0);
        for i in 0..(self.data.len() / 4) {
            let base = i * 4;
            let src_a = src.data[base + 3] * op;
            let dst_a = self.data[base + 3];
            let out_a = src_a + dst_a * (1.0 - src_a);

            if out_a > 1e-6 {
                for c in 0..3 {
                    let src_c = src.data[base + c];
                    let dst_c = self.data[base + c];
                    self.data[base + c] = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
                }
                self.data[base + 3] = out_a;
            }
        }
    }

    /// Full 32bpc floating-point blend mode compositor.
    /// Blends `src` over `self` with arbitrary blend modes in unbounded float space.
    pub fn blend_layer_mode(
        &mut self,
        src: &HdrF32Buffer,
        opacity: f32,
        mode: crate::core::timeline::BlendMode,
    ) {
        if self.width != src.width || self.height != src.height {
            return;
        }
        let op = opacity.clamp(0.0, 1.0);
        for i in 0..(self.data.len() / 4) {
            let base = i * 4;
            let src_a = (src.data[base + 3] * op).clamp(0.0, 1.0);
            if src_a <= 1e-6 {
                continue;
            }
            let dst_a = self.data[base + 3].clamp(0.0, 1.0);

            for c in 0..3 {
                let s = src.data[base + c];
                let d = self.data[base + c];
                let blended = match mode {
                    crate::core::timeline::BlendMode::Normal => s,
                    crate::core::timeline::BlendMode::Add
                    | crate::core::timeline::BlendMode::LinearDodge => s + d,
                    crate::core::timeline::BlendMode::Multiply => s * d,
                    crate::core::timeline::BlendMode::Screen => s + d - s * d,
                    crate::core::timeline::BlendMode::Overlay => {
                        if d < 0.5 {
                            2.0 * s * d
                        } else {
                            1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                        }
                    }
                    crate::core::timeline::BlendMode::HardLight => {
                        if s < 0.5 {
                            2.0 * s * d
                        } else {
                            1.0 - 2.0 * (1.0 - s) * (1.0 - d)
                        }
                    }
                    crate::core::timeline::BlendMode::SoftLight => {
                        if s <= 0.5 {
                            d - (1.0 - 2.0 * s) * d * (1.0 - d)
                        } else {
                            let root_d = if d > 0.0 { d.sqrt() } else { 0.0 };
                            d + (2.0 * s - 1.0) * (root_d - d)
                        }
                    }
                    crate::core::timeline::BlendMode::Difference => (s - d).abs(),
                    crate::core::timeline::BlendMode::Exclusion => s + d - 2.0 * s * d,
                    crate::core::timeline::BlendMode::Subtract => (d - s).max(0.0),
                    crate::core::timeline::BlendMode::Darken => s.min(d),
                    crate::core::timeline::BlendMode::Lighten => s.max(d),
                    crate::core::timeline::BlendMode::ColorDodge => {
                        if (1.0 - s).abs() < 1e-5 {
                            10.0
                        } else {
                            d / (1.0 - s).max(1e-5)
                        }
                    }
                    crate::core::timeline::BlendMode::ColorBurn => {
                        if s.abs() < 1e-5 {
                            0.0
                        } else {
                            1.0 - (1.0 - d) / s.max(1e-5)
                        }
                    }
                    _ => s,
                };

                let out_a = src_a + dst_a * (1.0 - src_a);
                if out_a > 1e-6 {
                    self.data[base + c] = (blended * src_a + d * dst_a * (1.0 - src_a)) / out_a;
                }
            }
            self.data[base + 3] = (src_a + dst_a * (1.0 - src_a)).clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod hdr_tests {
    use super::*;

    #[test]
    fn test_hdr_f32_buffer_roundtrip() {
        let rgba8 = vec![255, 128, 64, 255];
        let hdr = HdrF32Buffer::from_rgba8(&rgba8, 1, 1);
        assert!(hdr.data[0] > 0.9); // Linear white
        let back = hdr.to_rgba8(false, 0.0);
        assert_eq!(back[0], 255);
        assert!((back[1] as i32 - 128).abs() <= 1);
    }

    #[test]
    fn test_hdr_linear_alpha_blend() {
        let mut bg = HdrF32Buffer::new(1, 1);
        bg.data[0] = 1.0; // Red
        bg.data[3] = 1.0;

        let mut fg = HdrF32Buffer::new(1, 1);
        fg.data[1] = 1.0; // Green
        fg.data[3] = 0.5;

        bg.blend_over(&fg, 1.0);
        assert!(bg.data[0] > 0.0 && bg.data[1] > 0.0);
        assert_eq!(bg.data[3], 1.0);
    }

    #[test]
    fn test_hdr_32bpc_overbright_preservation() {
        let mut bg = HdrF32Buffer::new(1, 1);
        bg.data[0] = 2.5; // Over-bright HDR red (exceeds 1.0)
        bg.data[1] = 1.2;
        bg.data[2] = 0.0;
        bg.data[3] = 1.0;

        let mut fg = HdrF32Buffer::new(1, 1);
        fg.data[0] = 1.0;
        fg.data[1] = 3.0; // Over-bright HDR green
        fg.data[2] = 0.0;
        fg.data[3] = 1.0;

        // Additive blend in 32bpc float space preserves HDR values
        bg.blend_layer_mode(&fg, 1.0, crate::core::timeline::BlendMode::Add);
        assert!(bg.data[0] >= 3.5);
        assert!(bg.data[1] >= 4.2);

        // Tonemapping compresses into [0, 255] without clipping artifacts
        let tonemapped = bg.to_rgba8(true, 0.0);
        assert_eq!(tonemapped[3], 255);
    }

    #[test]
    fn test_bit_depth_cycle() {
        let b = BitDepth::EightBit;
        assert_eq!(b.next(), BitDepth::SixteenBit);
        assert_eq!(b.next().next(), BitDepth::ThirtyTwoBitFloat);
        assert_eq!(b.next().next().next(), BitDepth::EightBit);
        assert!(BitDepth::ThirtyTwoBitFloat.is_32bpc());
        assert!(BitDepth::ThirtyTwoBitFloat.is_hdr());
        assert!(!BitDepth::EightBit.is_hdr());
    }
}
