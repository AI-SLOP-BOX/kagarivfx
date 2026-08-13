/// AE-style Bezier mask system.
///
/// Each Layer can have multiple named masks. Each mask is a closed or open
/// Bezier path with per-vertex tangent handles, a blend mode, feathering radius,
/// and an expansion/contraction value.
///
/// Data model follows AE's mask architecture:
///   Layer → Vec<Mask> → MaskPath → Vec<MaskVertex>

use serde::{Deserialize, Serialize};
use crate::core::property::Animatable;

// ─── Mask Blend Mode ───────────────────────────────────────────────────────

/// How this mask combines with masks below it on the same layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskMode {
    /// Add to the existing alpha (default AE mode)
    Add,
    /// Subtract from the existing alpha
    Subtract,
    /// Only keep the intersection of this and existing alpha
    Intersect,
    /// Exclude the union (XOR-style)
    Lighten,
    /// Darken the existing alpha
    Darken,
    /// Difference with existing alpha
    Difference,
    /// Disable this mask (show full layer)
    None,
}

impl Default for MaskMode {
    fn default() -> Self { MaskMode::Add }
}

// ─── Mask Vertex ───────────────────────────────────────────────────────────

/// A single vertex in a Bezier mask path.
/// Positions are in composition pixel coordinates (0,0 = top-left).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskVertex {
    /// Anchor point (the vertex position)
    pub position: [f32; 2],
    /// Outgoing tangent handle (relative to position)
    pub tangent_out: [f32; 2],
    /// Incoming tangent handle (relative to position)
    pub tangent_in: [f32; 2],
}

#[allow(dead_code)]
impl MaskVertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            tangent_out: [0.0, 0.0],
            tangent_in: [0.0, 0.0],
        }
    }

    pub fn with_tangents(x: f32, y: f32, tx: f32, ty: f32) -> Self {
        Self {
            position: [x, y],
            tangent_out: [tx, ty],
            tangent_in: [-tx, -ty],
        }
    }
}

// ─── Cubic Bezier Helper ───────────────────────────────────────────────────

/// Compute a point on a 2D cubic Bezier curve at parameter t in [0, 1].
pub fn eval_cubic_bezier(p0: [f32; 2], c0: [f32; 2], c1: [f32; 2], p1: [f32; 2], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let u2 = u * u;
    let u3 = u2 * u;
    let t2 = t * t;
    let t3 = t2 * t;

    let x = u3 * p0[0] + 3.0 * u2 * t * c0[0] + 3.0 * u * t2 * c1[0] + t3 * p1[0];
    let y = u3 * p0[1] + 3.0 * u2 * t * c0[1] + 3.0 * u * t2 * c1[1] + t3 * p1[1];
    [x, y]
}

// ─── Mask Path ─────────────────────────────────────────────────────────────

/// An animatable Bezier mask path — a list of vertices that can be keyframed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskPath {
    pub vertices: Animatable<Vec<[f32; 2]>>,
    /// Optional incoming/outgoing tangent handles per vertex for cubic Bezier curves.
    /// Format per vertex: ([in_x, in_y], [out_x, out_y]) relative to position.
    #[serde(default)]
    pub tangents: Option<Vec<([f32; 2], [f32; 2])>>,
    pub is_closed: bool,
}

impl MaskPath {
    pub fn new_rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        let verts = vec![
            [x, y],
            [x + w, y],
            [x + w, y + h],
            [x, y + h],
        ];
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: None,
            is_closed: true,
        }
    }

    pub fn new_ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        // Approximate circle/ellipse with 4 cubic bezier segments
        // Control point distance k ≈ 0.55228475 * radius
        let kx = rx * 0.55228475;
        let ky = ry * 0.55228475;
        let verts = vec![
            [cx, cy - ry],        // top
            [cx + rx, cy],        // right
            [cx, cy + ry],        // bottom
            [cx - rx, cy],        // left
        ];
        let tangents = vec![
            ([-kx, 0.0], [kx, 0.0]),   // top: in left, out right
            ([0.0, -ky], [0.0, ky]),   // right: in up, out down
            ([kx, 0.0], [-kx, 0.0]),   // bottom: in right, out left
            ([0.0, ky], [0.0, -ky]),   // left: in down, out up
        ];
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: Some(tangents),
            is_closed: true,
        }
    }

    /// Sample the path as a series of screen-space points for CPU rendering into a caller-provided output vector.
    /// Reuses existing vector allocations to eliminate heap allocation pressure during interactive scrubbing.
    pub fn to_polygon_into(&self, frame: u32, segments_per_edge: u32, out_vec: &mut Vec<[f32; 2]>) {
        out_vec.clear();
        let verts = match &self.vertices {
            Animatable::Constant(v) => v.clone(),
            Animatable::Animated(kfs) => {
                if kfs.is_empty() {
                    Vec::new()
                } else if kfs.len() == 1 || frame <= kfs[0].frame {
                    kfs[0].value.clone()
                } else if let Some(last_kf) = kfs.last() {
                    if frame >= last_kf.frame {
                        last_kf.value.clone()
                    } else {
                        let mut prev = &kfs[0];
                        let mut next = &kfs[0];
                        for kf in kfs {
                            if kf.frame <= frame { prev = kf; }
                            if kf.frame >= frame { next = kf; break; }
                        }
                        if prev.frame == next.frame {
                            prev.value.clone()
                        } else {
                            let t = (frame - prev.frame) as f32 / (next.frame - prev.frame) as f32;
                            prev.value.iter().zip(next.value.iter()).map(|(&p0, &p1)| {
                                [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
                            }).collect()
                        }
                    }
                } else {
                    Vec::new()
                }
            }
        };
        if verts.len() < 2 {
            out_vec.extend(verts);
            return;
        }

        let n = verts.len();
        let end = if self.is_closed { n } else { n - 1 };

        out_vec.reserve(end * segments_per_edge as usize + 1);

        for i in 0..end {
            let p0 = verts[i];
            let next_i = (i + 1) % n;
            let p1 = verts[next_i];

            let bezier_control_points = if let Some(tangs) = &self.tangents {
                if tangs.len() == n {
                    let out_t0 = tangs[i].1;
                    let in_t1 = tangs[next_i].0;
                    let c0 = [p0[0] + out_t0[0], p0[1] + out_t0[1]];
                    let c1 = [p1[0] + in_t1[0], p1[1] + in_t1[1]];
                    Some((c0, c1))
                } else {
                    None
                }
            } else {
                None
            };

            for s in 0..segments_per_edge {
                let t = s as f32 / segments_per_edge as f32;
                let pt = if let Some((c0, c1)) = bezier_control_points {
                    eval_cubic_bezier(p0, c0, c1, p1, t)
                } else {
                    [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
                };
                out_vec.push(pt);
            }
        }

        if self.is_closed {
            if let Some(&first) = out_vec.first() {
                out_vec.push(first);
            }
        }
    }

    /// Sample the path as a series of screen-space points for CPU rendering.
    /// Returns a flat list of [x, y] pairs. Uses cubic Bezier curves when tangents exist.
    pub fn to_polygon(&self, frame: u32, segments_per_edge: u32) -> Vec<[f32; 2]> {
        let mut result = Vec::new();
        self.to_polygon_into(frame, segments_per_edge, &mut result);
        result
    }
}

// ─── Mask ──────────────────────────────────────────────────────────────────

/// A single named mask attached to a layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub mode: MaskMode,
    pub path: MaskPath,
    /// Feather/blur radius in pixels applied to the mask edge
    pub feather: Animatable<f32>,
    /// Mask opacity (0–100)
    pub opacity: Animatable<f32>,
    /// Positive = expand, negative = contract (pixels)
    pub expansion: Animatable<f32>,
    /// Whether the mask is inverted
    pub inverted: bool,
}

impl Mask {
    pub fn new_rect(id: String, name: String, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id,
            name,
            enabled: true,
            mode: MaskMode::Add,
            path: MaskPath::new_rect(x, y, w, h),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
        }
    }

    pub fn new_ellipse(id: String, name: String, cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self {
            id,
            name,
            enabled: true,
            mode: MaskMode::Add,
            path: MaskPath::new_ellipse(cx, cy, rx, ry),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
        }
    }
}

// ─── Point-in-Polygon Test (CPU Alpha Cutout) ─────────────────────────────

/// Ray-casting point-in-polygon test.
/// Returns true if (px, py) is inside the polygon defined by `verts`.
#[allow(dead_code)]
pub fn point_in_polygon(px: f32, py: f32, verts: &[[f32; 2]]) -> bool {
    let n = verts.len();
    if n < 3 { return false; }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let xi = verts[i][0];
        let yi = verts[i][1];
        let xj = verts[j][0];
        let yj = verts[j][1];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_in_polygon_square() {
        let square = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        assert!(point_in_polygon(50.0, 50.0, &square), "center should be inside");
        assert!(!point_in_polygon(150.0, 50.0, &square), "outside right");
        assert!(!point_in_polygon(-10.0, 50.0, &square), "outside left");
    }

    #[test]
    fn test_mask_rect_path() {
        let path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
        let poly = path.to_polygon(0, 4);
        assert!(!poly.is_empty(), "polygon should have points");
    }

    #[test]
    fn test_mask_mode_default() {
        let mask = Mask::new_rect("m0".into(), "Mask 1".into(), 0.0, 0.0, 100.0, 100.0);
        assert_eq!(mask.mode, MaskMode::Add);
        assert!(mask.enabled);
        assert!(!mask.inverted);
    }

    #[test]
    fn test_eval_cubic_bezier() {
        let p0 = [0.0, 0.0];
        let c0 = [0.0, 50.0];
        let c1 = [100.0, 50.0];
        let p1 = [100.0, 100.0];
        let mid = eval_cubic_bezier(p0, c0, c1, p1, 0.5);
        assert_eq!(mid, [50.0, 50.0]);
    }

    #[test]
    fn test_mask_ellipse_bezier_sampling() {
        let path = MaskPath::new_ellipse(100.0, 100.0, 50.0, 50.0);
        assert!(path.tangents.is_some(), "ellipse mask should have tangent handles");
        let poly = path.to_polygon(0, 8);
        assert_eq!(poly.len(), 4 * 8 + 1, "sampled polygon should contain curve segments");
    }
}
