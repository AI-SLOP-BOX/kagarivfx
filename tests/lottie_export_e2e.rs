//! Integration tests for the Lottie / Bodymovin exporter.
//!
//! The exporter was historically dead code (never called from the app or
//! CLI); these tests pin down its output contract now that it is wired to
//! both the GUI export dialog and the `kagari lottie` CLI subcommand.

use kagari_vfx::core::lottie_exporter::export_project_to_json;
use kagari_vfx::core::timeline::{Layer, LayerType, Project};

fn project_with_layers(layer_count: usize, bg: [f32; 4]) -> Project {
    let mut comp = kagari_vfx::core::timeline::Composition::new(
        "c1".into(),
        "Main".into(),
        100,
        100,
        30,
        30,
    );
    comp.background_color = bg;
    for i in 0..layer_count {
        comp.layers.push(Layer::new(
            format!("l{i}"),
            format!("Solid {i}"),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            30,
        ));
    }
    Project {
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    }
}

#[test]
fn lottie_output_is_valid_json_with_comp_settings() {
    let project = project_with_layers(1, [0.0, 0.5, 1.0, 1.0]);
    let json = export_project_to_json(&project);

    let v: serde_json::Value =
        serde_json::from_str(&json).expect("export must produce parseable JSON");

    assert_eq!(v["fr"], 30, "fps must come from the composition");
    assert_eq!(v["w"], 100, "width must come from the composition");
    assert_eq!(v["h"], 100, "height must come from the composition");
    // Background colour is hex-encoded #rrggbb ([0, 0.5, 1] -> #0080ff).
    let bg = v["bg"].as_str().expect("bg must be a string");
    assert_eq!(bg, "#0080ff");
}

#[test]
fn lottie_export_includes_all_layers() {
    let project = project_with_layers(3, [0.0, 0.0, 0.0, 1.0]);
    let json = export_project_to_json(&project);
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let layers = v["layers"].as_array().expect("layers array");
    assert_eq!(layers.len(), 3, "every layer must be exported");
}

#[test]
fn lottie_export_is_deterministic() {
    let a = export_project_to_json(&project_with_layers(2, [0.1, 0.2, 0.3, 1.0]));
    let b = export_project_to_json(&project_with_layers(2, [0.1, 0.2, 0.3, 1.0]));
    assert_eq!(a, b, "same input must yield byte-identical Lottie JSON");
}
