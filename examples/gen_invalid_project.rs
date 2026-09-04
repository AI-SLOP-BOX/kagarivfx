use kagari_vfx::core::property::Animatable;
/// Generates a deliberately broken project for validator robustness testing.
use kagari_vfx::core::timeline::{Composition, Layer, LayerType, Project};

fn main() {
    let mut comp = Composition::new("comp1".into(), "CycleTest".into(), 64, 64, 30, 30);
    let mut bg = Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid { color: [0.2; 4] },
        30,
    );
    bg.transform.position = Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(bg);

    // Sub-comp B with a PreComp layer pointing back to comp1 (cycle)
    let mut sub_b = Composition::new("B".into(), "SubB".into(), 64, 64, 30, 30);
    let back = Layer::new(
        "pcb".into(),
        "BackToRoot".into(),
        LayerType::PreComp {
            comp_id: "comp1".into(),
        },
        30,
    );
    sub_b.layers.push(back);
    comp.sub_compositions.push(sub_b);

    // PreComp layer referencing B
    let mut pc = Layer::new(
        "pc1".into(),
        "Nested".into(),
        LayerType::PreComp {
            comp_id: "B".into(),
        },
        30,
    );
    pc.transform.position = Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(pc);

    // Missing pre-comp reference
    let missing = Layer::new(
        "miss".into(),
        "MissingRef".into(),
        LayerType::PreComp {
            comp_id: "GHOST".into(),
        },
        30,
    );
    comp.layers.push(missing);

    // Suspicious scale
    let mut bad = Layer::new(
        "bad".into(),
        "BadScale".into(),
        LayerType::Solid { color: [1.0; 4] },
        30,
    );
    bad.transform.scale = Animatable::new_constant([1e9, 1e9]);
    comp.layers.push(bad);

    // Parent cycle
    let mut a = Layer::new("pa".into(), "ParentA".into(), LayerType::Null, 30);
    a.parent_id = Some("pb".into());
    let mut b = Layer::new("pb".into(), "ParentB".into(), LayerType::Null, 30);
    b.parent_id = Some("pa".into());
    comp.layers.push(a);
    comp.layers.push(b);

    let project = Project {
        use_gpu_compute: false,
        compositions: vec![comp],
        active_composition_idx: 0,
        assets: Vec::new(),
    };
    let json = serde_json::to_string_pretty(&project).expect("serialize");
    std::fs::write("test_invalid.json", json).expect("write");
    println!("Wrote test_invalid.json");
}
