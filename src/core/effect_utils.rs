//! Shared pixel-effect utilities to eliminate duplication across ae_effects_pack_* modules.
//!
//! Provides canonical implementations of:
//! - Bilinear sampling
//! - Luminance (BT.601)
//! - Pixel clamping (f32 → u8)
//! - Buffer validation guards
//! - Snapshot (double-buffer) pattern

// ────────────────────────── Bilinear Sampling ──────────────────────────

/// Clamp-to-edge bilinear RGBA sample from a packed u8 buffer.
/// Returns `[0,0,0,0]` for degenerate (empty/zero-dim) buffers.
#[inline]
pub fn sample_bilinear(pixels: &[u8], w: u32, h: u32, fx: f32, fy: f32) -> [u8; 4] {
    if w == 0 || h == 0 || pixels.len() < (w as usize) * (h as usize) * 4 {
        return [0, 0, 0, 0];
    }
    let x = fx.clamp(0.0, w as f32 - 1.0);
    let y = fy.clamp(0.0, h as f32 - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let idx = |xx: u32, yy: u32| ((yy * w + xx) * 4) as usize;

    let mut out = [0u8; 4];
    for c in 0..4 {
        let top = pixels[idx(x0, y0) + c] as f32 * (1.0 - tx)
            + pixels[idx(x1, y0) + c] as f32 * tx;
        let bot = pixels[idx(x0, y1) + c] as f32 * (1.0 - tx)
            + pixels[idx(x1, y1) + c] as f32 * tx;
        out[c] = (top * (1.0 - ty) + bot * ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Write bilinear sample into a caller-provided output buffer (avoids return value).
#[inline]
pub fn sample_bilinear_into(pixels: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    *out = sample_bilinear(pixels, w, h, fx, fy);
}

// ────────────────────────── Luminance (BT.601) ──────────────────────────

/// BT.601 luminance from linear f32 channels (each 0.0..=1.0).
#[inline]
pub fn luma_f32(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// BT.601 luminance from u8 channels.
#[inline]
pub fn luma_u8(r: u8, g: u8, b: u8) -> u8 {
    ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114 + 500) / 1000) as u8
}

// ────────────────────────── Pixel Clamping ──────────────────────────

/// Clamp an f32 to [0, 255] and convert to u8. Handles NaN (→ 0) and Inf.
#[inline]
pub fn f32_to_u8(v: f32) -> u8 {
    if v.is_finite() {
        v.round().clamp(0.0, 255.0) as u8
    } else {
        0
    }
}

/// Convert a [0.0, 1.0] float to [0, 255] u8.
#[inline]
pub fn f32_to_byte_01(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Convert a [0.0, 1.0] float to [0.0, 255.0] f32 (for intermediate math).
#[inline]
pub fn f32_to_f32_255(v: f32) -> f32 {
    v.clamp(0.0, 1.0) * 255.0
}

// ────────────────────────── Buffer Guards ──────────────────────────

/// Returns true if the buffer dimensions are valid for processing.
#[inline]
pub fn valid_buffer(pixels: &[u8], w: u32, h: u32) -> bool {
    w > 0 && h > 0 && pixels.len() >= (w as usize) * (h as usize) * 4
}

/// Early-return guard for effect functions. Returns `true` if the buffer is
/// degenerate and the caller should return immediately.
#[inline]
pub fn degenerate_buffer(pixels: &[u8], w: u32, h: u32) -> bool {
    !valid_buffer(pixels, w, h)
}

// ────────────────────────── Snapshot Pattern ──────────────────────────

/// Execute a closure with an immutable snapshot of the pixel buffer, then
/// return the snapshot. Useful for effects that read the "before" state
/// while writing the "after" state.
#[inline]
pub fn with_snapshot<R>(pixels: &[u8], f: impl FnOnce(&[u8]) -> R) -> R {
    let snapshot = pixels.to_vec();
    f(&snapshot)
}

/// Create a snapshot of the pixel buffer (convenience for the common pattern).
#[inline]
pub fn snapshot(pixels: &[u8]) -> Vec<u8> {
    pixels.to_vec()
}

// ────────────────────────── Color Conversion ──────────────────────────

/// RGB to HSL. Returns (h, s, l) each in [0, 1].
/// Handles achromatic (s=0) and wrap-around hue correctly.
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;

    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    (h, s, l)
}

/// HSL to RGB. Inputs in [0, 1].
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < f32::EPSILON {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = h.rem_euclid(1.0);

    fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    (
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

// ────────────────────────── Tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilinear_identity() {
        let mut buf = vec![0u8; 4 * 4 * 4];
        // Fill pixel (1,1) with red
        let idx = (1 * 4 + 1) * 4;
        buf[idx] = 255;
        buf[idx + 3] = 255;
        let sample = sample_bilinear(&buf, 4, 4, 1.0, 1.0);
        assert_eq!(sample[0], 255);
        assert_eq!(sample[3], 255);
    }

    #[test]
    fn bilinear_out_of_bounds_clamps() {
        let buf = vec![128u8; 4 * 4 * 4];
        let sample = sample_bilinear(&buf, 4, 4, -10.0, 100.0);
        assert_eq!(sample[0], 128);
    }

    #[test]
    fn bilinear_degenerate_buffer() {
        let sample = sample_bilinear(&[], 0, 0, 0.0, 0.0);
        assert_eq!(sample, [0, 0, 0, 0]);
    }

    #[test]
    fn luma_f32_range() {
        assert!((luma_f32(0.0, 0.0, 0.0) - 0.0).abs() < 0.001);
        assert!((luma_f32(1.0, 1.0, 1.0) - 1.0).abs() < 0.001);
        assert!(luma_f32(1.0, 0.0, 0.0) > 0.2);
        assert!(luma_f32(0.0, 1.0, 0.0) > 0.5);
    }

    #[test]
    fn luma_u8_range() {
        assert_eq!(luma_u8(0, 0, 0), 0);
        assert_eq!(luma_u8(255, 255, 255), 255);
    }

    #[test]
    fn f32_to_u8_safety() {
        assert_eq!(f32_to_u8(f32::NAN), 0);
        assert_eq!(f32_to_u8(f32::INFINITY), 0);
        assert_eq!(f32_to_u8(f32::NEG_INFINITY), 0);
        assert_eq!(f32_to_u8(128.4), 128);
        assert_eq!(f32_to_u8(128.6), 129);
    }

    #[test]
    fn f32_to_byte_01_range() {
        assert_eq!(f32_to_byte_01(0.0), 0);
        assert_eq!(f32_to_byte_01(1.0), 255);
        assert_eq!(f32_to_byte_01(0.5), 128);
    }

    #[test]
    fn valid_buffer_check() {
        assert!(valid_buffer(&[0u8; 16], 2, 2));
        assert!(!valid_buffer(&[0u8; 15], 2, 2));
        assert!(!valid_buffer(&[], 0, 0));
        assert!(!valid_buffer(&[0u8; 4], 0, 0));
    }

    #[test]
    fn hsl_roundtrip() {
        for r in 0..=10u8 {
            for g in 0..=10u8 {
                for b in 0..=10u8 {
                    let (h, s, l) = rgb_to_hsl(
                        r as f32 / 10.0,
                        g as f32 / 10.0,
                        b as f32 / 10.0,
                    );
                    let (r2, g2, b2) = hsl_to_rgb(h, s, l);
                    assert!(
                        (r2 - r as f32 / 10.0).abs() < 0.01,
                        "R mismatch: {r2} vs {}",
                        r as f32 / 10.0
                    );
                    assert!(
                        (g2 - g as f32 / 10.0).abs() < 0.01,
                        "G mismatch: {g2} vs {}",
                        g as f32 / 10.0
                    );
                    assert!(
                        (b2 - b as f32 / 10.0).abs() < 0.01,
                        "B mismatch: {b2} vs {}",
                        b as f32 / 10.0
                    );
                }
            }
        }
    }
}
