//! AE Daily Workflow integration tests.
//! Covers: mask path manipulation, time remap, precompose, track matte, render consistency.

use kagari_vfx::core::keyframe::{BezierControlPoint, InterpolationType, Keyframe};
use kagari_vfx::core::mask::MaskPath;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{
    Composition, Effect, EffectType, Layer, LayerType, PrecompAttributesMode,
};

// ── Mask Path Manipulation ──

#[test]
fn mask_insert_vertex_interpolates_position() {
    let mut mask = MaskPath::new_closed(vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]]);
    mask.insert_vertex_at_frame(0, 0, 0.5);
    let verts = mask.vertices_at_frame(0);
    assert_eq!(verts.len(), 4);
    assert!((verts[1][0] - 50.0).abs() < 1.0);
    assert!((verts[1][1] - 0.0).abs() < 1.0);
}

#[test]
fn mask_remove_vertex_preserves_count() {
    let mut mask = MaskPath::new_closed(vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0], [50.0, 100.0]]);
    mask.remove_vertex_at_frame(1);
    let verts = mask.vertices_at_frame(0);
    assert_eq!(verts.len(), 3);
    assert!(mask.is_closed);
}

#[test]
fn mask_tangents_bezier_polygon_has_extra_points() {
    let mut mask = MaskPath::new_closed(vec![[0.0, 50.0], [100.0, 50.0]]);
    mask.set_tangents_at_vertex(0, [-20.0, 0.0], [20.0, 0.0], false);
    mask.set_tangents_at_vertex(1, [-20.0, 0.0], [20.0, 0.0], false);

    let poly = mask.to_polygon(0, 8);
    assert!(poly.len() >= 4);
    assert!(poly[1][0] > 0.0 && poly[1][0] < 100.0);
}

#[test]
fn mask_vertices_evaluated_at_frame() {
    let mask = MaskPath::new_rect(10.0, 20.0, 30.0, 40.0);
    let verts = mask.get_vertices(0);
    assert_eq!(verts.len(), 4);
    assert_eq!(verts[0], [10.0, 20.0]);
    assert_eq!(verts[2], [40.0, 60.0]);
}

// ── Time Remap Operations ──

#[test]
fn time_remapping_enable_and_freeze_consistent() {
    let mut layer = Layer::new(
        "l1".into(),
        "Layer".into(),
        LayerType::Solid { color: [1.0; 4] },
        60,
    );
    layer.in_frame = 10;
    layer.out_frame = 50;

    layer.enable_time_remapping();
    assert!(layer.time_remap.is_some());
    assert_eq!(layer.remap_frame(10), 10);
    assert_eq!(layer.remap_frame(50), 50);

    layer.freeze_at(25);
    assert_eq!(layer.remap_frame(10), 25);
    assert_eq!(layer.remap_frame(50), 25);

    layer.clear_time_remap();
    assert!(layer.time_remap.is_none());
    assert_eq!(layer.remap_frame(10), 10);
}

#[test]
fn time_reverse_flips_endpoints() {
    let mut layer = Layer::new(
        "l1".into(),
        "Layer".into(),
        LayerType::Solid { color: [1.0; 4] },
        100,
    );
    layer.in_frame = 0;
    layer.out_frame = 100;

    layer.time_reverse();
    assert_eq!(layer.remap_frame(0), 100);
    assert_eq!(layer.remap_frame(50), 50);
    assert_eq!(layer.remap_frame(100), 0);
}

#[test]
fn time_stretch_doubles_duration() {
    let mut layer = Layer::new(
        "l1".into(),
        "Layer".into(),
        LayerType::Solid { color: [1.0; 4] },
        60,
    );
    layer.in_frame = 0;
    layer.out_frame = 60;

    layer.time_stretch(2.0);
    assert_eq!(layer.out_frame, 120);
    assert_eq!(layer.remap_frame(0), 0);
    assert_eq!(layer.remap_frame(120), 60);
}

// ── Precompose Workflow ──

#[test]
fn precompose_move_to_new_comp() {
    let mut comp = Composition::new("c1".into(), "Main".into(), 1920, 1080, 30, 120);
    for i in 0..3 {
        comp.layers.push(Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            LayerType::Solid {
                color: [i as f32 / 3.0, 0.5, 0.5, 1.0],
            },
            120,
        ));
    }

    let result = comp.precompose_layers(
        &["l0".into(), "l1".into()],
        "precomp1".into(),
        "Pre-comp 1".into(),
        PrecompAttributesMode::MoveToNewComp,
    );
    assert!(result.is_some());
    assert_eq!(comp.layers.len(), 2);
    assert!(!comp.sub_compositions.is_empty());

    let precomp_ref = &comp.layers[0];
    assert!(matches!(precomp_ref.layer_type, LayerType::PreComp { .. }));
}

#[test]
fn precompose_leave_in_parent_preserves_effects() {
    let mut comp = Composition::new("c1".into(), "Main".into(), 1920, 1080, 30, 60);
    let mut layer = Layer::new(
        "l1".into(),
        "Layer".into(),
        LayerType::Solid { color: [1.0; 4] },
        60,
    );
    layer.effects.push(Effect {
        id: "e1".into(),
        name: "Glow".into(),
        effect_type: EffectType::Glow {
            threshold: Animatable::new_constant(0.5),
            radius: Animatable::new_constant(5.0),
            intensity: Animatable::new_constant(0.8),
            color: Animatable::new_constant([1.0; 4]),
        },
        enabled: true,
    });
    comp.layers.push(layer);

    let result = comp.precompose_layers(
        &["l1".into()],
        "precomp1".into(),
        "Pre-comp 1".into(),
        PrecompAttributesMode::LeaveInParent,
    );
    assert!(result.is_some());
    let precomp_layer = &comp.layers[0];
    assert!(!precomp_layer.effects.is_empty());
}

#[test]
fn precompose_returns_none_for_empty_selection() {
    let mut comp = Composition::new("c1".into(), "Main".into(), 1920, 1080, 30, 60);
    let result = comp.precompose_layers(
        &[],
        "precomp1".into(),
        "Pre-comp 1".into(),
        PrecompAttributesMode::MoveToNewComp,
    );
    assert!(result.is_none());
}

#[test]
fn solid_layer_renders_nonempty_pixels() {
    let mut comp = Composition::new("c1".into(), "Main".into(), 64, 64, 30, 1);
    comp.layers.push(Layer::new(
        "fg".into(),
        "FG".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    ));

    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    assert_eq!(pixels.len(), 64 * 64 * 4);
    let any_nonzero = pixels.iter().any(|&b| b > 0);
    assert!(any_nonzero, "Rendered pixels should not be all-zero");
}

#[test]
fn opacity_keyframes_produce_different_pixel_values() {
    let mut comp = Composition::new("c1".into(), "Test".into(), 64, 64, 30, 60);
    let mut layer = Layer::new(
        "l1".into(),
        "Mover".into(),
        LayerType::Solid { color: [1.0; 4] },
        60,
    );
    layer.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(30, 100.0, InterpolationType::Linear),
    ]);
    comp.layers.push(layer);

    let p0 = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let p30 = render_frame_to_pixels(&comp, 30, 64, 64, 0.0, 0);

    let sum_0: u32 = p0.iter().map(|&b| b as u32).sum();
    let sum_30: u32 = p30.iter().map(|&b| b as u32).sum();
    assert!(
        sum_30 > sum_0,
        "Frame 30 (opacity=100) should produce more nonzero pixels than frame 0 (opacity=0): sum {} vs {}",
        sum_30, sum_0
    );
}

#[test]
fn bezier_velocity_control_points_correctly_sized() {
    let outgoing = BezierControlPoint {
        influence: 0.333,
        speed: 0.0,
    };
    let incoming = BezierControlPoint {
        influence: 0.333,
        speed: 0.0,
    };
    let cp = kagari_vfx::core::keyframe::compute_ae_bezier_control_points(
        &outgoing, &incoming, 30.0, 100.0, 30.0,
    );
    assert_eq!(cp.len(), 4);
    assert_eq!(cp[1], 0.0);
    assert_eq!(cp[3], 1.0);
    let [x1, _, x2, _] = cp;
    assert!(
        x1 < x2,
        "Outgoing handle x ({}) should be less than incoming x ({})",
        x1,
        x2
    );
}
