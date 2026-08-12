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

// ─── Mask Path ─────────────────────────────────────────────────────────────

/// An animatable Bezier mask path — a list of vertices that can be keyframed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskPath {
    pub vertices: Animatable<Vec<[f32; 2]>>,
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
            is_closed: true,
        }
    }

    pub fn new_ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        // Approximate circle with 4 cubic bezier segments
        // Control point distance k ≈ 0.5523 * radius
        let verts = vec![
            [cx, cy - ry],        // top
            [cx + rx, cy],        // right
            [cx, cy + ry],        // bottom
            [cx - rx, cy],        // left
        ];
        Self {
            vertices: Animatable::new_constant(verts),
            is_closed: true,
        }
    }

    /// Sample the path as a series of screen-space points for CPU rendering.
    /// Returns a flat list of [x, y] pairs.
    pub fn to_polygon(&self, frame: u32, segments_per_edge: u32) -> Vec<[f32; 2]> {
        let verts = match &self.vertices {
            Animatable::Constant(v) => v.clone(),
            Animatable::Animated(kfs) => {
                if kfs.is_empty() {
                    Vec::new()
                } else if kfs.len() == 1 || frame <= kfs[0].frame {
                    kfs[0].value.clone()
                } else if frame >= kfs.last().unwrap().frame {
                    kfs.last().unwrap().value.clone()
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
            }
        };
        if verts.len() < 2 {
            return verts;
        }

        let mut result = Vec::with_capacity(verts.len() * segments_per_edge as usize);
        let n = verts.len();
        let end = if self.is_closed { n } else { n - 1 };

        for i in 0..end {
            let p0 = verts[i];
            let p1 = verts[(i + 1) % n];
            for s in 0..segments_per_edge {
                let t = s as f32 / segments_per_edge as f32;
                let x = p0[0] + (p1[0] - p0[0]) * t;
                let y = p0[1] + (p1[1] - p0[1]) * t;
                result.push([x, y]);
            }
        }

        if self.is_closed {
            if let Some(&first) = result.first() {
                result.push(first);
            }
        }

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
}
