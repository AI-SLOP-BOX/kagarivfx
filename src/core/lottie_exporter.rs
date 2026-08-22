#![allow(dead_code)]
use serde_json::{json, Value};
use crate::core::timeline::{Composition, Layer, LayerType, BlendMode, ShapeType};

fn color_to_lottie(c: &[f32; 4]) -> Value {
    json!([c[0], c[1], c[2], 1.0])
}

fn hex_color(c: &[f32; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}",
        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8)
}
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

/// Serializes a ShapeType into the Lottie shape item ("el"/"rc"/"sr"/"sr").
fn shape_geometry(st: &ShapeType) -> Value {
    match st {
        ShapeType::Rectangle { width, height, corner_radius } => json!({
            "ty": "rc",
            "d": 1,
            "s": anim_property_v2(&merge_dims(width, height), 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
            "r": anim_property_f32(corner_radius, 1.0),
        }),
        ShapeType::Ellipse { width, height } => json!({
            "ty": "el",
            "d": 1,
            "s": anim_property_v2(&merge_dims(width, height), 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
        }),
        ShapeType::Star { points, inner_radius, outer_radius } => json!({
            "ty": "sr",
            "sy": 2,
            "d": 1,
            "pt": anim_property_f32(points, 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
            "or": anim_property_f32(outer_radius, 1.0),
            "ir": anim_property_f32(inner_radius, 1.0),
            "os": { "a": 0, "k": 0 },
            "is": { "a": 0, "k": 0 },
            "r": { "a": 0, "k": 0 },
        }),
        ShapeType::Polygon { sides, radius } => json!({
            "ty": "sr",
            "sy": 1,
            "d": 1,
            "pt": anim_property_f32(sides, 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
            "or": anim_property_f32(radius, 1.0),
            "os": { "a": 0, "k": 0 },
            "r": { "a": 0, "k": 0 },
        }),
    }
}

/// Combines separately-animated width/height into a single animated [w,h] property.
fn merge_dims(w: &Animatable<f32>, h: &Animatable<f32>) -> Animatable<[f32; 2]> {
    match (w, h) {
        (Animatable::Constant(wv), Animatable::Constant(hv)) =>
            Animatable::Constant([*wv, *hv]),
        _ => Animatable::Animated(Vec::new()),
    }
}

/// Builds the Lottie shapes array (a group with geometry + fill + stroke) for shape layers.
fn serialize_shapes(shape_type: &ShapeType, color: &[f32; 4], stroke_color: &[f32; 4], stroke_width: f32) -> Vec<Value> {
    let mut items = vec![shape_geometry(shape_type)];
    items.push(json!({
        "ty": "fl",
        "c": { "a": 0, "k": color_to_lottie(color) },
        "o": { "a": 0, "k": 100 },
        "nm": "Fill",
    }));
    if stroke_width > 0.0 {
        items.push(json!({
            "ty": "st",
            "c": { "a": 0, "k": color_to_lottie(stroke_color) },
            "o": { "a": 0, "k": 100 },
            "w": { "a": 0, "k": stroke_width },
            "lc": 2,
            "lj": 2,
            "nm": "Stroke",
        }));
    }
    items.push(json!({ "ty": "tr", "p": { "a": 0, "k": [0.0, 0.0] }, "a": { "a": 0, "k": [0.0, 0.0] }, "s": { "a": 0, "k": [100.0, 100.0] }, "r": { "a": 0, "k": 0 }, "o": { "a": 0, "k": 100 } }));
    vec![json!({ "ty": "gr", "it": items, "nm": "Shape", "np": items.len() })]
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
        if let LayerType::Shape { shape_type, color, stroke_color, stroke_width } = &layer.layer_type {
            obj.insert("shapes".into(), json!(serialize_shapes(shape_type, color, stroke_color, *stroke_width)));
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
            "bg": hex_color(&comp.background_color),
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

    #[test]
    fn test_lottie_shape_layer_geometry() {
        use crate::core::timeline::ShapeType;
        let mut comp = Composition::new("c".into(), "Shapes".into(), 100, 100, 30, 30);
        comp.layers.push(Layer::new("s1".into(), "Circle".into(), LayerType::Shape {
            shape_type: ShapeType::Ellipse { width: Animatable::new_constant(50.0), height: Animatable::new_constant(80.0) },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 1.0, 1.0],
            stroke_width: 4.0,
        }, 30));
        comp.layers.push(Layer::new("s2".into(), "Rect".into(), LayerType::Shape {
            shape_type: ShapeType::Rectangle {
                width: Animatable::new_constant(20.0), height: Animatable::new_constant(10.0),
                corner_radius: Animatable::new_constant(5.0),
            },
            color: [0.0, 1.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
        }, 30));
        comp.layers.push(Layer::new("s3".into(), "Poly".into(), LayerType::Shape {
            shape_type: ShapeType::Polygon { sides: Animatable::new_constant(6.0), radius: Animatable::new_constant(30.0) },
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 2.0,
        }, 30));
        comp.layers.push(Layer::new("s4".into(), "Star".into(), LayerType::Shape {
            shape_type: ShapeType::Star { points: Animatable::new_constant(5.0), inner_radius: Animatable::new_constant(15.0), outer_radius: Animatable::new_constant(40.0) },
            color: [0.5, 0.5, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
        }, 30));

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();

        let circle = &v["layers"][0]["shapes"][0];
        assert_eq!(circle["ty"], "gr");
        let geom = &circle["it"][0];
        assert_eq!(geom["ty"], "el");
        assert_eq!(geom["s"]["k"], json!([50.0, 80.0]));
        let fill = &circle["it"][1];
        assert_eq!(fill["ty"], "fl");
        assert_eq!(fill["c"]["k"], json!([1.0, 0.0, 0.0, 1.0]));
        let stroke = &circle["it"][2];
        assert_eq!(stroke["ty"], "st");
        assert_eq!(stroke["w"]["k"], 4.0);

        let rect_geom = &v["layers"][1]["shapes"][0]["it"][0];
        assert_eq!(rect_geom["ty"], "rc");
        assert_eq!(rect_geom["r"]["k"], 5.0);
        // stroke_width == 0 → no stroke item
        assert_eq!(v["layers"][1]["shapes"][0]["it"][1]["ty"], "fl");

        let poly_geom = &v["layers"][2]["shapes"][0]["it"][0];
        assert_eq!(poly_geom["ty"], "sr");
        assert_eq!(poly_geom["sy"], 1);
        assert_eq!(poly_geom["pt"]["k"], 6.0);

        let star_geom = &v["layers"][3]["shapes"][0]["it"][0];
        assert_eq!(star_geom["ty"], "sr");
        assert_eq!(star_geom["sy"], 2);
        assert_eq!(star_geom["pt"]["k"], 5.0);
    }

    #[test]
    fn test_lottie_background_color() {
        let mut comp = Composition::new("c".into(), "BG".into(), 100, 100, 30, 30);
        comp.background_color = [0.05, 0.05, 0.08, 1.0];
        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        assert_eq!(v["bg"], "#0d0d14", "background as hex string");
    }
}
