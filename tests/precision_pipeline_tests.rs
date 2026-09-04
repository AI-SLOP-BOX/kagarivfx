//! Precision, determinism and pipeline-lifecycle regression tests.
//!
//! Areas covered (all pure/headless — no GPU, no audio devices, no GUI):
//!   §1 Color science golden values: sRGB transfer curves (pow-approx vs IEC
//!      piecewise), Porter-Duff `over`, premultiply/lerp/clamp, HSL anchors &
//!      hue rotation, Levels mapping, tetrahedral identity LUT, working-color-
//!      space round trips, HdrF32Buffer exposure / tonemap / blend modes.
//!   §2 Quantization: exact None-dither mapping for every 8-bit level, ordered
//!      Bayer checkerboard at mid-gray, dither determinism/bounds, Posterize
//!      Time golden grids and invariants.
//!   §3 Unified time & tempo math: exact frame round trips across fractional
//!      (NTSC) frame rates and negative frames, cross-rate floors, tempo-map
//!      segment math and beat/time inversion.
//!   §4 Render pipeline lifecycle: async frame/batch delivery, stale-version
//!      cancellation after flush, and panic isolation inside the worker.
//!   §5 Effect presets: JSON file round trip, unique re-apply to layers,
//!      directory discovery ordering/filtering, and IO error surfacing.
//!
//! All expectations are derived from the reference implementations; tolerances
//! are deliberately loose enough to absorb f32 rounding but tight enough to
//! catch real regressions.

use aftereffects_oss::core::color as color;
use aftereffects_oss::core::color_science::{self, HdrF32Buffer, Lut3D, WorkingColorSpace};
use aftereffects_oss::core::effect_presets::{self, EffectPreset};
use aftereffects_oss::core::hdr_dither::{quantize_hdr_slice_dithered, DitherMethod};
use aftereffects_oss::core::posterize_time::{quantize_frame_posterize, PosterizeTimeSettings};
use aftereffects_oss::core::property::Animatable;
use aftereffects_oss::core::render_pipeline::{RenderCommand, RenderPipeline, RenderResult};
use aftereffects_oss::core::timeline::{BlendMode, Effect, EffectType, Layer, LayerType};
use aftereffects_oss::core::unified_time::{FrameRate, TempoChange, TempoMap, Time};
use std::time::{Duration, Instant};

fn assert_approx(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within ±{tolerance} of {expected}"
    );
}

fn assert_approx_msg(actual: f32, expected: f32, tolerance: f32, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: expected {actual} to be within ±{tolerance} of {expected}"
    );
}

fn seconds_of(time: Time) -> f64 {
    time.numerator as f64 / f64::from(time.denominator)
}

fn hdr_from_pixels(pixels: &[[f32; 4]]) -> HdrF32Buffer {
    let mut buffer = HdrF32Buffer::new(pixels.len() as u32, 1);
    for (i, px) in pixels.iter().enumerate() {
        let base = i * 4;
        buffer.data[base..base + 4].copy_from_slice(px);
    }
    buffer
}

fn hdr_channel(buffer: &HdrF32Buffer, pixel: usize, channel: usize) -> f32 {
    buffer.data[pixel * 4 + channel]
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  Color science precision
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn srgb_transfer_functions_hit_reference_anchors() {
    // IEC 61966-2-1 reference values (computed in higher precision).
    assert_approx(color::srgb_to_linear_piecewise(0.5), 0.214_041_1, 1e-5);
    assert_approx(color::linear_to_srgb_piecewise(0.5), 0.735_356_98, 1e-5);
    assert_approx(color::linear_to_srgb_piecewise(0.18), 0.461_356_13, 1e-5);
    assert_approx(color::srgb_to_linear_piecewise(0.040_45), 0.003_130_805, 1e-6);
    assert_approx(color::linear_to_srgb_piecewise(0.003_130_8), 0.040_449_9, 1e-5);

    // The renderer's legacy pow(2.2) approximation diverges visibly at mid-gray.
    assert_approx(color::linear_to_srgb(0.5), 0.729_740_05, 1e-5);
    assert_approx(color::srgb_to_linear(0.5), 0.217_637_64, 1e-5);
    assert!(
        (color::srgb_to_linear_piecewise(0.5) - color::srgb_to_linear(0.5)).abs() > 0.002,
        "piecewise and pow-approx curves must differ at mid-gray"
    );

    // A linear 0.18 → sRGB → 8-bit conversion lands on the canonical 118.
    assert_eq!((color::linear_to_srgb_piecewise(0.18) * 255.0).round() as u8, 118);
    // Linear mid-gray quantizes to 188.
    assert_eq!((color::linear_to_srgb_piecewise(0.5) * 255.0).round() as u8, 188);
}

#[test]
fn transfer_curves_agree_across_color_and_color_science_modules() {
    // The two modules re-implement the same piecewise curves; they must not drift.
    for i in 0..=100 {
        let v = i as f32 / 100.0;
        assert_approx(
            color::srgb_to_linear_piecewise(v),
            color_science::srgb_to_linear(v),
            1e-7,
        );
        assert_approx(
            color::linear_to_srgb_piecewise(v),
            color_science::linear_to_srgb(v),
            1e-7,
        );
    }
}

#[test]
fn transfer_curves_are_monotonic_and_stay_in_unit_range() {
    let mut prev_piece_l = 0.0f32;
    let mut prev_piece_s = 0.0f32;
    let mut prev_pow_l = 0.0f32;
    let mut prev_pow_s = 0.0f32;
    for i in 0..=4096 {
        let v = i as f32 / 4096.0;
        let pl = color::srgb_to_linear_piecewise(v);
        let ps = color::linear_to_srgb_piecewise(v);
        let wl = color::srgb_to_linear(v);
        let ws = color::linear_to_srgb(v);
        assert!(pl >= prev_piece_l - 1e-7 && pl <= 1.0, "srgb_to_linear_piecewise non-monotonic at {v}");
        assert!(ps >= prev_piece_s - 1e-7 && ps <= 1.0, "linear_to_srgb_piecewise non-monotonic at {v}");
        assert!(wl >= prev_pow_l - 1e-7 && wl <= 1.0, "srgb_to_linear non-monotonic at {v}");
        assert!(ws >= prev_pow_s - 1e-7 && ws <= 1.0, "linear_to_srgb non-monotonic at {v}");
        prev_piece_l = pl;
        prev_piece_s = ps;
        prev_pow_l = wl;
        prev_pow_s = ws;
    }
    assert_eq!(color::linear_to_srgb_piecewise(-1.0), 0.0);
    assert_eq!(color::linear_to_srgb_piecewise(2.0), 1.0);
}

#[test]
fn rgba_over_operator_matches_porter_duff_reference_values() {
    use color::Rgbaf;

    let cases: Vec<(Rgbaf, Rgbaf, [f32; 4])> = vec![
        // Red at 50% over opaque blue.
        (Rgbaf::new(1.0, 0.0, 0.0, 0.5), Rgbaf::new(0.0, 0.0, 1.0, 1.0), [0.5, 0.0, 0.5, 1.0]),
        // Red at 50% over blue at 50%.
        (Rgbaf::new(1.0, 0.0, 0.0, 0.5), Rgbaf::new(0.0, 0.0, 1.0, 0.5), [2.0 / 3.0, 0.0, 1.0 / 3.0, 0.75]),
        // White at 50% over opaque black.
        (Rgbaf::new(1.0, 1.0, 1.0, 0.5), Rgbaf::new(0.0, 0.0, 0.0, 1.0), [0.5, 0.5, 0.5, 1.0]),
        // 80% gray at 50% over opaque white → 90% gray.
        (Rgbaf::new(0.8, 0.8, 0.8, 0.5), Rgbaf::WHITE, [0.9, 0.9, 0.9, 1.0]),
    ];
    for (fg, bg, expected) in cases {
        let out = fg.over(bg);
        assert_approx(out.r, expected[0], 1e-5);
        assert_approx(out.g, expected[1], 1e-5);
        assert_approx(out.b, expected[2], 1e-5);
        assert_approx(out.a, expected[3], 1e-5);
    }

    // Fully transparent composites behave as identities in both directions.
    let opaque_red = Rgbaf::new(1.0, 0.0, 0.0, 1.0);
    let clear = Rgbaf::TRANSPARENT;
    let over_clear = opaque_red.over(clear);
    assert_approx(over_clear.r, 1.0, 1e-6);
    assert_approx(over_clear.a, 1.0, 1e-6);
    let clear_over = clear.over(opaque_red);
    assert_approx(clear_over.r, 1.0, 1e-6);
    assert_approx(clear_over.a, 1.0, 1e-6);
}

#[test]
fn rgba_premultiply_lerp_and_clamp_edges() {
    use color::Rgbaf;

    let c = Rgbaf::new(0.7, 0.2, 0.1, 0.5);
    let p = c.premultiply();
    assert_approx(p.r, 0.35, 1e-6);
    assert_approx(p.g, 0.1, 1e-6);
    assert_approx(p.b, 0.05, 1e-6);
    assert_approx(p.a, 0.5, 1e-6);
    let u = p.unpremultiply();
    assert_approx(u.r, 0.7, 1e-6);
    assert_approx(u.g, 0.2, 1e-6);
    assert_approx(u.b, 0.1, 1e-6);

    // Zero alpha can never be unpremultiplied; it must fail closed to ZERO.
    assert_eq!(Rgbaf::ZERO.premultiply(), Rgbaf::ZERO);
    assert_eq!(Rgbaf::new(1.0, 1.0, 1.0, 0.0).unpremultiply(), Rgbaf::ZERO);
    assert_eq!(Rgbaf::new(1.0, 1.0, 1.0, 1.0).premultiply(), Rgbaf::new(1.0, 1.0, 1.0, 1.0));

    // Lerp endpoints must be exact.
    let a = Rgbaf::new(0.1, 0.2, 0.3, 0.4);
    let b = Rgbaf::new(0.5, 0.6, 0.7, 0.8);
    assert_eq!(a.lerp(b, 0.0), a);
    assert_eq!(a.lerp(b, 1.0), b);

    let clamped = Rgbaf::new(-0.5, 1.5, 0.25, 2.0).clamp_01();
    assert_eq!(clamped, Rgbaf::new(0.0, 1.0, 0.25, 1.0));
}

#[test]
fn hsl_primary_anchors_and_120_degree_hue_rotation() {
    // (name, rgb, hue) — all primaries/secondaries sit at s=1, l=0.5.
    let anchors = [
        ("red", [1.0f32, 0.0, 0.0], 0.0f32),
        ("yellow", [1.0, 1.0, 0.0], 60.0),
        ("green", [0.0, 1.0, 0.0], 120.0),
        ("cyan", [0.0, 1.0, 1.0], 180.0),
        ("blue", [0.0, 0.0, 1.0], 240.0),
        ("magenta", [1.0, 0.0, 1.0], 300.0),
    ];
    for (name, rgb, hue) in anchors {
        let hsl = color_science::rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
        assert_approx(hsl[0], hue, 1e-4);
        assert_approx(hsl[1], 1.0, 1e-5);
        assert_approx(hsl[2], 0.5, 1e-5);
        let back = color_science::hsl_to_rgb(hsl[0], hsl[1], hsl[2]);
        for c in 0..3 {
            assert_approx_msg(back[c], rgb[c], 1e-5, &format!("{name} round trip channel {c}"));
        }
    }

    // Rotating hues by 120° maps red→green→blue and yellow→cyan→magenta.
    let rgb_of = |h: f32, s: f32, l: f32| color_science::hsl_to_rgb(h, s, l);
    for (hue, expected) in [(0.0f32, [0.0f32, 1.0, 0.0]), (120.0, [0.0, 0.0, 1.0]), (240.0, [1.0, 0.0, 0.0])] {
        let rotated = rgb_of((hue + 120.0) % 360.0, 1.0, 0.5);
        for c in 0..3 {
            assert_approx(rotated[c], expected[c], 1e-5);
        }
    }
    for (hue, expected) in [(60.0f32, [0.0f32, 1.0, 1.0]), (180.0, [1.0, 0.0, 1.0]), (300.0, [1.0, 1.0, 0.0])] {
        let rotated = rgb_of((hue + 120.0) % 360.0, 1.0, 0.5);
        for c in 0..3 {
            assert_approx(rotated[c], expected[c], 1e-5);
        }
    }

    // Grays have s=0 and round-trip to themselves.
    for v in [0.0f32, 0.25, 0.5, 1.0] {
        let hsl = color_science::rgb_to_hsl(v, v, v);
        assert_approx(hsl[1], 0.0, 1e-6);
        assert_approx(hsl[2], v, 1e-6);
        let back = color_science::hsl_to_rgb(hsl[0], hsl[1], hsl[2]);
        assert_approx(back[0], v, 1e-6);
    }

    // shift_hsl with +120° must turn pure red into green and back with −120°.
    let green = color_science::shift_hsl([1.0, 0.0, 0.0], 120.0, 1.0, 1.0);
    assert_approx(green[0], 0.0, 1e-4);
    assert_approx(green[1], 1.0, 1e-4);
    let red_again = color_science::shift_hsl(green, -120.0, 1.0, 1.0);
    assert_approx(red_again[0], 1.0, 1e-4);
    assert_approx(red_again[1], 0.0, 1e-4);
}

#[test]
fn hsl_roundtrip_holds_across_full_color_mesh() {
    let mut checked = 0usize;
    for i in 0..=20 {
        for j in 0..=20 {
            for k in 0..=20 {
                let rgb = [i as f32 / 20.0, j as f32 / 20.0, k as f32 / 20.0];
                // Skip nearly-neutral colors where hue is undefined.
                let range = rgb.iter().copied().fold(f32::NAN, f32::max)
                    - rgb.iter().copied().fold(f32::NAN, f32::min);
                if range < 0.02 {
                    continue;
                }
                let hsl = color_science::rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
                let back = color_science::hsl_to_rgb(hsl[0], hsl[1], hsl[2]);
                for c in 0..3 {
                    // f32 chain tolerance; real regressions drift orders of magnitude more.
                    assert_approx(back[c], rgb[c], 5e-4);
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 1000, "mesh should exercise a large color set");
}

#[test]
fn levels_mapping_identity_clamp_and_gamma() {
    // Identity settings pass values through unchanged.
    for i in 0..=20 {
        let v = i as f32 / 20.0;
        assert_approx(color_science::apply_levels(v, 0.0, 1.0, 1.0, 0.0, 1.0), v, 1e-6);
    }
    // Values outside the input range clamp to the output endpoints.
    assert_approx(color_science::apply_levels(-0.25, 0.2, 0.8, 1.0, 0.0, 1.0), 0.0, 1e-6);
    assert_approx(color_science::apply_levels(1.5, 0.2, 0.8, 1.0, 0.0, 1.0), 1.0, 1e-6);
    // Normalized remap: (v − 0.2) / 0.6 at v=0.5 is exactly 0.5.
    assert_approx(color_science::apply_levels(0.5, 0.2, 0.8, 1.0, 0.0, 1.0), 0.5, 1e-6);
    // Gamma 2 applies sqrt().
    assert_approx(color_science::apply_levels(0.5, 0.0, 1.0, 2.0, 0.0, 1.0), 0.5f32.sqrt(), 1e-6);
    // Output black/white remap.
    assert_approx(color_science::apply_levels(0.0, 0.0, 1.0, 1.0, 0.2, 0.8), 0.2, 1e-6);
    assert_approx(color_science::apply_levels(1.0, 0.0, 1.0, 1.0, 0.2, 0.8), 0.8, 1e-6);
    assert_approx(color_science::apply_levels(0.25, 0.0, 1.0, 1.0, 0.2, 0.8), 0.35, 1e-6);
    // Inverted output range maps 0→0.8 and 1→0.2.
    assert_approx(color_science::apply_levels(0.0, 0.0, 1.0, 1.0, 0.8, 0.2), 0.8, 1e-6);
    assert_approx(color_science::apply_levels(1.0, 0.0, 1.0, 1.0, 0.8, 0.2), 0.2, 1e-6);
}

#[test]
fn identity_lut_is_exact_on_grid_nodes_and_sane_outside() {
    for size in [2usize, 4, 17, 33] {
        let lut = Lut3D::identity(size);
        let s = (size - 1) as f32;
        // Grid nodes below the top edge (r = i/s for i < s) must reproduce
        // their stored values exactly. r = 1.0 maps to the clamped cell
        // boundary and is therefore checked in the edge assertions instead.
        for i in 0..(size - 1) {
            for j in 0..(size - 1) {
                for k in 0..(size - 1) {
                    let r = i as f32 / s;
                    let g = j as f32 / s;
                    let b = k as f32 / s;
                    let out = lut.apply(r, g, b);
                    assert_approx(out.0, r, 1e-6);
                    assert_approx(out.1, g, 1e-6);
                    assert_approx(out.2, b, 1e-6);
                }
            }
        }
        // Off-grid interior points interpolate back to the input. The upper
        // edge is intentionally excluded: r=1.0 is clamped to the last grid
        // cell, which reproduces 1 − 1/s (verified separately above).
        for i in 1..50 {
            let v = i as f32 / 50.0;
            let out = lut.apply(v, 0.5, 1.0 - v);
            assert_approx(out.0, v, 1e-3);
            assert_approx(out.1, 0.5, 1e-3);
            assert_approx(out.2, 1.0 - v, 1e-3);
        }
        // Out-of-range and NaN inputs clamp to the LUT domain (never exceed 1.0
        // or drift far below it on the upper edge).
        let hi = lut.apply(2.0, 2.0, 2.0);
        assert!(hi.0 <= 1.0 && hi.0 >= 0.998, "size {size}: {hi:?}");
        let lo = lut.apply(-1.0, -1.0, -1.0);
        assert!(lo.0.abs() < 1e-6);
        let nan = lut.apply(f32::NAN, f32::NAN, f32::NAN);
        assert!(nan.0.is_finite() && nan.1.is_finite() && nan.2.is_finite());
    }
    // A degenerate (empty) LUT must be a passthrough.
    let empty = Lut3D {
        size: 0,
        data: Vec::new(),
    };
    assert_eq!(empty.apply(0.3, 0.6, 0.9), (0.3, 0.6, 0.9));
}

#[test]
fn working_color_space_roundtrips_and_anchors() {
    // src == dst returns the input unchanged (exact).
    let sample = [0.4f32, 0.7, 0.2];
    assert_eq!(
        color_science::convert_color_space(sample, WorkingColorSpace::Rec709, WorkingColorSpace::Rec709),
        sample
    );

    // White and black are fixed points of every round trip.
    for src in [WorkingColorSpace::Rec709, WorkingColorSpace::DisplayP3, WorkingColorSpace::Rec2020] {
        for dst in [WorkingColorSpace::Rec709, WorkingColorSpace::DisplayP3, WorkingColorSpace::Rec2020] {
            let white = color_science::convert_color_space([1.0, 1.0, 1.0], src, dst);
            for c in 0..3 {
                assert_approx(white[c], 1.0, 1e-3);
            }
            let black = color_science::convert_color_space([0.0, 0.0, 0.0], src, dst);
            for c in 0..3 {
                assert_approx(black[c], 0.0, 1e-6);
            }
        }
    }

    // Rec.709 red in Display P3 (reference primaries).
    let p3_red = color_science::convert_color_space(
        [1.0, 0.0, 0.0],
        WorkingColorSpace::Rec709,
        WorkingColorSpace::DisplayP3,
    );
    assert_approx(p3_red[0], 0.9175, 1e-3);
    assert_approx(p3_red[1], 0.2003, 1e-3);
    assert_approx(p3_red[2], 0.1386, 1e-3);

    // Round trips stay within the (small) matrix-approximation error.
    let values = [0.0f32, 0.25, 0.5, 0.75, 1.0];
    for &r in &values {
        for &g in &values {
            for &b in &values {
                let rgb = [r, g, b];
                let rt = color_science::convert_color_space(
                    color_science::convert_color_space(
                        rgb,
                        WorkingColorSpace::Rec709,
                        WorkingColorSpace::DisplayP3,
                    ),
                    WorkingColorSpace::DisplayP3,
                    WorkingColorSpace::Rec709,
                );
                for c in 0..3 {
                    assert_approx(rt[c], rgb[c], 6e-3);
                }
            }
        }
    }
}

#[test]
fn hdr_buffer_exposure_tonemap_and_8bit_quantization() {
    // Linear 0.5 → sRGB 0.735… → byte 188.
    let half = hdr_from_pixels(&[[0.5, 0.5, 0.5, 1.0]]);
    let bytes = half.to_rgba8(false, 0.0);
    assert_eq!(bytes, vec![188, 188, 188, 255]);

    // One stop down on a unit value lands on mid-gray (188).
    let one = hdr_from_pixels(&[[1.0, 1.0, 1.0, 1.0]]);
    let bytes = one.to_rgba8(false, -1.0);
    assert_eq!(bytes, vec![188, 188, 188, 255]);
    // One stop up saturates after clamping to the sRGB gamut.
    let bytes = one.to_rgba8(false, 1.0);
    assert_eq!(bytes, vec![255, 255, 255, 255]);

    // ACES-ish Reinhard tonemap compresses 4.0 → 0.8 → byte 231.
    let hot = hdr_from_pixels(&[[4.0, 4.0, 4.0, 1.0]]);
    assert_eq!(hot.to_rgba8(true, 0.0), vec![231, 231, 231, 255]);
    // Without tonemapping, values above 1.0 clamp to white.
    assert_eq!(hot.to_rgba8(false, 0.0), vec![255, 255, 255, 255]);

    // Dithering never moves a byte by more than one level.
    let half_dithered = half.to_rgba8_dithered(false, 0.0, true);
    for c in 0..3 {
        assert!((i32::from(half_dithered[c]) - 188).abs() <= 1);
    }
    let one_dithered = one.to_rgba8_dithered(false, -1.0, true);
    for c in 0..3 {
        assert!((i32::from(one_dithered[c]) - 188).abs() <= 1);
    }

    // RGBA8 → float → RGBA8 keeps the endpoints exact.
    let from8 = HdrF32Buffer::from_rgba8(&[255, 255, 255, 255], 1, 1);
    assert_eq!(from8.to_rgba8(false, 0.0), vec![255, 255, 255, 255]);

    // Alpha is quantized independently of the transfer curve.
    let alpha_half = hdr_from_pixels(&[[1.0, 1.0, 1.0, 0.5]]);
    assert_eq!(alpha_half.to_rgba8(false, 0.0), vec![255, 255, 255, 128]);

    // Empty buffers produce empty output.
    let empty = HdrF32Buffer::new(0, 0);
    assert!(empty.to_rgba8(false, 0.0).is_empty());
}

#[test]
fn hdr_buffer_dimension_mismatch_is_a_safe_noop() {
    let mut dst = hdr_from_pixels(&[[0.3, 0.3, 0.3, 1.0]]);
    let before = dst.data.clone();
    let wrong_size = HdrF32Buffer::new(3, 1);
    dst.blend_over(&wrong_size, 0.5);
    assert_eq!(dst.data, before);
    dst.blend_layer_mode(&wrong_size, 0.5, BlendMode::Normal);
    assert_eq!(dst.data, before);
}

#[test]
fn hdr_buffer_normal_over_honors_straight_alpha() {
    // Red 50% over opaque blue → (0.5, 0, 0.5, 1).
    let mut dst = hdr_from_pixels(&[[0.0, 0.0, 1.0, 1.0]]);
    let src = hdr_from_pixels(&[[1.0, 0.0, 0.0, 0.5]]);
    dst.blend_layer_mode(&src, 1.0, BlendMode::Normal);
    assert_approx(hdr_channel(&dst, 0, 0), 0.5, 1e-5);
    assert_approx(hdr_channel(&dst, 0, 2), 0.5, 1e-5);
    assert_approx(hdr_channel(&dst, 0, 3), 1.0, 1e-6);

    // White 50% over black 50% → 2/3 alpha-composited over 0.75 alpha.
    let mut dst = hdr_from_pixels(&[[0.0, 0.0, 0.0, 0.5]]);
    let white = hdr_from_pixels(&[[1.0, 1.0, 1.0, 0.5]]);
    dst.blend_layer_mode(&white, 1.0, BlendMode::Normal);
    assert_approx(hdr_channel(&dst, 0, 0), 2.0 / 3.0, 1e-5);
    assert_approx(hdr_channel(&dst, 0, 3), 0.75, 1e-6);

    // Multiply of white at 50% over opaque black stays black; Screen grays up.
    let mut dst = hdr_from_pixels(&[[0.0, 0.0, 0.0, 1.0]]);
    let white_half = hdr_from_pixels(&[[1.0, 1.0, 1.0, 0.5]]);
    dst.blend_layer_mode(&white_half, 1.0, BlendMode::Multiply);
    assert_approx(hdr_channel(&dst, 0, 0), 0.0, 1e-6);
    let mut dst = hdr_from_pixels(&[[0.0, 0.0, 0.0, 1.0]]);
    let gray_half = hdr_from_pixels(&[[0.5, 0.5, 0.5, 0.5]]);
    dst.blend_layer_mode(&gray_half, 1.0, BlendMode::Screen);
    assert_approx(hdr_channel(&dst, 0, 0), 0.25, 1e-5);
    assert_approx(hdr_channel(&dst, 0, 3), 1.0, 1e-6);

    // Zero opacity leaves the backdrop untouched.
    let mut dst = hdr_from_pixels(&[[0.4, 0.4, 0.4, 1.0]]);
    let before = dst.data.clone();
    let loud = hdr_from_pixels(&[[0.9, 0.1, 0.5, 1.0]]);
    dst.blend_layer_mode(&loud, 0.0, BlendMode::Difference);
    assert_eq!(dst.data, before);

    // blend_over is consistent with blend_layer_mode(Normal).
    let mut dst = hdr_from_pixels(&[[0.0, 0.0, 0.0, 0.5]]);
    dst.blend_over(&white, 1.0);
    assert_approx(hdr_channel(&dst, 0, 0), 2.0 / 3.0, 1e-5);
    assert_approx(hdr_channel(&dst, 0, 3), 0.75, 1e-6);
}

#[test]
fn hdr_blend_modes_match_reference_formulas_on_opaque_backdrop() {
    // On an opaque backdrop the stored color equals the raw blend formula.
    let cases: Vec<(BlendMode, f32)> = vec![
        (BlendMode::Normal, 0.5),
        (BlendMode::Multiply, 0.125),
        (BlendMode::Screen, 0.625),
        (BlendMode::Add, 0.75),
        (BlendMode::Darken, 0.25),
        (BlendMode::Lighten, 0.5),
        (BlendMode::Difference, 0.25),
        (BlendMode::Exclusion, 0.5),
        (BlendMode::Overlay, 0.25),
        (BlendMode::HardLight, 0.25),
        (BlendMode::SoftLight, 0.25),
        (BlendMode::Subtract, 0.0),
        (BlendMode::ColorDodge, 0.5),
        (BlendMode::ColorBurn, -0.5),
    ];
    for (mode, expected) in cases {
        let mut dst = hdr_from_pixels(&[[0.25, 0.25, 0.25, 1.0]]);
        let src = hdr_from_pixels(&[[0.5, 0.5, 0.5, 1.0]]);
        dst.blend_layer_mode(&src, 1.0, mode);
        assert_approx_msg(hdr_channel(&dst, 0, 0), expected, 1e-5, &format!("{mode:?}"));
        assert_approx_msg(hdr_channel(&dst, 0, 3), 1.0, 1e-6, &format!("{mode:?} alpha"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Quantization (dithering + posterize time)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn quantize_none_maps_every_8bit_value_exactly() {
    let mut hdr = Vec::with_capacity(256 * 4);
    for k in 0..=255u16 {
        let v = f32::from(k) / 255.0;
        hdr.extend_from_slice(&[v, v, v, 1.0]);
    }
    let out = quantize_hdr_slice_dithered(&hdr, 256, 1, DitherMethod::None);
    assert_eq!(out.len(), 256 * 4);
    for k in 0..=255usize {
        assert_eq!(out[k * 4], k as u8, "red channel at level {k}");
        assert_eq!(out[k * 4 + 1], k as u8, "green channel at level {k}");
        assert_eq!(out[k * 4 + 2], k as u8, "blue channel at level {k}");
        assert_eq!(out[k * 4 + 3], 255, "alpha at level {k}");
    }
}

#[test]
fn quantize_none_clamps_out_of_range_and_sanitizes_nan() {
    let hdr = vec![
        -5.0, 0.25, 9.0, 1.0, // r clamps 0, g = 64, b clamps 255
        f32::NAN, 0.5, 0.5, 1.0, // NaN channel quantizes to 0 without panicking
        0.5, 0.5, 0.5, 1.0,
    ];
    let out = quantize_hdr_slice_dithered(&hdr, 3, 1, DitherMethod::None);
    assert_eq!(out[0], 0);
    assert_eq!(out[1], 64); // 0.25 * 255 = 63.75 → 64
    assert_eq!(out[2], 255);
    assert_eq!(out[4], 0); // NaN sanitized
    assert_eq!(out[8], 128); // 0.5 * 255 = 127.5 → 128 (half away from zero)
}

#[test]
fn ordered_bayer_midgray_produces_exact_checkerboard() {
    let mut hdr = Vec::with_capacity(16 * 4);
    for _ in 0..16 {
        hdr.extend_from_slice(&[0.5, 0.5, 0.5, 1.0]);
    }
    let out = quantize_hdr_slice_dithered(&hdr, 4, 4, DitherMethod::OrderedBayer);
    let expected = [
        [127, 128, 127, 128],
        [128, 127, 128, 127],
        [127, 128, 127, 128],
        [128, 127, 128, 127],
    ];
    for y in 0..4usize {
        for x in 0..4usize {
            let base = (y * 4 + x) * 4;
            assert_eq!(out[base], expected[y][x], "pixel ({x},{y}) red");
            assert_eq!(out[base + 1], expected[y][x], "pixel ({x},{y}) green");
            assert_eq!(out[base + 2], expected[y][x], "pixel ({x},{y}) blue");
            assert_eq!(out[base + 3], 255, "pixel ({x},{y}) alpha");
        }
    }
}

#[test]
fn dithering_is_deterministic_and_bounded() {
    let mut hdr = Vec::with_capacity(32 * 4);
    for _ in 0..32 {
        hdr.extend_from_slice(&[0.5, 0.5, 0.5, 1.0]);
    }
    let a = quantize_hdr_slice_dithered(&hdr, 32, 1, DitherMethod::TriangularPdf);
    let b = quantize_hdr_slice_dithered(&hdr, 32, 1, DitherMethod::TriangularPdf);
    assert_eq!(a, b, "TPDF dither must be reproducible");

    let none = quantize_hdr_slice_dithered(&hdr, 32, 1, DitherMethod::None);
    // The origin pixel of TPDF dither has a strictly negative offset, so mid-gray
    // must round down (127) rather than tie-round up (128).
    assert_eq!(a[0], 127);
    assert_eq!(none[0], 128);
    // TPDF keeps solid black/white exact at the origin, and nothing escapes bounds.
    let mut bw = Vec::new();
    for _ in 0..32 {
        bw.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
    }
    let bw_out = quantize_hdr_slice_dithered(&bw, 32, 1, DitherMethod::TriangularPdf);
    assert_eq!(bw_out[0], 0);
}

#[test]
fn posterize_time_12fps_golden_grids() {
    let settings = PosterizeTimeSettings {
        target_fps: 12.0,
        enabled: true,
    };
    // 12 fps inside a 30 fps comp: grid lands on frames 0, 3, 5, 8, 10, …
    let expected_30 = [0u32, 0, 0, 3, 3, 5, 5, 5, 8, 8];
    for (f, expected) in expected_30.iter().enumerate() {
        assert_eq!(
            quantize_frame_posterize(f as u32, 30, &settings),
            *expected,
            "frame {f} at 12 fps in a 30 fps comp"
        );
    }
    // 12 fps inside a 60 fps comp: every 5-frame block collapses to its start.
    let expected_60 = [0u32, 0, 0, 0, 0, 5, 5, 5, 5, 5, 10, 10, 10, 10, 10];
    for (f, expected) in expected_60.iter().enumerate() {
        assert_eq!(
            quantize_frame_posterize(f as u32, 60, &settings),
            *expected,
            "frame {f} at 12 fps in a 60 fps comp"
        );
    }
}

#[test]
fn posterize_time_invariants_and_passthrough_modes() {
    let enabled = PosterizeTimeSettings {
        target_fps: 12.0,
        enabled: true,
    };
    for (comp_fps, target) in [(30u32, 12.0f32), (60, 12.0), (25, 24.0), (50, 15.0)] {
        let settings = PosterizeTimeSettings {
            target_fps: target,
            enabled: true,
        };
        let mut previous = 0u32;
        for f in 0..=600u32 {
            let q = quantize_frame_posterize(f, comp_fps, &settings);
            assert!(q <= f, "posterize must never jump past the current frame ({comp_fps}fps @ {target}fps, frame {f} → {q})");
            assert!(q >= previous, "posterize must be monotonic ({comp_fps}fps @ {target}fps, frame {f} → {q})");
            previous = q;
        }
    }
    // Frame boundaries land exactly on themselves at 60 fps / 12 fps target.
    let settings60 = PosterizeTimeSettings {
        target_fps: 12.0,
        enabled: true,
    };
    for f in (0..=120u32).step_by(5) {
        assert_eq!(quantize_frame_posterize(f, 60, &settings60), f);
    }

    // Disabled, degenerate, and above-native target rates are pass-throughs.
    let disabled = PosterizeTimeSettings {
        target_fps: 12.0,
        enabled: false,
    };
    let zero_target = PosterizeTimeSettings {
        target_fps: 0.0,
        enabled: true,
    };
    let negative_target = PosterizeTimeSettings {
        target_fps: -5.0,
        enabled: true,
    };
    let above_native = PosterizeTimeSettings {
        target_fps: 120.0,
        enabled: true,
    };
    for settings in [&disabled, &zero_target, &negative_target, &above_native] {
        for f in [0u32, 1, 5, 37, 99] {
            assert_eq!(quantize_frame_posterize(f, 30, settings), f);
        }
    }
    // A zero-rate comp cannot be posterized; pass through.
    assert_eq!(quantize_frame_posterize(7, 0, &enabled), 7);
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Unified time & tempo math
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn frame_time_roundtrips_exactly_across_rates_and_signs() {
    let rates = [
        FrameRate::new(24, 1).unwrap(),
        FrameRate::new(60, 1).unwrap(),
        FrameRate::new(30000, 1001).unwrap(), // NTSC 29.97
        FrameRate::new(24000, 1001).unwrap(), // NTSC 23.976
        FrameRate::new(25, 1).unwrap(),
    ];
    for rate in rates {
        for f in -120i64..=120 {
            assert_eq!(
                Time::from_frame(f, rate).to_frame_floor(rate),
                f,
                "frame round trip at {rate:?}"
            );
        }
        assert_eq!(Time::from_frame(-10_000, rate).to_frame_floor(rate), -10_000);
        assert_eq!(Time::from_frame(10_000, rate).to_frame_floor(rate), 10_000);
    }
}

#[test]
fn sample_time_roundtrips_at_audio_rates() {
    for sample_rate in [44_100u32, 48_000, 96_000] {
        for n in [0i64, 1, 1234, 44_099, -7, -48_000] {
            assert_eq!(
                Time::from_samples(n, sample_rate).to_sample_floor(sample_rate),
                n,
                "sample round trip at {sample_rate}"
            );
        }
    }
    // One second of 48 kHz audio spans exactly 44 100 samples at 44.1 kHz.
    assert_eq!(
        Time::from_samples(48_000, 48_000).to_sample_floor(44_100),
        44_100
    );
}

#[test]
fn cross_rate_frame_floors_match_rational_math() {
    let ntsc2997 = FrameRate::new(30000, 1001).unwrap();
    let ntsc23976 = FrameRate::new(24000, 1001).unwrap();
    let rate24 = FrameRate::new(24, 1).unwrap();

    // 2 frames at 29.97 fps last 2·1001/30000 s ≈ 0.0667 s → 1 full 24 fps frame.
    assert_eq!(Time::from_frame(2, ntsc2997).to_frame_floor(rate24), 1);
    assert_eq!(Time::from_frame(1, ntsc2997).to_frame_floor(rate24), 0);

    // 10 frames at 23.976 fps ≈ 0.41708 s → exactly 10.01 frames at 24 fps → 10.
    assert_eq!(Time::from_frame(10, ntsc23976).to_frame_floor(rate24), 10);

    // Exact NTSC frame-time representation is 1001/30000 s per frame.
    assert_eq!(Time::from_frame(1, ntsc2997), Time::new(1001, 30_000));
    assert_eq!(Time::from_frame(1, ntsc23976), Time::new(1001, 24_000));
}

#[test]
fn time_new_reduces_fractions_canonically() {
    assert_eq!(Time::new(3, 6), Time::new(1, 2));
    assert_eq!(Time::new(-4, 8), Time::new(-1, 2));
    assert_eq!(Time::new(0, 7), Time::ZERO);
    assert_eq!(Time::new(5, 0), Time::ZERO); // zero denominator fails closed

    let rate = FrameRate::new(60, 2).unwrap();
    assert_eq!(rate, FrameRate::new(30, 1).unwrap());
    assert!(FrameRate::new(0, 30).is_none());
    assert!(FrameRate::new(30, 0).is_none());
}

#[test]
fn tempo_map_segment_math_and_beat_time_inversion() {
    let mut map = TempoMap::new(120.0);
    map.changes.push(TempoChange {
        at: Time::new(1, 1),
        bpm: 60.0,
    });
    map.changes.push(TempoChange {
        at: Time::new(3, 1),
        bpm: 240.0,
    });
    assert!(map.validate().is_ok());

    // Hand-computed beat positions.
    assert_approx(map.beat_at(Time::new(0, 1)) as f32, 0.0, 1e-5);
    assert_approx(map.beat_at(Time::new(1, 1)) as f32, 2.0, 1e-5);
    assert_approx(map.beat_at(Time::new(2, 1)) as f32, 3.0, 1e-5);
    assert_approx(map.beat_at(Time::new(3, 1)) as f32, 4.0, 1e-5);
    assert_approx(map.beat_at(Time::new(4, 1)) as f32, 8.0, 1e-5);
    assert_approx(map.beat_at(Time::new(1, 2)) as f32, 1.0, 1e-5);

    // Inverse mapping lands exactly on the segment grid.
    assert_eq!(map.time_at_beat(2.0), Time::new(1, 1));
    assert_eq!(map.time_at_beat(3.0), Time::new(2, 1));
    assert_eq!(map.time_at_beat(8.0), Time::new(4, 1));
    assert_eq!(map.time_at_beat(5.0), Time::new(13, 4)); // 4 beats @1s, 1 beat @240 → +0.25 s
    assert_eq!(map.time_at_beat(0.0), Time::ZERO);

    // beat_at(time_at_beat(b)) == b and vice versa (within f64 rounding).
    for b in [0.5f64, 1.0, 2.0, 2.25, 2.5, 2.7, 5.0, 8.0, 12.0] {
        let t = map.time_at_beat(b);
        let beat_back = map.beat_at(t);
        assert!(
            (beat_back - b).abs() < 1e-6,
            "beat inversion drifted: {b} → {:?} → {beat_back}",
            t
        );
    }
    for t in [Time::new(1, 4), Time::new(1, 3), Time::new(3, 2), Time::new(7, 2)] {
        let b = map.beat_at(t);
        let time_back = map.time_at_beat(b);
        assert!(
            (seconds_of(time_back) - seconds_of(t)).abs() < 1e-9,
            "time inversion drifted: {t:?} → {b} → {time_back:?}"
        );
    }
}

#[test]
fn tempo_map_time_at_beat_fails_closed_for_degenerate_queries() {
    let map = TempoMap::new(120.0);
    // Non-finite targets fail closed to zero time.
    assert_eq!(map.time_at_beat(f64::NAN), Time::ZERO);
    assert_eq!(map.time_at_beat(f64::INFINITY), Time::ZERO);
    assert_eq!(map.time_at_beat(f64::NEG_INFINITY), Time::ZERO);
    // Finite targets extrapolate linearly below the map start:
    // −1 beat at 120 BPM is −0.5 s.
    assert_eq!(map.time_at_beat(-1.0), Time::new(-1, 2));
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Render pipeline lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Drains `RenderPipeline` results until `predicate` succeeds, then returns
/// everything seen so far (panic on timeout so failures fail loudly).
fn drain_until(
    pipeline: &RenderPipeline,
    predicate: impl Fn(&[RenderResult]) -> bool,
) -> Vec<RenderResult> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = Vec::new();
    loop {
        seen.extend(pipeline.poll_results());
        if predicate(&seen) {
            return seen;
        }
        assert!(Instant::now() < deadline, "timed out waiting for render pipeline results");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn echo_pipeline() -> RenderPipeline {
    RenderPipeline::new(|cmd, tx| match cmd {
        RenderCommand::RenderFrame { frame, version } => {
            let _ = tx.send(RenderResult::FrameReady {
                frame,
                cache_version: version,
            });
        }
        RenderCommand::PrefetchRange { start, end, .. } => {
            let _ = tx.send(RenderResult::BatchReady { start, end });
        }
        RenderCommand::Flush | RenderCommand::Shutdown => {}
    })
}

#[test]
fn render_pipeline_delivers_frame_and_batch_results() {
    let pipeline = echo_pipeline();
    pipeline.request_frame(42, 1);
    pipeline.prefetch_range(3, 7, 1);

    let seen = drain_until(&pipeline, |results| {
        results.iter().any(|r| matches!(r, RenderResult::FrameReady { frame: 42, .. }))
            && results
                .iter()
                .any(|r| matches!(r, RenderResult::BatchReady { start: 3, end: 7 }))
    });
    assert_eq!(
        seen.iter()
            .filter(|r| matches!(r, RenderResult::FrameReady { frame: 42, cache_version: 1 }))
            .count(),
        1
    );
    assert_eq!(
        seen.iter()
            .filter(|r| matches!(r, RenderResult::BatchReady { start: 3, end: 7 }))
            .count(),
        1
    );
    pipeline.shutdown();
}

#[test]
fn render_pipeline_aborts_stale_versions_after_flush() {
    let pipeline = echo_pipeline();
    // Bump the cancellation token to 2 before enqueuing anything.
    pipeline.flush();
    // Version 1 is now stale; version 2 is current. FIFO guarantees the stale
    // command is processed (and dropped) before the current one renders.
    pipeline.request_frame(1, 1);
    pipeline.request_frame(2, 2);

    let seen = drain_until(&pipeline, |results| {
        results.iter().any(|r| matches!(r, RenderResult::FrameReady { frame: 2, cache_version: 2 }))
    });
    assert!(
        !seen.iter().any(|r| matches!(r, RenderResult::FrameReady { frame: 1, .. })),
        "stale frame 1 must never be delivered"
    );
    assert_eq!(
        seen.iter()
            .filter(|r| matches!(r, RenderResult::FrameReady { frame: 2, cache_version: 2 }))
            .count(),
        1,
        "exactly one current frame should be delivered"
    );
    pipeline.shutdown();
}

#[test]
fn render_pipeline_contains_panicking_render_tasks() {
    let pipeline = RenderPipeline::new(|cmd, tx| match cmd {
        RenderCommand::RenderFrame { frame, .. } => {
            if frame == 7 {
                panic!("deliberate panic in render worker");
            }
            let _ = tx.send(RenderResult::FrameReady {
                frame,
                cache_version: 1,
            });
        }
        RenderCommand::PrefetchRange { .. } | RenderCommand::Flush | RenderCommand::Shutdown => {}
    });

    pipeline.request_frame(7, 1); // panics inside the worker
    pipeline.request_frame(9, 1); // must still render afterwards

    let seen = drain_until(&pipeline, |results| {
        results.iter().any(|r| matches!(r, RenderResult::FrameReady { frame: 9, .. }))
    });
    assert!(
        !seen.iter().any(|r| matches!(r, RenderResult::FrameReady { frame: 7, .. })),
        "panicking frame must not produce a result"
    );
    assert_eq!(seen.len(), 1, "worker should deliver only the surviving frame");
    pipeline.shutdown();
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Effect presets (file-backed persistence)
// ─────────────────────────────────────────────────────────────────────────────

fn blur_effect() -> Effect {
    Effect {
        id: "fx-001".into(),
        name: "Gaussian Blur".into(),
        effect_type: EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(3.5),
        },
        enabled: false,
    }
}

fn unique_tmp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kagari_preset_tests_{}_{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create preset test dir");
    dir
}

fn assert_presets_equal(a: &EffectPreset, b: &EffectPreset) {
    assert_eq!(a.name, b.name);
    assert_eq!(a.description, b.description);
    assert_eq!(a.category, b.category);
    assert_eq!(a.created_at, b.created_at);
    assert_eq!(a.effect.id, b.effect.id);
    assert_eq!(a.effect.name, b.effect.name);
    assert_eq!(a.effect.enabled, b.effect.enabled);
    assert_eq!(
        format!("{:?}", a.effect.effect_type),
        format!("{:?}", b.effect.effect_type)
    );
}

#[test]
fn effect_preset_roundtrips_through_json_file() {
    let dir = unique_tmp_dir("roundtrip");
    let path = dir.join("preset.json");

    let preset = EffectPreset::from_effect(&blur_effect(), "Soft Glow".into());
    preset.save_to_file(&path).expect("save preset");

    let loaded = EffectPreset::load_from_file(&path).expect("load preset");
    assert_presets_equal(&preset, &loaded);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn effect_preset_apply_generates_unique_enabled_effects() {
    let preset = EffectPreset::from_effect(&blur_effect(), "Soft Glow".into());
    let mut layer = Layer::new(
        "layer-1".into(),
        "Backdrop".into(),
        LayerType::Solid {
            color: [0.2, 0.4, 0.6, 1.0],
        },
        30,
    );
    assert!(layer.effects.is_empty());

    preset.apply_to_layer(&mut layer);
    preset.apply_to_layer(&mut layer);

    assert_eq!(layer.effects.len(), 2);
    for (i, fx) in layer.effects.iter().enumerate() {
        assert_eq!(fx.id, format!("preset_{i}"), "apply must assign unique ids");
        assert!(fx.enabled, "preset application must force effects on");
        assert_eq!(fx.name, "Gaussian Blur");
        assert_eq!(
            format!("{:?}", fx.effect_type),
            format!("{:?}", preset.effect.effect_type)
        );
    }
}

#[test]
fn discover_presets_filters_broken_files_and_sorts_by_name() {
    let dir = unique_tmp_dir("discover");

    let zulu = EffectPreset::from_effect(&blur_effect(), "Zulu".into());
    zulu.save_to_file(&dir.join("z.json")).expect("save zulu");

    let alpha = EffectPreset::from_effect(&blur_effect(), "Alpha".into());
    alpha.save_to_file(&dir.join("a.aevfx-preset")).expect("save alpha");

    // Broken JSON and non-preset extensions must be ignored.
    std::fs::write(dir.join("broken.json"), "this is not valid json {").expect("write broken");
    std::fs::write(dir.join("notes.txt"), "not a preset").expect("write notes");
    std::fs::create_dir_all(dir.join("sub.json")).expect("dir masquerading as preset");

    let found = effect_presets::discover_presets_in_dir(&dir);
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["Alpha", "Zulu"], "discovery must sort by preset name");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn effect_preset_save_surfaces_io_errors() {
    let dir = unique_tmp_dir("ioerror");
    let preset = EffectPreset::from_effect(&blur_effect(), "Blur".into());
    // Writing "to" an existing directory is an IO error, not a panic.
    let result = preset.save_to_file(&dir);
    assert!(result.is_err(), "save into a directory path must fail");
    let _ = std::fs::remove_dir_all(&dir);
}
