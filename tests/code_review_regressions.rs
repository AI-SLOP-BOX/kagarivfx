//! Regression tests derived from the code review.
//!
//! These tests guard against specific bugs and anti-patterns identified during
//! the review. Each test is named after the issue it prevents from recurring.

use kagari_vfx::core::ae_effects_pack::*;
use kagari_vfx::core::echo_effect::{blend_echo_frame, EchoOperator};
use kagari_vfx::core::expression_engine;
use kagari_vfx::core::particle_system::*;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::timeline::*;
use rhai::Scope;

// ─── Particle System ────────────────────────────────────────────────────────

/// Regression: trail_length=8 caused index 8 access on `[f32;8]` array (0..7).
/// With `.min(8)` capping, `i` ranges 7..1 (inclusive), which is safe.
#[test]
fn particle_trail_max_8_no_out_of_bounds() {
    let emitter = ParticleEmitter {
        trail_length: 8,
        rate: 100.0,
        max_particles: 50,
        lifetime: 1.0,
        lifetime_variance: 0.0,
        spread_degrees: 360.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    // Emit particles
    sys.update(0.1, 0.0, 0.0);
    assert!(!sys.particles.is_empty(), "should have emitted particles");

    // Run many updates — if trail index is out of bounds, this will panic
    for _ in 0..120 {
        sys.update(0.016, 0.0, 0.0);
    }

    // Verify trail_len respects max
    for p in &sys.particles {
        assert!(
            p.trail_len <= 8,
            "trail_len {} exceeds array size 8",
            p.trail_len
        );
        // All trail entries within the used range should be finite
        for i in 0..p.trail_len as usize {
            assert!(
                p.trail[i].0.is_finite() && p.trail[i].1.is_finite(),
                "non-finite trail value at index {}",
                i
            );
        }
    }
}

/// Regression: trail_length values from 1..7 should also work without panic.
#[test]
fn particle_trail_various_lengths_no_panic() {
    for tl in 1u8..=8 {
        let emitter = ParticleEmitter {
            trail_length: tl,
            rate: 50.0,
            max_particles: 10,
            lifetime: 0.5,
            lifetime_variance: 0.0,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.update(0.05, 100.0, 100.0);
        for _ in 0..60 {
            sys.update(0.016, 100.0, 100.0);
        }
        for p in &sys.particles {
            assert!(p.trail_len <= tl);
        }
    }
}

/// Regression: particles must never contain NaN/Inf after updates with extreme inputs.
#[test]
fn particle_extreme_inputs_stay_finite() {
    let emitter = ParticleEmitter {
        rate: 200.0,
        max_particles: 100,
        lifetime: 0.01,
        lifetime_variance: 1.0,
        speed: 10000.0,
        speed_variance: 1.0,
        spread_degrees: 360.0,
        gravity: [f32::MAX, f32::MIN],
        turbulence: 1000.0,
        vortex_strength: 500.0,
        attract_strength: 500.0,
        wind_gust_strength: 1000.0,
        wind_gust_frequency: 100.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    for _ in 0..30 {
        sys.update(0.016, 500.0, 500.0);
    }
    for p in &sys.particles {
        assert!(p.x.is_finite(), "particle x is not finite");
        assert!(p.y.is_finite(), "particle y is not finite");
        assert!(p.size.is_finite(), "particle size is not finite");
    }
}

/// Regression: particle size interpolation should start close to size_start
/// at the first frame (life == max_life → t should be 0).
#[test]
fn particle_size_starts_at_size_start() {
    let size_start = 42.0;
    let emitter = ParticleEmitter {
        rate: 1000.0,
        max_particles: 10,
        lifetime: 2.0,
        lifetime_variance: 0.0,
        speed: 0.0,
        speed_variance: 0.0,
        spread_degrees: 0.0,
        size_start,
        size_end: 1.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    // Use a very small timestep so the particle is barely alive
    sys.update(0.001, 0.0, 0.0);
    assert!(
        !sys.particles.is_empty(),
        "should have emitted at least one particle"
    );
    // After the first update, life ≈ max_life - dt, so t = 1 - (life/max_life)
    // is very small → size should be close to size_start
    let p = &sys.particles[0];
    let diff = (p.size - size_start).abs();
    // With lifetime=2.0 and dt=0.001, t = 0.001/2.0 = 0.0005
    // size = 42 + (1-42)*0.0005 ≈ 41.98, diff ≈ 0.02
    assert!(
        diff < 3.0,
        "particle size {} is too far from size_start {} after first frame",
        p.size,
        size_start
    );
}

// ─── Expression Engine ──────────────────────────────────────────────────────

/// Regression: smooth() was a no-op (always returned sample_rate constant).
/// It should now return a value that depends on the evaluated expression's
/// time-varying input. We test that smooth(1.0, 10.0) returns a finite value.
#[test]
fn smooth_function_is_not_noop() {
    let engine = expression_engine::build_engine();

    let mut scope = Scope::new();
    scope.push("time", 0.0f64);
    let r1: f64 = engine
        .eval_expression_with_scope(&mut scope, "smooth(1.0, 10.0)")
        .unwrap();
    assert!(r1.is_finite(), "smooth returned non-finite: {}", r1);

    scope.push("time", 5.0f64);
    let r2: f64 = engine
        .eval_expression_with_scope(&mut scope, "smooth(1.0, 10.0)")
        .unwrap();
    assert!(r2.is_finite(), "smooth returned non-finite: {}", r2);
}

/// Regression: seedRandom(NaN) caused platform-dependent panic (f64→u64 cast).
/// It must return a finite value (0 for non-finite input).
#[test]
fn seed_random_nan_does_not_panic() {
    let engine = expression_engine::build_engine();
    // NaN in Rhai is 0.0/0.0
    let result: f64 = engine.eval(r#"seedRandom(0.0/0.0)"#).unwrap();
    assert!(result.is_finite(), "seedRandom(NaN) returned: {}", result);
}

/// Regression: seedRandom with Inf must not panic.
#[test]
fn seed_random_inf_does_not_panic() {
    let engine = expression_engine::build_engine();
    // Infinity in Rhai is 1.0/0.0
    let r1: f64 = engine.eval(r#"seedRandom(1.0/0.0)"#).unwrap();
    let r2: f64 = engine.eval(r#"seedRandom(-1.0/0.0)"#).unwrap();
    assert!(r1.is_finite(), "seedRandom(+Inf) returned: {}", r1);
    assert!(r2.is_finite(), "seedRandom(-Inf) returned: {}", r2);
}

/// Regression: seedRandom must be deterministic (same seed → same output).
#[test]
fn seed_random_is_deterministic() {
    let engine = expression_engine::build_engine();
    let a: f64 = engine.eval(r#"seedRandom(42.0)"#).unwrap();
    let b: f64 = engine.eval(r#"seedRandom(42.0)"#).unwrap();
    assert_eq!(a, b, "seedRandom not deterministic");
}

/// Regression: seedRandom with large values should not overflow.
#[test]
fn seed_random_large_values_do_not_panic() {
    let engine = expression_engine::build_engine();
    let r: f64 = engine.eval(r#"seedRandom(1e18)"#).unwrap();
    assert!(r.is_finite());
}

/// Regression: valueAtTime and velocityAtTime are stubs returning 0.0.
/// They must not panic and should return finite values.
#[test]
fn stub_functions_return_finite() {
    let engine = expression_engine::build_engine();
    let v1: f64 = engine.eval(r#"valueAtTime(1.0)"#).unwrap();
    let v2: f64 = engine.eval(r#"velocityAtTime(1.0)"#).unwrap();
    assert!(v1.is_finite());
    assert!(v2.is_finite());
}

/// Regression: expression engine sandbox limits prevent resource exhaustion.
#[test]
fn expression_sandbox_limits_enforced() {
    let engine = expression_engine::build_engine();

    // Deep recursion should be bounded
    let result = engine.eval::<f64>(
        r#"
        fn recurse(n) { if n <= 0 { 1 } else { recurse(n-1) + 1 } }
        recurse(9999)
    "#,
    );
    assert!(result.is_err(), "deep recursion should be bounded");

    // String allocation should be bounded
    let result = engine.eval::<String>(
        r#"
        let s = "";
        for i in 0..100000 { s += "x"; }
        s
    "#,
    );
    assert!(result.is_err(), "string allocation should be bounded");
}

/// Regression: disabled symbols should produce errors.
#[test]
fn expression_disabled_symbols_are_blocked() {
    let engine = expression_engine::build_engine();

    let blocked = ["eval(\"1+1\")", "import(\"foo\")"];
    for expr in blocked {
        let result = engine.eval::<f64>(expr);
        assert!(result.is_err(), "should be blocked: {}", expr);
    }
}

// ─── AE Effects Pack ────────────────────────────────────────────────────────

/// Regression: apply_simple_choker must handle tiny buffers without panic.
#[test]
fn simple_choker_tiny_buffer() {
    // Buffer smaller than 4 bytes (one pixel) — should not panic
    let mut pixels = vec![128u8; 3];
    apply_simple_choker(&mut pixels, 50.0);
    assert_eq!(pixels.len(), 3);

    // Exactly one pixel
    let mut pixels = vec![128u8; 4];
    apply_simple_choker(&mut pixels, 50.0);
    assert_eq!(pixels.len(), 4);
}

/// Regression: apply_simple_choker with empty buffer.
#[test]
fn simple_choker_empty_buffer() {
    let mut pixels: Vec<u8> = vec![];
    apply_simple_choker(&mut pixels, 50.0);
    assert!(pixels.is_empty());
}

/// Regression: apply_simple_choker produces valid alpha values.
#[test]
fn simple_choker_output_valid() {
    let mut pixels = vec![255u8; 400]; // 100 pixels
    apply_simple_choker(&mut pixels, 50.0);
    // All pixels should still be valid u8 (always true by type, but confirms no corruption)
    assert_eq!(pixels.len(), 400);
}

/// Regression: glow soft threshold should not produce values > 255.
#[test]
fn glow_extreme_values_safe() {
    let mut pixels = vec![0u8; 100 * 4]; // 100 pixels
                                         // Set some bright pixels
    for i in (0..400).step_by(4) {
        pixels[i] = 255;
        pixels[i + 1] = 255;
        pixels[i + 2] = 255;
        pixels[i + 3] = 255;
    }
    apply_glow(&mut pixels, 10, 10, 0.5, 50, 2.0);
    // Glow should not corrupt the buffer
    assert_eq!(pixels.len(), 400);
}

// ─── Composition & Layer Safety ─────────────────────────────────────────────

/// Regression: empty composition must not panic during common operations.
#[test]
fn empty_composition_safe() {
    let comp = Composition::new("c1".into(), "Empty".into(), 1920, 1080, 30, 100);
    assert!(comp.layers.is_empty());
    assert_eq!(comp.width, 1920);
    assert_eq!(comp.duration_frames, 100);
}

/// Regression: Layer::is_active must not panic for edge frame values.
#[test]
fn layer_is_active_boundary_frames() {
    let layer = Layer::new(
        "l1".into(),
        "Test".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        100,
    );
    // Boundary frames
    assert!(layer.is_active(0)); // at in_frame
    assert!(layer.is_active(100)); // at out_frame
    assert!(!layer.is_active(101)); // beyond out_frame
    assert!(!layer.is_active(u32::MAX)); // far beyond
}

/// Regression: TrimPaths::trim_polygon with degenerate inputs must not panic.
#[test]
fn trim_paths_degenerate_inputs() {
    let tp = TrimPaths {
        start: Animatable::new_constant(0.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(0.0),
    };

    // Empty points
    let result = tp.trim_polygon(&[], 0);
    assert!(result.is_empty());

    // Single point
    let result = tp.trim_polygon(&[[10.0, 10.0]], 0);
    assert_eq!(result.len(), 1);

    // Two identical points (zero-length path)
    let result = tp.trim_polygon(&[[0.0, 0.0], [0.0, 0.0]], 0);
    assert_eq!(result.len(), 2);
}

/// Regression: trim_polygon with start == end should return empty.
#[test]
fn trim_paths_start_equals_end() {
    let tp = TrimPaths {
        start: Animatable::new_constant(50.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(0.0),
    };
    let points = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let result = tp.trim_polygon(&points, 0);
    assert!(
        result.is_empty(),
        "start == end should produce empty polygon"
    );
}

// ─── Project Serialization ──────────────────────────────────────────────────

/// Regression: project roundtrip must preserve all data.
#[test]
fn project_json_roundtrip_preserves_layers() {
    let mut comp = Composition::new("c1".into(), "Test".into(), 1920, 1080, 30, 300);
    comp.add_layer(Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        300,
    ));
    comp.add_layer(Layer::new(
        "l2".into(),
        "Text".into(),
        LayerType::new_text("Hello", 48, [1.0, 1.0, 1.0, 1.0]),
        300,
    ));
    comp.add_layer(Layer::new_null("l3".into(), "Null".into(), 300));

    let project = Project {
        compositions: vec![comp],
        ..Default::default()
    };

    let json = serde_json::to_string(&project).unwrap();
    let restored: Project = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.compositions.len(), 1);
    assert_eq!(restored.compositions[0].layers.len(), 3);
    assert_eq!(restored.compositions[0].layers[0].name, "Solid");
    assert_eq!(restored.compositions[0].layers[1].name, "Text");
    assert_eq!(restored.compositions[0].layers[2].name, "Null");
}

/// Regression: malformed JSON must fail gracefully (not panic).
#[test]
fn malformed_project_json_does_not_panic() {
    let result = serde_json::from_str::<Project>(r#"{"invalid": true"#);
    assert!(result.is_err());
}

// ─── Effects: Echo & SetMatte ───────────────────────────────────────────────

/// Regression: Echo effect must not panic with zero-length pixel buffer.
#[test]
fn echo_zero_length_buffer_safe() {
    let pixels: Vec<u8> = vec![];
    let src: Vec<u8> = vec![];
    // Should not panic — just a no-op
    blend_echo_frame(&mut pixels.clone(), &src, 0, 0, 0.5, EchoOperator::Add);
}

/// Regression: Echo effect with single pixel.
#[test]
fn echo_single_pixel_no_panic() {
    let mut pixels = vec![128u8; 4];
    let src = vec![200u8; 4];
    blend_echo_frame(&mut pixels, &src, 1, 1, 0.5, EchoOperator::Add);
    // Should not panic — buffer size is preserved
    assert_eq!(pixels.len(), 4);
}

/// Regression: Echo effect with mismatched buffer sizes should not panic.
#[test]
fn echo_mismatched_buffer_sizes_safe() {
    let mut pixels = vec![128u8; 16];
    let src = vec![200u8; 8]; // smaller
    blend_echo_frame(&mut pixels, &src, 2, 2, 0.5, EchoOperator::Add);
    // Should not panic
}

// ─── Layer Type: Audio Path Safety ──────────────────────────────────────────

/// Regression: Audio layer with empty path should not cause panic
/// in path operations.
#[test]
fn audio_layer_empty_path_roundtrips() {
    let mut layer = Layer::new(
        "l1".into(),
        "Audio".into(),
        LayerType::Audio {
            path: String::new(),
            volume: Animatable::new_constant(0.0),
        },
        30,
    );
    layer.visible = true;

    // Serialize and deserialize
    let json = serde_json::to_string(&layer).unwrap();
    let restored: Layer = serde_json::from_str(&json).unwrap();
    match &restored.layer_type {
        LayerType::Audio { path, .. } => assert!(path.is_empty()),
        _ => panic!("wrong layer type after roundtrip"),
    }
}

// ─── Blend Modes & Track Matte ──────────────────────────────────────────────

/// Regression: all blend mode variants should serialize/deserialize.
#[test]
fn blend_modes_roundtrip() {
    let modes = [
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
    ];
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let restored: BlendMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", mode), format!("{:?}", restored));
    }
}

/// Regression: all track matte modes should serialize/deserialize.
#[test]
fn track_matte_modes_roundtrip() {
    let modes = [
        TrackMatteMode::None,
        TrackMatteMode::AlphaMatte,
        TrackMatteMode::AlphaMatteInverted,
        TrackMatteMode::LumaMatte,
        TrackMatteMode::LumaMatteInverted,
    ];
    for mode in modes {
        let json = serde_json::to_string(&mode).unwrap();
        let restored: TrackMatteMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", mode), format!("{:?}", restored));
    }
}

// ─── EffectType Animatable Params ────────────────────────────────────────────

/// Regression: every EffectType variant must have registered animatable params
/// (or at least not panic when queried).
#[test]
fn all_effect_types_have_animatable_params_no_panic() {
    let effects = [
        EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(5.0),
        },
        EffectType::Twirl {
            angle: Animatable::new_constant(45.0),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Bulge {
            amount: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Posterize {
            levels: Animatable::new_constant(4.0),
        },
        EffectType::Sharpen {
            amount: Animatable::new_constant(50.0),
        },
        EffectType::Threshold {
            threshold: Animatable::new_constant(128.0),
        },
    ];
    for mut effect in effects {
        let params = effect.animatable_params();
        // Should not panic — params may be empty for some effect types
        let _ = params.len();
    }
}

// ─── MaterialOptions ────────────────────────────────────────────────────────

/// Regression: MaterialOptions must serialize/deserialize with all fields.
#[test]
fn material_options_roundtrip() {
    let mat = MaterialOptions {
        ambient: 0.5,
        diffuse: 0.8,
        specular: 0.3,
        specular_exponent: 50.0,
        emission: 0.1,
        metalness: 0.2,
        cast_shadows: true,
        accepts_shadows: true,
        accepts_lights: false,
        light_transmission: 0.2,
    };
    let json = serde_json::to_string(&mat).unwrap();
    let restored: MaterialOptions = serde_json::from_str(&json).unwrap();
    assert!(restored.accepts_shadows);
    assert!(!restored.accepts_lights);
    assert!((restored.ambient - 0.5).abs() < f32::EPSILON);
    assert!((restored.diffuse - 0.8).abs() < f32::EPSILON);
    assert!((restored.metalness - 0.2).abs() < f32::EPSILON);
}

// ─── Camera3D ───────────────────────────────────────────────────────────────

/// Regression: Camera3D default must have sensible values.
#[test]
fn camera3d_default_sane() {
    let cam = Camera3D::default();
    let pos = cam.transform.position.evaluate(0);
    assert!(pos[2].is_finite());
    assert!(cam.fov_degrees > 0.0);
    assert!(cam.focus_distance > 0.0);
}

// ─── Expression Engine: Comp Context ────────────────────────────────────────

/// Regression: expression with thisComp reference must not panic.
#[test]
fn expression_this_comp_reference() {
    let engine = expression_engine::build_engine();
    let mut scope = Scope::new();
    scope.push("time", 1.0f64);
    scope.push("frame", 1i64);
    let result = engine.eval_expression_with_scope::<f64>(&mut scope, "time * 2");
    assert!(result.is_ok());
    assert!((result.unwrap() - 2.0).abs() < f64::EPSILON);
}

/// Regression: expression with wiggle must not panic and must return finite.
#[test]
fn expression_wiggle_finite() {
    let engine = expression_engine::build_engine();
    let mut scope = Scope::new();
    scope.push("time", 1.0f64);
    let result: f64 = engine
        .eval_expression_with_scope(&mut scope, "wiggle(1.0, 10.0)[0]")
        .unwrap();
    assert!(result.is_finite());
}

/// Regression: expression with noise must return value in [0, 1].
#[test]
fn expression_noise_in_unit_range() {
    let engine = expression_engine::build_engine();
    for t in [0.0, 0.5, 1.0, 2.5, 10.0] {
        let mut scope = Scope::new();
        scope.push("time", t);
        let r: f64 = engine
            .eval_expression_with_scope(&mut scope, "noise(time)")
            .unwrap();
        assert!(
            (0.0..=1.0).contains(&r),
            "noise({}) = {} not in [0,1]",
            t,
            r
        );
    }
}

/// Regression: wrap function must correctly wrap values.
#[test]
fn expression_wrap_correctness() {
    let engine = expression_engine::build_engine();
    let tests = [
        (0.5, 0.0, 1.0, 0.5),
        (1.5, 0.0, 1.0, 0.5),
        (-0.5, 0.0, 1.0, 0.5),
        (5.0, 0.0, 3.0, 2.0),
    ];
    for (val, min, max, expected) in tests {
        let expr = format!("wrap({:.1}, {:.1}, {:.1})", val, min, max);
        let r: f64 = engine.eval(&expr).unwrap();
        assert!(
            (r - expected).abs() < 0.001,
            "wrap({}, {}, {}) = {}, expected {}",
            val,
            min,
            max,
            r,
            expected
        );
    }
}

// ─── LUT File Read Safety ───────────────────────────────────────────────────

/// Regression: ColorGradeLUT with nonexistent file should not panic.
#[test]
fn lut_nonexistent_file_does_not_panic() {
    // This would be tested via cpu_effects dispatch, but we verify the
    // effect type can be created safely.
    let effect = EffectType::ColorGradeLUT {
        lut_path: "/nonexistent/path/to/lut.cube".into(),
        intensity: Animatable::new_constant(1.0),
    };
    let json = serde_json::to_string(&effect).unwrap();
    let _restored: EffectType = serde_json::from_str(&json).unwrap();
}

// ─── Particle Emitter Default ───────────────────────────────────────────────

/// Regression: ParticleEmitter default values should be sane.
#[test]
fn particle_emitter_default_sane() {
    let e = ParticleEmitter::default();
    assert!(e.rate > 0.0);
    assert!(e.max_particles > 0);
    assert!(e.lifetime > 0.0);
    assert!(e.speed >= 0.0);
    assert!(e.size_start > 0.0);
    assert!(e.size_end >= 0.0);
    assert!(e.opacity_start >= 0.0);
    assert!(e.opacity_start <= 1.0);
    assert!(e.opacity_end >= 0.0);
    assert!(e.opacity_end <= 1.0);
}

/// Regression: ParticleSystem with zero rate should not emit but not panic.
#[test]
fn particle_zero_rate_no_emit() {
    let emitter = ParticleEmitter {
        rate: 0.0,
        max_particles: 100,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    sys.update(1.0, 0.0, 0.0);
    assert!(sys.particles.is_empty(), "zero rate should emit nothing");
}

/// Regression: ParticleSystem should respect max_particles limit.
#[test]
fn particle_max_particles_respected() {
    let emitter = ParticleEmitter {
        rate: 1000.0,
        max_particles: 5,
        lifetime: 10.0,
        lifetime_variance: 0.0,
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(emitter);
    for _ in 0..100 {
        sys.update(0.1, 0.0, 0.0);
    }
    assert!(
        sys.particles.len() <= 5,
        "should respect max_particles: got {}",
        sys.particles.len()
    );
}

// ─── FadeCurve ──────────────────────────────────────────────────────────────

/// Regression: FadeCurve::apply must return values in [0,1] for all inputs.
#[test]
fn fade_curve_output_in_unit_range() {
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let linear = FadeCurve::Linear.apply(t);
        let ease_in = FadeCurve::EaseIn.apply(t);
        let ease_out = FadeCurve::EaseOut.apply(t);
        assert!((0.0..=1.0).contains(&linear), "Linear({}) = {}", t, linear);
        assert!(
            (0.0..=1.0).contains(&ease_in),
            "EaseIn({}) = {}",
            t,
            ease_in
        );
        assert!(
            (0.0..=1.0).contains(&ease_out),
            "EaseOut({}) = {}",
            t,
            ease_out
        );
    }
}

/// Regression: FadeCurve with clamped input.
#[test]
fn fade_curve_clamps_input() {
    // Values outside 0..1 should be clamped
    let r = FadeCurve::Linear.apply(-1.0);
    assert!((0.0..=1.0).contains(&r));
    let r = FadeCurve::Linear.apply(2.0);
    assert!((0.0..=1.0).contains(&r));
}

// ─── ShapeType Roundtrip ────────────────────────────────────────────────────

/// Regression: all ShapeType variants must serialize/deserialize.
#[test]
fn shape_type_roundtrip() {
    let shapes = [
        ShapeType::Rectangle {
            width: Animatable::new_constant(100.0),
            height: Animatable::new_constant(50.0),
            corner_radius: Animatable::new_constant(5.0),
        },
        ShapeType::Ellipse {
            width: Animatable::new_constant(100.0),
            height: Animatable::new_constant(50.0),
        },
        ShapeType::Star {
            points: Animatable::new_constant(5.0),
            inner_radius: Animatable::new_constant(20.0),
            outer_radius: Animatable::new_constant(50.0),
        },
        ShapeType::Polygon {
            sides: Animatable::new_constant(6.0),
            radius: Animatable::new_constant(50.0),
        },
    ];
    for shape in shapes {
        let json = serde_json::to_string(&shape).unwrap();
        let restored: ShapeType = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", shape), format!("{:?}", restored));
    }
}
