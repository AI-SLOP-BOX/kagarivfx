//! Stress tests: large compositions, many layers, deep nesting, long timelines.
//! These verify the renderer stays bounded in time/memory and never panics
//! under pathological-but-plausible project sizes.

use kagari_vfx::core::history::ProjectHistory;
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::Project;
use kagari_vfx::core::timeline::{Composition, Layer, LayerType};

#[test]
fn stress_many_layers_render_bounded() {
    // 500 layers — far beyond a typical comp but must not blow up
    let mut comp = Composition::new("c".into(), "Stress500".into(), 128, 128, 30, 30);
    for i in 0..500 {
        let mut l = Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            LayerType::Solid {
                color: [0.5, 0.3, (i % 10) as f32 / 10.0, 1.0],
            },
            30,
        );
        l.transform.position =
            Animatable::new_constant([(i * 7 % 128) as f32, (i * 13 % 128) as f32]);
        comp.layers.push(l);
    }

    let pixels = render_frame_to_pixels(&comp, 0, 128, 128, 0.0, 0);
    assert_eq!(pixels.len(), 128 * 128 * 4);
}

#[test]
fn stress_long_timeline_with_keyframes() {
    // 1-minute timeline at 60fps with animated layers
    let mut comp = Composition::new("c".into(), "LongTimeline".into(), 64, 64, 60, 3600);
    let mut l = Layer::new(
        "m".into(),
        "Mover".into(),
        LayerType::Solid { color: [1.0; 4] },
        60,
    );
    let kfs: Vec<Keyframe<[f32; 2]>> = (0..=600)
        .step_by(10)
        .map(|f| {
            Keyframe::new(
                f,
                [(f % 64) as f32, ((f / 10) % 64) as f32],
                InterpolationType::Linear,
            )
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
        let pc = Layer::new(
            next.clone(),
            next.clone(),
            LayerType::PreComp { comp_id: next },
            30,
        );
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
    use kagari_vfx::core::timeline::Expression;
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
    assert!(
        pos[0].is_finite(),
        "chained expressions must produce finite values"
    );
}

#[test]
fn stress_timeline_coordinate_consistency() {
    let mut proj = Project::default();
    proj.compositions.clear();
    let mut comp = Composition::new("c".into(), "Stress".into(), 1920, 1080, 30, 300);
    for i in 0..200 {
        let mut l = Layer::new(
            format!("l{}", i),
            format!("Layer {}", i),
            LayerType::Null,
            30,
        );
        l.in_frame = (i * 5) as u32;
        l.out_frame = (i * 5 + 100) as u32;
        for kf in 0..20 {
            let frame = (i * 5 + kf * 5) as u32;
            l.transform.position.add_keyframe(Keyframe {
                frame,
                value: [kf as f32 * 10.0, kf as f32 * 5.0],
                interpolation: InterpolationType::Linear,
            });
        }
        comp.layers.push(l);
    }
    proj.compositions.push(comp);
    let mut history = ProjectHistory::new(proj);

    // Commit some edits first so undo/redo has something to work with
    for i in 0..10 {
        let mut p = history.current().clone();
        p.compositions[0].layers[0].name = format!("edit_{}", i);
        history.commit(p);
    }
    let gen_before = history.generation();

    // Simulate 10 undo/redo cycles — generation must always advance
    for _ in 0..10 {
        let _ = history.undo();
        let _ = history.redo();
    }
    let gen_after = history.generation();
    assert!(
        gen_after > gen_before,
        "generation must advance through undo/redo cycles"
    );
}

#[test]
fn stress_large_project_undo_redo_no_corruption() {
    let mut proj = Project::default();
    proj.compositions.clear();
    let mut comp = Composition::new("c".into(), "Big".into(), 1920, 1080, 30, 300);
    for i in 0..500 {
        comp.layers.push(Layer::new(
            format!("l{}", i),
            format!("L{}", i),
            LayerType::Null,
            30,
        ));
    }
    proj.compositions.push(comp);
    let mut history = ProjectHistory::new(proj);

    // Commit 30 mutations (within the 50-entry history limit)
    for i in 0..30 {
        let mut p = history.current().clone();
        p.compositions[0].layers[0].name = format!("mutated_{}", i);
        history.commit(p);
    }

    // Undo all
    for _ in 0..30 {
        history.undo();
    }
    assert_eq!(history.current().compositions[0].layers[0].name, "L0");

    // Redo all
    for _ in 0..30 {
        history.redo();
    }
    assert_eq!(
        history.current().compositions[0].layers[0].name,
        "mutated_29"
    );
}

#[test]
fn undo_redo_drag_transaction_semantics() {
    // Simulates: commit a base state, then a drag-edit, then verify undo/redo
    let mut proj = Project::default();
    proj.compositions.clear();
    let mut comp = Composition::new("c".into(), "Test".into(), 100, 100, 30, 100);
    comp.layers.push(Layer::new(
        "l0".into(),
        "Layer0".into(),
        LayerType::Null,
        30,
    ));
    proj.compositions.push(comp);
    let mut history = ProjectHistory::new(proj);

    // Commit base state
    let mut p = history.current().clone();
    p.compositions[0].layers[0].name = "base".into();
    history.commit(p);

    // Simulate a drag: capture pre-edit, mutate, commit_drag_action
    let pre_edit = history.current().clone();
    let mut post_edit = pre_edit.clone();
    post_edit.compositions[0].layers[0].name = "after_drag".into();
    history.commit_drag_action(pre_edit, post_edit, "Drag Edit");

    assert_eq!(
        history.current().compositions[0].layers[0].name,
        "after_drag"
    );

    // Undo should restore the base state
    history.undo();
    assert_eq!(history.current().compositions[0].layers[0].name, "base");

    // Redo should restore the drag result
    history.redo();
    assert_eq!(
        history.current().compositions[0].layers[0].name,
        "after_drag"
    );
}

#[test]
fn undo_redo_drag_noop_creates_no_entry() {
    let mut proj = Project::default();
    proj.compositions.clear();
    let mut comp = Composition::new("c".into(), "Test".into(), 100, 100, 30, 100);
    comp.layers.push(Layer::new(
        "l0".into(),
        "Layer0".into(),
        LayerType::Null,
        30,
    ));
    proj.compositions.push(comp);
    let mut history = ProjectHistory::new(proj);

    // Commit base
    let mut p = history.current().clone();
    p.compositions[0].layers[0].name = "base".into();
    history.commit(p);
    let gen_before = history.generation();

    // Drag that produces no change (pre == post)
    let pre_edit = history.current().clone();
    let post_edit = pre_edit.clone();
    history.commit_drag_action(pre_edit, post_edit, "No-op Drag");

    assert_eq!(
        history.generation(),
        gen_before,
        "no-op drag should not create an entry"
    );
}

#[test]
fn undo_redo_separate_drags_create_separate_entries() {
    let mut proj = Project::default();
    proj.compositions.clear();
    let mut comp = Composition::new("c".into(), "Test".into(), 100, 100, 30, 100);
    comp.layers.push(Layer::new(
        "l0".into(),
        "Layer0".into(),
        LayerType::Null,
        30,
    ));
    proj.compositions.push(comp);
    let mut history = ProjectHistory::new(proj);

    // Drag 1: "base" -> "drag1"
    let pre1 = history.current().clone();
    let mut post1 = pre1.clone();
    post1.compositions[0].layers[0].name = "drag1".into();
    history.commit_drag_action(pre1, post1, "Drag 1");

    // Drag 2: "drag1" -> "drag2"
    let pre2 = history.current().clone();
    let mut post2 = pre2.clone();
    post2.compositions[0].layers[0].name = "drag2".into();
    history.commit_drag_action(pre2, post2, "Drag 2");

    // Undo drag 2 -> "drag1"
    history.undo();
    assert_eq!(history.current().compositions[0].layers[0].name, "drag1");

    // Undo drag 1 -> initial
    history.undo();
    assert_eq!(history.current().compositions[0].layers[0].name, "Layer0");

    // Redo drag 1 -> "drag1"
    history.redo();
    assert_eq!(history.current().compositions[0].layers[0].name, "drag1");

    // Redo drag 2 -> "drag2"
    history.redo();
    assert_eq!(history.current().compositions[0].layers[0].name, "drag2");
}

#[test]
fn stress_effect_slider_drag_undo_redo_chain() {
    use kagari_vfx::core::timeline::{Effect, EffectType};

    let mut project = Project::default();
    project.compositions.clear();
    let mut comp = Composition::new("c".into(), "Stress".into(), 100, 100, 10, 10);
    let mut layer = Layer::new(
        "l0".into(),
        "Layer0".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        10,
    );
    layer.effects.push(Effect {
        id: "blur_0".into(),
        name: "Gaussian Blur".into(),
        effect_type: EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(5.0),
        },
        enabled: true,
    });
    comp.layers.push(layer);
    project.compositions.push(comp);

    let mut history = ProjectHistory::new(project.clone());

    // Verify initial state has the effect
    assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);

    // Simulate 49 rapid slider drags on blur_radius (50 total entries with initial)
    for i in 1..=49u32 {
        let mut post = history.current().clone();
        {
            let effect = &mut post.compositions[0].layers[0].effects[0];
            if let EffectType::GaussianBlur { blur_radius } = &mut effect.effect_type {
                *blur_radius = Animatable::new_constant(i as f32);
            }
        }
        history.commit_action(post, &format!("Drag {}", i));
    }

    // Verify final state (49.0 because max_history_entries=50 trims the initial entry)
    {
        let effect = &history.current().compositions[0].layers[0].effects[0];
        if let EffectType::GaussianBlur { blur_radius } = &effect.effect_type {
            assert_eq!(blur_radius.value_at(0), 49.0);
        } else {
            panic!("Expected GaussianBlur");
        }
    }

    // Undo all 49 drags (back to initial state)
    for _ in 0..49 {
        history.undo();
    }
    {
        let effect = &history.current().compositions[0].layers[0].effects[0];
        if let EffectType::GaussianBlur { blur_radius } = &effect.effect_type {
            assert_eq!(blur_radius.value_at(0), 5.0);
        } else {
            panic!("Expected GaussianBlur");
        }
    }

    // Redo all 49 drags
    for _ in 0..49 {
        history.redo();
    }
    {
        let effect = &history.current().compositions[0].layers[0].effects[0];
        if let EffectType::GaussianBlur { blur_radius } = &effect.effect_type {
            assert_eq!(blur_radius.value_at(0), 49.0);
        } else {
            panic!("Expected GaussianBlur");
        }
    }
}

#[test]
fn stress_history_generation_monotonic_increment() {
    let project = Project::default();
    let mut history = ProjectHistory::new(project.clone());

    let gen0 = history.generation();

    // Commit should increment generation
    let mut p1 = project.clone();
    p1.compositions[0].name = "A".into();
    history.commit_action(p1, "A");
    assert!(history.generation() > gen0);
    let gen1 = history.generation();

    // Commit identical state should NOT increment generation
    let p1_again = history.current().clone();
    history.commit_action(p1_again, "A again");
    assert_eq!(history.generation(), gen1);

    // Undo should increment generation
    history.undo();
    assert!(history.generation() > gen1);
    let gen2 = history.generation();

    // Redo should increment generation
    history.redo();
    assert!(history.generation() > gen2);
}

#[test]
fn stress_parallel_cache_version_isolation() {
    use kagari_vfx::core::tile_cache::{TileCache, TileCoord};
    use std::thread;

    // Spawn 8 threads each doing 100 version bumps
    let handles: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..100 {
                    kagari_vfx::core::frame_cache::bump_version();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    // All tile caches using with_version should be unaffected by global bumps
    let mut cache_a = TileCache::with_version(16, 1024, 1);
    let mut cache_b = TileCache::with_version(16, 1024, 2);
    let coord = TileCoord { tx: 0, ty: 0 };
    cache_a.insert(0, coord, vec![1u8; 16]);
    cache_b.insert(0, coord, vec![2u8; 16]);

    // Both should still find their entries
    assert!(cache_a.get(0, coord).is_some());
    assert!(cache_b.get(0, coord).is_some());
    assert_eq!(cache_a.get(0, coord), Some(&[1u8; 16][..]));
    assert_eq!(cache_b.get(0, coord), Some(&[2u8; 16][..]));
}
