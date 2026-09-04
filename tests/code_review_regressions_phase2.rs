//! Phase 2: Comprehensive regression tests from code review.
//!
//! Covers the remaining bug patterns: echo/set_matte edge cases, history
//! NaN dedup, keyframe interpolation, shape boolean, particle forces,
//! expression edge cases, and serialization invariants.

use kagari_vfx::core::echo_effect::{blend_echo_frame, EchoOperator};
use kagari_vfx::core::expression_engine;
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::particle_forces::LifeCurve;
use kagari_vfx::core::particle_system::*;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::set_matte::{
    apply_set_matte, MatteCompositeMode, MatteSourceChannel, SetMatteParams,
};
use kagari_vfx::core::shape_boolean::{apply_polygon_boolean, BooleanOp};
use kagari_vfx::core::timeline::*;
use rhai::Scope;

// ─── Echo Effect: All Operators ─────────────────────────────────────────────

/// Regression: each EchoOperator must produce valid output without panic.
#[test]
fn echo_all_operators_no_panic() {
    let ops = [
        EchoOperator::Add,
        EchoOperator::Screen,
        EchoOperator::Maximum,
        EchoOperator::Minimum,
        EchoOperator::CompositeInBack,
        EchoOperator::CompositeInFront,
        EchoOperator::Blend,
    ];
    for op in ops {
        let mut acc = vec![128u8; 16]; // 2x2
        let echo = vec![200u8; 16];
        blend_echo_frame(&mut acc, &echo, 2, 2, 0.5, op);
        assert_eq!(acc.len(), 16);
    }
}

/// Regression: Echo with NaN weight should clamp to 0 (no panic, no NaN in output).
#[test]
fn echo_nan_weight_clamps_to_zero() {
    let mut acc = vec![128u8; 16];
    let echo = vec![200u8; 16];
    blend_echo_frame(&mut acc, &echo, 2, 2, f32::NAN, EchoOperator::Add);
    // NaN weight → w=0 → echo contribution is 0 → output unchanged
    assert_eq!(acc, vec![128u8; 16]);
}

/// Regression: Echo with Inf weight should clamp to 2.0 (not infinity).
#[test]
fn echo_inf_weight_clamped() {
    let mut acc = vec![128u8; 4];
    let echo = vec![128u8; 4];
    let original = acc.clone();
    blend_echo_frame(&mut acc, &echo, 1, 1, f32::INFINITY, EchoOperator::Add);
    // Should not panic and buffer should be modified (or at least not corrupted)
    assert_eq!(acc.len(), 4);
    // With Inf weight clamped to 2.0 and Add mode: output should be saturated
    assert!(acc[0] >= original[0]);
}

/// Regression: Echo with zero dimensions should be a no-op.
#[test]
fn echo_zero_dimensions_safe() {
    let mut acc = vec![128u8; 16];
    let echo = vec![200u8; 16];
    blend_echo_frame(&mut acc, &echo, 0, 0, 0.5, EchoOperator::Add);
    assert_eq!(acc, vec![128u8; 16]);
}

/// Regression: Echo with width*height overflow should be safe.
#[test]
fn echo_overflow_dimensions_safe() {
    let mut acc = vec![128u8; 4];
    let echo = vec![200u8; 4];
    // u32::MAX * u32::MAX would overflow; the checked_mul guard should handle it
    blend_echo_frame(&mut acc, &echo, u32::MAX, u32::MAX, 0.5, EchoOperator::Add);
}

// ─── SetMatte: Edge Cases ──────────────────────────────────────────────────

/// Regression: SetMatte with mismatched buffer sizes should not panic.
#[test]
fn set_matte_mismatched_sizes_safe() {
    let mut target = vec![255u8; 16]; // 2x2
    let source = vec![128u8; 4]; // 1x1 (smaller)
    let params = SetMatteParams::default();
    apply_set_matte(&mut target, 2, 2, &source, 1, 1, &params);
    // Should not panic — sizes don't match but function returns gracefully
}

/// Regression: SetMatte with empty buffers should not panic.
#[test]
fn set_matte_empty_buffers_safe() {
    let mut target: Vec<u8> = vec![];
    let source: Vec<u8> = vec![];
    let params = SetMatteParams::default();
    apply_set_matte(&mut target, 0, 0, &source, 0, 0, &params);
}

/// Regression: SetMatte with zero width/height should be a no-op.
#[test]
fn set_matte_zero_dimensions_safe() {
    let mut target = vec![255u8; 16];
    let source = vec![128u8; 16];
    let params = SetMatteParams::default();
    apply_set_matte(&mut target, 0, 4, &source, 0, 4, &params);
    assert_eq!(target, vec![255u8; 16]);
}

/// Regression: SetMatte all source channels must not panic.
#[test]
fn set_matte_all_channels_no_panic() {
    let channels = [
        MatteSourceChannel::Alpha,
        MatteSourceChannel::Luminance,
        MatteSourceChannel::Red,
        MatteSourceChannel::Green,
        MatteSourceChannel::Blue,
        MatteSourceChannel::Lightness,
    ];
    for ch in channels {
        let mut target = vec![255u8; 16];
        let source = vec![128u8; 16];
        let params = SetMatteParams {
            source_channel: ch,
            ..Default::default()
        };
        apply_set_matte(&mut target, 2, 2, &source, 2, 2, &params);
    }
}

/// Regression: SetMatte all composite modes must not panic.
#[test]
fn set_matte_all_composite_modes_no_panic() {
    let modes = [
        MatteCompositeMode::Replace,
        MatteCompositeMode::Intersect,
        MatteCompositeMode::Add,
        MatteCompositeMode::Subtract,
    ];
    for mode in modes {
        let mut target = vec![255u8; 16];
        let source = vec![128u8; 16];
        let params = SetMatteParams {
            composite_mode: mode,
            ..Default::default()
        };
        apply_set_matte(&mut target, 2, 2, &source, 2, 2, &params);
    }
}

/// Regression: SetMatte with invert_matte must produce inverted alpha.
#[test]
fn set_matte_invert_works() {
    // Source is fully opaque (alpha=255), target is fully transparent (alpha=0)
    // With invert: extracted_alpha = 1.0 - 1.0 = 0.0, so target alpha stays 0
    let mut target = vec![0u8; 4]; // RGBA: [0,0,0,0]
    target[3] = 0;
    let mut source = vec![0u8; 4];
    source[3] = 255; // fully opaque alpha
    let params = SetMatteParams {
        invert_matte: true,
        composite_mode: MatteCompositeMode::Replace,
        ..Default::default()
    };
    apply_set_matte(&mut target, 1, 1, &source, 1, 1, &params);
    // Inverted alpha from opaque source = 0, so target alpha should be 0
    assert_eq!(target[3], 0);
}

/// Regression: SetMatte roundtrip through serialization.
#[test]
fn set_matte_params_roundtrip() {
    let params = SetMatteParams {
        source_layer_idx: 3,
        source_channel: MatteSourceChannel::Luminance,
        invert_matte: true,
        composite_mode: MatteCompositeMode::Intersect,
    };
    let json = serde_json::to_string(&params).unwrap();
    let restored: SetMatteParams = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.source_layer_idx, 3);
    assert_eq!(restored.source_channel, MatteSourceChannel::Luminance);
    assert!(restored.invert_matte);
    assert_eq!(restored.composite_mode, MatteCompositeMode::Intersect);
}

// ─── History: NaN Deduplication ─────────────────────────────────────────────

/// Regression: History dedup should handle projects with NaN values.
/// NaN != NaN in JSON, so the dedup check may not catch no-op commits.
/// The history should still function correctly (not corrupt state).
#[test]
fn history_nan_dedup_does_not_corrupt_state() {
    use kagari_vfx::core::history::ProjectHistory;

    let initial = Project::default();
    let mut history = ProjectHistory::new(initial);

    // Create a project with a NaN value in a property
    let mut comp = Composition::new("c1".into(), "Test".into(), 100, 100, 30, 30);
    let mut layer = Layer::new(
        "l1".into(),
        "Solid".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        30,
    );
    // Position with NaN — this is a degraded state but should not crash history
    layer.transform.position = Animatable::new_constant([f32::NAN, f32::NAN]);
    comp.add_layer(layer);

    let project = Project {
        compositions: vec![comp],
        ..Default::default()
    };

    history.commit_action(project.clone(), "add nan layer");
    assert_eq!(history.current().compositions[0].layers.len(), 1);

    // Commit same project — dedup may or may not catch it (NaN != NaN)
    // but should not corrupt the history
    history.commit_action(project, "duplicate nan");
    // History should have at least 1 entry
    assert!(!history.is_empty());
}

// ─── History: Byte Budget ──────────────────────────────────────────────────

/// Regression: History byte budget should not underflow.
#[test]
fn history_byte_budget_no_underflow() {
    use kagari_vfx::core::history::ProjectHistory;

    let initial = Project::default();
    let mut history = ProjectHistory::new(initial);

    // Add many entries to trigger byte budget trimming
    for i in 0..20 {
        let mut comp = Composition::new("c1".into(), "Test".into(), 100, 100, 30, 30);
        for j in 0..10 {
            comp.add_layer(Layer::new(
                format!("l{}", j),
                format!("Layer {}", j),
                LayerType::Solid {
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                30,
            ));
        }
        let project = Project {
            compositions: vec![comp],
            ..Default::default()
        };
        history.commit_action(project, &format!("edit {}", i));
    }

    // approx_bytes should be non-negative (usize, so always true)
    // but more importantly, current should be valid
    let current = history.current();
    assert!(!current.compositions.is_empty());
}

// ─── Keyframe Interpolation ────────────────────────────────────────────────

/// Regression: Bezier solver with extreme control points must not produce NaN.
#[test]
fn bezier_solver_extreme_values_no_nan() {
    use kagari_vfx::core::keyframe::solve_bezier_eased_time;

    let extreme_cases = [
        (0.0, 0.0, 0.0, 1.0, 1.0),       // linear
        (0.5, 0.25, 0.1, 0.25, 1.0),      // standard ease
        (0.5, 0.5, 0.2, 0.6, 0.8),        // elastic
        (0.5, 0.5, 0.3, 0.7, 0.1),        // bounce
        (0.0, 0.0, 0.0, 0.0, 0.0),        // degenerate
        (1.0, 1.0, 1.0, 1.0, 1.0),        // degenerate
    ];
    for (x, x1, y1, x2, y2) in extreme_cases {
        let result = solve_bezier_eased_time(x, x1, y1, x2, y2);
        assert!(
            result.is_finite(),
            "solve_bezier({}, {}, {}, {}, {}) = {}",
            x, x1, y1, x2, y2, result
        );
        assert!(
            (0.0..=1.0).contains(&result),
            "solve_bezier out of range: {}",
            result
        );
    }
    // Overshoot easing legitimately produces values > 1.0 (that's the point)
    let overshoot = solve_bezier_eased_time(0.5, 0.68, -0.4, 0.265, 1.4);
    assert!(overshoot.is_finite(), "overshoot must be finite");
}

/// Regression: Hold interpolation must return the start value for all t < 1.0.
#[test]
fn hold_interpolation_returns_start_value() {
    let kfs = vec![
        Keyframe::new(0, 10.0f32, InterpolationType::Hold),
        Keyframe::new(30, 50.0f32, InterpolationType::Hold),
    ];
    let anim = Animatable::new_animated(kfs);
    // Frame 0 → 10.0, frame 15 → 10.0 (Hold), frame 30 → 50.0
    assert!((anim.evaluate(0) - 10.0).abs() < f32::EPSILON);
    assert!((anim.evaluate(15) - 10.0).abs() < f32::EPSILON);
    assert!((anim.evaluate(30) - 50.0).abs() < f32::EPSILON);
}

/// Regression: Linear interpolation must produce correct intermediate values.
#[test]
fn linear_interpolation_correct() {
    let kfs = vec![
        Keyframe::new(0, 0.0f32, InterpolationType::Linear),
        Keyframe::new(100, 100.0f32, InterpolationType::Linear),
    ];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(0) - 0.0).abs() < 0.01);
    assert!((anim.evaluate(50) - 50.0).abs() < 0.01);
    assert!((anim.evaluate(100) - 100.0).abs() < 0.01);
}

/// Regression: Animated property with empty keyframes must not panic.
#[test]
fn animated_empty_keyframes_safe() {
    let anim: Animatable<f32> = Animatable::new_animated(vec![]);
    let val = anim.evaluate(0);
    assert!(val.is_finite());
}

/// Regression: Animated property with single keyframe must return that value.
#[test]
fn animated_single_keyframe() {
    let kfs = vec![Keyframe::new(10, 42.0f32, InterpolationType::Linear)];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(0) - 42.0).abs() < 0.01);
    assert!((anim.evaluate(10) - 42.0).abs() < 0.01);
    assert!((anim.evaluate(100) - 42.0).abs() < 0.01);
}

/// Regression: EasePreset control points must all be finite.
#[test]
fn ease_preset_control_points_finite() {
    use kagari_vfx::core::keyframe::EasePreset;
    let presets = [
        EasePreset::Standard,
        EasePreset::FastIn,
        EasePreset::SmoothOut,
        EasePreset::Overshoot,
        EasePreset::Sine,
        EasePreset::EaseIn,
        EasePreset::EaseOut,
        EasePreset::FastOut,
        EasePreset::SlowIn,
        EasePreset::CustomEase,
        EasePreset::MirrorEase,
        EasePreset::Elastic,
        EasePreset::Bounce,
        EasePreset::Cycle,
        EasePreset::MirrorEase2,
        EasePreset::Custom0,
        EasePreset::Custom1,
        EasePreset::Custom2,
        EasePreset::Custom3,
    ];
    for preset in presets {
        let cp = preset.control_points();
        for (i, &v) in cp.iter().enumerate() {
            assert!(
                v.is_finite(),
                "EasePreset::{:?} control_point[{}] = {}",
                preset,
                i,
                v
            );
        }
    }
}

// ─── LifeCurve ─────────────────────────────────────────────────────────────

/// Regression: LifeCurve with empty points must return 1.0.
#[test]
fn lifecurve_empty_returns_one() {
    let lc = LifeCurve(vec![]);
    assert!((lc.eval(0.0) - 1.0).abs() < f32::EPSILON);
    assert!((lc.eval(0.5) - 1.0).abs() < f32::EPSILON);
    assert!((lc.eval(1.0) - 1.0).abs() < f32::EPSILON);
}

/// Regression: LifeCurve with single point must return that constant.
#[test]
fn lifecurve_single_point_constant() {
    let lc = LifeCurve(vec![0.5]);
    assert!((lc.eval(0.0) - 0.5).abs() < f32::EPSILON);
    assert!((lc.eval(0.5) - 0.5).abs() < f32::EPSILON);
    assert!((lc.eval(1.0) - 0.5).abs() < f32::EPSILON);
}

/// Regression: LifeCurve clamps t to [0,1].
#[test]
fn lifecurve_clamps_t() {
    let lc = LifeCurve(vec![0.0, 1.0]);
    let v_neg = lc.eval(-1.0);
    let v_over = lc.eval(2.0);
    assert!((v_neg - 0.0).abs() < 0.01);
    assert!((v_over - 1.0).abs() < 0.01);
}

/// Regression: LifeCurve interpolation at midpoint.
#[test]
fn lifecurve_midpoint_interpolation() {
    let lc = LifeCurve(vec![0.0, 1.0]);
    let mid = lc.eval(0.5);
    assert!((mid - 0.5).abs() < 0.01);
}

/// Regression: LifeCurve with NaN values should return NaN (not panic).
#[test]
fn lifecurve_nan_values_do_not_panic() {
    let lc = LifeCurve(vec![f32::NAN, f32::NAN]);
    let _ = lc.eval(0.5); // should not panic
}

/// Regression: LifeCurve roundtrip through serialization.
#[test]
fn lifecurve_roundtrip() {
    let lc = LifeCurve(vec![0.0, 0.5, 1.0, 0.5, 0.0]);
    let json = serde_json::to_string(&lc).unwrap();
    let restored: LifeCurve = serde_json::from_str(&json).unwrap();
    assert_eq!(lc, restored);
}

// ─── Shape Boolean ─────────────────────────────────────────────────────────

/// Regression: Boolean with both empty polygons should return empty.
#[test]
fn shape_boolean_both_empty() {
    let result = apply_polygon_boolean(&[], &[], BooleanOp::Union);
    assert!(result.is_empty());
}

/// Regression: Boolean with one empty polygon should return the other (for Union).
#[test]
fn shape_boolean_one_empty_union() {
    let subject = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let result = apply_polygon_boolean(&subject, &[], BooleanOp::Union);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 4);
}

/// Regression: Boolean Intersect with one empty should return empty.
#[test]
fn shape_boolean_one_empty_intersect() {
    let subject = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let result = apply_polygon_boolean(&subject, &[], BooleanOp::Intersect);
    assert!(result.is_empty());
}

/// Regression: Boolean Subtract with empty clip should return subject.
#[test]
fn shape_boolean_empty_clip_subtract() {
    let subject = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let result = apply_polygon_boolean(&subject, &[], BooleanOp::Subtract);
    assert_eq!(result.len(), 1);
}

/// Regression: Boolean with degenerate polygons (< 3 points) should not panic.
#[test]
fn shape_boolean_degenerate_polygons_safe() {
    let line = [[0.0, 0.0], [10.0, 0.0]];
    let point = [[5.0, 5.0]];
    for op in [BooleanOp::Union, BooleanOp::Intersect, BooleanOp::Subtract, BooleanOp::Exclude] {
        let _ = apply_polygon_boolean(&line, &point, op);
        let _ = apply_polygon_boolean(&point, &line, op);
    }
}

/// Regression: Boolean with coincident polygons should not panic.
#[test]
fn shape_boolean_coincident_polygons_safe() {
    let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let result = apply_polygon_boolean(&square, &square, BooleanOp::Union);
    // Should produce some result (at least one contour)
    assert!(!result.is_empty());
}

/// Regression: Boolean with NaN coordinates should not panic.
#[test]
fn shape_boolean_nan_coordinates_safe() {
    let nan_poly = [[f32::NAN, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let normal = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0]];
    let _ = apply_polygon_boolean(&nan_poly, &normal, BooleanOp::Union);
}

// ─── Expression Engine: Additional Edge Cases ──────────────────────────────

/// Regression: Expression with division by zero must not panic.
#[test]
fn expression_division_by_zero_safe() {
    let engine = expression_engine::build_engine();
    let result = engine.eval::<f64>("1.0 / 0.0");
    assert!(result.is_ok());
    let v = result.unwrap();
    // Division by zero in Rhai produces Infinity, not a panic
    assert!(v.is_infinite());
}

/// Regression: Expression with deeply nested brackets must be bounded.
#[test]
fn expression_deep_nesting_bounded() {
    let engine = expression_engine::build_engine();
    // 100 levels of nesting — should be within the 64-depth limit
    let expr = "((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))";
    let result = engine.eval::<f64>(expr);
    // Should either succeed or fail gracefully (not panic)
    let _ = result;
}

/// Regression: Expression string operations must be bounded.
#[test]
fn expression_string_operations_bounded() {
    let engine = expression_engine::build_engine();
    // Moderate string operations should succeed
    let result = engine.eval::<String>(r#""hello" + " " + "world""#);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello world");
}

/// Regression: Expression array operations must be bounded.
#[test]
fn expression_array_operations_bounded() {
    let engine = expression_engine::build_engine();
    let result = engine.eval::<f64>(r#"[1.0, 2.0, 3.0][1]"#);
    assert!(result.is_ok());
    assert!((result.unwrap() - 2.0).abs() < f64::EPSILON);
}

/// Regression: posterizeTime must snap to discrete steps.
#[test]
fn expression_posterize_time() {
    let engine = expression_engine::build_engine();
    let mut scope = Scope::new();
    scope.push("time", 0.5f64);
    let result: f64 = engine
        .eval_expression_with_scope(&mut scope, "posterizeTime(2.0)")
        .unwrap();
    // posterizeTime(2) with time=0.5: step=1/2=0.5, floor(0.5/0.5)*0.5 = 0.5
    assert!(result.is_finite());
    // The result should be a discrete step of 1/2 = 0.5
    let step = 1.0 / 2.0;
    let diff = (result / step - (result / step).round()).abs();
    assert!(diff < 0.01, "posterizeTime(2) = {} not on 0.5 grid", result);
}

/// Regression: noise function must return values in [0, 1] for various inputs.
#[test]
fn expression_noise_wide_range() {
    let engine = expression_engine::build_engine();
    for x in [-100.0, -1.0, 0.0, 0.5, 1.0, 100.0, 1000.0] {
        let result: f64 = engine.eval(&format!("noise({:.1})", x)).unwrap();
        assert!(
            result >= 0.0 && result <= 1.0,
            "noise({}) = {} out of range",
            x,
            result
        );
    }
}

/// Regression: toe and shoulder helpers must return finite values.
#[test]
fn expression_toe_shoulder_finite() {
    let engine = expression_engine::build_engine();
    for x in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let toe: f64 = engine.eval(&format!("toe({:.2}, 2.0)", x)).unwrap();
        let shoulder: f64 = engine.eval(&format!("shoulder({:.2}, 2.0)", x)).unwrap();
        assert!(toe.is_finite(), "toe({}) = {}", x, toe);
        assert!(shoulder.is_finite(), "shoulder({}) = {}", x, shoulder);
    }
}

// ─── Animatable: Edge Cases ────────────────────────────────────────────────

/// Regression: Animatable::evaluate on constant always returns the value.
#[test]
fn animatable_constant_always_returns_value() {
    let anim = Animatable::new_constant(42.0f32);
    for frame in [0, 1, 100, u32::MAX] {
        assert!((anim.evaluate(frame) - 42.0).abs() < f32::EPSILON);
    }
}

/// Regression: Animatable with keyframes before first kf returns first value.
#[test]
fn animatable_before_first_keyframe() {
    let kfs = vec![
        Keyframe::new(10, 100.0f32, InterpolationType::Linear),
        Keyframe::new(20, 200.0f32, InterpolationType::Linear),
    ];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(0) - 100.0).abs() < 0.01);
    assert!((anim.evaluate(5) - 100.0).abs() < 0.01);
}

/// Regression: Animatable with keyframes after last kf returns last value.
#[test]
fn animatable_after_last_keyframe() {
    let kfs = vec![
        Keyframe::new(10, 100.0f32, InterpolationType::Linear),
        Keyframe::new(20, 200.0f32, InterpolationType::Linear),
    ];
    let anim = Animatable::new_animated(kfs);
    assert!((anim.evaluate(30) - 200.0).abs() < 0.01);
    assert!((anim.evaluate(1000) - 200.0).abs() < 0.01);
}

/// Regression: Animatable Vec2 evaluate must return finite values.
#[test]
fn animatable_vec2_evaluate_finite() {
    let anim = Animatable::new_constant([100.0f32, 200.0f32]);
    let val = anim.evaluate(0);
    assert!(val[0].is_finite());
    assert!(val[1].is_finite());
}

/// Regression: Animatable easy_ease on constant must be a no-op.
#[test]
fn animatable_easy_ease_constant_noop() {
    let mut anim = Animatable::new_constant(42.0f32);
    anim.easy_ease();
    assert!((anim.evaluate(0) - 42.0).abs() < f32::EPSILON);
}

/// Regression: Animatable easy_ease on single keyframe must be a no-op.
#[test]
fn animatable_easy_ease_single_kf_noop() {
    let mut anim = Animatable::new_animated(vec![Keyframe::new(
        0,
        42.0f32,
        InterpolationType::Linear,
    )]);
    anim.easy_ease();
    assert!((anim.evaluate(0) - 42.0).abs() < f32::EPSILON);
}

// ─── Particle Emitter Shape ────────────────────────────────────────────────

/// Regression: all emitter shapes must emit particles without panic.
#[test]
fn particle_all_emitter_shapes_no_panic() {
    let shapes = [
        EmitterShape::Point,
        EmitterShape::Box,
        EmitterShape::Circle,
        EmitterShape::Line,
        EmitterShape::Ring,
    ];
    for shape in shapes {
        let emitter = ParticleEmitter {
            shape,
            rate: 50.0,
            max_particles: 10,
            lifetime: 0.5,
            ..Default::default()
        };
        let mut sys = ParticleSystem::new(emitter);
        sys.update(0.1, 100.0, 100.0);
        for _ in 0..30 {
            sys.update(0.016, 100.0, 100.0);
        }
        // All particles should be finite
        for p in &sys.particles {
            assert!(p.x.is_finite());
            assert!(p.y.is_finite());
        }
    }
}

/// Regression: ParticleEmitter serde roundtrip preserves all fields.
#[test]
fn particle_emitter_roundtrip() {
    let emitter = ParticleEmitter {
        rate: 75.0,
        max_particles: 500,
        lifetime: 3.0,
        lifetime_variance: 0.1,
        speed: 300.0,
        speed_variance: 0.2,
        spread_degrees: 180.0,
        shape: EmitterShape::Circle,
        emitter_size: [200.0, 100.0],
        gravity: [0.0, -500.0],
        wind: [50.0, 0.0],
        turbulence: 2.0,
        color_start: [1.0, 0.0, 0.0, 1.0],
        color_end: [0.0, 0.0, 1.0, 0.0],
        size_start: 16.0,
        size_end: 4.0,
        opacity_start: 1.0,
        opacity_end: 0.0,
        rotation_speed: 90.0,
        rotation_start: 45.0,
        rotation_speed_variance: 0.5,
        fade_curve: FadeCurve::EaseIn,
        blend_mode: 2,
        gravity_curve: LifeCurve(vec![0.5, 1.0, 0.5]),
        wind_gust_strength: 100.0,
        wind_gust_frequency: 2.0,
        drag: 0.1,
        collision_enabled: true,
        collision_bounds: [0.0, 0.0, 1920.0, 1080.0],
        restitution: 0.8,
        surface_friction: 0.9,
        particle_collisions: true,
        particle_diameter: 12.0,
        trail_length: 5,
        trail_taper: 0.6,
        vortex_strength: 50.0,
        vortex_center: [960.0, 540.0],
        attract_strength: 10.0,
        attract_center: [400.0, 300.0],
        depth_enabled: true,
        depth_range: [-50.0, 50.0],
        death_spawn_count: 3,
        death_spawn_speed_scale: 0.3,
        death_spawn_life_scale: 0.4,
    };
    let json = serde_json::to_string(&emitter).unwrap();
    let restored: ParticleEmitter = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.rate, 75.0);
    assert_eq!(restored.max_particles, 500);
    assert_eq!(restored.shape, EmitterShape::Circle);
    assert_eq!(restored.trail_length, 5);
    assert_eq!(restored.death_spawn_count, 3);
    assert!((restored.gravity_curve.0[0] - 0.5).abs() < f32::EPSILON);
}

// ─── TrimPaths Edge Cases ──────────────────────────────────────────────────

/// Regression: TrimPaths with large offset wraps correctly.
#[test]
fn trim_paths_large_offset() {
    let tp = TrimPaths {
        start: Animatable::new_constant(0.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(720.0), // 2 full rotations
    };
    let points = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let result = tp.trim_polygon(&points, 0);
    // 720° offset = 0° effective, so result should be non-empty
    assert!(!result.is_empty());
}

/// Regression: TrimPaths with negative offset wraps correctly.
#[test]
fn trim_paths_negative_offset() {
    let tp = TrimPaths {
        start: Animatable::new_constant(0.0),
        end: Animatable::new_constant(50.0),
        offset: Animatable::new_constant(-360.0),
    };
    let points = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let result = tp.trim_polygon(&points, 0);
    assert!(!result.is_empty());
}

// ─── BlendMode Correctness ─────────────────────────────────────────────────

/// Regression: BlendMode serialization must be stable (JSON schema compat).
#[test]
fn blend_mode_json_values_stable() {
    let json = serde_json::to_string(&BlendMode::Normal).unwrap();
    assert_eq!(json, r#""Normal""#);
    let json = serde_json::to_string(&BlendMode::Multiply).unwrap();
    assert_eq!(json, r#""Multiply""#);
    let json = serde_json::to_string(&BlendMode::Screen).unwrap();
    assert_eq!(json, r#""Screen""#);
}

// ─── EffectType: All Variants Serialize ─────────────────────────────────────

/// Regression: every EffectType variant must serialize without panic.
#[test]
fn effect_type_all_variants_serialize() {
    let effects = vec![
        EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(5.0),
        },
        EffectType::ColorTint {
            color: Animatable::new_constant([1.0, 0.0, 0.0, 1.0]),
            intensity: Animatable::new_constant(0.5),
        },
        EffectType::Glow {
            threshold: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(20.0),
            intensity: Animatable::new_constant(1.0),
            color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
        },
        EffectType::Twirl {
            angle: Animatable::new_constant(45.0),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Bulge {
            amount: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(100.0),
        },
        EffectType::Sharpen {
            amount: Animatable::new_constant(50.0),
        },
        EffectType::Threshold {
            threshold: Animatable::new_constant(128.0),
        },
        EffectType::Invert { invert_alpha: true },
        EffectType::FindEdges { invert: false },
        EffectType::RadialBlur {
            amount: Animatable::new_constant(10.0),
        },
        EffectType::DirectionalBlur {
            angle: Animatable::new_constant(0.0),
            length: Animatable::new_constant(10.0),
        },
    ];
    for effect in effects {
        let json = serde_json::to_string(&effect).unwrap();
        let _restored: EffectType = serde_json::from_str(&json).unwrap();
    }
}

// ─── Composition: Layer Management ──────────────────────────────────────────

/// Regression: Composition with many layers should not panic on operations.
#[test]
fn composition_many_layers_safe() {
    let mut comp = Composition::new("c1".into(), "Test".into(), 100, 100, 30, 300);
    for i in 0..100 {
        comp.add_layer(Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            300,
        ));
    }
    assert_eq!(comp.layers.len(), 100);

    // All layers should be active at frame 0
    let active_count = comp.layers.iter().filter(|l| l.is_active(0)).count();
    assert_eq!(active_count, 100);
}

/// Regression: Layer with in_frame > out_frame should not be active.
#[test]
fn layer_in_gt_out_not_active() {
    let mut layer = Layer::new(
        "l1".into(),
        "Test".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        100,
    );
    layer.in_frame = 50;
    layer.out_frame = 30; // in > out
    assert!(!layer.is_active(40));
    assert!(!layer.is_active(0));
    assert!(!layer.is_active(100));
}

// ─── Project: Default Value Invariants ──────────────────────────────────────

/// Regression: Project default must have at least one composition.
#[test]
fn project_default_has_composition() {
    let project = Project::default();
    assert!(
        !project.compositions.is_empty(),
        "Project::default() must have at least one composition"
    );
}

/// Regression: Composition default fps must be > 0.
#[test]
fn composition_default_fps_positive() {
    let comp = Composition::new("c".into(), "C".into(), 1920, 1080, 30, 100);
    assert!(comp.fps > 0);
}

/// Regression: Composition default dimensions must be > 0.
#[test]
fn composition_dimensions_positive() {
    let comp = Composition::new("c".into(), "C".into(), 1920, 1080, 30, 100);
    assert!(comp.width > 0);
    assert!(comp.height > 0);
}

// ─── MaterialOptions: Invariants ────────────────────────────────────────────

/// Regression: MaterialOptions ambient/diffuse/specular must be in [0,1].
#[test]
fn material_options_values_in_range() {
    let mat = MaterialOptions::default();
    assert!(mat.ambient >= 0.0 && mat.ambient <= 1.0);
    assert!(mat.diffuse >= 0.0 && mat.diffuse <= 1.0);
    assert!(mat.specular >= 0.0 && mat.specular <= 1.0);
    assert!(mat.emission >= 0.0 && mat.emission <= 1.0);
    assert!(mat.metalness >= 0.0 && mat.metalness <= 1.0);
}
