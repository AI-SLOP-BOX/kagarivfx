//! Project automation scripting (Rhai) — the ExtendScript/JSX analogue.
//!
//! Exposes a mutating API over the active project so scripts can batch-create
//! comps, layers, keyframes and save results. Runs identically from the CLI
//! (`aevfx script file.rhai`) and from in-app tooling.
//!
//! Implementation note: Rhai evaluation is strictly single-threaded within
//! `run_script`, so a thread-local project pointer + log sink is the simplest
//! sound way to expose mutation without cloning the whole project per call.

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType, Project};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

thread_local! {
    /// Project currently being scripted (valid only inside run_script).
    static CURRENT_PROJECT: RefCell<*mut Project> = const { RefCell::new(std::ptr::null_mut()) };
}

#[derive(Debug)]
struct ProjectScope;

impl ProjectScope {
    fn enter(project: &mut Project) -> Result<Self, String> {
        CURRENT_PROJECT.with(|current| {
            let mut ptr = current.borrow_mut();
            if !ptr.is_null() {
                return Err("nested automation execution is not supported".to_string());
            }
            *ptr = project as *mut Project;
            Ok(Self)
        })
    }
}

impl Drop for ProjectScope {
    fn drop(&mut self) {
        CURRENT_PROJECT.with(|current| {
            *current.borrow_mut() = std::ptr::null_mut();
        });
    }
}

fn with_project<R>(f: impl FnOnce(&mut Project) -> R) -> Option<R> {
    CURRENT_PROJECT.with(|p| {
        let ptr = *p.borrow();
        if ptr.is_null() {
            None
        } else {
            Some(f(unsafe { &mut *ptr }))
        }
    })
}

/// Execute an automation script against the project.
/// Returns captured `log()` lines on success.
pub fn run_script(project: &mut Project, source: &str) -> Result<Vec<String>, String> {
    let log_sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let engine = build_engine(Arc::clone(&log_sink));

    let _scope = ProjectScope::enter(project)?;
    engine
        .run(source)
        .map_err(|e| format!("script error: {e}"))?;

    let logs = log_sink.lock().map(|g| g.clone()).unwrap_or_default();
    Ok(logs)
}

/// Convenience wrapper executing a `main() { ... }` program body.
pub fn run_script_main(project: &mut Project, body: &str) -> Result<Vec<String>, String> {
    let wrapped = format!("main();\nfn main() {{\n{body}\n}}");
    run_script(project, &wrapped)
}

fn gen_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("script_layer_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn parse_hex(hex: &str) -> [f32; 3] {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
        }
    }
    [1.0, 1.0, 1.0]
}

fn build_engine(log_sink: Arc<Mutex<Vec<String>>>) -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    engine.set_max_operations(2_000_000);

    engine.on_print({
        let sink = Arc::clone(&log_sink);
        move |msg: &str| {
            if let Ok(mut g) = sink.lock() {
                g.push(msg.to_string());
            }
        }
    });

    // ── Composition management ──
    engine.register_fn("log", move |msg: &str| {
        if let Ok(mut g) = log_sink.lock() {
            g.push(msg.to_string());
        }
    });
    engine.register_fn(
        "new_comp",
        |name: &str, w: i64, ht: i64, fps: i64, dur: i64| {
            with_project(|p| {
                let comp = Composition::new(
                    format!("comp_{}", p.compositions.len()),
                    name.to_string(),
                    w.max(1) as u32,
                    ht.max(1) as u32,
                    fps.max(1) as u32,
                    dur.max(1) as u32,
                );
                p.compositions.push(comp);
                p.active_composition_idx = p.compositions.len() - 1;
            });
        },
    );
    engine.register_fn("select_comp", |idx: i64| {
        with_project(|p| {
            if idx >= 0 && (idx as usize) < p.compositions.len() {
                p.active_composition_idx = idx as usize;
            }
        });
    });

    // ── Layer creation ──
    engine.register_fn("add_solid", |name: &str, color_hex: &str| -> String {
        with_project(|p| {
            let Some(idx) = p.compositions.get(p.active_composition_idx).map(|_| p.active_composition_idx) else {
                return String::new();
            };
            let c = parse_hex(color_hex);
            let comp_len = {
                p.compositions[idx].duration_frames
            };
            let mut layer = Layer::new(
                gen_id(),
                name.to_string(),
                LayerType::Solid {
                    color: [c[0], c[1], c[2], 1.0],
                },
                comp_len,
            );
            let comp = &mut p.compositions[idx];
            let cw = comp.width as f32;
            let ch = comp.height as f32;
            layer.transform.scale = Animatable::new_constant([cw, ch]);
            layer.transform.position = Animatable::new_constant([cw / 2.0, ch / 2.0]);
            let n = layer.name.clone();
            comp.layers.push(layer);
            n
        })
        .unwrap_or_default()
    });
    engine.register_fn("add_text", |name: &str, text: &str, size: i64| -> String {
        with_project(|p| {
            let Some(idx) = p.compositions.get(p.active_composition_idx).map(|_| p.active_composition_idx) else {
                return String::new();
            };
            let comp_len = {
                p.compositions[idx].duration_frames
            };
            let mut layer = Layer::new(
                gen_id(),
                name.to_string(),
                LayerType::Text {
                    text: text.to_string(),
                    font_size: size.max(4) as u32,
                    color: [1.0, 1.0, 1.0, 1.0],
                    font_family: "Inter".to_string(),
                    tracking: 0.0,
                    leading: 1.2,
                    align: 0,
                    stroke_color: [1.0, 1.0, 1.0, 1.0],
                    stroke_width: 0.0,
                    text_on_path: false,
                },
                comp_len,
            );
            let comp = &mut p.compositions[idx];
            let cw = comp.width as f32;
            let ch = comp.height as f32;
            layer.transform.scale = Animatable::new_constant([100.0, 100.0]);
            layer.transform.position = Animatable::new_constant([cw / 2.0, ch / 2.0]);
            let n = layer.name.clone();
            comp.layers.push(layer);
            n
        })
        .unwrap_or_default()
    });

    // ── Property animation ──
    engine.register_fn("set_position", |layer: &str, x: f64, y: f64| {
        with_project(|p| {
            let Some(idx) = p.compositions.get(p.active_composition_idx).map(|_| p.active_composition_idx) else {
                return;
            };
            if let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            {
                p.compositions[idx].layers[i].transform.position =
                    Animatable::new_constant([x as f32, y as f32]);
            }
        });
    });
    engine.register_fn("set_opacity", |layer: &str, pct: f64| {
        with_project(|p| {
            let Some(idx) = p.compositions.get(p.active_composition_idx).map(|_| p.active_composition_idx) else {
                return;
            };
            if let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            {
                p.compositions[idx].layers[i].transform.opacity =
                    Animatable::new_constant(pct.clamp(0.0, 100.0) as f32);
            }
        });
    });
    engine.register_fn("set_opacity", |layer: &str, pct: i64| {
        with_project(|p| {
            let Some(idx) = p.compositions.get(p.active_composition_idx).map(|_| p.active_composition_idx) else {
                return;
            };
            if let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            {
                p.compositions[idx].layers[i].transform.opacity =
                    Animatable::new_constant((pct as f64).clamp(0.0, 100.0) as f32);
            }
        });
    });
    engine.register_fn("key_position", |layer: &str, frame: i64, x: f64, y: f64| {
        with_project(|p| {
            let Some(idx) = p.compositions.get(p.active_composition_idx).map(|_| p.active_composition_idx) else {
                return;
            };
            let Some(i) = p.compositions[idx]
                .layers
                .iter()
                .position(|l| l.name == layer)
            else {
                return;
            };
            let kf = Keyframe::new(
                frame.max(0) as u32,
                [x as f32, y as f32],
                InterpolationType::Linear,
            );
            match &mut p.compositions[idx].layers[i].transform.position {
                Animatable::Animated(kfs) => {
                    kfs.push(kf);
                    kfs.sort_by_key(|k| k.frame);
                }
                Animatable::Constant(v0) => {
                    let v = *v0;
                    p.compositions[idx].layers[i].transform.position = Animatable::Animated(vec![
                        Keyframe::new(0, v, InterpolationType::Linear),
                        kf,
                    ]);
                }
            }
        });
    });

    // ── Persistence ──
    engine.register_fn("save_project", |path: &str| -> bool {
        with_project(|p| {
            let path = std::path::Path::new(path);
            crate::core::project_migration::save_project_atomic(p, path).is_ok()
        })
        .unwrap_or(false)
    });

    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_creation_script() {
        let mut project = Project::default();
        let script = r#"
            new_comp("Titles", 1920, 1080, 30, 60);
            add_solid("BG", "101018");
            add_text("Hello", "こんにちは", 72);
            set_opacity("BG", 80);
            log("done");
        "#;
        let logs = run_script(&mut project, script).expect("script runs");
        assert!(logs.contains(&"done".to_string()));
        let comp = project.active_composition();
        assert_eq!(comp.name, "Titles");
        assert_eq!(comp.layers.len(), 2, "solid + text added");
        assert_eq!(project.compositions.len(), 2, "default + new");
    }

    #[test]
    fn test_keyframe_script_animates_position() {
        let mut project = Project::default();
        let script = r#"
            add_text("T", "Slide", 48);
            key_position("T", 0, -200.0, 540.0);
            key_position("T", 30, 960.0, 540.0);
        "#;
        run_script(&mut project, script).unwrap();
        let comp = project.active_composition();
        let layer = comp.layers.last().unwrap();
        match &layer.transform.position {
            Animatable::Animated(kfs) => {
                // Implicit start keyframe from the static value + two scripted
                assert_eq!(kfs.len(), 3);
                assert_eq!(kfs.last().unwrap().frame, 30);
            }
            _ => panic!("expected animated position"),
        }
    }

    #[test]
    fn test_hex_parsing() {
        assert_eq!(parse_hex("#FF0000"), [1.0, 0.0, 0.0]);
        assert_eq!(parse_hex("00ff00"), [0.0, 1.0, 0.0]);
        assert_eq!(parse_hex("zzz"), [1.0, 1.0, 1.0], "fallback white");
    }

    #[test]
    fn test_error_propagates_and_unsets_pointer() {
        let mut project = Project::default();
        let err = run_script(&mut project, "this_is_not_a_fn();").unwrap_err();
        assert!(err.contains("script error"));
        // Pointer cleared: further calls safe
        assert!(with_project(|_| 1).is_none());
    }

    #[test]
    fn test_project_scope_clears_pointer_during_unwind() {
        let mut project = Project::default();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = ProjectScope::enter(&mut project).unwrap();
            panic!("simulated native callback panic");
        }));

        assert!(result.is_err());
        assert!(with_project(|_| 1).is_none());
    }

    #[test]
    fn test_nested_project_scope_is_rejected() {
        let mut first = Project::default();
        let mut second = Project::default();
        let _scope = ProjectScope::enter(&mut first).unwrap();

        let error = ProjectScope::enter(&mut second).unwrap_err();
        assert!(error.contains("nested automation"));
    }

    #[test]
    fn test_mutations_fail_closed_when_active_composition_is_missing() {
        let mut project = Project::default();
        project.compositions.clear();
        let script = r#"
            add_solid("BG", "ffffff");
            add_text("T", "safe", 32);
            set_opacity("BG", 50);
            key_position("T", 10, 1.0, 2.0);
        "#;
        assert!(run_script(&mut project, script).is_ok());
        assert!(project.compositions.is_empty());
    }
}
