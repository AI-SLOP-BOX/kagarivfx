use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
/// AE-style Bezier mask system.
///
/// Each Layer can have multiple named masks. Each mask is a closed or open
/// Bezier path with per-vertex tangent handles, a blend mode, feathering radius,
/// and an expansion/contraction value.
///
/// Data model follows AE's mask architecture:
///   Layer → Vec<Mask> → MaskPath → Vec<MaskVertex>
use serde::{Deserialize, Serialize};

// ─── Mask Blend Mode ───────────────────────────────────────────────────────

/// How this mask combines with masks below it on the same layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MaskMode {
    /// Add to the existing alpha (default AE mode)
    #[default]
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

// ─── Mask Vertex ───────────────────────────────────────────────────────────

/// A single vertex in a Bezier mask path.
/// Positions are in composition pixel coordinates (0,0 = top-left).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub fn eval_cubic_bezier(
    p0: [f32; 2],
    c0: [f32; 2],
    c1: [f32; 2],
    p1: [f32; 2],
    t: f32,
) -> [f32; 2] {
    let t_safe = t.clamp(0.0, 1.0);
    let u = 1.0 - t_safe;
    let u2 = u * u;
    let u3 = u2 * u;
    let t2 = t_safe * t_safe;
    let t3 = t2 * t_safe;

    let x = u3 * p0[0] + 3.0 * u2 * t_safe * c0[0] + 3.0 * u * t2 * c1[0] + t3 * p1[0];
    let y = u3 * p0[1] + 3.0 * u2 * t_safe * c0[1] + 3.0 * u * t2 * c1[1] + t3 * p1[1];
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
        let verts = vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: None,
            is_closed: true,
        }
    }

    pub fn new_closed(verts: Vec<[f32; 2]>) -> Self {
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: None,
            is_closed: true,
        }
    }

    pub fn new_ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        // Approximate circle/ellipse with 4 cubic bezier segments
        // Control point distance k ≈ 0.55228475 * radius
        let kx = rx * 0.552_284_8;
        let ky = ry * 0.552_284_8;
        let verts = vec![
            [cx, cy - ry], // top
            [cx + rx, cy], // right
            [cx, cy + ry], // bottom
            [cx - rx, cy], // left
        ];
        let tangents = vec![
            ([-kx, 0.0], [kx, 0.0]), // top: in left, out right
            ([0.0, -ky], [0.0, ky]), // right: in up, out down
            ([kx, 0.0], [-kx, 0.0]), // bottom: in right, out left
            ([0.0, ky], [0.0, -ky]), // left: in down, out up
        ];
        Self {
            vertices: Animatable::new_constant(verts),
            tangents: Some(tangents),
            is_closed: true,
        }
    }

    /// Get evaluated vertices at a given frame.
    pub fn get_vertices(&self, frame: u32) -> Vec<[f32; 2]> {
        match &self.vertices {
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
                            if kf.frame <= frame {
                                prev = kf;
                            }
                            if kf.frame >= frame {
                                next = kf;
                                break;
                            }
                        }
                        if prev.frame == next.frame {
                            prev.value.clone()
                        } else {
                            let t = (frame - prev.frame) as f32 / (next.frame - prev.frame) as f32;
                            prev.value
                                .iter()
                                .zip(next.value.iter())
                                .map(|(&p0, &p1)| {
                                    [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
                                })
                                .collect()
                        }
                    }
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Sample the path as a series of screen-space points for CPU rendering into a caller-provided output vector.
    /// Reuses existing vector allocations to eliminate heap allocation pressure during interactive scrubbing.
    pub fn to_polygon_into(&self, frame: u32, segments_per_edge: u32, out_vec: &mut Vec<[f32; 2]>) {
        out_vec.clear();
        let verts = self.get_vertices(frame);
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

    /// Return mask anchor vertices at the given frame (not curve-sampled).
    pub fn vertices_at_frame(&self, frame: u32) -> Vec<[f32; 2]> {
        match &self.vertices {
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
                            if kf.frame <= frame {
                                prev = kf;
                            }
                            if kf.frame >= frame {
                                next = kf;
                                break;
                            }
                        }
                        if prev.frame == next.frame {
                            prev.value.clone()
                        } else {
                            let t = (frame - prev.frame) as f32 / (next.frame - prev.frame) as f32;
                            prev.value
                                .iter()
                                .zip(next.value.iter())
                                .map(|(&p0, &p1)| {
                                    [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
                                })
                                .collect()
                        }
                    }
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Move a single anchor vertex at `frame`, preserving tangents and keyframe structure.
    pub fn set_vertex_at_frame(&mut self, frame: u32, vertex_idx: usize, new_pos: [f32; 2]) {
        match &mut self.vertices {
            Animatable::Constant(verts) => {
                if vertex_idx < verts.len() {
                    verts[vertex_idx] = new_pos;
                }
            }
            Animatable::Animated(keyframes) => {
                if let Some(kf) = keyframes.iter_mut().find(|k| k.frame == frame) {
                    if vertex_idx < kf.value.len() {
                        kf.value[vertex_idx] = new_pos;
                    }
                } else {
                    let mut current = self.vertices_at_frame(frame);
                    if vertex_idx < current.len() {
                        current[vertex_idx] = new_pos;
                        self.vertices.add_keyframe(Keyframe::new(
                            frame,
                            current,
                            InterpolationType::Linear,
                        ));
                    }
                }
            }
        }
    }

    /// Subdivides segment `segment_idx` at parameter `t` in [0, 1] using de Casteljau split,
    /// inserting a new anchor point and updating tangent handles smoothly.
    pub fn insert_vertex_at_frame(&mut self, frame: u32, segment_idx: usize, t: f32) -> Option<usize> {
        let t_clamped = t.clamp(0.01, 0.99);
        let verts = self.vertices_at_frame(frame);
        let n = verts.len();
        if n < 2 || segment_idx >= n {
            return None;
        }

        let next_idx = (segment_idx + 1) % n;
        let p0 = verts[segment_idx];
        let p3 = verts[next_idx];

        let (c0, c1) = if let Some(tangents) = &self.tangents {
            let out_h = tangents.get(segment_idx).map(|t| t.1).unwrap_or([0.0, 0.0]);
            let in_h = tangents.get(next_idx).map(|t| t.0).unwrap_or([0.0, 0.0]);
            ([p0[0] + out_h[0], p0[1] + out_h[1]], [p3[0] + in_h[0], p3[1] + in_h[1]])
        } else {
            (p0, p3)
        };

        // de Casteljau subdivision at parameter t
        let q0 = [p0[0] + (c0[0] - p0[0]) * t_clamped, p0[1] + (c0[1] - p0[1]) * t_clamped];
        let q1 = [c0[0] + (c1[0] - c0[0]) * t_clamped, c0[1] + (c1[1] - c0[1]) * t_clamped];
        let q2 = [c1[0] + (p3[0] - c1[0]) * t_clamped, c1[1] + (p3[1] - c1[1]) * t_clamped];

        let r0 = [q0[0] + (q1[0] - q0[0]) * t_clamped, q0[1] + (q1[1] - q0[1]) * t_clamped];
        let r1 = [q1[0] + (q2[0] - q1[0]) * t_clamped, q1[1] + (q2[1] - q1[1]) * t_clamped];

        let mid = [r0[0] + (r1[0] - r0[0]) * t_clamped, r0[1] + (r1[1] - r0[1]) * t_clamped];
        let new_idx = segment_idx + 1;

        match &mut self.vertices {
            Animatable::Constant(verts) => {
                verts.insert(new_idx, mid);
            }
            Animatable::Animated(kfs) => {
                for kf in kfs.iter_mut() {
                    if new_idx <= kf.value.len() {
                        kf.value.insert(new_idx, mid);
                    }
                }
            }
        }

        if let Some(tangents) = &mut self.tangents {
            // Update segment_idx out tangent to (q0 - p0)
            if segment_idx < tangents.len() {
                tangents[segment_idx].1 = [q0[0] - p0[0], q0[1] - p0[1]];
            }
            // Insert new vertex tangents: in=(r0 - mid), out=(r1 - mid)
            tangents.insert(new_idx, ([r0[0] - mid[0], r0[1] - mid[1]], [r1[0] - mid[0], r1[1] - mid[1]]));
            // Update next vertex in tangent to (p3 - q2)
            let updated_next = (new_idx + 1) % tangents.len();
            if updated_next < tangents.len() {
                tangents[updated_next].0 = [q2[0] - p3[0], q2[1] - p3[1]];
            }
        }

        Some(new_idx)
    }

    /// Removes anchor vertex at `vertex_idx` maintaining polygon continuity.
    pub fn remove_vertex_at_frame(&mut self, vertex_idx: usize) -> bool {
        let mut removed = false;
        match &mut self.vertices {
            Animatable::Constant(verts) => {
                if verts.len() > 3 && vertex_idx < verts.len() {
                    verts.remove(vertex_idx);
                    removed = true;
                }
            }
            Animatable::Animated(kfs) => {
                for kf in kfs.iter_mut() {
                    if kf.value.len() > 3 && vertex_idx < kf.value.len() {
                        kf.value.remove(vertex_idx);
                        removed = true;
                    }
                }
            }
        }

        if removed {
            if let Some(tangents) = &mut self.tangents {
                if vertex_idx < tangents.len() {
                    tangents.remove(vertex_idx);
                }
            }
        }

        removed
    }

    /// Sets or updates incoming/outgoing tangent handles with optional smooth collinear link.
    pub fn set_tangents_at_vertex(
        &mut self,
        vertex_idx: usize,
        tangent_in: [f32; 2],
        tangent_out: [f32; 2],
        link_collinear: bool,
    ) {
        let count = match &self.vertices {
            Animatable::Constant(v) => v.len(),
            Animatable::Animated(kfs) => kfs.first().map(|k| k.value.len()).unwrap_or(0),
        };
        if vertex_idx >= count {
            return;
        }

        let mut t_in = tangent_in;
        let mut t_out = tangent_out;

        if link_collinear {
            // Mirror out handle based on in handle opposite vector
            let len_out = (t_out[0].powi(2) + t_out[1].powi(2)).sqrt().max(1.0);
            let len_in = (t_in[0].powi(2) + t_in[1].powi(2)).sqrt().max(1.0);
            let ratio = len_out / len_in;
            t_out = [-t_in[0] * ratio, -t_in[1] * ratio];
        }

        if self.tangents.is_none() {
            self.tangents = Some(vec![([0.0, 0.0], [0.0, 0.0]); count]);
        }

        if let Some(tangents) = &mut self.tangents {
            if vertex_idx < tangents.len() {
                tangents[vertex_idx] = (t_in, t_out);
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
    /// Wiggle Paths organic deformation (AE parity)
    #[serde(default)]
    pub wiggle: Option<crate::core::wiggle_paths::WigglePathsOptions>,
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
            wiggle: None,
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
            wiggle: None,
        }
    }

    pub fn new_closed(id: String, name: String, verts: Vec<[f32; 2]>) -> Self {
        Self {
            id,
            name,
            enabled: true,
            mode: MaskMode::Add,
            path: MaskPath::new_closed(verts),
            feather: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            expansion: Animatable::new_constant(0.0),
            inverted: false,
            wiggle: None,
        }
    }
}

// ─── Point-in-Polygon Test (CPU Alpha Cutout) ─────────────────────────────

/// Ray-casting point-in-polygon test.
/// Returns true if (px, py) is inside the polygon defined by `verts`.
#[allow(dead_code)]
pub fn point_in_polygon(px: f32, py: f32, verts: &[[f32; 2]]) -> bool {
    let n = verts.len();
    if n < 3 {
        return false;
    }

    // Check if point is directly on any polygon boundary edge within tolerance
    for i in 0..n {
        let j = (i + 1) % n;
        let x1 = verts[i][0];
        let y1 = verts[i][1];
        let x2 = verts[j][0];
        let y2 = verts[j][1];
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len_sq = dx * dx + dy * dy;
        if len_sq > 1e-6 {
            let t = (((px - x1) * dx + (py - y1) * dy) / len_sq).clamp(0.0, 1.0);
            let proj_x = x1 + t * dx;
            let proj_y = y1 + t * dy;
            let dist_sq = (px - proj_x).powi(2) + (py - proj_y).powi(2);
            if dist_sq < 1e-4 {
                return true;
            }
        }
    }

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
        assert!(
            point_in_polygon(50.0, 50.0, &square),
            "center should be inside"
        );
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
    fn test_set_vertex_at_frame_preserves_tangents() {
        let mut path = MaskPath::new_ellipse(100.0, 100.0, 50.0, 50.0);
        let tangents_before = path.tangents.clone();
        path.set_vertex_at_frame(0, 0, [120.0, 80.0]);
        let verts = path.vertices_at_frame(0);
        assert_eq!(verts[0], [120.0, 80.0]);
        assert_eq!(path.tangents, tangents_before);
    }

    #[test]
    fn test_mask_ellipse_bezier_sampling() {
        let path = MaskPath::new_ellipse(100.0, 100.0, 50.0, 50.0);
        assert!(
            path.tangents.is_some(),
            "ellipse mask should have tangent handles"
        );
        let poly = path.to_polygon(0, 8);
        assert_eq!(
            poly.len(),
            4 * 8 + 1,
            "sampled polygon should contain curve segments"
        );
    }

    #[test]
    fn test_insert_and_remove_vertex_maintains_topology() {
        let mut path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
        assert_eq!(path.vertices_at_frame(0).len(), 4);

        // Insert point in segment 0 (top edge) at t = 0.5
        let new_idx = path.insert_vertex_at_frame(0, 0, 0.5).expect("insert succeeds");
        assert_eq!(new_idx, 1);
        let verts = path.vertices_at_frame(0);
        assert_eq!(verts.len(), 5);
        assert_eq!(verts[1], [50.0, 0.0]); // Midpoint of [0,0] -> [100,0]

        // Remove inserted vertex
        assert!(path.remove_vertex_at_frame(1));
        assert_eq!(path.vertices_at_frame(0).len(), 4);
    }

    #[test]
    fn test_set_tangents_collinear_link() {
        let mut path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
        path.set_tangents_at_vertex(0, [-10.0, 0.0], [20.0, 0.0], true);
        let tangents = path.tangents.as_ref().unwrap();
        let (t_in, t_out) = tangents[0];
        assert_eq!(t_in, [-10.0, 0.0]);
        assert_eq!(t_out, [20.0, 0.0]); // Collinear opposite direction
    }
}
