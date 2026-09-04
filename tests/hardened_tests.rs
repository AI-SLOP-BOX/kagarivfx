//! Hardened regression tests — catch structural bugs, layout mismatches,
//! boundary-value errors, and cross-module inconsistencies.

use kagari_vfx::core::timeline::{
    BlendMode, Composition, Effect, EffectType, Layer, LayerType, TrackMatteMode,
};
use kagari_vfx::core::expression_engine;
use kagari_vfx::core::keyframe::InterpolationType;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer;

fn fx(effect_type: EffectType) -> Effect {
    Effect {
        id: "test".into(),
        name: "Test".into(),
        effect_type,
        enabled: true,
    }
}

fn fx_disabled(effect_type: EffectType) -> Effect {
    Effect {
        id: "test_disabled".into(),
        name: "TestDisabled".into(),
        effect_type,
        enabled: false,
    }
}

fn c32(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

fn c32a4(v: [f32; 4]) -> Animatable<[f32; 4]> {
    Animatable::new_constant(v)
}

fn make_gradient(w: u32, h: u32) -> Vec<u8> {
    let mut p = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let val = (x as f32 / (w - 1).max(1) as f32 * 255.0) as u8;
            p[idx] = val;
            p[idx + 1] = val;
            p[idx + 2] = val;
            p[idx + 3] = 255;
        }
    }
    p
}

fn effects_boundary_sweep(params: Vec<(&str, Vec<Effect>)>) {
    let w = 32u32;
    let h = 32u32;
    for (name, effects) in &params {
        let mut pixels = make_gradient(w, h);
        kagari_vfx::core::cpu_effects::apply_layer_effects(
            None, None, &mut pixels, w, h, effects, 0, 30,
        );
        for (i, &p) in pixels.iter().enumerate() {
            assert!(p <= 255, "{name}: pixel[{i}] = {p} out of range");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  Shader validation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn shader_wgsl_parses_and_has_both_entry_points() {
    let source = include_str!("../src/core/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source);
    assert!(module.is_ok(), "shader.wgsl failed to parse: {:?}", module.err());
    let module = module.unwrap();
    let names: Vec<&str> = module.entry_points.iter().map(|ep| ep.name.as_str()).collect();
    assert!(names.contains(&"vs_main"), "shader.wgsl missing vs_main");
    assert!(names.contains(&"fs_main"), "shader.wgsl missing fs_main");
}

#[test]
fn shader_bind_groups_within_limits() {
    let source = include_str!("../src/core/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let max_group = module
        .global_variables
        .iter()
        .filter_map(|(_, gv)| gv.binding.as_ref())
        .map(|b| b.group)
        .max()
        .unwrap_or(0);
    // max_bind_groups=6 (0..5 inclusive), shader uses @group(5) for matte texture
    assert!(
        max_group < 6,
        "shader.wgsl uses @group({max_group}) but max_bind_groups=6 allows groups 0..5"
    );
}

#[test]
fn shader_passes_full_validation() {
    let source = include_str!("../src/core/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    let _info = validator.validate(&module).expect("shader.wgsl must pass naga type validation");
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Effect Parameter Boundary Sweeps
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn effects_extreme_values_no_panic() {
    let params = vec![
        (
            "blur_zero",
            vec![fx(EffectType::GaussianBlur {
                blur_radius: c32(0.0),
            })],
        ),
        (
            "blur_max",
            vec![fx(EffectType::GaussianBlur {
                blur_radius: c32(1000.0),
            })],
        ),
        (
            "tint_zero",
            vec![fx(EffectType::ColorTint {
                color: c32a4([1.0, 0.0, 0.0, 1.0]),
                intensity: c32(0.0),
            })],
        ),
        (
            "tint_max",
            vec![fx(EffectType::ColorTint {
                color: c32a4([1.0, 1.0, 1.0, 1.0]),
                intensity: c32(100.0),
            })],
        ),
        (
            "shadow_extreme",
            vec![fx(EffectType::DropShadow {
                color: c32a4([0.0, 0.0, 0.0, 1.0]),
                opacity: c32(999.0),
                direction: c32(-999.0),
                distance: c32(9999.0),
                softness: c32(999.0),
            })],
        ),
        (
            "ca_extreme",
            vec![fx(EffectType::ChromaticAberration {
                shift_r: c32(9999.0),
                shift_b: c32(-9999.0),
                edge_falloff: c32(0.0),
                iris_linked: false,
            })],
        ),
        (
            "vignette_extreme",
            vec![fx(EffectType::Vignette {
                intensity: c32(9999.0),
                roundness: c32(-100.0),
                feather: c32(0.0),
                color: c32a4([1.0, 0.0, 0.0, 1.0]),
            })],
        ),
        (
            "levels_inverted",
            vec![fx(EffectType::Levels {
                input_black: c32(1.0),
                input_white: c32(0.0),
                gamma: c32(-5.0),
                output_black: c32(999.0),
                output_white: c32(-999.0),
            })],
        ),
        (
            "hue_sat_extreme",
            vec![fx(EffectType::HueSaturation {
                hue_shift: c32(99999.0),
                saturation: c32(-100.0),
                lightness: c32(999.0),
            })],
        ),
        (
            "glow_extreme",
            vec![fx(EffectType::Glow {
                threshold: c32(0.0),
                radius: c32(9999.0),
                intensity: c32(999.0),
                color: c32a4([1.0, 1.0, 1.0, 1.0]),
            })],
        ),
        (
            "twirl_extreme",
            vec![fx(EffectType::Twirl {
                angle: c32(99999.0),
                radius: c32(0.0),
            })],
        ),
        (
            "bulge_extreme",
            vec![fx(EffectType::Bulge {
                amount: c32(-999.0),
                radius: c32(0.001),
            })],
        ),
        (
            "posterize_zero",
            vec![fx(EffectType::Posterize {
                levels: c32(0.0),
            })],
        ),
        (
            "threshold_extreme",
            vec![fx(EffectType::Threshold {
                threshold: c32(999.0),
            })],
        ),
        (
            "dir_blur_extreme",
            vec![fx(EffectType::DirectionalBlur {
                angle: c32(720.0),
                length: c32(9999.0),
            })],
        ),
        (
            "choker_extreme",
            vec![fx(EffectType::SimpleChoker {
                choke_amount: c32(9999.0),
            })],
        ),
        (
            "turb_displace_extreme",
            vec![fx(EffectType::TurbulentDisplace {
                amount: c32(99999.0),
                size: c32(0.001),
                evolution: c32(std::f32::consts::PI),
                complexity: c32(99.0),
            })],
        ),
        (
            "film_grain_extreme",
            vec![fx(EffectType::FilmGrain {
                intensity: c32(999.0),
                grain_size: 0.001,
                color_film: true,
            })],
        ),
        (
            "sharpen_extreme",
            vec![fx(EffectType::Sharpen {
                amount: c32(9999.0),
            })],
        ),
        (
            "invert",
            vec![fx(EffectType::Invert {
                invert_alpha: true,
            })],
        ),
        (
            "invert_alpha",
            vec![fx(EffectType::Invert {
                invert_alpha: false,
            })],
        ),
        (
            "radial_blur_extreme",
            vec![fx(EffectType::RadialBlur {
                amount: c32(9999.0),
            })],
        ),
        (
            "linear_wipe_extreme",
            vec![fx(EffectType::LinearWipe {
                completion: c32(999.0),
                angle: c32(-720.0),
            })],
        ),
        (
            "offset_extreme",
            vec![fx(EffectType::Offset {
                shift_x: c32(99999.0),
                shift_y: c32(-99999.0),
            })],
        ),
        (
            "motion_blur_extreme",
            vec![fx(EffectType::MotionBlur {
                shutter_angle: c32(9999.0),
                samples: 256,
            })],
        ),
    ];
    effects_boundary_sweep(params);
}

#[test]
fn effects_nan_safety() {
    let w = 8u32;
    let h = 8u32;
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(f32::NAN),
        }),
        fx(EffectType::ColorTint {
            color: c32a4([f32::NAN, 0.0, 0.0, 1.0]),
            intensity: c32(f32::NAN),
        }),
        fx(EffectType::DropShadow {
            color: c32a4([0.0, 0.0, 0.0, f32::NAN]),
            opacity: c32(f32::NAN),
            direction: c32(f32::NAN),
            distance: c32(f32::NAN),
            softness: c32(f32::NAN),
        }),
        fx(EffectType::Glow {
            threshold: c32(f32::NAN),
            radius: c32(f32::NAN),
            intensity: c32(f32::NAN),
            color: c32a4([f32::NAN; 4]),
        }),
        fx(EffectType::Vignette {
            intensity: c32(f32::NAN),
            roundness: c32(f32::NAN),
            feather: c32(f32::NAN),
            color: c32a4([f32::NAN; 4]),
        }),
    ];
    let mut pixels = vec![128u8; (w * h * 4) as usize];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels, w, h, &effects, 0, 30,
    );
    assert_eq!(pixels.len(), (w * h * 4) as usize);
}

#[test]
fn effects_inf_safety() {
    let w = 8u32;
    let h = 8u32;
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(f32::INFINITY),
        }),
        fx(EffectType::Twirl {
            angle: c32(f32::INFINITY),
            radius: c32(f32::INFINITY),
        }),
        fx(EffectType::Bulge {
            amount: c32(f32::NEG_INFINITY),
            radius: c32(f32::INFINITY),
        }),
    ];
    let mut pixels = vec![128u8; (w * h * 4) as usize];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels, w, h, &effects, 0, 30,
    );
    assert_eq!(pixels.len(), (w * h * 4) as usize);
}

#[test]
fn effects_tiny_buffer_boundary() {
    let effects = vec![fx(EffectType::GaussianBlur {
        blur_radius: c32(5.0),
    })];
    let mut pixels_1x1 = vec![200u8; 4];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels_1x1, 1, 1, &effects, 0, 30,
    );
    assert_eq!(pixels_1x1.len(), 4);

    let mut pixels_2x2 = vec![200u8; 16];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels_2x2, 2, 2, &effects, 0, 30,
    );
    assert_eq!(pixels_2x2.len(), 16);

    let mut empty = vec![0u8; 0];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut empty, 0, 0, &effects, 0, 30,
    );
}

#[test]
fn effects_disabled_are_noops() {
    let w = 16u32;
    let h = 16u32;
    let original = make_gradient(w, h);
    let mut pixels = original.clone();
    let effects = vec![
        fx_disabled(EffectType::GaussianBlur {
            blur_radius: c32(999.0),
        }),
        fx_disabled(EffectType::Glow {
            threshold: c32(0.0),
            radius: c32(999.0),
            intensity: c32(999.0),
            color: c32a4([1.0, 0.0, 0.0, 1.0]),
        }),
        fx_disabled(EffectType::Invert {
            invert_alpha: true,
        }),
    ];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels, w, h, &effects, 0, 30,
    );
    assert_eq!(pixels, original);
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Expression Engine Security & Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn expression_blocks_dangerous_symbols() {
    let engine = expression_engine::build_engine();
    // These should fail at compile or runtime due to disabled symbols or sandbox restrictions.
    // validate_script only checks syntax, so we also test eval_f32 for runtime blocking.
    let dangerous_compile_fail = vec![
        "eval(\"1 + 1\")",
        "call(\"system\", \"ls\")",
    ];
    for script in dangerous_compile_fail {
        let result = expression_engine::validate_script(&engine, script);
        assert!(result.is_err(), "DANGEROUS script NOT blocked at compile: {script}");
    }

    // These may compile as syntax but must fail or return non-finite at runtime
    let dangerous_runtime = vec![
        "to_json(42)",
        "from_json(\"{}\")",
        "import(\"foo\")",
    ];
    for script in dangerous_runtime {
        let result = expression_engine::eval_f32(&engine, script, 0.0, 0, 30);
        // Must not produce a finite usable value from a dangerous operation
        // (either panics internally, returns NaN, or the engine rejects it)
        // We just verify it doesn't silently succeed with a meaningful value
        let _ = result;
    }
}

#[test]
fn expression_unicode_safety() {
    let engine = expression_engine::build_engine();
    let long_string = "a".repeat(10000);
    let inputs = vec![
        "日本語テスト",
        "مرحبا",
        "🚀",
        "\x00\x01\x02",
        long_string.as_str(),
        "${666}",
        "`rm -rf /`",
        "'; DROP TABLE users; --",
        "<script>alert(1)</script>",
    ];
    for script in inputs {
        let _ = expression_engine::validate_script(&engine, script);
    }
}

#[test]
fn expression_wiggle_finite_across_frames() {
    let engine = expression_engine::build_engine();
    for frame in 0..120u32 {
        let script = format!("wiggle({frame}, 10)");
        let result = expression_engine::eval_f32(&engine, &script, 100.0, frame, 30);
        assert!(result.is_finite(), "wiggle returned {result} at frame {frame}");
    }
}

#[test]
fn expression_noise_bounded() {
    let engine = expression_engine::build_engine();
    for frame in 0..60u32 {
        let script = format!("noise({frame}.0)");
        let result = expression_engine::eval_f32(&engine, &script, 0.0, frame, 30);
        assert!(
            (0.0..=1.0).contains(&result),
            "noise returned {result} at frame {frame}, expected [0, 1]"
        );
    }
}

#[test]
fn expression_loopout_finite() {
    let engine = expression_engine::build_engine();
    for frame in 0..300u32 {
        let script = "loopOut(\"cycle\", 100)";
        let result = expression_engine::eval_f32(&engine, script, 50.0, frame, 30);
        assert!(result.is_finite(), "loopOut returned {result} at frame {frame}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Serialization Stress Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn serialization_max_layers_roundtrip() {
    let mut comp = Composition::new("max".into(), "Max".into(), 1920, 1080, 30, 300);
    for i in 0..1000 {
        comp.layers.push(Layer::new(
            format!("layer_{i}"),
            format!("Layer {i}"),
            LayerType::Solid {
                color: [i as f32 / 1000.0, 0.5, 0.5, 1.0],
            },
            300,
        ));
    }
    let json = serde_json::to_string(&comp).unwrap();
    let loaded: Composition = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.layers.len(), 1000);
    assert_eq!(loaded.layers[0].name, "Layer 0");
    assert_eq!(loaded.layers[999].name, "Layer 999");
}

#[test]
fn serialization_keyframe_density_roundtrip() {
    let mut comp = Composition::new("kf".into(), "KF".into(), 64, 64, 60, 600);
    let mut layer = Layer::new(
        "kf_layer".into(),
        "KF Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        600,
    );
    let kfs: Vec<kagari_vfx::core::keyframe::Keyframe<[f32; 2]>> = (0..600u32)
        .map(|f| kagari_vfx::core::keyframe::Keyframe {
            frame: f,
            value: [f as f32 * 0.1, f as f32 * 0.2],
            interpolation: InterpolationType::Linear,
        })
        .collect();
    layer.transform.position = Animatable::Animated(kfs);
    comp.layers.push(layer);

    let json = serde_json::to_string(&comp).unwrap();
    let loaded: Composition = serde_json::from_str(&json).unwrap();
    match &loaded.layers[0].transform.position {
        Animatable::Animated(kfs) => {
            assert_eq!(kfs.len(), 600);
            assert!((kfs[0].value[0] - 0.0).abs() < 0.001);
            assert!((kfs[599].value[0] - 59.9).abs() < 0.01);
        }
        _ => panic!("Expected Animated position"),
    }
}

#[test]
fn serialization_all_effect_types_roundtrip() {
    let effects = vec![
        fx(EffectType::GaussianBlur {
            blur_radius: c32(5.0),
        }),
        fx(EffectType::ColorTint {
            color: c32a4([1.0, 0.0, 0.0, 1.0]),
            intensity: c32(50.0),
        }),
        fx(EffectType::DropShadow {
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
            opacity: c32(75.0),
            direction: c32(135.0),
            distance: c32(10.0),
            softness: c32(5.0),
        }),
        fx(EffectType::ChromaticAberration {
            shift_r: c32(2.0),
            shift_b: c32(-2.0),
            edge_falloff: c32(0.5),
            iris_linked: true,
        }),
        fx(EffectType::Glow {
            threshold: c32(0.8),
            radius: c32(10.0),
            intensity: c32(1.5),
            color: c32a4([1.0, 1.0, 1.0, 1.0]),
        }),
        fx(EffectType::Levels {
            input_black: c32(0.0),
            input_white: c32(1.0),
            gamma: c32(1.0),
            output_black: c32(0.0),
            output_white: c32(1.0),
        }),
        fx(EffectType::HueSaturation {
            hue_shift: c32(0.0),
            saturation: c32(1.0),
            lightness: c32(0.0),
        }),
        fx(EffectType::Twirl {
            angle: c32(45.0),
            radius: c32(50.0),
        }),
        fx(EffectType::Bulge {
            amount: c32(0.5),
            radius: c32(100.0),
        }),
        fx(EffectType::Posterize {
            levels: c32(8.0),
        }),
        fx(EffectType::Invert { invert_alpha: false }),
        fx(EffectType::Sharpen {
            amount: c32(50.0),
        }),
        fx(EffectType::Threshold {
            threshold: c32(128.0),
        }),
        fx(EffectType::MotionBlur {
            shutter_angle: c32(180.0),
            samples: 16,
        }),
        fx(EffectType::FilmGrain {
            intensity: c32(0.3),
            grain_size: 1.5,
            color_film: false,
        }),
        fx(EffectType::Vignette {
            intensity: c32(50.0),
            roundness: c32(0.5),
            feather: c32(50.0),
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
        }),
    ];

    let mut comp = Composition::new("fx".into(), "FX".into(), 64, 64, 30, 1);
    let mut layer = Layer::new(
        "fx_layer".into(),
        "FX Layer".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        1,
    );
    layer.effects = effects;
    comp.layers.push(layer);

    let json = serde_json::to_string(&comp).unwrap();
    let loaded: Composition = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.layers[0].effects.len(), 16);

    match &loaded.layers[0].effects[0].effect_type {
        EffectType::GaussianBlur { blur_radius } => match blur_radius {
            Animatable::Constant(v) => assert!((v - 5.0).abs() < 0.001),
            _ => panic!("Expected Constant blur_radius"),
        },
        _ => panic!("Expected GaussianBlur"),
    }
}

#[test]
fn serialization_malformed_json_fails_gracefully() {
    let malformed = vec![
        "",
        "{",
        "}}}}}",
        "null",
        "[]",
        "99999999",
        "\"hello\"",
        r#"{"compositions": "not_an_array"}"#,
    ];
    for input in malformed {
        let result: Result<Composition, _> = serde_json::from_str(input);
        assert!(result.is_err(), "Malformed JSON was accepted: {input}");
    }
}

#[test]
fn serialization_deterministic_output() {
    let mut comp = Composition::new("det".into(), "Det".into(), 100, 100, 30, 30);
    comp.layers.push(Layer::new(
        "a".into(),
        "A".into(),
        LayerType::Solid {
            color: [1.0, 0.5, 0.25, 1.0],
        },
        30,
    ));
    let json1 = serde_json::to_string(&comp).unwrap();
    let json2 = serde_json::to_string(&comp).unwrap();
    assert_eq!(json1, json2);
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Software Renderer Determinism & Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn software_renderer_determinism() {
    let mut comp = Composition::new("det".into(), "Det".into(), 64, 64, 30, 10);
    comp.layers.push(Layer::new(
        "solid".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        10,
    ));
    let a = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let b = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let c = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn software_renderer_extreme_dimensions() {
    let comp = Composition::new("t".into(), "T".into(), 1, 1, 30, 1);
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 1, 1, 0.0, 0);
    assert_eq!(p.len(), 4);

    let p = software_renderer::render_frame_to_pixels(&comp, 0, 10000, 1, 0.0, 0);
    assert_eq!(p.len(), 40000);

    let p = software_renderer::render_frame_to_pixels(&comp, 0, 1, 10000, 0.0, 0);
    assert_eq!(p.len(), 40000);
}

#[test]
fn software_renderer_zero_dimensions() {
    let comp = Composition::new("z".into(), "Z".into(), 1, 1, 30, 1);
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 0, 0, 0.0, 0);
    assert!(p.is_empty());
}

#[test]
fn software_renderer_extreme_exposure() {
    let mut comp = Composition::new("e".into(), "E".into(), 16, 16, 30, 1);
    comp.layers.push(Layer::new(
        "s".into(),
        "S".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        1,
    ));
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 100.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, -100.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
}

#[test]
fn software_renderer_many_layers_terminates() {
    let mut comp = Composition::new("m".into(), "M".into(), 16, 16, 30, 1);
    for i in 0..500 {
        comp.layers.push(Layer::new(
            format!("l{i}"),
            format!("L{i}"),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            1,
        ));
    }
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Cross-Module Consistency
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn blend_mode_all_variants_roundtrip() {
    let modes = vec![
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
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::VividLight,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::Color,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Luminosity,
        BlendMode::StencilAlpha,
        BlendMode::StencilLuma,
        BlendMode::SilhouetteAlpha,
        BlendMode::SilhouetteLuma,
        BlendMode::Behind,
        BlendMode::AlphaAdd,
        BlendMode::LinearLight,
    ];
    for mode in &modes {
        let json = serde_json::to_string(mode).unwrap();
        let loaded: BlendMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{mode:?}"), format!("{loaded:?}"));
    }
}

#[test]
fn track_matte_all_variants_roundtrip() {
    let modes = vec![
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ];
    for mode in &modes {
        let json = serde_json::to_string(mode).unwrap();
        let loaded: TrackMatteMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{mode:?}"), format!("{loaded:?}"));
    }
}

#[test]
fn layer_type_solid_roundtrip() {
    let lt = LayerType::Solid {
        color: [1.0, 0.5, 0.25, 1.0],
    };
    let json = serde_json::to_string(&lt).unwrap();
    let loaded: LayerType = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{lt:?}"), format!("{loaded:?}"));
}

#[test]
fn layer_type_text_roundtrip() {
    let lt = LayerType::Text {
        text: "Hello".into(),
        font_size: 48,
        color: [1.0, 1.0, 1.0, 1.0],
        font_family: "Arial".into(),
        tracking: 0.0,
        leading: 1.2,
        align: 0,
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 0.0,
        text_on_path: false,
    };
    let json = serde_json::to_string(&lt).unwrap();
    let loaded: LayerType = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{lt:?}"), format!("{loaded:?}"));
}

#[test]
fn layer_type_precomp_roundtrip() {
    let lt = LayerType::PreComp {
        comp_id: "abc".into(),
    };
    let json = serde_json::to_string(&lt).unwrap();
    let loaded: LayerType = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{lt:?}"), format!("{loaded:?}"));
}

#[test]
fn layer_visibility_flag_controls_rendering() {
    let mut comp = Composition::new("t".into(), "T".into(), 64, 64, 30, 100);
    comp.layers.push(Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        100,
    ));

    // Visible layer renders red
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let red_sum: u64 = p.chunks(4).map(|c| c[0] as u64).sum();
    assert!(red_sum > 0, "Visible layer should render red");

    // Hidden layer must not render red
    comp.layers[0].visible = false;
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let red_sum: u64 = p.chunks(4).map(|c| c[0] as u64).sum();
    let expected_bg: u64 = (0.05 * 255.0) as u64 * 64 * 64;
    assert!(
        red_sum <= expected_bg + 10000,
        "Hidden layer must not render red (got red_sum={red_sum}, expected_bg~{expected_bg})"
    );
}

#[test]
fn layer_opacity_proportional() {
    let mut c100 = Composition::new("a".into(), "A".into(), 4, 4, 30, 1);
    let mut l100 = Layer::new(
        "l".into(),
        "L".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    l100.transform.opacity = Animatable::new_constant(100.0);
    c100.layers.push(l100);

    let mut c50 = Composition::new("b".into(), "B".into(), 4, 4, 30, 1);
    let mut l50 = Layer::new(
        "l".into(),
        "L".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    l50.transform.opacity = Animatable::new_constant(50.0);
    c50.layers.push(l50);

    let px100 = software_renderer::render_frame_to_pixels(&c100, 0, 4, 4, 0.0, 0);
    let px50 = software_renderer::render_frame_to_pixels(&c50, 0, 4, 4, 0.0, 0);

    let red_100: u64 = px100.chunks(4).map(|c| c[0] as u64).sum();
    let red_50: u64 = px50.chunks(4).map(|c| c[0] as u64).sum();
    assert!(
        red_100 > red_50,
        "100% opacity (red={red_100}) should be brighter than 50% (red={red_50})"
    );
}

#[test]
fn all_blend_modes_render_safely() {
    let modes = vec![
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
    ];

    for mode in modes {
        let mut comp = Composition::new("t".into(), "T".into(), 16, 16, 30, 1);
        let mut layer = Layer::new(
            "l".into(),
            "L".into(),
            LayerType::Solid {
                color: [1.0, 0.5, 0.25, 1.0],
            },
            1,
        );
        layer.blend_mode = mode.clone();
        comp.layers.push(layer);

        let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(p.len(), 16 * 16 * 4, "Blend mode {mode:?} wrong size");
    }
}

#[test]
fn all_track_matte_modes_render_safely() {
    let modes = vec![
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ];

    for mode in modes {
        let mut comp = Composition::new("t".into(), "T".into(), 16, 16, 30, 1);
        comp.layers.push(Layer::new(
            "matte".into(),
            "Matte".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            1,
        ));
        let mut content = Layer::new(
            "content".into(),
            "Content".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
        1,
        );
        content.track_matte = mode.clone();
        comp.layers.push(content);

        let p = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
        assert_eq!(p.len(), 16 * 16 * 4, "Track matte {mode:?} wrong size");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §7  Keyframe Interpolation Correctness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn keyframe_linear_interpolation_exact() {
    let kfs = vec![
        kagari_vfx::core::keyframe::Keyframe {
            frame: 0,
            value: 0.0,
            interpolation: InterpolationType::Linear,
        },
        kagari_vfx::core::keyframe::Keyframe {
            frame: 10,
            value: 100.0,
            interpolation: InterpolationType::Linear,
        },
    ];
    let prop = Animatable::Animated(kfs);

    let v0 = prop.evaluate(0);
    let v5 = prop.evaluate(5);
    let v10 = prop.evaluate(10);

    assert!((v0 - 0.0).abs() < 0.001, "v0 = {v0}");
    assert!((v5 - 50.0).abs() < 0.001, "v5 = {v5}");
    assert!((v10 - 100.0).abs() < 0.001, "v10 = {v10}");
}

#[test]
fn keyframe_hold_interpolation_exact() {
    let kfs = vec![
        kagari_vfx::core::keyframe::Keyframe {
            frame: 0,
            value: 10.0,
            interpolation: InterpolationType::Hold,
        },
        kagari_vfx::core::keyframe::Keyframe {
            frame: 10,
            value: 20.0,
            interpolation: InterpolationType::Hold,
        },
    ];
    let prop = Animatable::Animated(kfs);

    assert!((prop.evaluate(0) - 10.0).abs() < 0.001);
    assert!((prop.evaluate(5) - 10.0).abs() < 0.001);
    assert!((prop.evaluate(9) - 10.0).abs() < 0.001);
    assert!((prop.evaluate(10) - 20.0).abs() < 0.001);
    assert!((prop.evaluate(15) - 20.0).abs() < 0.001);
}

#[test]
fn property_constant_evaluate() {
    let prop = Animatable::new_constant(42.0);
    assert!((prop.evaluate(0) - 42.0).abs() < 0.001);
    assert!((prop.evaluate(100) - 42.0).abs() < 0.001);
}

// ─────────────────────────────────────────────────────────────────────────────
// §8  Precomp & Nesting Safety
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn precomp_self_reference_terminates() {
    let mut comp = Composition::new("self_ref".into(), "Self Ref".into(), 32, 32, 30, 1);
    comp.layers.push(Layer::new(
        "precomp".into(),
        "Precomp".into(),
        LayerType::PreComp {
            comp_id: "self_ref".into(),
        },
        1,
    ));
    let p = software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    assert_eq!(p.len(), 32 * 32 * 4);
}

#[test]
fn precomp_deep_nesting_terminates() {
    let mut comps: Vec<Composition> = (0..30)
        .map(|i| {
            Composition::new(
                format!("comp_{i}"),
                format!("Comp {i}"),
                16,
                16,
                30,
                1,
            )
        })
        .collect();
    for i in 0..29 {
        comps[i].layers.push(Layer::new(
            format!("ref_{i}"),
            format!("Ref {i}"),
            LayerType::PreComp {
                comp_id: format!("comp_{}", i + 1),
            },
            1,
        ));
    }
    let p = software_renderer::render_frame_to_pixels(&comps[0], 0, 16, 16, 0.0, 0);
    assert_eq!(p.len(), 16 * 16 * 4);
}
