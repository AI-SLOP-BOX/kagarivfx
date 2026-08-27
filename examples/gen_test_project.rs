/// Generates a test project JSON for CLI smoke testing.
use aftereffects_oss::core::timeline::{Project, Composition, Layer, LayerType};
use aftereffects_oss::core::property::Animatable;
use aftereffects_oss::core::particle_system::ParticleEmitter;

fn main() {
    let mut comp = Composition::new("comp1".into(), "SmokeTest".into(), 320, 180, 30, 30);

    let mut bg = Layer::new("bg".into(), "BG Solid".into(), LayerType::Solid { color: [0.1, 0.15, 0.3, 1.0] }, 30);
    bg.transform.position = Animatable::new_constant([160.0, 90.0]);
    comp.layers.push(bg);

    let mut parts = Layer::new("parts".into(), "Particles".into(), LayerType::Particle {
        emitter: ParticleEmitter { rate: 200.0, ..Default::default() },
    }, 30);
    parts.transform.position = Animatable::new_constant([160.0, 140.0]);
    parts.blend_mode = aftereffects_oss::core::timeline::BlendMode::Add;
    comp.layers.push(parts);

    let mut moving = Layer::new("moving".into(), "Moving Shape".into(), LayerType::Shape {
        shape_type: aftereffects_oss::core::timeline::ShapeType::Ellipse {
            width: Animatable::new_constant(60.0),
            height: Animatable::new_constant(60.0),
        },
        color: [0.9, 0.3, 0.2, 1.0],
        stroke_color: [0.0, 0.0, 0.0, 1.0],
        stroke_width: 0.0,
    }, 30);
    // Horizontal motion for motion blur testing
    moving.motion_blur = true;
    moving.transform.position = Animatable::new_animated(vec![
        aftereffects_oss::core::keyframe::Keyframe::new(5, [80.0, 60.0], aftereffects_oss::core::keyframe::InterpolationType::Linear),
        aftereffects_oss::core::keyframe::Keyframe::new(25, [240.0, 60.0], aftereffects_oss::core::keyframe::InterpolationType::Linear),
    ]);
    comp.layers.push(moving);

    // Text layer with stroke + eased keyframes (GPU text rendering / stroke baking test)
    let mut text = Layer::new("title".into(), "Title Text".into(), LayerType::Text {
        text: "AE OSS".into(),
        font_size: 36,
        color: [1.0, 1.0, 1.0, 1.0],
        font_family: "Helvetica".into(),
        tracking: 2.0,
        leading: 1.2,
        align: 1,
        stroke_color: [0.9, 0.4, 0.1, 1.0],
        stroke_width: 3.0,
        text_on_path: false,
    }, 30);
    text.transform.position = Animatable::new_animated(vec![
        aftereffects_oss::core::keyframe::Keyframe::new(0, [160.0, 40.0], aftereffects_oss::core::keyframe::InterpolationType::Bezier {
            outgoing: aftereffects_oss::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            incoming: aftereffects_oss::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            custom_bezier: Some(aftereffects_oss::core::keyframe::EasePreset::Overshoot.control_points()),
        }),
        aftereffects_oss::core::keyframe::Keyframe::new(20, [160.0, 90.0], aftereffects_oss::core::keyframe::InterpolationType::Bezier {
            outgoing: aftereffects_oss::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            incoming: aftereffects_oss::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            custom_bezier: Some(aftereffects_oss::core::keyframe::EasePreset::Overshoot.control_points()),
        }),
    ]);
    comp.layers.push(text);

    let project = Project { use_gpu_compute: false, 
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
    };

    let json = serde_json::to_string_pretty(&project).expect("serialize");
    std::fs::write("test_project.json", json).expect("write");
    println!("Wrote test_project.json");
}
