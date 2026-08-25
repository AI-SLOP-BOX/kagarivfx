//! Linear-light float color type for 16bpc/32bpc compositing.
//!
//! All compositing math should happen in linear space. sRGB↔linear
//! conversions use the standard 2.2 gamma approximation (matching
//! the existing `blend_linear` path in the renderer).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgbaf {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgbaf {
    pub const ZERO: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const TRANSPARENT: Self = Self::ZERO;
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    #[inline]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: srgb_to_linear(r as f32 / 255.0),
            g: srgb_to_linear(g as f32 / 255.0),
            b: srgb_to_linear(b as f32 / 255.0),
            a: a as f32 / 255.0,
        }
    }

    #[inline]
    pub fn from_slice_rgba8(px: &[u8]) -> Self {
        debug_assert!(px.len() >= 4);
        Self::from_rgba8(px[0], px[1], px[2], px[3])
    }

    #[inline]
    pub fn to_rgba8(self) -> [u8; 4] {
        [
            (linear_to_srgb(self.r) * 255.0).round().clamp(0.0, 255.0) as u8,
            (linear_to_srgb(self.g) * 255.0).round().clamp(0.0, 255.0) as u8,
            (linear_to_srgb(self.b) * 255.0).round().clamp(0.0, 255.0) as u8,
            (self.a * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }

    #[inline]
    pub fn write_rgba8(self, dst: &mut [u8]) {
        debug_assert!(dst.len() >= 4);
        let rgba = self.to_rgba8();
        dst[..4].copy_from_slice(&rgba);
    }

    /// Premultiplied alpha in linear space.
    #[inline]
    pub fn premultiply(self) -> Self {
        Self { r: self.r * self.a, g: self.g * self.a, b: self.b * self.a, a: self.a }
    }

    /// Reverse premultiplication (divide by alpha, safe for alpha > 0).
    #[inline]
    pub fn unpremultiply(self) -> Self {
        if self.a <= 1e-6 { return Self::ZERO; }
        let inv = 1.0 / self.a;
        Self { r: self.r * inv, g: self.g * inv, b: self.b * inv, a: self.a }
    }

    /// Over operator (Porter-Duff): self over background, both in premultiplied linear.
    #[inline]
    pub fn over(self, bg: Self) -> Self {
        let src = self.premultiply();
        let out_a = src.a + bg.a * (1.0 - src.a);
        if out_a <= 1e-6 { return Self::ZERO; }
        let inv = 1.0 / out_a;
        Self {
            r: (src.r + bg.r * bg.a * (1.0 - src.a)) * inv,
            g: (src.g + bg.g * bg.a * (1.0 - src.a)) * inv,
            b: (src.b + bg.b * bg.a * (1.0 - src.a)) * inv,
            a: out_a,
        }
    }

    /// Lerp between self and other by t ∈ [0, 1].
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let it = 1.0 - t;
        Self {
            r: self.r * it + other.r * t,
            g: self.g * it + other.g * t,
            b: self.b * it + other.b * t,
            a: self.a * it + other.a * t,
        }
    }

    /// Clamp all channels to [0, 1].
    #[inline]
    pub fn clamp_01(self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// Convert a whole RGBA8 buffer to linear Rgbaf pixels.
    pub fn buffer_from_rgba8(src: &[u8], width: u32, height: u32) -> Vec<Self> {
        let n = (width as usize) * (height as usize);
        let mut out = Vec::with_capacity(n);
        for chunk in src.chunks_exact(4) {
            out.push(Self::from_rgba8(chunk[0], chunk[1], chunk[2], chunk[3]));
        }
        out
    }

    /// Write linear Rgbaf pixels back to an RGBA8 buffer.
    pub fn buffer_to_rgba8(src: &[Self], dst: &mut [u8]) {
        for (px, chunk) in src.iter().zip(dst.chunks_exact_mut(4)) {
            px.write_rgba8(chunk);
        }
    }
}

/// sRGB companding: linear → sRGB (for display/output).
/// Uses the standard piecewise formula (simpler pow(1/2.2) approximation
/// matches the existing renderer's `blend_linear` path).
#[inline]
pub fn linear_to_srgb(v: f32) -> f32 {
    v.clamp(0.0, 1.0).powf(1.0 / 2.2)
}

/// sRGB expansion: sRGB → linear (for compositing).
#[inline]
pub fn srgb_to_linear(v: f32) -> f32 {
    v.clamp(0.0, 1.0).powf(2.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_rgba8() {
        let c = Rgbaf::new(0.5, 0.8, 0.3, 1.0);
        let rgba = c.to_rgba8();
        let c2 = Rgbaf::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
        // Allow 1/255 tolerance due to quantization
        assert!((c.r - c2.r).abs() < 0.01);
        assert!((c.g - c2.g).abs() < 0.01);
        assert!((c.b - c2.b).abs() < 0.01);
    }

    #[test]
    fn test_over_operator() {
        let fg = Rgbaf::new(1.0, 0.0, 0.0, 0.5);
        let bg = Rgbaf::new(0.0, 0.0, 1.0, 1.0);
        let result = fg.over(bg);
        assert!(result.a > 0.99);
        // Red should be partially visible, blue partially visible
        assert!(result.r > 0.0);
        assert!(result.b > 0.0);
    }

    #[test]
    fn test_premultiply_unpremultiply() {
        let c = Rgbaf::new(0.8, 0.6, 0.4, 0.5);
        let p = c.premultiply();
        assert!((p.r - 0.4).abs() < 0.001);
        let u = p.unpremultiply();
        assert!((u.r - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_linear_srgb_roundtrip() {
        for v in [0.0, 0.1, 0.25, 0.5, 0.75, 1.0] {
            let round = linear_to_srgb(srgb_to_linear(v));
            assert!((round - v).abs() < 0.001, "roundtrip {}: {}", v, round);
        }
    }

    #[test]
    fn test_buffer_roundtrip() {
        let src = vec![100u8, 150, 200, 255, 50, 80, 120, 200];
        let linear = Rgbaf::buffer_from_rgba8(&src, 2, 1);
        assert_eq!(linear.len(), 2);
        let mut dst = vec![0u8; 8];
        Rgbaf::buffer_to_rgba8(&linear, &mut dst);
        // Allow quantization tolerance
        for i in 0..4 {
            assert!((src[i] as i32 - dst[i] as i32).abs() <= 1);
        }
    }

    #[test]
    fn test_clamp_01() {
        let c = Rgbaf::new(-0.5, 1.5, 0.5, 2.0).clamp_01();
        assert_eq!(c, Rgbaf::new(0.0, 1.0, 0.5, 1.0));
    }

    #[test]
    fn test_zero_alpha_over() {
        let fg = Rgbaf::new(1.0, 0.0, 0.0, 0.0);
        let bg = Rgbaf::new(0.0, 0.0, 1.0, 1.0);
        let result = fg.over(bg);
        assert!((result.b - 1.0).abs() < 0.001);
        assert!((result.a - 1.0).abs() < 0.001);
    }
}
