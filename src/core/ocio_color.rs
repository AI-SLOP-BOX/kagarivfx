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
        1.705051, -0.621792, -0.083259, -0.100236, 1.146599, -0.046363, -0.024007, -0.128969,
        1.152976,
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
                r = if r <= 0.04045 {
                    r / 12.92
                } else {
                    ((r + 0.055) / 1.055).powf(2.4)
                };
                g = if g <= 0.04045 {
                    g / 12.92
                } else {
                    ((g + 0.055) / 1.055).powf(2.4)
                };
                b = if b <= 0.04045 {
                    b / 12.92
                } else {
                    ((b + 0.055) / 1.055).powf(2.4)
                };
            }

            // 2. Transform Color Primaries if converting ACEScg -> sRGB
            if src_space == OcioColorSpace::AcesCg && dst_space == OcioColorSpace::SRgb {
                let m = Self::ACESCG_TO_SRGB_MAT;
                let nr = r * m[0] + g * m[1] + b * m[2];
                let ng = r * m[3] + g * m[4] + b * m[5];
                let nb = r * m[6] + g * m[7] + b * m[8];
                r = nr;
                g = ng;
                b = nb;
            }

            // 3. Apply Target Gamma / OETF Display Curve
            if dst_space == OcioColorSpace::SRgb {
                r = if r <= 0.0031308 {
                    r * 12.92
                } else {
                    1.055 * r.powf(1.0 / 2.4) - 0.055
                };
                g = if g <= 0.0031308 {
                    g * 12.92
                } else {
                    1.055 * g.powf(1.0 / 2.4) - 0.055
                };
                b = if b <= 0.0031308 {
                    b * 12.92
                } else {
                    1.055 * b.powf(1.0 / 2.4) - 0.055
                };
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
        OcioColorEngine::transform_colorspace(
            &mut pixels,
            OcioColorSpace::SRgb,
            OcioColorSpace::LinearSRgb,
        );
        assert!(pixels[0] < 0.5); // Gamma uncompressed linear intensity is lower

        OcioColorEngine::transform_colorspace(
            &mut pixels,
            OcioColorSpace::LinearSRgb,
            OcioColorSpace::SRgb,
        );
        assert!((pixels[0] - 0.5).abs() < 0.01);
    }
}

// ── 3D LUT support (ported & adapted from NextVFX aura-core) ────────────────

/// 3D lookup table with tetrahedral interpolation — O(1) unrolled hot path.
#[derive(Debug, Clone)]
pub struct Lut3D {
    pub size: usize,
    /// Flat RGB data, layout: r-major (r fastest? no: index = (b*size + g)*size + r)
    pub data: Vec<f32>,
}

impl Lut3D {
    /// Tetrahedral interpolation: highest-quality trilinear alternative,
    /// branch-light in the hot loop.
    pub fn apply(&self, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let s = (self.size - 1) as f32;
        let fr = (r * s).clamp(0.0, s - 0.001);
        let fg = (g * s).clamp(0.0, s - 0.001);
        let fb = (b * s).clamp(0.0, s - 0.001);

        let ir = fr as usize;
        let ig = fg as usize;
        let ib = fb as usize;
        let dr = fr - ir as f32;
        let dg = fg - ig as f32;
        let db = fb - ib as f32;

        // 8 surrounding lattice points
        let idx = |x: usize, y: usize, z: usize| -> [f32; 3] {
            let base = ((z * self.size + y) * self.size + x) * 3;
            [self.data[base], self.data[base + 1], self.data[base + 2]]
        };

        let c000 = idx(ir, ig, ib);
        let c100 = idx(ir + 1, ig, ib);
        let c010 = idx(ir, ig + 1, ib);
        let c110 = idx(ir + 1, ig + 1, ib);
        let c001 = idx(ir, ig, ib + 1);
        let c101 = idx(ir + 1, ig, ib + 1);
        let c011 = idx(ir, ig + 1, ib + 1);
        let c111 = idx(ir + 1, ig + 1, ib + 1);

        // Tetrahedral subdivision on the largest fractional component
        if dr > dg {
            if dg > db {
                // R > G > B
                self.tetra(c000, c100, c110, c111, dr, dg, db)
            } else if dr > db {
                // R > B > G
                self.tetra(c000, c100, c111, c101, dr, db, dg)
            } else {
                // B > R > G
                self.tetra(c000, c001, c101, c111, db, dr, dg)
            }
        } else if dr > db {
            // G > R > B
            self.tetra(c000, c010, c110, c111, dg, dr, db)
        } else if dg > db {
            // G > B > R
            self.tetra(c000, c010, c011, c111, dg, db, dr)
        } else {
            // B > G > R
            self.tetra(c000, c001, c011, c111, db, dg, dr)
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn tetra(
        &self,
        c0: [f32; 3],
        ca: [f32; 3],
        cb: [f32; 3],
        cc: [f32; 3],
        da: f32,
        db: f32,
        dc: f32,
    ) -> (f32, f32, f32) {
        let mut out = [0.0f32; 3];
        for i in 0..3 {
            out[i] = c0[i] + (ca[i] - c0[i]) * da + (cb[i] - ca[i]) * db + (cc[i] - cb[i]) * dc;
        }
        (out[0], out[1], out[2])
    }

    /// Parses an .cube LUT file (TITLE/LUT_1D_SIZE skipped, LUT_3D_SIZE required).
    pub fn parse_cube(text: &str) -> Result<Self, String> {
        let mut size = 0usize;
        let mut values: Vec<f32> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("TITLE")
                || line.starts_with("DOMAIN_")
                || line.starts_with("LUT_1D_SIZE")
            {
                continue;
            }
            if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
                size = rest
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "bad LUT_3D_SIZE")?;
                continue;
            }
            let mut it = line.split_whitespace();
            let (Some(r), Some(g), Some(b), None) = (it.next(), it.next(), it.next(), it.next())
            else {
                continue; // skip malformed lines rather than failing whole file
            };
            values.push(r.parse::<f32>().map_err(|_| "bad float in .cube")?);
            values.push(g.parse::<f32>().map_err(|_| "bad float in .cube")?);
            values.push(b.parse::<f32>().map_err(|_| "bad float in .cube")?);
        }
        if size == 0 {
            return Err("missing LUT_3D_SIZE".into());
        }
        let expected = size * size * size * 3;
        if values.len() != expected {
            return Err(format!(
                "expected {} values for size {}, got {}",
                expected,
                size,
                values.len()
            ));
        }
        Ok(Self { size, data: values })
    }
}

thread_local! {
    static ACTIVE_LUT: std::cell::RefCell<Option<std::sync::Arc<Lut3D>>> =
        const { std::cell::RefCell::new(None) };
}

/// Sets the globally active LUT used by renderers when lut_mode == 3.
pub fn set_active_lut(lut: Option<std::sync::Arc<Lut3D>>) {
    ACTIVE_LUT.with(|l| *l.borrow_mut() = lut);
}

/// Per-pixel LUT application for float pipelines; identity when no LUT loaded.
pub fn apply_lut_pixel(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let applied = ACTIVE_LUT.with(|l| l.borrow().clone());
    match applied {
        Some(lut) => {
            let (nr, ng, nb) = lut.apply(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
            (nr.clamp(0.0, 1.0), ng.clamp(0.0, 1.0), nb.clamp(0.0, 1.0))
        }
        None => (r, g, b),
    }
}

/// Applies the active LUT in place to an RGBA u8 buffer (if one is loaded).
pub fn apply_active_lut(pixels: &mut [u8]) -> bool {
    let applied = ACTIVE_LUT.with(|l| l.borrow().clone());
    match applied {
        Some(lut) => {
            for p in pixels.chunks_exact_mut(4) {
                let r = p[0] as f32 / 255.0;
                let g = p[1] as f32 / 255.0;
                let b = p[2] as f32 / 255.0;
                let (nr, ng, nb) = lut.apply(r, g, b);
                p[0] = (nr.clamp(0.0, 1.0) * 255.0).round() as u8;
                p[1] = (ng.clamp(0.0, 1.0) * 255.0).round() as u8;
                p[2] = (nb.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod lut_tests {
    use super::*;

    /// Builds a size-2 identity-ish LUT where out == in.
    fn identity_lut(size: usize) -> Lut3D {
        let mut data = Vec::with_capacity(size * size * size * 3);
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let d = size - 1;
                    data.push(r as f32 / d as f32);
                    data.push(g as f32 / d as f32);
                    data.push(b as f32 / d as f32);
                }
            }
        }
        Lut3D { size, data }
    }

    #[test]
    fn test_identity_lut_is_identity() {
        let lut = identity_lut(17);
        for &(r, g, b) in &[
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (0.25, 0.5, 0.75),
            (0.9, 0.1, 0.4),
        ] {
            let (nr, ng, nb) = lut.apply(r, g, b);
            assert!((nr - r).abs() < 0.05, "r {} -> {}", r, nr);
            assert!((ng - g).abs() < 0.05, "g {} -> {}", g, ng);
            assert!((nb - b).abs() < 0.05, "b {} -> {}", b, nb);
        }
    }

    #[test]
    fn test_parse_cube_minimal() {
        let cube = "\
# A minimal test LUT
TITLE \"Test\"
LUT_3D_SIZE 2
DOMAIN_MIN 0 0 0
DOMAIN_MAX 1 1 1
0 0 0
1 0 0
0 1 0
1 1 0
0 0 1
1 0 1
0 1 1
1 1 1
";
        let lut = Lut3D::parse_cube(cube).expect("must parse");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 2 * 2 * 2 * 3);

        // Corner lookups return themselves (identity arrangement)
        let (r, g, b) = lut.apply(1.0, 0.0, 0.0);
        assert!(r > 0.9 && g < 0.1 && b < 0.1);
    }

    #[test]
    fn test_parse_cube_rejects_bad_files() {
        assert!(Lut3D::parse_cube("no size here\n").is_err());
        assert!(Lut3D::parse_cube("LUT_3D_SIZE 2\n0 0 0\n").is_err()); // too few values
        assert!(Lut3D::parse_cube("LUT_3D_SIZE notanumber\n").is_err());
    }

    #[test]
    fn test_apply_active_lut_to_buffer() {
        // Warm tint: red up, blue down everywhere
        let mut data = Vec::new();
        for b in 0..2usize {
            for g in 0..2usize {
                for r in 0..2usize {
                    data.push(if r > 0 { 1.0 } else { 0.0 });
                    data.push(g as f32);
                    data.push(if b > 0 { 0.0 } else { 0.5 });
                }
            }
        }
        set_active_lut(Some(std::sync::Arc::new(Lut3D { size: 2, data })));

        let mut px = vec![128u8, 64, 64, 255];
        assert!(apply_active_lut(&mut px));
        set_active_lut(None); // cleanup for other tests

        // Red channel must have increased relative to blue
        assert!(px[0] >= 128, "red should rise, got {}", px[0]);
    }

    #[test]
    fn test_apply_without_lut_is_noop_false() {
        set_active_lut(None);
        let mut px = vec![10u8, 20, 30, 255];
        assert!(!apply_active_lut(&mut px));
        assert_eq!(px, vec![10, 20, 30, 255]);
    }
}
