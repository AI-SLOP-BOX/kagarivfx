#![allow(dead_code)]
/// Supported Color Spaces matching OpenColorIO (OCIO) / ACES standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcioColorSpace {
    SRgb,
    LinearSRgb,
    AcesCc,
    AcesCg,
    DciP3,
}

/// OpenColorIO (OCIO) / ACES 32-bit Float Color Management Engine.
pub struct OcioColorEngine;

impl OcioColorEngine {
    /// 3x3 Matrix multiplication for ACEScg (AP1) to sRGB (Rec.709) conversion.
    const ACESCG_TO_SRGB_MAT: [f32; 9] = [
        1.705051, -0.621792, -0.083259,
       -0.100236,  1.146599, -0.046363,
       -0.024007, -0.128969,  1.152976,
    ];

    /// Transforms 32-bit float RGBA pixel buffer between OCIO color spaces.
    pub fn transform_colorspace(
        pixels: &mut [f32],
        src_space: OcioColorSpace,
        dst_space: OcioColorSpace,
    ) {
        if src_space == dst_space || pixels.is_empty() {
            return;
        }

        let num_pixels = pixels.len() / 4;
        for i in 0..num_pixels {
            let idx = i * 4;
            let mut r = pixels[idx];
            let mut g = pixels[idx + 1];
            let mut b = pixels[idx + 2];

            // 1. Convert to Linear Working Space
            if src_space == OcioColorSpace::SRgb {
                r = if r <= 0.04045 { r / 12.92 } else { ((r + 0.055) / 1.055).powf(2.4) };
                g = if g <= 0.04045 { g / 12.92 } else { ((g + 0.055) / 1.055).powf(2.4) };
                b = if b <= 0.04045 { b / 12.92 } else { ((b + 0.055) / 1.055).powf(2.4) };
            }

            // 2. Transform Color Primaries if converting ACEScg -> sRGB
            if src_space == OcioColorSpace::AcesCg && dst_space == OcioColorSpace::SRgb {
                let m = Self::ACESCG_TO_SRGB_MAT;
                let nr = r * m[0] + g * m[1] + b * m[2];
                let ng = r * m[3] + g * m[4] + b * m[5];
                let nb = r * m[6] + g * m[7] + b * m[8];
                r = nr; g = ng; b = nb;
            }

            // 3. Apply Target Gamma / OETF Display Curve
            if dst_space == OcioColorSpace::SRgb {
                r = if r <= 0.0031308 { r * 12.92 } else { 1.055 * r.powf(1.0 / 2.4) - 0.055 };
                g = if g <= 0.0031308 { g * 12.92 } else { 1.055 * g.powf(1.0 / 2.4) - 0.055 };
                b = if b <= 0.0031308 { b * 12.92 } else { 1.055 * b.powf(1.0 / 2.4) - 0.055 };
            }

            pixels[idx] = r.clamp(0.0, 1.0);
            pixels[idx + 1] = g.clamp(0.0, 1.0);
            pixels[idx + 2] = b.clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocio_srgb_roundtrip() {
        let mut pixels = vec![0.5f32, 0.5f32, 0.5f32, 1.0f32];
        OcioColorEngine::transform_colorspace(&mut pixels, OcioColorSpace::SRgb, OcioColorSpace::LinearSRgb);
        assert!(pixels[0] < 0.5); // Gamma uncompressed linear intensity is lower

        OcioColorEngine::transform_colorspace(&mut pixels, OcioColorSpace::LinearSRgb, OcioColorSpace::SRgb);
        assert!((pixels[0] - 0.5).abs() < 0.01);
    }
}
