#![allow(dead_code)]
use serde_json::{json, Value};
use crate::core::timeline::{
    Composition, Layer, LayerType, BlendMode, ShapeType, TrackMatteMode, Project,
};
use crate::core::mask::{Mask, MaskMode};
use crate::core::mask::MaskPath;
use crate::core::keyframe::{Keyframe, InterpolationType};

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
        | BlendMode::Exclusion | BlendMode::Divide | BlendMode::Subtract
        | BlendMode::ColorBurn | BlendMode::LinearBurn | BlendMode::VividLight
        | BlendMode::ColorDodge | BlendMode::LinearDodge
        | BlendMode::Color | BlendMode::Hue | BlendMode::Saturation | BlendMode::Luminosity => 4,
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
        ShapeType::FreeformBezier { points, .. } => {
            // Convert points to Lottie shape vertices
            let verts: Vec<Value> = points.iter()
                .map(|p| json!([p[0], p[1]]))
                .collect();
            json!({
                "ty": "sh",
                "d": 1,
                "ks": { "a": 0, "k": { "i": Vec::<Value>::new(), "o": Vec::<Value>::new(), "v": verts, "c": true } },
            })
        }
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
        LayerType::PreComp { comp_id } => (0, json!({ "refId": comp_id })),
        _ => (4, json!({})), // Shape / others default to shape
    }
}

/// Lottie matte type code from the app's TrackMatteMode.
/// 1=Alpha 2=AlphaInv 3=Luma 4=LumaInv (Lottie bodymovin spec).
fn track_matte_code(mode: &TrackMatteMode) -> i32 {
    match mode {
        TrackMatteMode::None => 0,
        TrackMatteMode::AlphaMatte => 1,
        TrackMatteMode::AlphaMatteInverted => 2,
        TrackMatteMode::LumaMatte => 3,
        TrackMatteMode::LumaMatteInverted => 4,
    }
}

/// Serializes a Text layer into the Lottie text document property ("t").
/// Without this the text content itself would be lost in export.
///
/// Key mapping follows the Bodymovin spec: "t" carries the TEXT STRING while
/// "tr" carries tracking — a duplicate "t" key would silently overwrite the
/// text with the tracking value.
fn text_document(layer_type: &LayerType) -> Option<Value> {
    if let LayerType::Text { text, font_size, color, font_family, tracking, leading, align, .. } = layer_type {
        // Alignment mapping: app 0=Left 1=Center 2=Right → Lottie j 0=Left 2=Center 1=Right.
        let justify = match align {
            1 => 2,
            2 => 1,
            _ => 0,
        };
        Some(json!({
            "d": { "k": [{ "s": {
                "t": text,
                "f": font_family,
                "s": font_size,
                "fc": [color[0], color[1], color[2]],
                "j": justify,
                "tr": tracking,
                "lh": *leading * (*font_size as f32),
            }, "t": 0 }] },
            "p": {},
            "m": { "g": 1, "a": { "a": 0, "k": [0.0, 0.0] } },
            "a": [],
        }))
    } else {
        None
    }
}

/// Lottie mask mode letter from the app's MaskMode.
fn mask_mode_code(mode: &MaskMode, enabled: bool) -> &'static str {
    if !enabled {
        return "n";
    }
    match mode {
        MaskMode::None => "n",
        MaskMode::Add => "a",
        MaskMode::Subtract => "s",
        MaskMode::Intersect => "i",
        MaskMode::Lighten => "l",
        MaskMode::Darken => "d",
        MaskMode::Difference => "f",
    }
}

/// Builds a Bodymovin bezier shape object (`{c,v,i,o}`) from position /
/// in-tangent / out-tangent triplets.
fn bezier_shape(triplets: &[([f32; 2], [f32; 2], [f32; 2])], is_closed: bool) -> Value {
    let mut v = Vec::with_capacity(triplets.len());
    let mut ti = Vec::with_capacity(triplets.len());
    let mut to = Vec::with_capacity(triplets.len());
    for (pos, tin, tout) in triplets {
        v.push(json!([pos[0], pos[1]]));
        ti.push(json!([tin[0], tin[1]]));
        to.push(json!([tout[0], tout[1]]));
    }
    json!({ "c": is_closed, "v": v, "i": ti, "o": to })
}

/// Zips mask path vertices with their tangent handles (zero handles when the
/// path has none).
fn mask_triplets(path: &MaskPath, verts: &[[f32; 2]]) -> Vec<([f32; 2], [f32; 2], [f32; 2])> {
    verts
        .iter()
        .enumerate()
        .map(|(i, pos)| match &path.tangents {
            Some(ts) if ts.len() > i => (*pos, ts[i].0, ts[i].1),
            _ => (*pos, [0.0, 0.0], [0.0, 0.0]),
        })
        .collect()
}

/// Serializes one scalar Animatable as a Lottie {"a","k"} property.
fn anim_property_scalar(prop: &Animatable<f32>) -> Value {
    match prop {
        Animatable::Constant(v) => json!({ "a": 0, "k": *v }),
        Animatable::Animated(kfs) => json!({
            "a": 1,
            "k": kfs.iter().map(|kf| json!({ "t": kf.frame, "s": [kf.value] })).collect::<Vec<_>>(),
        }),
    }
}

/// Serializes one mask into a bodymovin masksProperties entry. Animated
/// vertex tracks become shape keyframes ("pt"."a" = 1).
fn serialize_mask(mask: &Mask) -> Value {
    let pt = match &mask.path.vertices {
        Animatable::Constant(verts) => json!({
            "a": 0,
            "k": bezier_shape(&mask_triplets(&mask.path, verts), mask.path.is_closed),
        }),
        Animatable::Animated(kfs) => json!({
            "a": 1,
            "k": kfs.iter().map(|kf| json!({
                "t": kf.frame,
                "s": [bezier_shape(&mask_triplets(&mask.path, &kf.value), mask.path.is_closed)],
            })).collect::<Vec<_>>(),
        }),
    };
    json!({
        "inv": mask.inverted,
        "mode": mask_mode_code(&mask.mode, mask.enabled),
        "pt": pt,
        "o": anim_property_scalar(&mask.opacity),
        "x": anim_property_scalar(&mask.expansion),
        "nm": mask.name,
    })
}

/// `Some((hasMask, masksProperties))` payload when the layer carries masks.
fn masks_properties(masks: &[Mask]) -> Option<(Value, Value)> {
    if masks.is_empty() {
        return None;
    }
    let props: Vec<Value> = masks.iter().map(serialize_mask).collect();
    Some((json!(true), json!(props)))
}

fn serialize_layer(layer: &Layer, comp: &Composition, index: usize, is_matte_source: bool) -> Value {
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
        // Precomp placeholders carry the containing comp's dimensions so
        // players can size the referenced composition slot.
        if matches!(layer.layer_type, LayerType::PreComp { .. }) {
            obj.insert("w".into(), json!(comp.width));
            obj.insert("h".into(), json!(comp.height));
        }
        if layer.motion_blur {
            obj.insert("mb".into(), json!(1));
        }
        if let LayerType::Shape { shape_type, color, stroke_color, stroke_width } = &layer.layer_type {
            obj.insert("shapes".into(), json!(serialize_shapes(shape_type, color, stroke_color, *stroke_width)));
        }
        if let Some(doc) = text_document(&layer.layer_type) {
            obj.insert("t".into(), doc);
        }
        // Track mattes: this layer consumes the layer above it as its matte.
        if !matches!(layer.track_matte, TrackMatteMode::None) {
            obj.insert("tt".into(), json!(track_matte_code(&layer.track_matte)));
        }
        // The layer directly above a matte consumer becomes the matte source.
        if is_matte_source {
            obj.insert("td".into(), json!(1));
        }
        if let Some((has_mask, props)) = masks_properties(&layer.masks) {
            obj.insert("hasMask".into(), has_mask);
            obj.insert("masksProperties".into(), props);
        }
    }
    l
}

impl LottieExporter {
    /// Serializes a Composition into a valid Lottie JSON string with animated transforms.
    pub fn export_to_json(comp: &Composition) -> String {
        let layers: Vec<Value> = comp.layers.iter().enumerate()
            .map(|(i, layer)| {
                // A layer is a matte source when the NEXT layer consumes it.
                let is_source = comp.layers.get(i + 1)
                    .is_some_and(|next| !matches!(next.track_matte, TrackMatteMode::None));
                serialize_layer(layer, comp, i, is_source)
            })
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

// ───────────────────── Project / Precomp Tree Export ─────────────────────

/// Flattens every composition reachable from the project (top-level list plus
/// nested `sub_compositions`, depth-first).
fn flatten_comps(project: &Project) -> Vec<&Composition> {
    let mut out = Vec::new();
    let mut stack: Vec<&Composition> = project.compositions.iter().collect();
    while let Some(c) = stack.pop() {
        for sub in &c.sub_compositions {
            stack.push(sub);
        }
        out.push(c);
    }
    out
}

fn serialize_comp_layers(comp: &Composition) -> Vec<Value> {
    comp.layers
        .iter()
        .enumerate()
        .map(|(i, layer)| {
            let is_source = comp
                .layers
                .get(i + 1)
                .is_some_and(|next| !matches!(next.track_matte, TrackMatteMode::None));
            serialize_layer(layer, comp, i, is_source)
        })
        .collect()
}

/// Exports the active composition together with every precomp it references
/// (transitively) as bodymovin assets. Precomp layers become `ty:0` slots
/// pointing at their asset id; unreferenced comps are omitted; missing
/// references keep their placeholder but emit no asset.
pub fn export_project_to_json(project: &Project) -> String {
    let Some(root) = project.compositions.get(project.active_composition_idx) else {
        return json!({ "error": "no active composition" }).to_string();
    };

    let all = flatten_comps(project);
    let mut emitted: Vec<String> = vec![root.id.clone()];
    let mut assets: Vec<Value> = Vec::new();

    let mut queue: Vec<String> = root
        .layers
        .iter()
        .filter_map(|l| match &l.layer_type {
            LayerType::PreComp { comp_id } => Some(comp_id.clone()),
            _ => None,
        })
        .collect();

    while let Some(id) = queue.pop() {
        if emitted.iter().any(|e| e == &id) {
            continue;
        }
        if let Some(c) = all.iter().find(|c| c.id == id) {
            emitted.push(id.clone());
            assets.push(json!({
                "id": c.id,
                "nm": c.name,
                "fr": c.fps,
                "ip": 0,
                "op": c.duration_frames,
                "w": c.width,
                "h": c.height,
                "layers": serialize_comp_layers(c),
            }));
            for l in &c.layers {
                if let LayerType::PreComp { comp_id } = &l.layer_type {
                    queue.push(comp_id.clone());
                }
            }
        }
    }

    // Root layers reuse the same serializer as assets.
    let layers = serialize_comp_layers(root);
    let lottie_json = json!({
        "v": "5.7.4",
        "fr": root.fps,
        "ip": 0,
        "op": root.duration_frames,
        "w": root.width,
        "h": root.height,
        "nm": root.name,
        "ddd": 0,
        "bg": hex_color(&root.background_color),
        "assets": assets,
        "layers": layers,
    });
    serde_json::to_string_pretty(&lottie_json).unwrap_or_default()
}

// ─────────────────────── Lottie / Bodymovin Import ───────────────────────

fn hex_to_rgba(s: &str) -> [f32; 4] {
    let h = s.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0);
    match h.len() {
        8 => [
            ((n >> 24) & 255) as f32 / 255.0,
            ((n >> 16) & 255) as f32 / 255.0,
            ((n >> 8) & 255) as f32 / 255.0,
            (n & 255) as f32 / 255.0,
        ],
        _ => [
            ((n >> 16) & 255) as f32 / 255.0,
            ((n >> 8) & 255) as f32 / 255.0,
            (n & 255) as f32 / 255.0,
            1.0,
        ],
    }
}

/// Reads a static 2-component value: `{"a":0,"k":[x,y]}` or bare `[x,y]`.
fn k_arr2(v: Option<&Value>) -> [f32; 2] {
    let Some(val) = v else { return [0.0; 2]; };
    let k = val.get("k").unwrap_or(val);
    let Some(arr) = k.as_array() else { return [0.0; 2]; };
    if arr.len() < 2 {
        return [0.0; 2];
    }
    [
        arr[0].as_f64().unwrap_or(0.0) as f32,
        arr[1].as_f64().unwrap_or(0.0) as f32,
    ]
}

/// Reads a static scalar value: `{"a":0,"k":v}`, `"k":v` or bare `v`.
fn k_f32(v: Option<&Value>, default: f32) -> f32 {
    let Some(val) = v else { return default; };
    let k = val.get("k").unwrap_or(val);
    k.as_f64().map(|f| f as f32).unwrap_or(default)
}

/// Reads fill/stroke color `[r,g,b]` or `[r,g,b,a]` (0..1 floats).
fn rgb_from_k(v: Option<&Value>) -> [f32; 4] {
    let Some(val) = v else { return [0.0; 4]; };
    let k = val.get("k").unwrap_or(val);
    let Some(arr) = k.as_array() else { return [0.0; 4]; };
    let comp = |i: usize| arr.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
    [comp(0), comp(1), comp(2), if arr.len() > 3 { comp(3) } else { 1.0 }]
}

/// Inverse of the exporter's anim_property_* writers: `{"a","k"}` → Animatable.
/// Terminal bodymovin keyframes lacking "s" hold the previous value.
fn parse_anim_value_f32(v: Option<&Value>) -> Animatable<f32> {
    let Some(val) = v else { return Animatable::new_constant(0.0); };
    if val.get("a").and_then(Value::as_i64) == Some(1) {
        if let Some(list) = val.get("k").and_then(Value::as_array) {
            let mut kfs: Vec<Keyframe<f32>> = Vec::new();
            let mut prev = 0.0f32;
            for e in list {
                let frame = e.get("t").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
                let value = e
                    .get("s")
                    .and_then(Value::as_array)
                    .and_then(|s| s.first())
                    .and_then(Value::as_f64)
                    .map(|f| f as f32)
                    .unwrap_or(prev);
                prev = value;
                kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
            }
            if !kfs.is_empty() {
                return Animatable::Animated(kfs);
            }
        }
    }
    Animatable::new_constant(val.get("k").and_then(Value::as_f64).unwrap_or(0.0) as f32)
}

fn parse_anim_value_v2(v: Option<&Value>) -> Animatable<[f32; 2]> {
    let Some(val) = v else { return Animatable::new_constant([0.0, 0.0]); };
    if val.get("a").and_then(Value::as_i64) == Some(1) {
        if let Some(list) = val.get("k").and_then(Value::as_array) {
            let mut kfs: Vec<Keyframe<[f32; 2]>> = Vec::new();
            let mut prev = [0.0f32; 2];
            for e in list {
                let frame = e.get("t").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
                let value = e
                    .get("s")
                    .and_then(Value::as_array)
                    .map(|s| {
                        // Handle both s: [[x, y]] and s: [x, y] formats
                        if let Some(first) = s.first() {
                            if let Some(arr) = first.as_array() {
                                // Nested: s: [[x, y]]
                                [
                                    arr.first().and_then(Value::as_f64).unwrap_or(prev[0] as f64) as f32,
                                    arr.get(1).and_then(Value::as_f64).unwrap_or(prev[1] as f64) as f32,
                                ]
                            } else {
                                // Flat: s: [x, y]
                                [
                                    s.first().and_then(Value::as_f64).unwrap_or(prev[0] as f64) as f32,
                                    s.get(1).and_then(Value::as_f64).unwrap_or(prev[1] as f64) as f32,
                                ]
                            }
                        } else {
                            prev
                        }
                    })
                    .unwrap_or(prev);
                prev = value;
                kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
            }
            if !kfs.is_empty() {
                return Animatable::Animated(kfs);
            }
        }
    }
    Animatable::new_constant(k_arr2(Some(val)))
}

fn parse_transform(ks: &Value) -> crate::core::timeline::Transform2D {
    crate::core::timeline::Transform2D {
        anchor_point: parse_anim_value_v2(ks.get("a")),
        position: parse_anim_value_v2(ks.get("p")),
        scale: parse_anim_value_v2(ks.get("s")),
        rotation: parse_anim_value_f32(ks.get("r")),
        opacity: parse_anim_value_f32(ks.get("o")),
        ..Default::default()
    }
}

fn blend_from_code(code: i64) -> BlendMode {
    match code {
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Overlay,
        4 => BlendMode::Add,
        _ => BlendMode::Normal,
    }
}

fn matte_from_code(code: i64) -> TrackMatteMode {
    match code {
        1 => TrackMatteMode::AlphaMatte,
        2 => TrackMatteMode::AlphaMatteInverted,
        3 => TrackMatteMode::LumaMatte,
        4 => TrackMatteMode::LumaMatteInverted,
        _ => TrackMatteMode::None,
    }
}

/// Reconstructs a Text layer from one bodymovin text-document keyframe.
fn text_layer_type(doc: &Value) -> LayerType {
    let s = doc.get("s").cloned().unwrap_or(Value::Null);
    let size = s.get("s").and_then(Value::as_f64).unwrap_or(24.0).max(1.0) as u32;
    let lh = s.get("lh").and_then(Value::as_f64).unwrap_or(size as f64 * 1.2);
    // Lottie j 0=Left 2=Center 1=Right → app align 0=Left 1=Center 2=Right.
    let align = match s.get("j").and_then(Value::as_i64).unwrap_or(0) {
        2 => 1u32,
        1 => 2,
        _ => 0,
    };
    LayerType::Text {
        text: s.get("t").and_then(Value::as_str).unwrap_or("").to_string(),
        font_size: size,
        color: rgb_from_k(s.get("fc")),
        font_family: s.get("f").and_then(Value::as_str).unwrap_or("Inter").to_string(),
        tracking: s.get("tr").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        leading: (lh / (size as f64).max(1.0)) as f32,
        align: align as usize,
        stroke_color: [0.0; 4],
        stroke_width: 0.0,
        text_on_path: false,
    }
}

/// Parses the first shape group into our ShapeType plus fill/stroke.
/// Animated shape sizes degrade to their first keyframe value.
fn parse_shape_group(shapes: &Value) -> Option<(ShapeType, [f32; 4], [f32; 4], f32)> {
    for g in shapes.as_array()? {
        let Some(it) = g.get("it").and_then(Value::as_array) else { continue; };
        let mut geo: Option<ShapeType> = None;
        let mut fill = [0.0f32; 4];
        let mut stroke = [0.0f32; 4];
        let mut stroke_w = 0.0f32;
        for item in it {
            match item.get("ty").and_then(Value::as_str).unwrap_or("") {
                "el" => {
                    let s = k_arr2(item.get("s"));
                    geo = Some(ShapeType::Ellipse {
                        width: Animatable::new_constant(s[0]),
                        height: Animatable::new_constant(s[1]),
                    });
                }
                "rc" => {
                    let s = k_arr2(item.get("s"));
                    geo = Some(ShapeType::Rectangle {
                        width: Animatable::new_constant(s[0]),
                        height: Animatable::new_constant(s[1]),
                        corner_radius: Animatable::new_constant(k_f32(item.get("r"), 0.0)),
                    });
                }
                "sr" => {
                    let sy = item.get("sy").and_then(Value::as_i64).unwrap_or(1);
                    let points = k_f32(item.get("pt"), 5.0);
                    let outer = k_f32(item.get("or"), 50.0);
                    let inner = k_f32(item.get("ir"), outer * 0.5);
                    geo = Some(if sy == 2 {
                        ShapeType::Star {
                            points: Animatable::new_constant(points),
                            inner_radius: Animatable::new_constant(inner),
                            outer_radius: Animatable::new_constant(outer),
                        }
                    } else {
                        ShapeType::Polygon {
                            sides: Animatable::new_constant(points),
                            radius: Animatable::new_constant(outer),
                        }
                    });
                }
                "fl" => fill = rgb_from_k(item.get("c")),
                "st" => {
                    stroke = rgb_from_k(item.get("c"));
                    stroke_w = k_f32(item.get("w"), 0.0);
                }
                _ => {}
            }
        }
        if let Some(shape_type) = geo {
            return Some((shape_type, fill, stroke, stroke_w));
        }
    }
    None
}

fn pt2(v: Option<&Value>) -> [f32; 2] {
    let Some(a) = v.and_then(Value::as_array) else { return [0.0; 2]; };
    [
        a.first().and_then(Value::as_f64).unwrap_or(0.0) as f32,
        a.get(1).and_then(Value::as_f64).unwrap_or(0.0) as f32,
    ]
}

fn shape_vertices(sh: &Value) -> Vec<[f32; 2]> {
    sh.get("v")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|p| pt2(Some(p))).collect())
        .unwrap_or_default()
}

/// Per-vertex tangent handles; None when every handle is zero so that
/// export→import→export round-trips stay lossless.
fn shape_tangents(sh: &Value, vert_count: usize) -> Option<Vec<([f32; 2], [f32; 2])>> {
    let ins: Vec<[f32; 2]> = sh
        .get("i")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|p| pt2(Some(p))).collect())
        .unwrap_or_else(|| vec![[0.0; 2]; vert_count]);
    let outs: Vec<[f32; 2]> = sh
        .get("o")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|p| pt2(Some(p))).collect())
        .unwrap_or_else(|| vec![[0.0; 2]; vert_count]);
    let mut any = false;
    let pairs: Vec<([f32; 2], [f32; 2])> = (0..vert_count)
        .map(|i| {
            let tin = ins.get(i).copied().unwrap_or([0.0; 2]);
            let tout = outs.get(i).copied().unwrap_or([0.0; 2]);
            if tin != [0.0; 2] || tout != [0.0; 2] {
                any = true;
            }
            (tin, tout)
        })
        .collect();
    if any {
        Some(pairs)
    } else {
        None
    }
}

fn import_mask(m: &Value, idx: usize) -> Mask {
    let letter = m.get("mode").and_then(Value::as_str).unwrap_or("a");
    let enabled = letter != "n";
    let mode = match letter {
        "s" => MaskMode::Subtract,
        "i" => MaskMode::Intersect,
        "l" => MaskMode::Lighten,
        "d" => MaskMode::Darken,
        "f" => MaskMode::Difference,
        _ => {
            if enabled {
                MaskMode::Add
            } else {
                MaskMode::None
            }
        }
    };

    let pt = m.get("pt");
    let is_closed = pt
        .and_then(|p| p.get("k"))
        .and_then(|k| k.get("c"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let animated = pt.and_then(|p| p.get("a")).and_then(Value::as_i64) == Some(1);

    let vertices: Animatable<Vec<[f32; 2]>> = if animated {
        let mut kfs: Vec<Keyframe<Vec<[f32; 2]>>> = Vec::new();
        if let Some(list) = pt.and_then(|p| p.get("k")).and_then(Value::as_array) {
            let mut prev: Vec<[f32; 2]> = Vec::new();
            for e in list {
                let frame = e.get("t").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
                let value = e
                    .get("s")
                    .and_then(Value::as_array)
                    .and_then(|s| s.first())
                    .map(shape_vertices)
                    .unwrap_or_else(|| prev.clone());
                prev = value.clone();
                kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
            }
        }
        if kfs.is_empty() {
            Animatable::new_constant(Vec::new())
        } else {
            Animatable::Animated(kfs)
        }
    } else {
        let verts = pt
            .and_then(|p| p.get("k"))
            .map(shape_vertices)
            .unwrap_or_default();
        Animatable::new_constant(verts)
    };

    // Tangents from whichever representation carries vertices. Animated
    // handles are rare; positions carry the essential shape there.
    let tangents = if animated {
        None
    } else {
        pt.and_then(|p| p.get("k")).and_then(|sh| {
            let n = shape_vertices(sh).len();
            shape_tangents(sh, n)
        })
    };

    Mask {
        id: format!("mask_{idx}"),
        name: m.get("nm").and_then(Value::as_str).unwrap_or("Mask").to_string(),
        enabled,
        mode,
        path: MaskPath {
            vertices,
            tangents,
            is_closed,
        },
        feather: Animatable::new_constant(0.0),
        opacity: parse_anim_value_f32(m.get("o")),
        expansion: parse_anim_value_f32(m.get("x")),
        inverted: m.get("inv").and_then(Value::as_bool).unwrap_or(false),
        wiggle: None,
    }
}

/// Imports Bodymovin/Lottie JSON into a [`Composition`]. Unknown constructs
/// degrade gracefully to null layers; invalid JSON yields None.
pub struct LottieImporter;

impl LottieImporter {
    pub fn import_from_file(path: &str) -> Option<Composition> {
        let json = std::fs::read_to_string(path).ok()?;
        Self::import_from_str(&json)
    }

    pub fn import_from_str(json: &str) -> Option<Composition> {
        let v: Value = serde_json::from_str(json).ok()?;
        Self::from_value(&v)
    }

    fn from_value(v: &Value) -> Option<Composition> {
        Self::parse_comp(v, None)
    }

    /// Builds one Composition from a bodymovin comp/asset object.
    /// `id_override` lets asset entries keep their published bodymovin id.
    fn parse_comp(v: &Value, id_override: Option<&str>) -> Option<Composition> {
        let layers_src = v.get("layers")?.as_array()?;
        let fps = v.get("fr").and_then(Value::as_f64).unwrap_or(30.0).round().max(1.0) as u32;
        let ip = v.get("ip").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
        let op = v.get("op").and_then(Value::as_f64).unwrap_or(ip + 1.0);
        let duration = ((op - ip).round() as u32).max(1);
        let width = v.get("w").and_then(Value::as_u64).unwrap_or(1920).clamp(1, 16384) as u32;
        let height = v.get("h").and_then(Value::as_u64).unwrap_or(1080).clamp(1, 16384) as u32;
        let name = v.get("nm").and_then(Value::as_str).unwrap_or("Lottie Import");
        let comp_id = id_override
            .map(str::to_string)
            .unwrap_or_else(|| format!("lottie_{}", name.replace(' ', "_")));

        let mut comp = Composition::new(comp_id, name.to_string(), width, height, fps, duration);
        comp.background_color =
            hex_to_rgba(v.get("bg").and_then(Value::as_str).unwrap_or("#000000"));

        let mut inds: Vec<i64> = Vec::with_capacity(layers_src.len());
        for (i, lj) in layers_src.iter().enumerate() {
            let ty = lj.get("ty").and_then(Value::as_i64).unwrap_or(3);
            let lname = lj.get("nm").and_then(Value::as_str).unwrap_or("Layer").to_string();
            let id = format!("imp_l{i}");

            let mut layer = match ty {
                1 => Layer::new(id, lname, LayerType::Solid {
                    color: hex_to_rgba(lj.get("sc").and_then(Value::as_str).unwrap_or("#ffffff")),
                }, duration),
                2 => Layer::new(id, lname, LayerType::Image {
                    path: lj.get("refId").and_then(Value::as_str).unwrap_or("missing_asset").to_string(),
                }, duration),
                5 => {
                    let doc = lj.get("t")
                        .and_then(|t| t.get("d"))
                        .and_then(|d| d.get("k"))
                        .and_then(Value::as_array)
                        .and_then(|k| k.first());
                    match doc {
                        Some(d) if d.get("s").is_some() => Layer::new(id, lname, text_layer_type(d), duration),
                        _ => Layer::new(id, lname, LayerType::Null, duration),
                    }
                }
                4 => match lj.get("shapes").and_then(parse_shape_group) {
                    Some((shape_type, color, stroke_color, stroke_width)) => Layer::new(id, lname, LayerType::Shape {
                        shape_type, color, stroke_color, stroke_width,
                    }, duration),
                    None => Layer::new(id, lname, LayerType::Null, duration),
                },
                0 => {
                    let ref_id = lj
                        .get("refId")
                        .and_then(Value::as_str)
                        .unwrap_or("missing_precomp")
                        .to_string();
                    Layer::new(id, lname, LayerType::PreComp { comp_id: ref_id }, duration)
                }
                _ => Layer::new(id, lname, LayerType::Null, duration),
            };

            if let Some(ks) = lj.get("ks") {
                layer.transform = parse_transform(ks);
            }
            let ipf = lj.get("ip").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
            let opf = lj.get("op").and_then(Value::as_f64).unwrap_or(ipf as f64 + 1.0);
            layer.in_frame = ipf;
            layer.out_frame = (opf as u32).max(ipf + 1);
            layer.blend_mode = blend_from_code(lj.get("bm").and_then(Value::as_i64).unwrap_or(0));
            if lj.get("mb").and_then(Value::as_i64) == Some(1) {
                layer.motion_blur = true;
            }
            if let Some(tt) = lj.get("tt").and_then(Value::as_i64) {
                layer.track_matte = matte_from_code(tt);
            }
            if let Some(mps) = lj.get("masksProperties").and_then(Value::as_array) {
                layer.masks = mps.iter().enumerate().map(|(mi, m)| import_mask(m, mi)).collect();
            }

            inds.push(lj.get("ind").and_then(Value::as_i64).unwrap_or(i as i64 + 1));
            comp.layers.push(layer);
        }

        // Resolve parenting: bodymovin "parent" references another layer's ind.
        for (i, lj) in layers_src.iter().enumerate() {
            if let Some(p) = lj.get("parent").and_then(Value::as_i64) {
                if let Some(pos) = inds.iter().position(|&x| x == p) {
                    if pos != i {
                        comp.layers[i].parent_id = Some(comp.layers[pos].id.clone());
                    }
                }
            }
        }

        Some(comp)
    }

    /// Imports a full bodymovin project: the root composition plus every
    /// precomp asset, preserving `ty:0` references as [`LayerType::PreComp`].
    pub fn import_project_from_str(json: &str) -> Option<crate::core::timeline::Project> {
        let v: Value = serde_json::from_str(json).ok()?;
        let mut comps = vec![Self::parse_comp(&v, None)?];
        if let Some(assets) = v.get("assets").and_then(Value::as_array) {
            for a in assets {
                if a.get("layers").and_then(Value::as_array).is_some() {
                    let id = a.get("id").and_then(Value::as_str);
                    if let Some(c) = Self::parse_comp(a, id) {
                        comps.push(c);
                    }
                }
            }
        }
        Some(crate::core::timeline::Project {
            compositions: comps,
            active_composition_idx: 0,
            assets: Vec::new(),
        })
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

    #[test]
    fn test_lottie_text_document_is_exported() {
        let mut comp = Composition::new("c".into(), "Txt".into(), 640, 360, 30, 60);
        comp.layers.push(Layer::new("t1".into(), "Title".into(), LayerType::Text {
            text: "Hello Lottie".into(),
            font_size: 48,
            color: [1.0, 0.5, 0.0, 1.0],
            font_family: "Inter".into(),
            tracking: 12.0,
            leading: 1.4,
            align: 1, // Center
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        }, 60));

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        let doc = &v["layers"][0]["t"];
        assert_eq!(doc["d"]["k"][0]["s"]["t"], "Hello Lottie", "text string preserved");
        assert_eq!(doc["d"]["k"][0]["s"]["f"], "Inter");
        assert_eq!(doc["d"]["k"][0]["s"]["s"], 48);
        assert_eq!(doc["d"]["k"][0]["s"]["fc"], json!([1.0, 0.5, 0.0]));
        // lh crosses f32→f64 JSON serialization, so compare with tolerance.
        let lh = doc["d"]["k"][0]["s"]["lh"].as_f64().expect("numeric lh");
        assert!((lh - (1.4f32 * 48.0) as f64).abs() < 1e-3, "line height = leading × size, got {lh}");
        // Tracking rides under its own "tr" key so it cannot clobber the text.
        assert_eq!(doc["d"]["k"][0]["s"]["tr"], 12.0);
        // App Center(1) maps to Lottie j=2.
        assert_eq!(doc["d"]["k"][0]["s"]["j"], 2);

        // Non-text layers must NOT carry a text document.
        let mut plain = Composition::new("c".into(), "NoTxt".into(), 10, 10, 30, 10);
        plain.layers.push(Layer::new("s".into(), "Solid".into(), LayerType::Solid { color: [1.0; 4] }, 10));
        let v2: Value = serde_json::from_str(&LottieExporter::export_to_json(&plain)).unwrap();
        assert!(v2["layers"][0].get("t").is_none());
    }

    #[test]
    fn test_lottie_track_matte_flags() {
        let mut comp = Composition::new("c".into(), "Matte".into(), 100, 100, 30, 30);
        // Layer 0 = matte source (above), layer 1 consumes it via alpha matte.
        comp.layers.push(Layer::new("src".into(), "MatteSource".into(), LayerType::Null, 30));
        let mut consumer = Layer::new("dst".into(), "Consumer".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        consumer.track_matte = TrackMatteMode::AlphaMatte;
        comp.layers.push(consumer);

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        assert_eq!(v["layers"][0]["td"], 1, "source layer flagged td=1");
        assert_eq!(v["layers"][1]["tt"], 1, "alpha matte → tt=1");

        // Luma inverted maps to 4.
        let mut comp2 = Composition::new("c".into(), "Matte2".into(), 100, 100, 30, 30);
        comp2.layers.push(Layer::new("src".into(), "S".into(), LayerType::Null, 30));
        let mut c2 = Layer::new("dst".into(), "C".into(), LayerType::Null, 30);
        c2.track_matte = TrackMatteMode::LumaMatteInverted;
        comp2.layers.push(c2);
        let v2: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp2)).unwrap();
        assert_eq!(v2["layers"][0]["td"], 1);
        assert_eq!(v2["layers"][1]["tt"], 4);

        // No matte anywhere → no flags at all.
        let mut comp3 = Composition::new("c".into(), "NoMatte".into(), 100, 100, 30, 30);
        comp3.layers.push(Layer::new("a".into(), "A".into(), LayerType::Null, 30));
        comp3.layers.push(Layer::new("b".into(), "B".into(), LayerType::Null, 30));
        let v3: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp3)).unwrap();
        assert!(v3["layers"][0].get("td").is_none());
        assert!(v3["layers"][1].get("tt").is_none());
    }

    #[test]
    fn test_lottie_masks_are_exported() {
        use crate::core::mask::MaskPath;
        let mut comp = Composition::new("c".into(), "Masked".into(), 200, 200, 30, 30);
        let mut layer = Layer::new("l".into(), "M".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        layer.masks.push(crate::core::mask::Mask {
            id: "m1".into(),
            name: "Window".into(),
            enabled: true,
            mode: crate::core::mask::MaskMode::Add,
            path: MaskPath::new_rect(10.0, 20.0, 100.0, 80.0),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(-4.0),
            inverted: false,
            wiggle: None,
        });
        comp.layers.push(layer);

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        assert_eq!(v["layers"][0]["hasMask"], true);
        let m = &v["layers"][0]["masksProperties"][0];
        assert_eq!(m["nm"], "Window");
        assert_eq!(m["mode"], "a", "Add → 'a'");
        assert_eq!(m["inv"], false);
        assert_eq!(m["o"]["k"], 100.0, "opacity preserved");
        assert_eq!(m["x"]["k"], -4.0, "expansion preserved");
        // Rect mask: 4 closed vertices.
        assert_eq!(m["pt"]["a"], 0, "static path");
        let shape = &m["pt"]["k"];
        assert_eq!(shape["c"], true);
        assert_eq!(shape["v"].as_array().unwrap().len(), 4);
        assert_eq!(shape["v"][0], json!([10.0, 20.0]));
    }

    #[test]
    fn test_lottie_mask_modes_and_disabled() {
        use crate::core::mask::{Mask, MaskMode, MaskPath};
        let mut comp = Composition::new("c".into(), "Modes".into(), 100, 100, 30, 30);
        let mk = |mode: MaskMode, name: &str| Mask {
            id: format!("id-{name}"),
            name: name.into(),
            enabled: true,
            mode,
            path: MaskPath::new_rect(0.0, 0.0, 50.0, 50.0),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: true,
            wiggle: None,
        };
        let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
        layer.masks.push(mk(MaskMode::Subtract, "cut"));
        layer.masks.push(mk(MaskMode::Intersect, "keep"));
        layer.masks.push(mk(MaskMode::Difference, "xor"));
        let mut disabled = mk(MaskMode::Add, "off");
        disabled.enabled = false;
        layer.masks.push(disabled);
        comp.layers.push(layer);

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        let props = &v["layers"][0]["masksProperties"];
        assert_eq!(props.as_array().unwrap().len(), 4);
        assert_eq!(props[0]["mode"], "s");
        assert_eq!(props[1]["mode"], "i");
        assert_eq!(props[2]["mode"], "f");
        assert_eq!(props[3]["mode"], "n", "disabled masks become 'none'");
        assert_eq!(props[2]["inv"], true);
    }

    #[test]
    fn test_lottie_animated_mask_path_becomes_shape_keyframes() {
        use crate::core::keyframe::{Keyframe, InterpolationType};
        use crate::core::mask::MaskPath;
        let mut comp = Composition::new("c".into(), "AnimMask".into(), 100, 100, 30, 30);
        let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
        let mut path = MaskPath::new_rect(0.0, 0.0, 40.0, 40.0);
        path.vertices = Animatable::new_animated(vec![
            Keyframe::new(0, vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]], InterpolationType::Linear),
            Keyframe::new(15, vec![[10.0, 5.0], [60.0, 5.0], [60.0, 55.0], [10.0, 55.0]], InterpolationType::Linear),
        ]);
        layer.masks.push(crate::core::mask::Mask {
            id: "ma".into(),
            name: "grow".into(),
            enabled: true,
            mode: crate::core::mask::MaskMode::Add,
            path,
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
            wiggle: None,
        });
        comp.layers.push(layer);

        let v: Value = serde_json::from_str(&LottieExporter::export_to_json(&comp)).unwrap();
        let pt = &v["layers"][0]["masksProperties"][0]["pt"];
        assert_eq!(pt["a"], 1, "animated vertices → shape keyframes");
        let kfs = pt["k"].as_array().unwrap();
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0]["t"], 0);
        assert_eq!(kfs[1]["t"], 15);
        assert_eq!(
            kfs[1]["s"][0]["v"][2],
            json!([60.0, 55.0]),
            "second keyframe carries moved vertices"
        );
    }

    #[test]
    fn test_importer_invalid_inputs_return_none() {
        assert!(LottieImporter::import_from_str("").is_none());
        assert!(LottieImporter::import_from_str("{not json").is_none());
        assert!(LottieImporter::import_from_str("{}").is_none(), "no layers key");
        assert!(LottieImporter::import_from_str("{\"fr\":30}").is_none());
    }

    #[test]
    fn test_import_minimal_animated_solid() {
        let json = r##"{
            "v": "5.7.4", "fr": 30, "ip": 0, "op": 20, "w": 64, "h": 64,
            "nm": "Rot", "bg": "#101010",
            "layers": [{
                "ty": 1, "nm": "S", "ind": 1, "ip": 0, "op": 20, "bm": 2,
                "ks": {
                    "o": {"a": 0, "k": 1},
                    "r": {"a": 1, "k": [{"t": 0, "s": [0]}, {"t": 10, "s": [45]}]},
                    "p": {"a": 0, "k": [10, 20]},
                    "a": {"a": 0, "k": [0, 0]},
                    "s": {"a": 0, "k": [100, 100]}
                },
                "sw": 8, "sh": 8, "sc": "#ff0000"
            }]
        }"##;
        let comp = LottieImporter::import_from_str(json).expect("valid lottie");
        assert_eq!(comp.name, "Rot");
        assert_eq!(comp.fps, 30);
        assert_eq!(comp.duration_frames, 20);
        assert!(comp.background_color[0] > 0.0, "background colour imported");
        assert_eq!(comp.layers.len(), 1);
        let layer = &comp.layers[0];
        match &layer.layer_type {
            LayerType::Solid { color } => {
                assert!((color[0] - 1.0).abs() < 1e-6 && color[1].abs() < 1e-6);
            }
            other => panic!("expected solid, got {other:?}"),
        }
        // Animated rotation lands on its keyframe values.
        assert_eq!(layer.transform.rotation.evaluate(10), 45.0);
        assert_eq!(layer.transform.rotation.evaluate(0), 0.0);
        assert_eq!(layer.transform.position.evaluate(5), [10.0, 20.0]);
        assert_eq!(layer.transform.scale.evaluate(0), [100.0, 100.0]);
        assert_eq!(layer.blend_mode, BlendMode::Screen, "bm=2 → Screen");
    }

    #[test]
    fn test_export_import_roundtrip_preserves_structure() {
        // Build a rich comp: parented text with track matte + masked shape
        // with animated position — then verify the round trip.
        let mut comp = Composition::new("c".into(), "RT".into(), 200, 200, 24, 30);
        let mut src = Layer::new("src".into(), "Src".into(), LayerType::Null, 30);
        src.transform.position = Animatable::new_constant([5.0, 6.0]);
        let mut txt = Layer::new("txt".into(), "T".into(), LayerType::Text {
            text: "RoundTrip".into(),
            font_size: 20,
            color: [0.2, 0.4, 0.8, 1.0],
            font_family: "Inter".into(),
            tracking: 3.0,
            leading: 1.2,
            align: 1,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            text_on_path: false,
        }, 30);
        txt.parent_id = Some("src".into());
        txt.track_matte = TrackMatteMode::AlphaMatte;
        let mut shape = Layer::new("shp".into(), "Sh".into(), LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(60.0),
                height: Animatable::new_constant(40.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 1.0, 0.0, 1.0],
            stroke_width: 2.0,
        }, 30);
        shape.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [10.0, 10.0], InterpolationType::Linear),
            Keyframe::new(12, [80.0, 40.0], InterpolationType::Linear),
        ]);
        shape.masks.push(crate::core::mask::Mask {
            id: "m".into(),
            name: "win".into(),
            enabled: true,
            mode: crate::core::mask::MaskMode::Add,
            path: crate::core::mask::MaskPath::new_rect(0.0, 0.0, 50.0, 50.0),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
            wiggle: None,
        });
        comp.layers.push(src);
        comp.layers.push(txt);
        comp.layers.push(shape);

        let exported = LottieExporter::export_to_json(&comp);
        let back = LottieImporter::import_from_str(&exported).expect("roundtrip parses");
        assert_eq!(back.layers.len(), 3);
        assert_eq!(back.fps, 24);

        // Parenting survives (text → source layer id).
        assert_eq!(back.layers[1].parent_id.as_deref(), Some(back.layers[0].id.as_str()));
        // Track matte survives.
        assert_eq!(back.layers[1].track_matte, TrackMatteMode::AlphaMatte);
        // Text content survives.
        match &back.layers[1].layer_type {
            LayerType::Text { text, font_size, .. } => {
                assert_eq!(text, "RoundTrip");
                assert_eq!(*font_size, 20);
            }
            other => panic!("expected text, got {other:?}"),
        }
        // Animated position keyframes survive with values and frames.
        assert_eq!(back.layers[2].transform.position.evaluate(12), [80.0, 40.0]);
        assert_eq!(back.layers[2].transform.position.evaluate(0), [10.0, 10.0]);
        // Mask survives with vertex count and mode.
        assert_eq!(back.layers[2].masks.len(), 1);
        let m = &back.layers[2].masks[0];
        if let Animatable::Constant(v) = &m.path.vertices {
            assert_eq!(v.len(), 4);
        } else {
            panic!("expected constant mask vertices");
        }
    }

    #[test]
    fn test_export_project_with_precomp_tree() {
        use crate::core::timeline::Project;
        // B: nested comp with one shape; C: deeper comp referenced from B.
        let mut b = Composition::new("B".into(), "NestedB".into(), 100, 100, 24, 30);
        b.layers.push(Layer::new("bs".into(), "BShape".into(), LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(20.0),
                height: Animatable::new_constant(20.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
        }, 30));
        let mut c = Composition::new("C".into(), "DeepC".into(), 80, 80, 24, 30);
        c.layers.push(Layer::new("cl".into(), "CNull".into(), LayerType::Null, 30));
        b.sub_compositions.push(c);
        b.layers.push(Layer::new("bpc".into(), "ToC".into(), LayerType::PreComp { comp_id: "C".into() }, 30));

        let mut a = Composition::new("A".into(), "Root".into(), 400, 300, 24, 60);
        a.layers.push(Layer::new("apc".into(), "ToB".into(), LayerType::PreComp { comp_id: "B".into() }, 60));
        a.layers.push(Layer::new("as".into(), "ASolid".into(), LayerType::Solid { color: [1.0; 4] }, 60));

        let project = Project {
            compositions: vec![a, b],
            active_composition_idx: 0,
            assets: Vec::new(),
        };

        let v: Value = serde_json::from_str(&export_project_to_json(&project)).unwrap();
        // Root precomp slot.
        let root_pc = &v["layers"][0];
        assert_eq!(root_pc["ty"], 0, "precomp layer type");
        assert_eq!(root_pc["refId"], "B");
        assert_eq!(root_pc["w"], 400, "slot sized to containing comp");
        // Both referenced comps emitted as assets (transitively through B→C).
        let assets = v["assets"].as_array().unwrap();
        assert_eq!(assets.len(), 2, "B and C must both be assets");
        let ids: Vec<&str> = assets.iter().filter_map(|a| a["id"].as_str()).collect();
        assert!(ids.contains(&"B") && ids.contains(&"C"), "ids {ids:?}");
        let b_asset = assets.iter().find(|a| a["id"] == "B").unwrap();
        assert_eq!(b_asset["layers"][0]["ty"], 4, "shape inside asset");
        assert_eq!(
            b_asset["layers"].as_array().unwrap().iter()
                .find(|l| l["refId"] == "C").map(|l| l["ty"].clone()),
            Some(json!(0)),
            "nested precomp slot inside asset B"
        );

        // Missing reference degrades gracefully: no crash, no phantom asset.
        let mut a2 = Composition::new("A2".into(), "R2".into(), 10, 10, 24, 5);
        a2.layers.push(Layer::new("x".into(), "Ghost".into(), LayerType::PreComp { comp_id: "NOPE".into() }, 5));
        let p2 = Project { compositions: vec![a2], active_composition_idx: 0, assets: Vec::new() };
        let v2: Value = serde_json::from_str(&export_project_to_json(&p2)).unwrap();
        assert_eq!(v2["layers"][0]["ty"], 0);
        assert_eq!(v2["assets"].as_array().unwrap().len(), 0);

        // Empty project → error payload, no panic.
        let empty = Project { compositions: vec![], active_composition_idx: 0, assets: Vec::new() };
        assert!(export_project_to_json(&empty).contains("error"));
    }

    #[test]
    fn test_project_roundtrip_through_precomp_export() {
        use crate::core::timeline::Project;
        let mut b = Composition::new("B".into(), "NestedB".into(), 100, 100, 24, 30);
        b.layers.push(Layer::new("bs".into(), "BShape".into(), LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(20.0),
                height: Animatable::new_constant(20.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
        }, 30));
        let mut c = Composition::new("C".into(), "DeepC".into(), 80, 80, 24, 30);
        c.layers.push(Layer::new("cl".into(), "CNull".into(), LayerType::Null, 30));
        b.sub_compositions.push(c);
        b.layers.push(Layer::new("bpc".into(), "ToC".into(), LayerType::PreComp { comp_id: "C".into() }, 30));

        let mut a = Composition::new("A".into(), "Root".into(), 400, 300, 24, 60);
        a.layers.push(Layer::new("apc".into(), "ToB".into(), LayerType::PreComp { comp_id: "B".into() }, 60));
        let project = Project {
            compositions: vec![a, b],
            active_composition_idx: 0,
            assets: Vec::new(),
        };

        let exported = export_project_to_json(&project);
        let back = LottieImporter::import_project_from_str(&exported)
            .expect("project roundtrip parses");
        // Root + B + C all present; root is active.
        assert_eq!(back.compositions.len(), 3);
        assert_eq!(back.active_composition_idx, 0);
        let root = &back.compositions[0];
        assert_eq!(root.name, "Root"); // bodymovin roots carry a name, not an id
        // Root's precomp slot points at imported asset B.
        match &root.layers[0].layer_type {
            LayerType::PreComp { comp_id } => assert_eq!(comp_id, "B"),
            other => panic!("expected precomp, got {other:?}"),
        }
        // Asset B kept its bodymovin id and nested precomp reference to C.
        let b_back = back.compositions.iter().find(|c| c.id == "B").expect("asset B");
        match &b_back.layers[1].layer_type {
            LayerType::PreComp { comp_id } => assert_eq!(comp_id, "C"),
            other => panic!("expected nested precomp in B, got {other:?}"),
        }
        assert!(back.compositions.iter().any(|c| c.id == "C"), "deep asset C imported");

        // Single-comp importer still ignores assets gracefully.
        let single = LottieImporter::import_from_str(&exported).expect("single-comp import");
        assert_eq!(single.name, "Root");
    }
}
