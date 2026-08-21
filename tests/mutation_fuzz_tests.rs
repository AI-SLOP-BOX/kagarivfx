//! Mutation-based fuzzing of project JSON parsing and expression evaluation.
//!
//! Takes valid seeds and applies deterministic random mutations, then verifies
//! that parsing never panics, rendering never panics, and the expression engine
//! always returns (fallback or value — never a crash or hang).

use aftereffects_oss::core::timeline::{Composition, Layer, LayerType, Project};
use aftereffects_oss::core::property::Animatable;
use aftereffects_oss::core::software_renderer::render_frame_to_pixels;
use aftereffects_oss::core::expression_engine::{build_engine, eval_f32};

fn seed_project_json() -> String {
    let mut comp = Composition::new("c1".into(), "Seed".into(), 32, 32, 30, 30);
    let mut l = Layer::new("l1".into(), "Solid".into(), LayerType::Solid { color: [0.5; 4] }, 30);
    l.transform.position = Animatable::new_animated(vec![
        aftereffects_oss::core::keyframe::Keyframe::new(0, [16.0, 16.0], aftereffects_oss::core::keyframe::InterpolationType::Linear),
        aftereffects_oss::core::keyframe::Keyframe::new(29, [20.0, 20.0], aftereffects_oss::core::keyframe::InterpolationType::Bezier {
            outgoing: aftereffects_oss::core::keyframe::BezierControlPoint::default(),
            incoming: aftereffects_oss::core::keyframe::BezierControlPoint::default(),
            custom_bezier: Some([0.33; 4]),
        }),
    ]);
    comp.layers.push(l);
    serde_json::to_string(&Project {
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
    })
    .unwrap()
}

/// Deterministic PRNG.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = ((self.0 >> 18) ^ self.0) >> 27;
        x.rotate_right((self.0 >> 59) as u32)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn fuzz_mutated_project_json_never_panics() {
    let seed = seed_project_json();
    let mut rng = Rng(0xDEADBEEF);

    for round in 0..500 {
        let mut bytes = seed.clone().into_bytes();
        let mutations = 1 + rng.below(8);
        for _ in 0..mutations {
            match rng.below(3) {
                // Byte flip
                0 => {
                    let i = rng.below(bytes.len());
                    bytes[i] = (rng.next() & 0xFF) as u8;
                }
                // Byte deletion
                1 if bytes.len() > 2 => {
                    let i = rng.below(bytes.len());
                    bytes.remove(i);
                }
                // Byte insertion
                _ => {
                    let i = rng.below(bytes.len());
                    bytes.insert(i, (rng.next() & 0xFF) as u8);
                }
            }
        }

        let payload = String::from_utf8_lossy(&bytes).to_string();
        // Parse must never panic (Ok or Err are both fine)
        if let Ok(project) = serde_json::from_str::<Project>(&payload) {
            // If it parses, rendering must never panic either
            for frame in [0u32, 15] {
                let pixels = render_frame_to_pixels(
                    &project.compositions[0],
                    frame,
                    32,
                    32,
                    0.0,
                    0,
                );
                assert_eq!(pixels.len(), 32 * 32 * 4);
            }
        }
    }
}

#[test]
fn fuzz_expression_scripts_never_crash_or_hang() {
    let engine = build_engine();
    // Token soup: operators, functions, nesting, huge numbers, deep parens
    let tokens: Vec<String> = vec![
        "1".into(), "0".into(), "-1".into(), "1e308".into(), "value".into(),
        "time".into(), "frame".into(), "fps".into(),
        "+".into(), "-".into(), "*".into(), "/".into(), "%".into(),
        "(".into(), ")".into(), "[".into(), "]".into(), ",".into(),
        "sin(".into(), "cos(".into(), "abs(".into(), "clamp(".into(),
        "wiggle(".into(), "linear(".into(),
        "ease(".into(), "loopOut()".into(), "__loop_out_cycle".into(),
        "thisComp.layer(\"x\").transform.position[0]".into(),
        ";".into(), "let x =".into(), "if true { 1 } else { 0 }".into(),
        "true".into(), "false".into(), "\"str\"".into(), "9".repeat(40),
    ];

    let mut rng = Rng(0xCAFEBABE);
    for round in 0..800 {
        let parts = 2 + rng.below(12);
        let mut script = String::new();
        for _ in 0..parts {
            script.push_str(&tokens[rng.below(tokens.len())]);
            script.push(' ');
        }

        // Must return some f32 (fallback base on error), never panic/hang.
        // max_operations caps runaway loops so this terminates quickly.
        let _result = eval_f32(&engine, &script, 42.0, 10, 30);
    }
}

#[test]
fn fuzz_deeply_nested_json_arrays() {
    // Deep nesting must not blow the recursion stack in serde
    for depth in [64usize, 512, 2000] {
        let mut payload = String::from("{\"compositions\": ");
        for _ in 0..depth {
            payload.push('[');
        }
        payload.push_str("]");
        for _ in 0..depth {
            payload.push(']');
        }
        payload.push('}');
        // Ok or Err — but no stack overflow / abort
        let _: Result<Project, _> = serde_json::from_str(&payload);
    }
}
