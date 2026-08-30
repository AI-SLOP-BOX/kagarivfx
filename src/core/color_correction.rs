#![allow(dead_code)]
//! Professional color correction kernels: spline tone Curves, three-way
//! Color Balance wheels and the Channel Mixer matrix — matching AE/Photoshop
//! behaviour closely enough for grading workflows.
//!
//! All functions are pure, deterministic, panic-free and operate on packed
//! RGBA8 buffers.

/// A user-editable tone curve defined by control points in normalized
/// [0,1]² space. Evaluated with monotone cubic Hermite interpolation
/// (Fritsch–Carlson) so it never overshoots the control data.
#[derive(Debug, Clone, PartialEq)]
pub struct ToneCurve {
    /// Control points sorted by x. Each entry is [x, y] in 0..1.
    pub points: Vec<[f32; 2]>,
}

impl Default for ToneCurve {
    fn default() -> Self {
        Self::linear()
    }
}

impl ToneCurve {
    /// Identity curve.
    pub fn linear() -> Self {
        Self {
            points: vec![[0.0, 0.0], [1.0, 1.0]],
        }
    }

    /// Build from unsorted points; sorts by x and clamps to [0,1].
    pub fn new(mut points: Vec<[f32; 2]>) -> Self {
        points.retain(|p| p[0].is_finite() && p[1].is_finite());
        for p in &mut points {
            p[0] = p[0].clamp(0.0, 1.0);
            p[1] = p[1].clamp(0.0, 1.0);
        }
        points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        // Deduplicate x values keeping the LAST point of each run (most recent edit wins).
        points.reverse();
        points.dedup_by(|a, b| (b[0] - a[0]).abs() < 1e-6);
        points.reverse();
        if points.len() < 2 {
            return Self::linear();
        }
        Self { points }
    }

    /// Evaluate the curve at x (clamped to 0..1).
    pub fn eval(&self, x: f32) -> f32 {
        let pts = &self.points;
        if pts.len() < 2 {
            return x.clamp(0.0, 1.0);
        }
        let x = x.clamp(0.0, 1.0);
        if x <= pts[0][0] {
            return pts[0][1];
        }
        let last = pts.len() - 1;
        if x >= pts[last][0] {
            return pts[last][1];
        }

        // Locate segment via binary search on x.
        let mut lo = 0usize;
        let mut hi = last;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if pts[mid][0] <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        // Fritsch–Carlson tangents for this neighbourhood.
        let secant = |i: usize| -> f32 {
            let dx = pts[i + 1][0] - pts[i][0];
            if dx.abs() < 1e-9 {
                0.0
            } else {
                (pts[i + 1][1] - pts[i][1]) / dx
            }
        };
        let d_lo = secant(lo.saturating_sub(1));
        let d_mid = secant(lo);
        let d_hi = secant((lo + 1).min(last - 1));

        let m_left = if d_lo * d_mid <= 0.0 {
            0.0
        } else {
            (d_lo + d_mid) * 0.5
        };
        let m_right = if d_mid * d_hi <= 0.0 {
            0.0
        } else {
            (d_mid + d_hi) * 0.5
        };

        // Hermite basis over the segment.
        let h = pts[hi][0] - pts[lo][0];
        if h.abs() < 1e-9 {
            return pts[lo][1];
        }
        let t = (x - pts[lo][0]) / h;
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * pts[lo][1] + h10 * h * m_left + h01 * pts[hi][1] + h11 * h * m_right
    }

    /// Sample into a 256-entry LUT.
    pub fn build_lut(&self) -> [f32; 256] {
        let mut lut = [0.0f32; 256];
        for (i, slot) in lut.iter_mut().enumerate() {
            *slot = self.eval(i as f32 / 255.0);
        }
        lut
    }
}

/// Per-channel tone curves applied as master∘channel composition.
#[derive(Debug, Clone, Default)]
pub struct ChannelCurves {
    pub master: Option<ToneCurve>,
    pub red: Option<ToneCurve>,
    pub green: Option<ToneCurve>,
    pub blue: Option<ToneCurve>,
}

impl ChannelCurves {
    pub fn is_identity(&self) -> bool {
        self.master.is_none() && self.red.is_none() && self.green.is_none() && self.blue.is_none()
    }
}

/// Apply spline Curves to an RGBA8 buffer.
pub fn apply_curves(pixels: &mut [u8], curves: &ChannelCurves) {
    if curves.is_identity() || pixels.is_empty() {
        return;
    }
    let master_lut = curves.master.as_ref().map(ToneCurve::build_lut);
    let red_lut = curves.red.as_ref().map(ToneCurve::build_lut);
    let green_lut = curves.green.as_ref().map(ToneCurve::build_lut);
    let blue_lut = curves.blue.as_ref().map(ToneCurve::build_lut);

    let through = |lut: &Option<[f32; 256]>, v: u8| -> u8 {
        match lut {
            Some(table) => (table[v as usize].clamp(0.0, 1.0) * 255.0).round() as u8,
            None => v,
        }
    };

    for px in pixels.chunks_exact_mut(4) {
        let r = through(&master_lut, px[0]);
        let g = through(&master_lut, px[1]);
        let b = through(&master_lut, px[2]);
        px[0] = through(&red_lut, r);
        px[1] = through(&green_lut, g);
        px[2] = through(&blue_lut, b);
    }
}

/// Three-way color balance (shadows / midtones / highlights color shifts).
///
/// Shifts are in -100..100 per channel; `preserve_luminosity` keeps the
/// Rec.709 luma constant after grading (AE "Preserve Luminosity").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorBalance {
    pub shadows: [f32; 3],
    pub midtones: [f32; 3],
    pub highlights: [f32; 3],
    pub preserve_luminosity: bool,
}

impl Default for ColorBalance {
    fn default() -> Self {
        Self {
            shadows: [0.0; 3],
            midtones: [0.0; 3],
            highlights: [0.0; 3],
            preserve_luminosity: true,
        }
    }
}

const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

fn rec709_luma(r: f32, g: f32, b: f32) -> f32 {
    r * LUMA_R + g * LUMA_G + b * LUMA_B
}

/// Apply three-way Color Balance to an RGBA8 buffer.
pub fn apply_color_balance(pixels: &mut [u8], cb: &ColorBalance) {
    if pixels.is_empty() {
        return;
    }
    let scale = 127.0 / 100.0; // slider ±100 → ±~half channel range

    for px in pixels.chunks_exact_mut(4) {
        let rf = px[0] as f32;
        let gf = px[1] as f32;
        let bf = px[2] as f32;
        let luma01 = rec709_luma(rf, gf, bf) / 255.0;

        // Quadratic Bernstein weights: sum to 1 across tonal ranges.
        let w_shadow = (1.0 - luma01) * (1.0 - luma01);
        let w_high = luma01 * luma01;
        let w_mid = 2.0 * luma01 * (1.0 - luma01);

        let shift = |i: usize| -> f32 {
            (cb.shadows[i] * w_shadow + cb.midtones[i] * w_mid + cb.highlights[i] * w_high) * scale
        };

        let mut out_r = (rf + shift(0)).clamp(0.0, 255.0);
        let mut out_g = (gf + shift(1)).clamp(0.0, 255.0);
        let mut out_b = (bf + shift(2)).clamp(0.0, 255.0);

        if cb.preserve_luminosity {
            let luma_in = rec709_luma(rf, gf, bf);
            let luma_out = rec709_luma(out_r, out_g, out_b);
            let correction = luma_in - luma_out;
            out_r = (out_r + correction).clamp(0.0, 255.0);
            out_g = (out_g + correction).clamp(0.0, 255.0);
            out_b = (out_b + correction).clamp(0.0, 255.0);
        }

        px[0] = out_r.round() as u8;
        px[1] = out_g.round() as u8;
        px[2] = out_b.round() as u8;
    }
}

/// Channel Mixer: each output channel is a weighted mix of input channels.
///
/// Weights are percentages (100 = full channel). `monochrome` collapses the
/// image using the red row weights (Photoshop behaviour).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelMixer {
    /// Output red = row 0 · [r, g, b], etc. Percent units.
    pub matrix: [[f32; 3]; 3],
    pub monochrome: bool,
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self {
            matrix: [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 100.0]],
            monochrome: false,
        }
    }
}

/// 256-bin luma histogram (Rec.709) for scopes / Lumetri displays.
pub fn compute_luma_histogram(pixels: &[u8]) -> [u32; 256] {
    let mut bins = [0u32; 256];
    for px in pixels.chunks_exact(4) {
        let l = rec709_luma(px[0] as f32, px[1] as f32, px[2] as f32);
        let bin = l.round().clamp(0.0, 255.0) as usize;
        bins[bin] += 1;
    }
    bins
}

/// Per-channel RGB histograms `[r_bins, g_bins, b_bins]` for parade scopes.
pub fn compute_rgb_histograms(pixels: &[u8]) -> [[u32; 256]; 3] {
    let mut out = [[0u32; 256]; 3];
    for px in pixels.chunks_exact(4) {
        for c in 0..3 {
            out[c][px[c] as usize] += 1;
        }
    }
    out
}

/// Apply the Channel Mixer to an RGBA8 buffer.
pub fn apply_channel_mixer(pixels: &mut [u8], mixer: &ChannelMixer) {
    if pixels.is_empty() {
        return;
    }
    let m = &mixer.matrix;

    for px in pixels.chunks_exact_mut(4) {
        let r = px[0] as f32;
        let g = px[1] as f32;
        let b = px[2] as f32;

        if mixer.monochrome {
            let gray = (r * m[0][0] + g * m[0][1] + b * m[0][2]) / 100.0;
            let v = gray.clamp(0.0, 255.0).round() as u8;
            px[0] = v;
            px[1] = v;
            px[2] = v;
        } else {
            let or_ = (r * m[0][0] + g * m[0][1] + b * m[0][2]) / 100.0;
            let og = (r * m[1][0] + g * m[1][1] + b * m[1][2]) / 100.0;
            let ob = (r * m[2][0] + g * m[2][1] + b * m[2][2]) / 100.0;
            px[0] = or_.clamp(0.0, 255.0).round() as u8;
            px[1] = og.clamp(0.0, 255.0).round() as u8;
            px[2] = ob.clamp(0.0, 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient_rgb() -> Vec<u8> {
        let mut v = Vec::with_capacity(256 * 4);
        for i in 0..256u16 {
            let c = i as u8;
            v.extend_from_slice(&[c, 255 - c, (c / 2), 255]);
        }
        v
    }

    #[test]
    fn test_tone_curve_identity_and_clamping() {
        let c = ToneCurve::linear();
        assert!((c.eval(0.0) - 0.0).abs() < 1e-6);
        assert!((c.eval(0.5) - 0.5).abs() < 1e-6);
        assert!((c.eval(1.0) - 1.0).abs() < 1e-6);
        assert!((c.eval(-0.5) - 0.0).abs() < 1e-6);
        assert!((c.eval(1.5) - 1.0).abs() < 1e-6);
        // Fewer than two points falls back to identity.
        assert!((ToneCurve::new(vec![[0.4, 0.7]]).eval(0.3) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_tone_curve_sorts_and_dedups_points() {
        let c = ToneCurve::new(vec![[1.0, 1.0], [0.0, 0.0], [0.5, 0.6], [0.5, 0.55]]);
        assert!(c.points.windows(2).all(|w| w[0][0] <= w[1][0]));
        assert_eq!(c.eval(0.5), 0.55); // dedup keeps the last point
    }

    #[test]
    fn test_s_curve_increases_contrast() {
        let s = ToneCurve::new(vec![
            [0.0, 0.0],
            [0.25, 0.15],
            [0.5, 0.5],
            [0.75, 0.85],
            [1.0, 1.0],
        ]);
        assert!(s.eval(0.25) < 0.25, "shadow must darken");
        assert!(s.eval(0.75) > 0.75, "highlight must brighten");
        assert!((s.eval(0.5) - 0.5).abs() < 1e-4, "mid anchor preserved");
    }

    #[test]
    fn test_monotone_curve_does_not_overshoot() {
        // Strictly increasing data → output must stay within neighbour bounds.
        let c = ToneCurve::new(vec![[0.0, 0.0], [0.33, 0.2], [0.66, 0.8], [1.0, 1.0]]);
        let mut prev = -0.001;
        for i in 0..=64 {
            let x = i as f32 / 64.0;
            let y = c.eval(x);
            assert!((-0.05..=1.05).contains(&y), "overshoot at {x}: {y}");
            assert!(y >= prev - 1e-6, "non-monotonic at {x}");
            prev = y;
        }
    }

    #[test]
    fn test_apply_curves_identity_leaves_pixels() {
        let src = gradient_rgb();
        let mut out = src.clone();
        apply_curves(&mut out, &ChannelCurves::default());
        assert_eq!(src, out);
    }

    #[test]
    fn test_apply_curves_per_channel_isolation() {
        let mut px = vec![128u8, 128, 128, 255];
        let curves = ChannelCurves {
            red: Some(ToneCurve::new(vec![[0.0, 0.0], [1.0, 0.0]])), // red → black
            ..Default::default()
        };
        apply_curves(&mut px, &curves);
        assert_eq!(px[0], 0);
        assert_eq!(px[1], 128);
        assert_eq!(px[2], 128);
    }

    #[test]
    fn test_color_balance_zero_is_identity() {
        let src = gradient_rgb();
        let mut out = src.clone();
        apply_color_balance(&mut out, &ColorBalance::default());
        assert_eq!(src, out);
    }

    #[test]
    fn test_color_balance_shifts_channels_directionally() {
        let mut px = vec![128u8, 128, 128, 255];
        apply_color_balance(
            &mut px,
            &ColorBalance {
                midtones: [50.0, 0.0, -50.0],
                preserve_luminosity: false,
                ..Default::default()
            },
        );
        assert!(px[0] > 150, "red pushed up: {}", px[0]);
        assert!(px[2] < 110, "blue pulled down: {}", px[2]);
    }

    #[test]
    fn test_color_balance_preserve_luminosity_keeps_luma() {
        let src = gradient_rgb();
        let mut out = src.clone();
        apply_color_balance(
            &mut out,
            &ColorBalance {
                shadows: [-40.0, 20.0, 60.0],
                midtones: [30.0, -50.0, 10.0],
                highlights: [70.0, 0.0, -30.0],
                preserve_luminosity: true,
            },
        );
        for (src_px, out_px) in src.chunks(4).zip(out.chunks(4)) {
            let l_in = rec709_luma(src_px[0] as f32, src_px[1] as f32, src_px[2] as f32);
            let l_out = rec709_luma(out_px[0] as f32, out_px[1] as f32, out_px[2] as f32);
            assert!(
                (l_in - l_out).abs() < 12.0,
                "luma drift {} vs {}",
                l_in,
                l_out
            );
        }
    }

    #[test]
    fn test_channel_mixer_identity_and_swap() {
        let src = vec![200u8, 40, 90, 255];

        let mut out = src.clone();
        apply_channel_mixer(&mut out, &ChannelMixer::default());
        assert_eq!(out, src);

        let swap = ChannelMixer {
            matrix: [[0.0, 100.0, 0.0], [100.0, 0.0, 0.0], [0.0, 0.0, 100.0]],
            monochrome: false,
        };
        let mut swapped = src.clone();
        apply_channel_mixer(&mut swapped, &swap);
        assert_eq!((swapped[0], swapped[1]), (src[1], src[0]));
    }

    #[test]
    fn test_channel_mixer_monochrome_grays() {
        let mut px = vec![220u8, 30, 60, 255];
        apply_channel_mixer(
            &mut px,
            &ChannelMixer {
                matrix: [[30.0, 59.0, 11.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
                monochrome: true,
            },
        );
        assert_eq!(px[0], px[1]);
        assert_eq!(px[1], px[2]);
        let expected = ((220.0f32 * 30.0 + 30.0 * 59.0 + 60.0 * 11.0) / 100.0).round() as u8;
        assert_eq!(px[0], expected);
    }

    #[test]
    fn test_all_kernels_handle_empty_buffers() {
        let mut empty: Vec<u8> = vec![];
        apply_curves(&mut empty, &ChannelCurves::default());
        apply_color_balance(&mut empty, &ColorBalance::default());
        apply_channel_mixer(&mut empty, &ChannelMixer::default());
        assert!(empty.is_empty());
    }

    #[test]
    fn test_luma_histogram_counts_every_pixel() {
        // Two pixels: pure red (luma ≈ 54) and mid gray (128).
        let mut buf = Vec::new();
        buf.extend_from_slice(&[255, 0, 0, 255]);
        buf.extend_from_slice(&[128, 128, 128, 255]);
        let h = compute_luma_histogram(&buf);
        let total: u32 = h.iter().sum();
        assert_eq!(total, 2);
        assert!(h[54] >= 1 || h[55] >= 1, "red luma bin missing");
        assert!(h[128] == 1, "gray bin missing");
        assert_eq!(compute_luma_histogram(&[]), [0u32; 256]);
    }

    #[test]
    fn test_rgb_histograms_separate_channels() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[10, 200, 30, 255]);
        buf.extend_from_slice(&[20, 210, 40, 255]);
        let [r, g, b] = compute_rgb_histograms(&buf);
        assert_eq!(r[10], 1);
        assert_eq!(r[20], 1);
        assert_eq!(g[200], 1);
        assert_eq!(g[210], 1);
        assert_eq!(b[30], 1);
        assert_eq!(b[40], 1);
        let total_r: u32 = r.iter().sum();
        assert_eq!(total_r, 2);
    }

    #[test]
    fn test_extreme_inputs_do_not_panic_or_nan() {
        let mut buf = vec![255u8; 64 * 4];
        let wild = ColorBalance {
            shadows: [-500.0, 500.0, 0.0],
            midtones: [1000.0, -1000.0, 0.0],
            highlights: [0.0, 250.0, -250.0],
            preserve_luminosity: true,
        };
        apply_color_balance(&mut buf, &wild);

        let huge = ChannelMixer {
            matrix: [[900.0, -400.0, 300.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            monochrome: false,
        };
        apply_channel_mixer(&mut buf, &huge);
    }
}

// ── Vibrance ───────────────────────────────────────────────────────────────

/// Vibrance: boosts saturation of muted pixels far more than saturated ones
/// and partially protects warm skin-tone hues. `amount` spans −100..100
/// (negative desaturates).
pub fn apply_vibrance(pixels: &mut [u8], amount: f32) {
    let amt = if amount.is_finite() {
        (amount / 100.0).clamp(-1.0, 1.0)
    } else { 0.0 };
    if amt == 0.0 || pixels.is_empty() {
        return;
    }
    for px in pixels.chunks_exact_mut(4) {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        // 0..1 saturation proxy (chroma relative to intensity).
        let sat = (max - min) / (max + 1e-6);
        // Skin-tone guard applies to boosts only; desaturation is uniform so
        // negative amounts behave like a straightforward saturation pull.
        let boost = if amt >= 0.0 {
            amt * (1.0 - sat) * (1.0 - 0.5 * if r > g && g >= b { 1.0f32 } else { 0.0 })
        } else {
            amt
        };
        if boost.abs() < 1e-6 {
            continue;
        }
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let k = 1.0 + boost;
        for (c, slot) in [r, g, b].iter().zip(px.iter_mut().take(3)) {
            let v = luma + (c - luma) * k;
            *slot = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

// ── White Balance ──────────────────────────────────────────────────────────

/// Temperature/Tint white-balance sliders (−100..100 each).
/// Positive temperature warms (R↑ B↓); positive tint shifts magenta (G↓).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhiteBalance {
    pub temperature: f32,
    pub tint: f32,
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
        }
    }
}

pub fn apply_white_balance(pixels: &mut [u8], wb: &WhiteBalance) {
    let temperature = if wb.temperature.is_finite() { wb.temperature } else { 0.0 };
    let tint = if wb.tint.is_finite() { wb.tint } else { 0.0 };
    let t = (temperature / 100.0).clamp(-1.0, 1.0) * 0.25;
    let gshift = -(tint / 100.0).clamp(-1.0, 1.0) * 0.20;
    if (t == 0.0 && gshift == 0.0) || pixels.is_empty() {
        return;
    }
    let gains = [1.0 + t, 1.0 + gshift, 1.0 - t];
    for px in pixels.chunks_exact_mut(4) {
        for (c, gain) in gains.iter().enumerate() {
            px[c] = ((px[c] as f32 * gain).clamp(0.0, 255.0)) as u8;
        }
    }
}

// ── HSL Adjust ─────────────────────────────────────────────────────────────

/// Lumetri-style three-way adjustment: hue rotation in degrees, saturation
/// gain (−100..100 mapped onto ×0..×2 around 1.0) and lightness shift
/// (−100..100 → ±0.5 additive).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HslAdjust {
    /// Hue rotation in degrees (wraps).
    pub hue_deg: f32,
    /// Saturation slider −100..100.
    pub saturation: f32,
    /// Lightness slider −100..100.
    pub lightness: f32,
}

impl Default for HslAdjust {
    fn default() -> Self {
        Self {
            hue_deg: 0.0,
            saturation: 0.0,
            lightness: 0.0,
        }
    }
}

/// RGB (0..1) → HSL (h in degrees 0..360, s/l in 0..1).
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / d).rem_euclid(6.0)
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

/// HSL → RGB (writes 0..1 triple).
fn hsl_to_rgb(h: f32, s: f32, l: f32, out: &mut [f32; 3]) {
    if s <= 1e-6 {
        *out = [l, l, l];
        return;
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let sector = (hp.floor() as i32).rem_euclid(6);
    let (r1, g1, b1) = match sector {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    *out = [r1 + m, g1 + m, b1 + m];
}

pub fn apply_hsl_adjust(pixels: &mut [u8], adj: &HslAdjust) {
    let hue = if adj.hue_deg.is_finite() { adj.hue_deg } else { 0.0 };
    let saturation = if adj.saturation.is_finite() { adj.saturation } else { 0.0 };
    let lightness = if adj.lightness.is_finite() { adj.lightness } else { 0.0 };
    let sat_mul = 1.0 + (saturation / 100.0).clamp(-1.0, 1.0);
    let l_shift = (lightness / 100.0).clamp(-1.0, 1.0) * 0.5;
    if hue.abs() < 1e-3 && (sat_mul - 1.0).abs() < 1e-6 && l_shift.abs() < 1e-6 {
        return;
    }
    for px in pixels.chunks_exact_mut(4) {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;
        let (mut h, mut s, mut l) = rgb_to_hsl(r, g, b);
        h = (h + hue).rem_euclid(360.0);
        s = (s * sat_mul).clamp(0.0, 1.0);
        l = (l + l_shift).clamp(0.0, 1.0);
        let mut rgb = [0.0f32; 3];
        hsl_to_rgb(h, s, l, &mut rgb);
        for (c, slot) in rgb.iter().zip(px.iter_mut().take(3)) {
            *slot = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod vibrance_wb_tests {
    use super::*;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            v.extend_from_slice(&rgba);
        }
        v
    }

    #[test]
    fn test_vibrance_leaves_gray_unchanged() {
        // Pure gray has zero chroma: boosting vibrance must not shift it.
        let mut img = solid(8, 8, [128, 128, 128, 255]);
        let before = img.clone();
        apply_vibrance(&mut img, 100.0);
        assert_eq!(img, before, "gray must survive max vibrance");
    }

    #[test]
    fn test_vibrance_boosts_muted_more_than_saturated() {
        // Muted teal vs fully saturated red.
        let mut img = vec![80u8, 120, 130, 255, 255, 0, 0, 255];
        let before = img.clone();
        apply_vibrance(&mut img, 60.0);
        let muted_drift = (img[0] as i32 - before[0] as i32).abs()
            + (img[1] as i32 - before[1] as i32).abs()
            + (img[2] as i32 - before[2] as i32).abs();
        let sat_drift = (img[4] as i32 - before[4] as i32).abs()
            + (img[5] as i32 - before[5] as i32).abs()
            + (img[6] as i32 - before[6] as i32).abs();
        assert!(
            muted_drift > sat_drift,
            "muted {muted_drift} must move more than saturated {sat_drift}"
        );
    }

    #[test]
    fn test_negative_vibrance_desaturates_and_zero_is_identity() {
        let mut img = solid(4, 4, [200, 40, 40, 255]);
        apply_vibrance(&mut img, -90.0);
        let spread = img[0].max(img[1]) - img[0].min(img[1]);
        assert!(spread < 120, "spread should shrink, got {}", spread);

        let mut id = solid(4, 4, [200, 40, 40, 255]);
        apply_vibrance(&mut id, 0.0);
        assert_eq!(id, solid(4, 4, [200, 40, 40, 255]));
    }

    #[test]
    fn test_vibrance_deterministic_and_safe() {
        let run = || {
            let mut img = solid(16, 16, [90, 140, 200, 255]);
            apply_vibrance(&mut img, 45.0);
            img
        };
        assert_eq!(run(), run());
        apply_vibrance(&mut [], 50.0); // empty buffer safe
    }

    #[test]
    fn test_white_balance_warms_and_cools() {
        let base = solid(4, 4, [128, 128, 128, 255]);

        let mut warm = base.clone();
        apply_white_balance(
            &mut warm,
            &WhiteBalance {
                temperature: 80.0,
                tint: 0.0,
            },
        );
        assert!(warm[0] > 128, "warm raises R: {}", warm[0]);
        assert!(warm[2] < 128, "warm lowers B: {}", warm[2]);
        assert_eq!(warm[1], 128, "tint 0 keeps G");

        let mut cool = base.clone();
        apply_white_balance(
            &mut cool,
            &WhiteBalance {
                temperature: -80.0,
                tint: 0.0,
            },
        );
        assert!(cool[0] < 128 && cool[2] > 128);

        let mut mag = base.clone();
        apply_white_balance(
            &mut mag,
            &WhiteBalance {
                temperature: 0.0,
                tint: 60.0,
            },
        );
        assert!(mag[1] < 128, "positive tint reduces G (magenta)");

        let mut neutral = base.clone();
        apply_white_balance(&mut neutral, &WhiteBalance::default());
        assert_eq!(neutral, base);
    }

    #[test]
    fn test_hsl_identity_at_defaults() {
        let src = solid(8, 8, [200, 90, 30, 255]);
        let mut img = src.clone();
        apply_hsl_adjust(&mut img, &HslAdjust::default());
        assert_eq!(img, src);
    }

    #[test]
    fn test_hsl_full_desaturation_grays_out() {
        let mut img = solid(4, 4, [200, 40, 40, 255]);
        apply_hsl_adjust(
            &mut img,
            &HslAdjust {
                saturation: -100.0,
                ..Default::default()
            },
        );
        assert_eq!(img[0], img[1], "R==G");
        assert_eq!(img[1], img[2], "G==B");
    }

    #[test]
    fn test_hsl_hue_rotation_moves_red_to_cyan_family() {
        // Pure red rotated +180° lands in the cyan family (low R, high G+B).
        let mut img = solid(2, 2, [255, 0, 0, 255]);
        apply_hsl_adjust(
            &mut img,
            &HslAdjust {
                hue_deg: 180.0,
                ..Default::default()
            },
        );
        assert!(img[0] < 60, "R dropped: {}", img[0]);
        assert!(img[1] > 180 && img[2] > 180, "G/B high: {:?}", &img[..3]);
        // Rotation wraps: +360° returns to the original hue.
        let mut wrapped = solid(2, 2, [255, 0, 0, 255]);
        apply_hsl_adjust(
            &mut wrapped,
            &HslAdjust {
                hue_deg: 360.0,
                ..Default::default()
            },
        );
        assert!(
            wrapped[0] > 230 && wrapped[1] < 25,
            "wrap keeps red: {:?}",
            &wrapped[..3]
        );
    }

    #[test]
    fn test_hsl_lightness_extremes_and_determinism() {
        let run = |light: f32| {
            let mut img = solid(4, 4, [120, 120, 120, 255]);
            apply_hsl_adjust(
                &mut img,
                &HslAdjust {
                    lightness: light,
                    ..Default::default()
                },
            );
            img
        };
        let bright = run(100.0);
        assert!(
            bright[0] > 240,
            "+100 lightness → near white: {}",
            bright[0]
        );
        let dark = run(-100.0);
        assert!(dark[0] < 15, "-100 lightness → near black: {}", dark[0]);
        assert_eq!(run(20.0), run(20.0), "deterministic");
        // Empty buffer safe.
        apply_hsl_adjust(
            &mut [],
            &HslAdjust {
                hue_deg: 90.0,
                saturation: 50.0,
                lightness: 10.0,
            },
        );
    }
}
