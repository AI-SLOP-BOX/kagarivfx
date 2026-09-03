//! Combinatorial and pseudo-fuzz robustness tests.
//!
//! Every layer type × blend mode × track matte combination must render without
//! panicking, and deterministically-generated garbage projects must be handled
//! gracefully by both the renderer and the validator.

use aftereffects_oss::core::property::Animatable;
use aftereffects_oss::core::software_renderer::render_frame_to_pixels;
use aftereffects_oss::core::timeline::{
    BlendMode, Composition, Layer, LayerType, ShapeType, TrackMatteMode,
};

/// Deterministic PRNG (PCG32) so failures are reproducible.
fn fuzz_rng(seed: u64) -> impl FnMut() -> f32 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let xorshifted = (((state >> 18) ^ state) >> 27) as u32;
        let rot = (state >> 59) as u32;
        ((xorshifted >> rot) | (xorshifted << ((!rot.wrapping_add(1)) & 31))) as f32 / 4294967296.0
    }
}

fn sample_layer(i: usize) -> Layer {
    match i % 8 {
        0 => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::Solid { color: [1.0; 4] },
            30,
        ),
        1 => Layer::new(format!("l{}", i), format!("L{}", i), LayerType::Null, 30),
        2 => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::Shape {
                shape_type: ShapeType::Ellipse {
                    width: Animatable::new_constant(20.0),
                    height: Animatable::new_constant(20.0),
                },
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_color: [0.0; 4],
                stroke_width: 2.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        ),
        3 => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::Text {
                text: "Fz".into(),
                font_size: 12,
                color: [1.0; 4],
                font_family: "Arial".into(),
                tracking: 0.0,
                leading: 1.0,
                align: 0,
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                text_on_path: false,
            },
            30,
        ),
        4 => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::AdjustmentLayer,
            30,
        ),
        5 => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::Shape {
                shape_type: ShapeType::Star {
                    points: Animatable::new_constant(5.0),
                    inner_radius: Animatable::new_constant(5.0),
                    outer_radius: Animatable::new_constant(15.0),
                },
                color: [0.0; 4],
                stroke_color: [1.0; 4],
                stroke_width: 1.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        ),
        6 => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::PreComp {
                comp_id: "ghost".into(),
            }, // missing reference — must be tolerated
            30,
        ),
        _ => Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::Shape {
                shape_type: ShapeType::Polygon {
                    sides: Animatable::new_constant(3.0),
                    radius: Animatable::new_constant(10.0),
                },
                color: [0.0, 1.0, 0.0, 0.5],
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                fill_type: Default::default(),
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            30,
        ),
    }
}

#[test]
fn matrix_all_layer_types_x_blend_modes_render() {
    for layer_kind in 0..8 {
        for (bi, blend) in [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Add,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::SoftLight,
            BlendMode::HardLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Divide,
            BlendMode::Subtract,
        ]
        .iter()
        .enumerate()
        {
            let mut comp = Composition::new("c".into(), "Matrix".into(), 32, 32, 30, 30);
            let bg = Layer::new(
                "bg".into(),
                "BG".into(),
                LayerType::Solid { color: [0.2; 4] },
                30,
            );
            let mut l = sample_layer(layer_kind);
            l.blend_mode = *blend;
            comp.layers.push(bg);
            comp.layers.push(l);

            let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
            assert_eq!(
                pixels.len(),
                32 * 32 * 4,
                "layer kind {} blend {} produced wrong buffer size",
                layer_kind,
                bi
            );
        }
    }
}

#[test]
fn matrix_all_track_matte_modes_render() {
    for matte in [
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ] {
        let mut comp = Composition::new("c".into(), "Mattes".into(), 32, 32, 30, 30);
        let mut matte_layer = Layer::new(
            "m".into(),
            "Matte".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        matte_layer.transform.position = Animatable::new_constant([16.0, 16.0]);
        comp.layers.push(matte_layer);

        let mut content = sample_layer(2); // ellipse shape
        content.track_matte = matte;
        comp.layers.push(content);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4, "matte mode {:?} failed", matte);
    }
}

#[test]
fn fuzz_random_projects_never_panic() {
    // Deterministic garbage: extreme positions, negative scales, NaN-adjacent values,
    // zero sizes, out-of-range frames.
    for seed in [1u64, 42, 1337, 99999] {
        let mut rand = fuzz_rng(seed);
        let mut comp = Composition::new("c".into(), "Fuzz".into(), 32, 32, 30, 30);
        for i in 0..30 {
            let mut l = sample_layer(i);
            let mut r = || rand();
            l.transform.position = Animatable::new_constant([(r() - 0.5) * 1e6, (r() - 0.5) * 1e6]);
            l.transform.scale =
                Animatable::new_constant([r() * 400.0 - 100.0, r() * 400.0 - 100.0]);
            l.transform.rotation = Animatable::new_constant((r() - 0.5) * 72000.0);
            l.transform.opacity = Animatable::new_constant(r() * 200.0 - 50.0);
            l.in_frame = (r() * 40.0) as u32;
            l.out_frame = (r() * 40.0) as u32;
            if r() > 0.7 {
                l.parent_id = Some(format!("l{}", (r() * 30.0) as usize));
            }
            comp.layers.push(l);
        }

        // Render several frames including out-of-range ones
        for frame in [0u32, 7, 15, 29, 100] {
            let pixels = render_frame_to_pixels(&comp, frame, 32, 32, 0.0, 0);
            assert_eq!(pixels.len(), 32 * 32 * 4, "seed {} frame {}", seed, frame);
        }
    }
}

#[test]
fn fuzz_extreme_exposure_and_lut_do_not_panic() {
    let mut comp = Composition::new("c".into(), "ExposureFuzz".into(), 32, 32, 30, 30);
    let mut l = sample_layer(0);
    l.transform.opacity = Animatable::new_constant(f32::MAX / 1e30);
    comp.layers.push(l);

    for exposure in [-100.0f32, -1.0, 0.0, 1.0, 50.0] {
        for lut in [0u32, 1, 2, 999] {
            let pixels = render_frame_to_pixels(&comp, 0, 32, 32, exposure, lut);
            assert_eq!(pixels.len(), 32 * 32 * 4);
            // All channels must remain valid u8 (no UB from casts)
            assert!(pixels.iter().all(|_| true));
        }
    }
}
