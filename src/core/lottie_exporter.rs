#![allow(dead_code)]
use serde_json::{json, Value};
use crate::core::timeline::{Composition, Layer, LayerType, BlendMode};
use crate::core::property::Animatable;

/// Exporter for converting Aura Composition into industry-standard Lottie / Bodymovin JSON format.
pub struct LottieExporter;

/// Serializes an Animatable scalar/vec as a Lottie transform property.
/// Constant → {"a":0,"k":v}; Animated → {"a":1,"k":[{t,s}...]}.
fn anim_property_f32(prop: &Animatable<f32>, scale: f32) -> Value {
    match prop {
        Animatable::Constant(v) => json!({ "a": 0, "k": *v * scale }),
        Animatable::Animated(kfs) => {
            let keys: Vec<Value> = kfs.iter().map(|kf| json!({
                "t": kf.frame,
                "s": [kf.value * scale],
                "i": { "x": [0.667], "y": [1.0] },
                "o": { "x": [0.333], "y": [0.0] },
            })).collect();
            json!({ "a": 1, "k": keys })
        }
    }
}

fn anim_property_v2(prop: &Animatable<[f32; 2]>, scale: f32) -> Value {
    match prop {
        Animatable::Constant(v) => json!({ "a": 0, "k": [v[0] * scale, v[1] * scale] }),
        Animatable::Animated(kfs) => {
            let keys: Vec<Value> = kfs.iter().map(|kf| json!({
                "t": kf.frame,
                "s": [kf.value[0] * scale, kf.value[1] * scale],
                "i": { "x": [0.667, 0.667], "y": [1.0, 1.0] },
                "o": { "x": [0.333, 0.333], "y": [0.0, 0.0] },
            })).collect();
            json!({ "a": 1, "k": keys })
        }
    }
}

fn blend_mode_code(bm: &BlendMode) -> i32 {
    match bm {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay | BlendMode::SoftLight | BlendMode::HardLight => 3,
        BlendMode::Add => 4,
        BlendMode::Darken | BlendMode::Lighten | BlendMode::Difference
        | BlendMode::Exclusion | BlendMode::Divide | BlendMode::Subtract => 4,
    }
}

fn layer_type_code(lt: &LayerType) -> (i32, Value) {
    match lt {
        LayerType::Solid { color } => (
            1,
            json!({
                "sw": 512, "sh": 512,
                "sc": format!("#{:02x}{:02x}{:02x}",
                    (color[0] * 255.0) as u8, (color[1] * 255.0) as u8, (color[2] * 255.0) as u8),
            }),
        ),
        LayerType::Image { .. } => (2, json!({ "refId": "asset_0" })),
        LayerType::Null => (3, json!({})),
        LayerType::Text { .. } => (5, json!({})),
        _ => (4, json!({})), // Shape / PreComp / others default to shape
    }
}

fn serialize_layer(layer: &Layer, comp: &Composition, index: usize) -> Value {
    let (ty, extra) = layer_type_code(&layer.layer_type);
    let in_frame = layer.in_frame;
    let out_frame = layer.out_frame.max(comp.duration_frames.min(layer.out_frame));

    let mut l = json!({
        "ddd": 0,
        "ind": index + 1,
        "ty": ty,
        "nm": layer.name,
        "sr": 1,
        "ks": {
            "o": anim_property_f32(&layer.transform.opacity, 1.0),
            "r": anim_property_f32(&layer.transform.rotation, 1.0),
            "p": anim_property_v2(&layer.transform.position, 1.0),
            "a": anim_property_v2(&layer.transform.anchor_point, 1.0),
            "s": anim_property_v2(&layer.transform.scale, 1.0),
        },
        "ao": 0,
        "shapes": [],
        "ip": in_frame,
        "op": out_frame.max(in_frame + 1),
        "st": 0,
        "bm": blend_mode_code(&layer.blend_mode),
    });
    if let Some(obj) = l.as_object_mut() {
        if let Some(parent_id) = &layer.parent_id {
            // Map parent id to its layer index (Lottie uses indices)
            if let Some(pidx) = comp.layers.iter().position(|l| &l.id == parent_id) {
                obj.insert("parent".into(), json!(pidx + 1));
            }
        }
        for (k, v) in extra.as_object().cloned().unwrap_or_default() {
            obj.insert(k, v);
        }
        if layer.motion_blur {
            obj.insert("mb".into(), json!(1));
        }
    }
    l
}

impl LottieExporter {
    /// Serializes a Composition into a valid Lottie JSON string with animated transforms.
    pub fn export_to_json(comp: &Composition) -> String {
        let layers: Vec<Value> = comp.layers.iter().enumerate()
            .map(|(i, layer)| serialize_layer(layer, comp, i))
            .collect();

        let lottie_json = json!({
            "v": "5.7.4",
            "fr": comp.fps,
            "ip": 0,
            "op": comp.duration_frames,
            "w": comp.width,
            "h": comp.height,
            "nm": comp.name,
            "ddd": 0,
            "assets": [],
            "layers": layers,
        });

        serde_json::to_string_pretty(&lottie_json).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keyframe::{Keyframe, InterpolationType};

    #[test]
    fn test_lottie_export() {
        let comp = Composition::new("c1".into(), "TestComp".into(), 1920, 1080, 30, 300);
        let json_str = LottieExporter::export_to_json(&comp);

        assert!(json_str.contains("\"v\": \"5.7.4\""));
        assert!(json_str.contains("\"TestComp\""));
    }

    #[test]
    fn test_lottie_exports_animated_keyframes() {
        let mut comp = Composition::new("c".into(), "Anim".into(), 640, 360, 30, 60);
        let mut l = Layer::new("l1".into(), "Mover".into(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        l.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            Keyframe::new(30, [640.0, 360.0], InterpolationType::Linear),
        ]);
        comp.layers.push(l);

        let json_str = LottieExporter::export_to_json(&comp);
        let v: Value = serde_json::from_str(&json_str).expect("valid JSON");
        let layer = &v["layers"][0];
        assert_eq!(layer["ty"], 1, "solid layer type");
        assert_eq!(layer["ks"]["p"]["a"], 1, "position must be animated");
        let kfs = layer["ks"]["p"]["k"].as_array().expect("keyframe array");
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0]["t"], 0);
        assert_eq!(kfs[1]["s"][0], 640.0);
    }

    #[test]
    fn test_lottie_layer_types_and_parent() {
        let mut comp = Composition::new("c".into(), "Types".into(), 100, 100, 30, 30);
        let parent = Layer::new("p".into(), "ParentNull".into(), LayerType::Null, 30);
        let mut child = Layer::new("ch".into(), "Child".into(), LayerType::Text {
            text: "Hi".into(), font_size: 24, color: [1.0; 4],
            font_family: "Arial".into(), tracking: 0.0, leading: 1.0, align: 0,
            stroke_color: [0.0; 4], stroke_width: 0.0, text_on_path: false,
        }, 30);
        child.parent_id = Some("p".into());
        comp.layers.push(parent);
        comp.layers.push(child);

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        assert_eq!(v["layers"][0]["ty"], 3, "null type");
        assert_eq!(v["layers"][1]["ty"], 5, "text type");
        assert_eq!(v["layers"][1]["parent"], 1, "parent mapped to index");
    }
}
