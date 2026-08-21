//! Stress tests: large compositions, many layers, deep nesting, long timelines.
//! These verify the renderer stays bounded in time/memory and never panics
//! under pathological-but-plausible project sizes.

use aftereffects_oss::core::timeline::{Composition, Layer, LayerType};
use aftereffects_oss::core::property::Animatable;
use aftereffects_oss::core::keyframe::{Keyframe, InterpolationType};
use aftereffects_oss::core::software_renderer::render_frame_to_pixels;

#[test]
fn stress_many_layers_render_bounded() {
    // 500 layers — far beyond a typical comp but must not blow up
    let mut comp = Composition::new("c".into(), "Stress500".into(), 128, 128, 30, 30);
    for i in 0..500 {
        let mut l = Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            LayerType::Solid { color: [0.5, 0.3, (i % 10) as f32 / 10.0, 1.0] },
            30,
        );
        l.transform.position = Animatable::new_constant([
            (i * 7 % 128) as f32,
            (i * 13 % 128) as f32,
        ]);
        comp.layers.push(l);
    }

    let pixels = render_frame_to_pixels(&comp, 0, 128, 128, 0.0, 0);
    assert_eq!(pixels.len(), 128 * 128 * 4);
}

#[test]
fn stress_long_timeline_with_keyframes() {
    // 1-minute timeline at 60fps with animated layers
    let mut comp = Composition::new("c".into(), "LongTimeline".into(), 64, 64, 60, 3600);
    let mut l = Layer::new("m".into(), "Mover".into(), LayerType::Solid { color: [1.0; 4] }, 60);
    let kfs: Vec<Keyframe<[f32; 2]>> = (0..=600)
        .step_by(10)
        .map(|f| {
            Keyframe::new(f, [(f % 64) as f32, ((f / 10) % 64) as f32], InterpolationType::Linear)
        })
        .collect();
    l.transform.position = Animatable::new_animated(kfs);
    comp.layers.push(l);

    // Sample across the full duration
    for frame in [0u32, 1000, 2000, 3599] {
        let pixels = render_frame_to_pixels(&comp, frame, 64, 64, 0.0, 0);
        assert_eq!(pixels.len(), 64 * 64 * 4);
    }
}

#[test]
fn stress_deep_precomp_nesting_terminates() {
    // 20-level precomp chain (within MAX_PRECOMP_DEPTH=16 → deepest levels skipped)
    let mut root = Composition::new("L00".into(), "Root".into(), 32, 32, 30, 30);
    for depth in 0..20 {
        let id = format!("L{:02}", depth);
        let next = format!("L{:02}", depth + 1);
        let pc = Layer::new(next.clone(), next.clone(), LayerType::PreComp { comp_id: next }, 30);
        root.sub_compositions.push({
            let mut sub = Composition::new(id.clone(), id.clone(), 32, 32, 30, 30);
            sub.layers.push(pc.clone());
            sub
        });
    }
    // Must terminate without stack overflow even beyond depth cap
    let pixels = render_frame_to_pixels(&root, 0, 32, 32, 0.0, 0);
    assert_eq!(pixels.len(), 32 * 32 * 4);
}

#[test]
fn stress_wide_expressions_evaluate() {
    // Every layer driven by expressions referencing others — O(n²) snapshot builds must stay sane
    use aftereffects_oss::core::timeline::Expression;
    let mut comp = Composition::new("c".into(), "ExprStress".into(), 64, 64, 30, 30);
    for i in 0..50 {
        let mut l = Layer::new(format!("e{}", i), format!("E{}", i), LayerType::Null, 30);
        if i > 0 {
            l.transform.position_expression = Some(Expression::Raw(format!(
                "thisComp.layer(\"E{}\").transform.position + [1.0, 0.0]",
                i - 1
            )));
        }
        comp.layers.push(l);
    }
    let (pos, _, _, _) = comp.resolve_world_transform(comp.layers.last().unwrap(), 0);
    assert!(pos[0].is_finite(), "chained expressions must produce finite values");
}
