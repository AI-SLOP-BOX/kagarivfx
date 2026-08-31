//! ACES (Academy Color Encoding System) building blocks.
//!
//! Implements the standard ACEScc / ACEScg logarithmic and linear working
//! spaces plus the RRT+ODT tone mapping spline approximation used for
//! preview. These are the pieces a professional color pipeline needs on top
//! of plain sRGB; full OCIO config support composes them via Lut3D.

#![allow(clippy::excessive_precision)] // ACES reference matrices need full digits
/// AP1 primaries → XYZ matrix rows (ACEScg working space).
const AP1_2_XYZ: [[f32; 3]; 3] = [
    [0.6624541811, 0.1340042065, 0.1561876870],
    [0.2722287168, 0.6740817658, 0.0536895174],
    [-0.0055746495, 0.0040607335, 1.0103391003],
];
const XYZ_2_AP1: [[f32; 3]; 3] = [
    [1.6410233797, -0.3248032942, -0.2364246952],
    [-0.6636628587, 1.6153315917, 0.0167563477],
    [0.0117218943, -0.0082844420, 0.9883948585],
];

/// AP0 primaries → XYZ (ACES container space).
const AP0_2_XYZ: [[f32; 3]; 3] = [
    [0.9525523959, 0.0000000000, 0.0000936786],
    [0.3439664498, 0.7281660966, -0.0721325464],
    [0.0000000000, 0.0000000000, 1.0088251844],
];
const XYZ_2_AP0: [[f32; 3]; 3] = [
    [1.0498110175, 0.0000000000, -0.0000974845],
    [-0.4959030231, 1.3733130458, 0.0982400362],
    [0.0000000000, 0.0000000000, 0.9913015268],
];

const SRGB_2_XYZ: [[f32; 3]; 3] = [
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
];
const XYZ_2_SRGB: [[f32; 3]; 3] = [
    [3.2404542, -1.5371385, -0.4985314],
    [-0.9692660, 1.8760108, 0.0415560],
    [0.0556434, -0.2040259, 1.0572252],
];

fn mat_mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Clean reimplementation without the confusing inline above.
pub fn lin_srgb_to_aces_cg(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&SRGB_2_XYZ, rgb);
    mat_mul(&XYZ_2_AP1, xyz)
}

/// ACEScg (AP1 linear) → linear sRGB.
pub fn aces_cg_to_lin_srgb(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&AP1_2_XYZ, rgb);
    mat_mul(&XYZ_2_SRGB, xyz)
}

/// Linear sRGB → ACES2065-1 (AP0 linear), the interchange space.
pub fn lin_srgb_to_aces2065(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&SRGB_2_XYZ, rgb);
    mat_mul(&XYZ_2_AP0, xyz)
}

/// ACES2065-1 → linear sRGB.
pub fn aces2065_to_lin_srgb(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&AP0_2_XYZ, rgb);
    mat_mul(&XYZ_2_SRGB, xyz)
}

/// ACEScc log encoding from ACEScg linear values (S2014-003).
#[inline]
pub fn acescg_to_acescc(rgb: [f32; 3]) -> [f32; 3] {
    let conv = |v: f32| -> f32 {
        if v <= 0.0 {
            return -0.3584474886; // log2(2^-16) folded: (−16+9.72)/17.52
        }
        ((v.max(2f32.powi(-16))).log2() + 9.72) / 17.52
    };
    [conv(rgb[0]), conv(rgb[1]), conv(rgb[2])]
}

pub fn acescc_to_acescg(rgb: [f32; 3]) -> [f32; 3] {
    // Inverse of the encoder above; values below the floor decode to zero.
    let conv_full = |v: f32| -> f32 {
        if v <= -0.3584474886 + 1e-6 {
            return 0.0;
        }
        (v * 17.52 - 9.72).exp2()
    };
    [conv_full(rgb[0]), conv_full(rgb[1]), conv_full(rgb[2])]
}

/// ACEScct quasi-logarithmic encoding with toe curve for color grading (S-2016-001).
pub fn acescg_to_acescct(rgb: [f32; 3]) -> [f32; 3] {
    let conv = |v: f32| -> f32 {
        if v > 0.0078125 {
            (v.log2() + 9.72) / 17.52
        } else {
            10.5402377416545 * v + 0.0729055341958355
        }
    };
    [conv(rgb[0]), conv(rgb[1]), conv(rgb[2])]
}

/// ACEScct decoding to ACEScg linear.
pub fn acescc_to_acescct_inv(rgb: [f32; 3]) -> [f32; 3] {
    let conv = |v: f32| -> f32 {
        if v > 0.155251141552511 {
            (v * 17.52 - 9.72).exp2()
        } else {
            (v - 0.0729055341958355) / 10.5402377416545
        }
    };
    [conv(rgb[0]), conv(rgb[1]), conv(rgb[2])]
}

const REC2020_2_XYZ: [[f32; 3]; 3] = [
    [0.6369580, 0.1446169, 0.1688810],
    [0.2627002, 0.6779981, 0.0593017],
    [0.0000000, 0.0280727, 1.0609851],
];
const XYZ_2_REC2020: [[f32; 3]; 3] = [
    [1.7166512, -0.3556708, -0.2533663],
    [-0.6666844, 1.6164812, 0.0157685],
    [0.0176399, -0.0427706, 0.9421031],
];

/// Linear Rec.2020 → ACEScg (AP1 linear).
pub fn rec2020_to_aces_cg(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&REC2020_2_XYZ, rgb);
    mat_mul(&XYZ_2_AP1, xyz)
}

/// ACEScg → Linear Rec.2020.
pub fn aces_cg_to_rec2020(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&AP1_2_XYZ, rgb);
    mat_mul(&XYZ_2_REC2020, xyz)
}

/// RRT + Rec.709 ODT tone mapping approximation (Narkowicz fit).
/// Input/output are linear [0..inf) → display-linear [0..1].
pub fn aces_filmic_tonemap(x: [f32; 3]) -> [f32; 3] {
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;
    x.map(|v| {
        let v = (v * (A * v + B)) / (v * (C * v + D) + E);
        v.clamp(0.0, 1.0)
    })
}

const DISPLAY_P3_2_XYZ: [[f32; 3]; 3] = [
    [0.4865709, 0.2656677, 0.1982173],
    [0.2289746, 0.6917385, 0.0792869],
    [0.0000000, 0.0451134, 1.0439444],
];
const XYZ_2_DISPLAY_P3: [[f32; 3]; 3] = [
    [2.4934969, -0.9313836, -0.4027108],
    [-0.8294890, 1.7626641, 0.0236247],
    [0.0358458, -0.0761724, 0.9568845],
];

/// Linear Display P3 (D65) → ACEScg (AP1 linear).
pub fn display_p3_to_aces_cg(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&DISPLAY_P3_2_XYZ, rgb);
    mat_mul(&XYZ_2_AP1, xyz)
}

/// ACEScg → Linear Display P3 (D65).
pub fn aces_cg_to_display_p3(rgb: [f32; 3]) -> [f32; 3] {
    let xyz = mat_mul(&AP1_2_XYZ, rgb);
    mat_mul(&XYZ_2_DISPLAY_P3, xyz)
}

/// ACES 1.3 Reference Gamut Compression algorithm (S-2021-001).
/// Compresses out-of-gamut / highly saturated colors cleanly without clipping.
pub fn aces_gamut_compress(rgb: [f32; 3], threshold: f32, limit: f32) -> [f32; 3] {
    let ach = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
    if ach.abs() < 1e-6 {
        return rgb;
    }

    let compress_channel = |v: f32| -> f32 {
        let dist = (ach - v) / ach.abs();
        if dist > threshold {
            let norm_dist = (dist - threshold) / (limit - threshold).max(1e-4);
            let compressed_norm = norm_dist / (1.0 + norm_dist.powi(2)).sqrt();
            let compressed_dist = threshold + compressed_norm * (limit - threshold);
            ach - compressed_dist * ach.abs()
        } else {
            v
        }
    };

    [
        compress_channel(rgb[0]),
        compress_channel(rgb[1]),
        compress_channel(rgb[2]),
    ]
}

/// Full preview transform: scene-linear sRGB → display via ACES tonemap,
/// then encode with the exact piecewise sRGB EOTF^-1.
pub fn aces_preview_transform(linear_srgb: [f32; 3]) -> [f32; 3] {
    let mapped = aces_filmic_tonemap(linear_srgb);
    [
        crate::core::color::linear_to_srgb_piecewise(mapped[0]).clamp(0.0, 1.0),
        crate::core::color::linear_to_srgb_piecewise(mapped[1]).clamp(0.0, 1.0),
        crate::core::color::linear_to_srgb_piecewise(mapped[2]).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_roundtrip_ap1() {
        // sRGB → AP1 → sRGB must round-trip within float tolerance
        for rgb in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.2, 0.5, 0.9], [0.5; 3]] {
            let cg = lin_srgb_to_aces_cg(rgb);
            let back = aces_cg_to_lin_srgb(cg);
            for c in 0..3 {
                assert!((back[c] - rgb[c]).abs() < 1e-4, "{rgb:?} → {back:?}");
            }
        }
    }

    #[test]
    fn test_primary_hue_preservation_in_cg() {
        // Pure red stays predominantly red in AP1
        let cg = lin_srgb_to_aces_cg([1.0, 0.0, 0.0]);
        assert!(cg[0] > cg[1] && cg[0] > cg[2], "red dominates: {cg:?}");
        // And green channel near zero-ish relative
        assert!(cg[1] < 0.35);
    }

    #[test]
    fn test_aces2065_interchange_roundtrip() {
        let rgb = [0.3, 0.55, 0.77];
        let ap0 = lin_srgb_to_aces2065(rgb);
        let back = aces2065_to_lin_srgb(ap0);
        for c in 0..3 {
            assert!((back[c] - rgb[c]).abs() < 1e-3);
        }
    }

    #[test]
    fn test_acescc_log_encoding_monotonic_and_specials() {
        let zero = acescg_to_acescc([0.0, 0.0, 0.0]);
        assert!(
            (zero[0] - (-0.3584474886)).abs() < 1e-6,
            "zero maps to floor constant"
        );
        let lo = acescg_to_acescc([0.01, 0.01, 0.01])[0];
        let hi = acescg_to_acescc([0.10, 0.10, 0.10])[0];
        assert!(hi > lo, "log encoding monotonic");
        let back = acescc_to_acescg(acescg_to_acescc([0.05, 0.05, 0.05]));
        assert!((back[0] - 0.05).abs() < 1e-4, "cc↔cg roundtrip: {back:?}");
    }

    #[test]
    fn test_tonemap_maps_midgray_reasonably_and_clamps() {
        let mid = aces_filmic_tonemap([0.18, 0.18, 0.18]);
        assert!(
            mid[0] > 0.08 && mid[0] < 0.30,
            "18% gray lands in filmic range: {:?}",
            mid
        );
        let bright = aces_filmic_tonemap([100.0, 100.0, 100.0]);
        assert!((bright[0] - 1.0).abs() < 1e-4, "highlights clip to white");
        let dark = aces_filmic_tonemap([0.0, 0.0, 0.0]);
        assert_eq!(dark[0], 0.0);
    }

    #[test]
    fn test_acescct_and_rec2020_roundtrip() {
        let rgb = [0.2, 0.4, 0.8];
        let cct = acescg_to_acescct(rgb);
        let back_cg = acescc_to_acescct_inv(cct);
        for c in 0..3 {
            assert!((back_cg[c] - rgb[c]).abs() < 1e-4);
        }

        let rec2020_cg = rec2020_to_aces_cg(rgb);
        let back_rec2020 = aces_cg_to_rec2020(rec2020_cg);
        for c in 0..3 {
            assert!((back_rec2020[c] - rgb[c]).abs() < 1e-4);
        }

        let p3_cg = display_p3_to_aces_cg(rgb);
        let back_p3 = aces_cg_to_display_p3(p3_cg);
        for c in 0..3 {
            assert!((back_p3[c] - rgb[c]).abs() < 1e-4);
        }
    }

    #[test]
    fn test_aces_gamut_compression_preserves_in_gamut_and_compresses_extreme() {
        let in_gamut = [0.2, 0.4, 0.6];
        let compressed_in = aces_gamut_compress(in_gamut, 0.8, 1.2);
        assert_eq!(in_gamut, compressed_in);

        let out_of_gamut = [2.5, -0.8, 0.1]; // Highly saturated with negative component
        let compressed_out = aces_gamut_compress(out_of_gamut, 0.8, 1.2);
        assert!(compressed_out[0].is_finite());
        assert!(compressed_out[1] > out_of_gamut[1], "Negative channel pulled towards achromatic");
    }
}
