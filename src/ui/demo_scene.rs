//! One-click demo composition so first-time users instantly see the app
//! rendering animated content instead of an empty void.
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Expression, Layer, LayerType, ShapeType};

fn kf(frame: u32, v: f32) -> Keyframe<f32> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}
fn kfv2(frame: u32, v: [f32; 2]) -> Keyframe<[f32; 2]> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}

pub fn build(app: &mut crate::AfterEffectsApp) {
    let count = app.history.current().compositions.len();
    let mut comp = Composition::new(
        format!("comp_demo_{}", count),
        "🎬 Demo Scene".to_string(),
        1280,
        720,
        30,
        150,
    );
    comp.blend_linear = false;
    comp.dither_output = true;

    // ── Background ──
    let bg = Layer::new(
        "demo_bg".into(),
        "Background".into(),
        LayerType::Solid {
            color: [0.05, 0.07, 0.11, 1.0],
        },
        comp.duration_frames,
    );
    comp.layers.push(bg);

    // ── Accent circle: scale bounce + constant spin ──
    let mut circle = Layer::new(
        "demo_circle".into(),
        "Accent Orb".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(180.0),
                height: Animatable::new_constant(180.0),
            },
            color: [0.0, 0.64, 1.0, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        comp.duration_frames,
    );
    circle.transform.position =
        Animatable::new_animated(vec![kfv2(0, [320.0, 500.0]), kfv2(150, [960.0, 220.0])]);
    circle.transform.position.easy_ease();
    circle.transform.scale = Animatable::new_animated(vec![
        kfv2(0, [0.0, 0.0]),
        kfv2(20, [115.0, 115.0]),
        kfv2(35, [100.0, 100.0]),
    ]);
    circle.transform.rotation_expression = Some(Expression::Raw("time * 60".into()));
    comp.layers.push(circle);

    // ── Title: fade + rise ──
    let mut title = Layer::new(
        "demo_title".into(),
        "Title".into(),
        LayerType::new_text("AFTER EFFECTS OSS", 88, [0.95, 0.96, 1.0, 1.0]),
        comp.duration_frames,
    );
    title.transform.position =
        Animatable::new_animated(vec![kfv2(10, [640.0, 400.0]), kfv2(50, [640.0, 350.0])]);
    title.transform.opacity = Animatable::new_animated(vec![kf(0, 0.0), kf(35, 100.0)]);
    title.transform.opacity.easy_ease();
    comp.layers.push(title);

    // ── Subtitle ──
    let mut sub = Layer::new(
        "demo_sub".into(),
        "Subtitle".into(),
        LayerType::new_text("Rust • GPU • Open Source", 34, [0.55, 0.75, 1.0, 1.0]),
        comp.duration_frames,
    );
    sub.transform.opacity = Animatable::new_animated(vec![kf(25, 0.0), kf(60, 90.0)]);
    sub.transform.opacity.easy_ease();
    sub.transform.position = Animatable::new_constant([640.0, 430.0]);
    comp.layers.push(sub);

    let proj = app.history.current_mut();
    proj.compositions.push(comp);
    proj.active_composition_idx = proj.compositions.len() - 1;
    crate::core::frame_cache::bump_version();
    app.toasts.info("Demo scene loaded — press Space to play!");
}
