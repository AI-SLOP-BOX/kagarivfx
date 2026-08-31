#![allow(dead_code)]
use crate::core::mask::MaskVertex;

/// Pucker & Bloat options matching After Effects Shape Pucker & Bloat modifier.
#[derive(Debug, Clone)]
pub struct PuckerBloatOptions {
    pub amount: f32, // Percentage (-100.0 = full Pucker, +100.0 = full Bloat)
}

/// Zig Zag options matching After Effects Shape Zig Zag modifier.
#[derive(Debug, Clone)]
pub struct ZigZagOptions {
    pub size: f32,               // Ridge amplitude in pixels
    pub ridges_per_segment: u32, // Number of ridges per path segment
    pub smooth: bool,            // Smooth curve vs sharp corner points
}

/// Applies Pucker & Bloat distortion to a vector shape path.
pub fn apply_pucker_bloat(
    vertices: &[MaskVertex],
    options: &PuckerBloatOptions,
) -> Vec<MaskVertex> {
    if vertices.is_empty() || options.amount.abs() < 0.001 {
        return vertices.to_vec();
    }

    // Compute centroid of shape vertices
    let mut center = [0.0f32, 0.0f32];
    for v in vertices {
        center[0] += v.position[0];
        center[1] += v.position[1];
    }
    let n = vertices.len() as f32;
    center[0] /= n;
    center[1] /= n;

    let factor = options.amount * 0.01;
    let mut modified = Vec::with_capacity(vertices.len());

    for v in vertices {
        let mut v_new = v.clone();

        // Vector from center to vertex position
        let vx = v.position[0] - center[0];
        let vy = v.position[1] - center[1];

        // Shift vertex towards or away from center
        v_new.position[0] += vx * factor;
        v_new.position[1] += vy * factor;

        // Scale Bezier tangents inversely for Pucker / Bloat organic curve bulging
        v_new.tangent_in[0] *= 1.0 - factor;
        v_new.tangent_in[1] *= 1.0 - factor;
        v_new.tangent_out[0] *= 1.0 - factor;
        v_new.tangent_out[1] *= 1.0 - factor;

        modified.push(v_new);
    }

    modified
}

/// Applies Zig Zag (sawtooth / wave) geometric sub-division distortion to a Bezier path.
pub fn apply_zig_zag(vertices: &[MaskVertex], options: &ZigZagOptions) -> Vec<MaskVertex> {
    if vertices.len() < 2 || options.ridges_per_segment == 0 || options.size <= 0.001 {
        return vertices.to_vec();
    }

    let mut result = Vec::new();
    let ridges = options.ridges_per_segment as usize;

    for i in 0..vertices.len() - 1 {
        let v0 = &vertices[i];
        let v1 = &vertices[i + 1];

        result.push(v0.clone());

        let dx = v1.position[0] - v0.position[0];
        let dy = v1.position[1] - v0.position[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);

        let normal = [-dy / len, dx / len];

        for r in 1..ridges {
            let t = r as f32 / ridges as f32;
            let px = v0.position[0] + dx * t;
            let py = v0.position[1] + dy * t;

            let side = if r % 2 == 1 { 1.0 } else { -1.0 };
            let offset_x = normal[0] * options.size * side;
            let offset_y = normal[1] * options.size * side;

            let mut ridge_vertex = MaskVertex::new(px + offset_x, py + offset_y);
            if !options.smooth {
                ridge_vertex.tangent_in = [0.0, 0.0];
                ridge_vertex.tangent_out = [0.0, 0.0];
            }
            result.push(ridge_vertex);
        }
    }

    if let Some(last) = vertices.last() {
        result.push(last.clone());
    }

    result
}

/// Applies Round Corners modifier to a Bezier vertex path.
pub fn apply_round_corners(vertices: &[MaskVertex], radius: f32, closed: bool) -> Vec<MaskVertex> {
    if vertices.len() < 3 || radius <= 0.001 {
        return vertices.to_vec();
    }

    let mut result = Vec::new();
    let n = vertices.len();

    for i in 0..n {
        if !closed && (i == 0 || i == n - 1) {
            result.push(vertices[i].clone());
            continue;
        }

        let prev = if i == 0 {
            &vertices[n - 1]
        } else {
            &vertices[i - 1]
        };
        let curr = &vertices[i];
        let next = if i == n - 1 {
            &vertices[0]
        } else {
            &vertices[i + 1]
        };

        // Vectors from curr to prev and curr to next
        let v_prev = [
            prev.position[0] - curr.position[0],
            prev.position[1] - curr.position[1],
        ];
        let v_next = [
            next.position[0] - curr.position[0],
            next.position[1] - curr.position[1],
        ];

        let len_prev = (v_prev[0].powi(2) + v_prev[1].powi(2)).sqrt().max(0.001);
        let len_next = (v_next[0].powi(2) + v_next[1].powi(2)).sqrt().max(0.001);

        let d_prev = radius.min(len_prev * 0.45);
        let d_next = radius.min(len_next * 0.45);

        let p_in = [
            curr.position[0] + (v_prev[0] / len_prev) * d_prev,
            curr.position[1] + (v_prev[1] / len_prev) * d_prev,
        ];
        let p_out = [
            curr.position[0] + (v_next[0] / len_next) * d_next,
            curr.position[1] + (v_next[1] / len_next) * d_next,
        ];

        let mut v_in = MaskVertex::new(p_in[0], p_in[1]);
        let mut v_out = MaskVertex::new(p_out[0], p_out[1]);

        let kappa = 0.5522847498f32; // Standard Bezier circular approximation constant
        v_in.tangent_out = [
            (curr.position[0] - p_in[0]) * kappa,
            (curr.position[1] - p_in[1]) * kappa,
        ];
        v_out.tangent_in = [
            (curr.position[0] - p_out[0]) * kappa,
            (curr.position[1] - p_out[1]) * kappa,
        ];

        result.push(v_in);
        result.push(v_out);
    }

    result
}

// ──────────────── Hierarchical Shape Content Tree Model ────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShapeTransform {
    pub anchor_point: [f32; 2],
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32, // Degrees
    pub opacity: f32,  // 0.0 to 100.0%
}

impl Default for ShapeTransform {
    fn default() -> Self {
        Self {
            anchor_point: [0.0, 0.0],
            position: [0.0, 0.0],
            scale: [100.0, 100.0],
            rotation: 0.0,
            opacity: 100.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MergePathsMode {
    #[default]
    Merge,
    Add,
    Subtract,
    Intersect,
    Exclude,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShapeContentItem {
    Group {
        name: String,
        items: Vec<ShapeContentItem>,
        transform: ShapeTransform,
    },
    Path {
        name: String,
        vertices: Vec<MaskVertex>,
        closed: bool,
    },
    Rectangle {
        name: String,
        size: [f32; 2],
        position: [f32; 2],
        roundness: f32,
    },
    Ellipse {
        name: String,
        size: [f32; 2],
        position: [f32; 2],
    },
    Fill {
        name: String,
        color: [f32; 4],
        opacity: f32,
    },
    Stroke {
        name: String,
        color: [f32; 4],
        width: f32,
        opacity: f32,
        line_cap: u8,
        line_join: u8,
    },
    TrimPaths {
        name: String,
        start: f32,  // 0.0 to 100.0%
        end: f32,    // 0.0 to 100.0%
        offset: f32, // Degrees or %
    },
    MergePaths {
        name: String,
        mode: MergePathsMode,
    },
    RoundCorners {
        name: String,
        radius: f32,
    },
    PuckerBloat {
        name: String,
        amount: f32,
    },
    ZigZag {
        name: String,
        size: f32,
        ridges: u32,
        smooth: bool,
    },
    Repeater {
        name: String,
        copies: u32,
        offset: f32,
        transform: ShapeTransform,
    },
}

/// Trims a polyline or closed Bezier path between start (0..100) and end (0..100) with offset.
pub fn trim_path_vertices(
    vertices: &[MaskVertex],
    start_pct: f32,
    end_pct: f32,
    offset_pct: f32,
    closed: bool,
) -> Vec<MaskVertex> {
    if vertices.len() < 2 {
        return vertices.to_vec();
    }

    let start = (start_pct * 0.01 + offset_pct * 0.01).rem_euclid(1.0);
    let end = (end_pct * 0.01 + offset_pct * 0.01).rem_euclid(1.0);

    if (start - end).abs() < 1e-4 {
        return Vec::new();
    }

    // Measure total cumulative segment lengths
    let n = vertices.len();
    let num_segs = if closed { n } else { n - 1 };
    let mut cum_lens = vec![0.0f32; num_segs + 1];
    let mut total_len = 0.0f32;

    for i in 0..num_segs {
        let p0 = vertices[i].position;
        let p1 = vertices[(i + 1) % n].position;
        let seg_len = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
        total_len += seg_len;
        cum_lens[i + 1] = total_len;
    }

    if total_len < 1e-4 {
        return vertices.to_vec();
    }

    let sample_at = |t: f32| -> MaskVertex {
        let target_dist = t.clamp(0.0, 1.0) * total_len;
        for i in 0..num_segs {
            if target_dist <= cum_lens[i + 1] || i == num_segs - 1 {
                let seg_l = (cum_lens[i + 1] - cum_lens[i]).max(1e-5);
                let frac = ((target_dist - cum_lens[i]) / seg_l).clamp(0.0, 1.0);
                let p0 = &vertices[i];
                let p1 = &vertices[(i + 1) % n];
                let px = p0.position[0] + (p1.position[0] - p0.position[0]) * frac;
                let py = p0.position[1] + (p1.position[1] - p0.position[1]) * frac;
                return MaskVertex::new(px, py);
            }
        }
        vertices[0].clone()
    };

    let mut result = Vec::new();
    if start < end {
        result.push(sample_at(start));
        for i in 0..num_segs {
            let seg_t = cum_lens[i + 1] / total_len;
            if seg_t > start && seg_t < end {
                result.push(vertices[(i + 1) % n].clone());
            }
        }
        result.push(sample_at(end));
    } else {
        // Wrapped around start > end
        result.push(sample_at(start));
        for i in 0..num_segs {
            let seg_t = cum_lens[i + 1] / total_len;
            if seg_t > start {
                result.push(vertices[(i + 1) % n].clone());
            }
        }
        for i in 0..num_segs {
            let seg_t = cum_lens[i + 1] / total_len;
            if seg_t < end {
                result.push(vertices[(i + 1) % n].clone());
            }
        }
        result.push(sample_at(end));
    }

    result
}

#[derive(Debug, Clone)]
pub struct RenderableShapePath {
    pub vertices: Vec<MaskVertex>,
    pub closed: bool,
    pub fill_color: Option<[f32; 4]>,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_width: f32,
}

/// Evaluates a recursive tree of ShapeContentItems, sequentially applying geometric modifiers
/// and outputting the final styled vector path list for the software/GPU renderer.
pub fn evaluate_shape_tree(items: &[ShapeContentItem]) -> Vec<RenderableShapePath> {
    let mut current_paths: Vec<(Vec<MaskVertex>, bool)> = Vec::new();
    let mut active_fill: Option<[f32; 4]> = None;
    let mut active_stroke: Option<[f32; 4]> = None;
    let mut active_stroke_w: f32 = 0.0;
    let mut output_paths: Vec<RenderableShapePath> = Vec::new();

    for item in items {
        match item {
            ShapeContentItem::Group {
                items: sub_items,
                transform,
                ..
            } => {
                let sub_rendered = evaluate_shape_tree(sub_items);
                let rad = transform.rotation.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let sx = transform.scale[0] * 0.01;
                let sy = transform.scale[1] * 0.01;

                for mut sp in sub_rendered {
                    for v in &mut sp.vertices {
                        // Apply local group transform: Anchor -> Scale/Rot -> Position
                        let lx = (v.position[0] - transform.anchor_point[0]) * sx;
                        let ly = (v.position[1] - transform.anchor_point[1]) * sy;
                        let rx = lx * cos_r - ly * sin_r + transform.position[0];
                        let ry = lx * sin_r + ly * cos_r + transform.position[1];
                        v.position = [rx, ry];
                    }
                    output_paths.push(sp);
                }
            }
            ShapeContentItem::Path {
                vertices, closed, ..
            } => {
                current_paths.push((vertices.clone(), *closed));
            }
            ShapeContentItem::Rectangle {
                size,
                position,
                roundness,
                ..
            } => {
                let hw = size[0] * 0.5;
                let hh = size[1] * 0.5;
                let raw_rect = vec![
                    MaskVertex::new(position[0] - hw, position[1] - hh),
                    MaskVertex::new(position[0] + hw, position[1] - hh),
                    MaskVertex::new(position[0] + hw, position[1] + hh),
                    MaskVertex::new(position[0] - hw, position[1] + hh),
                ];
                let verts = if *roundness > 0.001 {
                    apply_round_corners(&raw_rect, *roundness, true)
                } else {
                    raw_rect
                };
                current_paths.push((verts, true));
            }
            ShapeContentItem::Ellipse { size, position, .. } => {
                let hw = size[0] * 0.5;
                let hh = size[1] * 0.5;
                let k_x = hw * 0.5522847498;
                let k_y = hh * 0.5522847498;

                let mut top = MaskVertex::new(position[0], position[1] - hh);
                top.tangent_in = [-k_x, 0.0];
                top.tangent_out = [k_x, 0.0];

                let mut right = MaskVertex::new(position[0] + hw, position[1]);
                right.tangent_in = [0.0, -k_y];
                right.tangent_out = [0.0, k_y];

                let mut bottom = MaskVertex::new(position[0], position[1] + hh);
                bottom.tangent_in = [k_x, 0.0];
                bottom.tangent_out = [-k_x, 0.0];

                let mut left = MaskVertex::new(position[0] - hw, position[1]);
                left.tangent_in = [0.0, k_y];
                left.tangent_out = [0.0, -k_y];

                current_paths.push((vec![top, right, bottom, left], true));
            }
            ShapeContentItem::TrimPaths {
                start, end, offset, ..
            } => {
                let mut trimmed = Vec::new();
                for (verts, closed) in current_paths {
                    let res = trim_path_vertices(&verts, *start, *end, *offset, closed);
                    trimmed.push((res, false));
                }
                current_paths = trimmed;
            }
            ShapeContentItem::MergePaths { mode, .. } => {
                if current_paths.len() >= 2 {
                    let mut polys: Vec<Vec<[f32; 2]>> = current_paths
                        .iter()
                        .map(|(verts, _)| verts.iter().map(|v| v.position).collect())
                        .collect();

                    let boolean_op = match mode {
                        MergePathsMode::Add | MergePathsMode::Merge => {
                            crate::core::shape_boolean::BooleanOp::Union
                        }
                        MergePathsMode::Subtract => crate::core::shape_boolean::BooleanOp::Subtract,
                        MergePathsMode::Intersect => {
                            crate::core::shape_boolean::BooleanOp::Intersect
                        }
                        MergePathsMode::Exclude => crate::core::shape_boolean::BooleanOp::Exclude,
                    };

                    let mut acc_polys = vec![polys.remove(0)];
                    for next_poly in polys {
                        let mut next_acc = Vec::new();
                        for acc in &acc_polys {
                            let res = crate::core::shape_boolean::apply_polygon_boolean(
                                acc, &next_poly, boolean_op,
                            );
                            next_acc.extend(res);
                        }
                        if next_acc.is_empty() {
                            acc_polys = vec![next_poly];
                        } else {
                            acc_polys = next_acc;
                        }
                    }

                    current_paths = acc_polys
                        .into_iter()
                        .map(|poly| {
                            let verts = poly
                                .into_iter()
                                .map(|p| MaskVertex::new(p[0], p[1]))
                                .collect();
                            (verts, true)
                        })
                        .collect();
                }
            }
            ShapeContentItem::RoundCorners { radius, .. } => {
                for (verts, closed) in &mut current_paths {
                    *verts = apply_round_corners(verts, *radius, *closed);
                }
            }
            ShapeContentItem::PuckerBloat { amount, .. } => {
                let opt = PuckerBloatOptions { amount: *amount };
                for (verts, _) in &mut current_paths {
                    *verts = apply_pucker_bloat(verts, &opt);
                }
            }
            ShapeContentItem::ZigZag {
                size,
                ridges,
                smooth,
                ..
            } => {
                let opt = ZigZagOptions {
                    size: *size,
                    ridges_per_segment: *ridges,
                    smooth: *smooth,
                };
                for (verts, _) in &mut current_paths {
                    *verts = apply_zig_zag(verts, &opt);
                }
            }
            ShapeContentItem::Repeater {
                copies,
                offset,
                transform,
                ..
            } => {
                let mut duplicated = Vec::new();
                if *copies == 0 {
                    current_paths.clear();
                    continue;
                }
                let count = (*copies).min(4096);
                let offset = if offset.is_finite() { *offset } else { 0.0 };
                let rotation = if transform.rotation.is_finite() {
                    transform.rotation
                } else {
                    0.0
                };
                let scale_x = if transform.scale[0].is_finite() {
                    transform.scale[0]
                } else {
                    100.0
                };
                let scale_y = if transform.scale[1].is_finite() {
                    transform.scale[1]
                } else {
                    100.0
                };
                for i in 0..count {
                    let progress = i as f32 + offset;
                    let rad = (rotation * progress).to_radians();
                    let cos_r = rad.cos();
                    let sin_r = rad.sin();
                    let sx = (100.0 + (scale_x - 100.0) * progress) * 0.01;
                    let sy = (100.0 + (scale_y - 100.0) * progress) * 0.01;
                    let tx = transform.position[0]
                        .is_finite()
                        .then_some(transform.position[0])
                        .unwrap_or(0.0)
                        * progress;
                    let ty = transform.position[1]
                        .is_finite()
                        .then_some(transform.position[1])
                        .unwrap_or(0.0)
                        * progress;

                    for (verts, closed) in &current_paths {
                        let mut rep_verts = verts.clone();
                        for v in &mut rep_verts {
                            let lx = (v.position[0] - transform.anchor_point[0]) * sx;
                            let ly = (v.position[1] - transform.anchor_point[1]) * sy;
                            let rx = lx * cos_r - ly * sin_r + tx;
                            let ry = lx * sin_r + ly * cos_r + ty;
                            v.position = [rx, ry];
                        }
                        duplicated.push((rep_verts, *closed));
                    }
                }
                current_paths = duplicated;
            }
            ShapeContentItem::Fill { color, opacity, .. } => {
                let mut c = *color;
                c[3] *= (*opacity * 0.01).clamp(0.0, 1.0);
                active_fill = Some(c);
            }
            ShapeContentItem::Stroke {
                color,
                width,
                opacity,
                ..
            } => {
                let mut c = *color;
                c[3] *= (*opacity * 0.01).clamp(0.0, 1.0);
                active_stroke = Some(c);
                active_stroke_w = *width;
            }
        }
    }

    // Flush current paths with active styling
    for (verts, closed) in current_paths {
        output_paths.push(RenderableShapePath {
            vertices: verts,
            closed,
            fill_color: active_fill,
            stroke_color: active_stroke,
            stroke_width: active_stroke_w,
        });
    }

    output_paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pucker_bloat_displacement() {
        let vertices = vec![MaskVertex::new(-10.0, -10.0), MaskVertex::new(10.0, 10.0)];

        let options = PuckerBloatOptions { amount: 50.0 };
        let bloat = apply_pucker_bloat(&vertices, &options);

        assert_eq!(bloat.len(), 2);
        assert!(bloat[1].position[0] > 10.0);
    }

    #[test]
    fn test_round_corners_generates_tangents() {
        let rect = vec![
            MaskVertex::new(0.0, 0.0),
            MaskVertex::new(100.0, 0.0),
            MaskVertex::new(100.0, 100.0),
            MaskVertex::new(0.0, 100.0),
        ];
        let rounded = apply_round_corners(&rect, 20.0, true);
        // Each corner splits into 2 vertices (incoming & outgoing tangent) => 8 vertices
        assert_eq!(rounded.len(), 8);
    }

    #[test]
    fn test_evaluate_shape_tree_with_repeater() {
        let tree = vec![
            ShapeContentItem::Rectangle {
                name: "Rect".into(),
                size: [50.0, 50.0],
                position: [0.0, 0.0],
                roundness: 0.0,
            },
            ShapeContentItem::Repeater {
                name: "Repeater".into(),
                copies: 3,
                offset: 0.0,
                transform: ShapeTransform {
                    position: [100.0, 0.0],
                    ..Default::default()
                },
            },
            ShapeContentItem::Fill {
                name: "Fill".into(),
                color: [1.0, 0.0, 0.0, 1.0],
                opacity: 100.0,
            },
        ];
        let paths = evaluate_shape_tree(&tree);
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].fill_color, Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(paths[1].vertices[0].position[0], 75.0); // 1st copy shifted by 100
    }

    #[test]
    fn test_shape_tree_zero_copy_repeater_removes_paths() {
        let tree = vec![
            ShapeContentItem::Rectangle {
                name: "Rect".into(),
                size: [20.0, 20.0],
                position: [0.0, 0.0],
                roundness: 0.0,
            },
            ShapeContentItem::Repeater {
                name: "Empty".into(),
                copies: 0,
                offset: 0.0,
                transform: ShapeTransform::default(),
            },
        ];
        assert!(evaluate_shape_tree(&tree).is_empty());
    }

    #[test]
    fn test_shape_tree_repeater_sanitizes_nonfinite_transform() {
        let tree = vec![
            ShapeContentItem::Rectangle {
                name: "Rect".into(),
                size: [20.0, 20.0],
                position: [0.0, 0.0],
                roundness: 0.0,
            },
            ShapeContentItem::Repeater {
                name: "Safe".into(),
                copies: 2,
                offset: f32::NAN,
                transform: ShapeTransform {
                    position: [f32::INFINITY, f32::NEG_INFINITY],
                    rotation: f32::NAN,
                    scale: [f32::INFINITY, f32::NAN],
                    ..ShapeTransform::default()
                },
            },
        ];
        for path in evaluate_shape_tree(&tree) {
            for vertex in path.vertices {
                assert!(vertex.position.iter().all(|value| value.is_finite()));
            }
        }
    }

    #[test]
    fn test_evaluate_shape_tree_with_merge_paths() {
        let tree = vec![
            ShapeContentItem::Rectangle {
                name: "RectA".into(),
                size: [100.0, 100.0],
                position: [0.0, 0.0],
                roundness: 0.0,
            },
            ShapeContentItem::Rectangle {
                name: "RectB".into(),
                size: [100.0, 100.0],
                position: [50.0, 50.0],
                roundness: 0.0,
            },
            ShapeContentItem::MergePaths {
                name: "Merge".into(),
                mode: MergePathsMode::Add,
            },
            ShapeContentItem::Fill {
                name: "Fill".into(),
                color: [0.0, 1.0, 0.0, 1.0],
                opacity: 100.0,
            },
        ];
        let paths = evaluate_shape_tree(&tree);
        assert!(!paths.is_empty());
        assert_eq!(paths[0].fill_color, Some([0.0, 1.0, 0.0, 1.0]));
    }

    #[test]
    fn test_trim_path_vertices_shortens_polyline() {
        let raw = vec![MaskVertex::new(0.0, 0.0), MaskVertex::new(100.0, 0.0)];
        let trimmed = trim_path_vertices(&raw, 25.0, 75.0, 0.0, false);
        assert!(trimmed.len() >= 2);
        assert!((trimmed[0].position[0] - 25.0).abs() < 1e-3);
        assert!((trimmed[1].position[0] - 75.0).abs() < 1e-3);
    }

    #[test]
    fn test_apply_offset_paths_expands_rect() {
        let rect = vec![
            MaskVertex::new(0.0, 0.0),
            MaskVertex::new(100.0, 0.0),
            MaskVertex::new(100.0, 100.0),
            MaskVertex::new(0.0, 100.0),
        ];
        let offset = apply_offset_paths(&rect, 10.0, OffsetPathsJoin::Miter, 4.0);
        assert_eq!(offset.len(), 4);
        assert!(offset[0].position[0] < 0.0 && offset[0].position[1] < 0.0);
    }
}

/// Line join style for Offset Paths modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OffsetPathsJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// Applies Offset Paths modifier to a polygon path.
pub fn apply_offset_paths(
    vertices: &[MaskVertex],
    amount: f32,
    join: OffsetPathsJoin,
    miter_limit: f32,
) -> Vec<MaskVertex> {
    let n = vertices.len();
    if n < 3 || amount.abs() < 1e-4 {
        return vertices.to_vec();
    }

    // Compute signed polygon area to detect winding direction
    let mut area = 0.0f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].position[0] * vertices[j].position[1]
            - vertices[j].position[0] * vertices[i].position[1];
    }
    let sign = if area > 0.0 { -1.0 } else { 1.0 };
    let eff_amount = amount * sign;

    let mut result = Vec::with_capacity(n * 2);

    for i in 0..n {
        let prev = &vertices[(i + n - 1) % n];
        let curr = &vertices[i];
        let next = &vertices[(i + 1) % n];

        let d1 = [curr.position[0] - prev.position[0], curr.position[1] - prev.position[1]];
        let d2 = [next.position[0] - curr.position[0], next.position[1] - curr.position[1]];

        let len1 = (d1[0] * d1[0] + d1[1] * d1[1]).sqrt().max(1e-4);
        let len2 = (d2[0] * d2[0] + d2[1] * d2[1]).sqrt().max(1e-4);

        // Outward normals
        let n1 = [-d1[1] / len1, d1[0] / len1];
        let n2 = [-d2[1] / len2, d2[0] / len2];

        let avg_n = [n1[0] + n2[0], n1[1] + n2[1]];
        let avg_len = (avg_n[0] * avg_n[0] + avg_n[1] * avg_n[1]).sqrt().max(1e-4);
        let bisector = [avg_n[0] / avg_len, avg_n[1] / avg_len];

        let cos_half = n1[0] * bisector[0] + n1[1] * bisector[1];
        let miter_len = if cos_half.abs() > 1e-3 {
            (eff_amount / cos_half).clamp(-miter_limit * eff_amount.abs(), miter_limit * eff_amount.abs())
        } else {
            eff_amount
        };

        match join {
            OffsetPathsJoin::Miter => {
                let offset_pos = [
                    curr.position[0] + bisector[0] * miter_len,
                    curr.position[1] + bisector[1] * miter_len,
                ];
                result.push(MaskVertex::new(offset_pos[0], offset_pos[1]));
            }
            OffsetPathsJoin::Bevel | OffsetPathsJoin::Round => {
                let p1 = [curr.position[0] + n1[0] * amount, curr.position[1] + n1[1] * amount];
                let p2 = [curr.position[0] + n2[0] * amount, curr.position[1] + n2[1] * amount];
                result.push(MaskVertex::new(p1[0], p1[1]));
                result.push(MaskVertex::new(p2[0], p2[1]));
            }
        }
    }

    result
}
