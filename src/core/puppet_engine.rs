//! Advanced Puppet Pin Deformation Engine (AE Parity).
//!
//! Provides Moving Least Squares (MLS) As-Rigid-As-Possible (ARAP) 2D triangle mesh deformation
//! with support for Position Pins, Starch (Rigidity) Pins, and Overlap (Depth Ordering) Pins.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum PuppetPinType {
    #[default]
    Position,
    Starch,
    Overlap,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PuppetPin {
    pub id: String,
    pub pin_type: PuppetPinType,
    pub rest_position: [f32; 2],
    pub current_position: [f32; 2],
    /// Extent / radius of influence (in pixels)
    pub extent: f32,
    /// Stiffness for Starch pins (0..100) or Depth offset for Overlap pins (-100..+100)
    pub stiffness_or_depth: f32,
}

impl PuppetPin {
    pub fn new_position(id: impl Into<String>, pos: [f32; 2]) -> Self {
        Self {
            id: id.into(),
            pin_type: PuppetPinType::Position,
            rest_position: pos,
            current_position: pos,
            extent: 50.0,
            stiffness_or_depth: 0.0,
        }
    }

    pub fn new_starch(id: impl Into<String>, pos: [f32; 2], extent: f32, stiffness: f32) -> Self {
        Self {
            id: id.into(),
            pin_type: PuppetPinType::Starch,
            rest_position: pos,
            current_position: pos,
            extent,
            stiffness_or_depth: stiffness,
        }
    }

    pub fn new_overlap(id: impl Into<String>, pos: [f32; 2], extent: f32, depth: f32) -> Self {
        Self {
            id: id.into(),
            pin_type: PuppetPinType::Overlap,
            rest_position: pos,
            current_position: pos,
            extent,
            stiffness_or_depth: depth,
        }
    }
}

/// Deforms a single 2D vertex point using Moving Least Squares (MLS) affine & rigid transformation.
pub fn deform_point_mls(vertex: [f32; 2], pins: &[PuppetPin]) -> [f32; 2] {
    if pins.is_empty() {
        return vertex;
    }

    let pos_pins: Vec<&PuppetPin> = pins
        .iter()
        .filter(|p| p.pin_type == PuppetPinType::Position)
        .collect();
    if pos_pins.is_empty() {
        return vertex;
    }

    let starch_pins: Vec<&PuppetPin> = pins
        .iter()
        .filter(|p| p.pin_type == PuppetPinType::Starch)
        .collect();

    // Compute weights w_i = 1 / |v - p_i|^(2*alpha)
    let mut weights = Vec::with_capacity(pos_pins.len());
    let mut total_w = 0.0f32;

    for p in &pos_pins {
        let dx = vertex[0] - p.rest_position[0];
        let dy = vertex[1] - p.rest_position[1];
        let dist_sq = dx * dx + dy * dy;

        // Exact match with pin
        if dist_sq < 1e-6 {
            return p.current_position;
        }

        let w = 1.0 / dist_sq.powf(1.0); // alpha = 1.0 standard MLS
        weights.push(w);
        total_w += w;
    }

    if total_w <= 0.0 {
        return vertex;
    }

    // Weighted centroids p* and q*
    let mut p_star = [0.0f32, 0.0f32];
    let mut q_star = [0.0f32, 0.0f32];

    for (i, p) in pos_pins.iter().enumerate() {
        let w = weights[i] / total_w;
        p_star[0] += w * p.rest_position[0];
        p_star[1] += w * p.rest_position[1];
        q_star[0] += w * p.current_position[0];
        q_star[1] += w * p.current_position[1];
    }

    // Calculate affine deformation matrix M = sum(w_i * (q_i - q*) * (p_i - p*)^T) * (sum(w_i * (p_i - p*) * (p_i - p*)^T))^-1
    let mut num_x = 0.0f32;
    let mut num_y = 0.0f32;

    for (i, p) in pos_pins.iter().enumerate() {
        let w = weights[i] / total_w;
        let px = p.rest_position[0] - p_star[0];
        let py = p.rest_position[1] - p_star[1];
        let qx = p.current_position[0] - q_star[0];
        let qy = p.current_position[1] - q_star[1];

        let vx = vertex[0] - p_star[0];
        let vy = vertex[1] - p_star[1];

        // Similarity MLS transformation
        let a = px * vx + py * vy;
        let b = px * vy - py * vx;
        num_x += w * (qx * a - qy * b);
        num_y += w * (qy * a + qx * b);
    }

    let mut result = [q_star[0] + num_x, q_star[1] + num_y];

    // Apply Starch (Rigidity) suppression: interpolate back to rest position based on proximity to starch pins
    for sp in starch_pins {
        let dx = vertex[0] - sp.rest_position[0];
        let dy = vertex[1] - sp.rest_position[1];
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < sp.extent && sp.extent > 0.0 {
            let falloff = (1.0 - dist / sp.extent).clamp(0.0, 1.0);
            let stiffness = (sp.stiffness_or_depth * 0.01).clamp(0.0, 1.0) * falloff;
            result[0] = result[0] * (1.0 - stiffness) + vertex[0] * stiffness;
            result[1] = result[1] * (1.0 - stiffness) + vertex[1] * stiffness;
        }
    }

    result
}

/// Applies Puppet Mesh Warp to an RGBA pixel buffer using inverse MLS mapping and bilinear sampling.
pub fn apply_puppet_mesh_warp(src: &[u8], width: u32, height: u32, pins: &[PuppetPin]) -> Vec<u8> {
    let Some(expected_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|s| s.checked_mul(4))
    else {
        return src.to_vec();
    };
    if pins.is_empty() || width == 0 || height == 0 || src.len() != expected_len {
        return src.to_vec();
    }

    // Inverted pins: mapping from deformed space (current) back to source space (rest)
    let inv_pins: Vec<PuppetPin> = pins
        .iter()
        .map(|p| PuppetPin {
            id: p.id.clone(),
            pin_type: p.pin_type,
            rest_position: p.current_position,
            current_position: p.rest_position,
            extent: p.extent,
            stiffness_or_depth: p.stiffness_or_depth,
        })
        .collect();

    let mut dst = vec![0u8; src.len()];

    let sample_bilinear = |x: f32, y: f32| -> [u8; 4] {
        if x < 0.0 || x >= (width - 1) as f32 || y < 0.0 || y >= (height - 1) as f32 {
            let cx = (x.round() as i32).clamp(0, width as i32 - 1) as u32;
            let cy = (y.round() as i32).clamp(0, height as i32 - 1) as u32;
            let idx = ((cy * width + cx) * 4) as usize;
            return [src[idx], src[idx + 1], src[idx + 2], src[idx + 3]];
        }

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let i00 = ((y0 * width + x0) * 4) as usize;
        let i10 = ((y0 * width + x1) * 4) as usize;
        let i01 = ((y1 * width + x0) * 4) as usize;
        let i11 = ((y1 * width + x1) * 4) as usize;

        let mut out = [0u8; 4];
        for c in 0..4 {
            let top = src[i00 + c] as f32 * (1.0 - fx) + src[i10 + c] as f32 * fx;
            let bot = src[i01 + c] as f32 * (1.0 - fx) + src[i11 + c] as f32 * fx;
            out[c] = (top * (1.0 - fy) + bot * fy).clamp(0.0, 255.0) as u8;
        }
        out
    };

    for y in 0..height {
        for x in 0..width {
            let src_pt = deform_point_mls([x as f32, y as f32], &inv_pins);
            let pixel = sample_bilinear(src_pt[0], src_pt[1]);
            let idx = ((y * width + x) * 4) as usize;
            dst[idx..idx + 4].copy_from_slice(&pixel);
        }
    }

    dst
}

/// 2D Triangle Mesh representation for puppet deformation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PuppetMesh {
    pub vertices: Vec<[f32; 2]>,
    pub triangles: Vec<[usize; 3]>,
}

/// 2D Delaunay Triangulation using the Bowyer-Watson incremental algorithm.
pub fn delaunay_triangulate_2d(points: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return vec![];
    }

    // Find bounding box
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for p in points {
        min_x = min_x.min(p[0]);
        min_y = min_y.min(p[1]);
        max_x = max_x.max(p[0]);
        max_y = max_y.max(p[1]);
    }

    let dx = (max_x - min_x).max(1.0);
    let dy = (max_y - min_y).max(1.0);
    let delta_max = dx.max(dy) * 2.0;
    let mid_x = (min_x + max_x) * 0.5;
    let mid_y = (min_y + max_y) * 0.5;

    // Super-triangle enclosing all points
    let mut all_points = points.to_vec();
    let super_0 = [mid_x - 2.0 * delta_max, mid_y - delta_max];
    let super_1 = [mid_x, mid_y + 2.0 * delta_max];
    let super_2 = [mid_x + 2.0 * delta_max, mid_y - delta_max];

    all_points.push(super_0);
    all_points.push(super_1);
    all_points.push(super_2);

    let super_indices = [n, n + 1, n + 2];
    let mut triangles = vec![super_indices];

    for (p_idx, &p) in points.iter().enumerate() {
        let mut polygon_edges: Vec<[usize; 2]> = Vec::new();
        let mut bad_triangles: Vec<usize> = Vec::new();

        for (t_idx, &tri) in triangles.iter().enumerate() {
            let a = all_points[tri[0]];
            let b = all_points[tri[1]];
            let c = all_points[tri[2]];

            if in_circumcircle(p, a, b, c) {
                bad_triangles.push(t_idx);
                let edges = [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]];
                for edge in edges {
                    polygon_edges.push(edge);
                }
            }
        }

        // Keep only boundary edges not shared with any other bad triangle
        let mut boundary_edges = Vec::new();
        for (i, &e1) in polygon_edges.iter().enumerate() {
            let mut is_shared = false;
            for (j, &e2) in polygon_edges.iter().enumerate() {
                if i != j
                    && ((e1[0] == e2[0] && e1[1] == e2[1]) || (e1[0] == e2[1] && e1[1] == e2[0]))
                {
                    is_shared = true;
                    break;
                }
            }
            if !is_shared {
                boundary_edges.push(e1);
            }
        }

        // Remove bad triangles (in reverse order)
        for &t_idx in bad_triangles.iter().rev() {
            triangles.swap_remove(t_idx);
        }

        // Re-triangulate the cavity polygon with new point
        for edge in boundary_edges {
            triangles.push([edge[0], edge[1], p_idx]);
        }
    }

    // Remove any triangles containing super-triangle vertices
    triangles.retain(|tri| tri[0] < n && tri[1] < n && tri[2] < n);

    triangles
}

fn in_circumcircle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let ax_ = a[0] - p[0];
    let ay_ = a[1] - p[1];
    let bx_ = b[0] - p[0];
    let by_ = b[1] - p[1];
    let cx_ = c[0] - p[0];
    let cy_ = c[1] - p[1];

    let det = (ax_ * ax_ + ay_ * ay_) * (bx_ * cy_ - cx_ * by_)
        - (bx_ * bx_ + by_ * by_) * (ax_ * cy_ - cx_ * ay_)
        + (cx_ * cx_ + cy_ * cy_) * (ax_ * by_ - bx_ * ay_);

    // Counter-clockwise orientation check
    let ccw = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if ccw > 0.0 {
        det > 1e-6
    } else {
        det < -1e-6
    }
}

/// Generates a complete Delaunay triangle mesh for a layer bounds and its puppet pins.
pub fn generate_puppet_mesh(
    width: f32,
    height: f32,
    grid_step: f32,
    pins: &[PuppetPin],
) -> PuppetMesh {
    let step = grid_step.clamp(16.0, 256.0);
    let mut vertices: Vec<[f32; 2]> = Vec::new();

    // 1. Boundary and interior regular grid points
    let nx = (width / step).ceil() as usize;
    let ny = (height / step).ceil() as usize;

    for y in 0..=ny {
        let py = (y as f32 * step).min(height);
        for x in 0..=nx {
            let px = (x as f32 * step).min(width);
            vertices.push([px, py]);
        }
    }

    // 2. Inject pin rest positions as exact mesh vertices
    for pin in pins {
        let px = pin.rest_position[0].clamp(0.0, width);
        let py = pin.rest_position[1].clamp(0.0, height);
        // Avoid duplicate vertices very close to existing ones
        if !vertices.iter().any(|v| (v[0] - px).hypot(v[1] - py) < 4.0) {
            vertices.push([px, py]);
        }
    }

    let triangles = delaunay_triangulate_2d(&vertices);

    PuppetMesh {
        vertices,
        triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puppet_pin_translation() {
        let pin1 = PuppetPin {
            id: "p1".into(),
            pin_type: PuppetPinType::Position,
            rest_position: [100.0, 100.0],
            current_position: [120.0, 110.0],
            extent: 50.0,
            stiffness_or_depth: 0.0,
        };

        let moved = deform_point_mls([100.0, 100.0], std::slice::from_ref(&pin1));
        assert!((moved[0] - 120.0).abs() < 1e-4);
        assert!((moved[1] - 110.0).abs() < 1e-4);
    }

    #[test]
    fn test_starch_pin_reduces_distortion() {
        let pin_pos = PuppetPin::new_position("p1", [100.0, 100.0]);
        let mut pin_pos_moved = pin_pos.clone();
        pin_pos_moved.current_position = [150.0, 100.0];

        let vertex = [110.0, 100.0];
        let unstarched = deform_point_mls(vertex, &[pin_pos_moved.clone()]);

        let starch = PuppetPin::new_starch("s1", [110.0, 100.0], 50.0, 100.0);
        let starched = deform_point_mls(vertex, &[pin_pos_moved, starch]);

        assert!((starched[0] - vertex[0]).abs() < (unstarched[0] - vertex[0]).abs());
    }

    #[test]
    fn test_apply_puppet_mesh_warp_buffer_transformation() {
        let mut src = vec![0u8; 10 * 10 * 4];
        let idx = (5 * 10 + 5) * 4;
        src[idx] = 255;
        src[idx + 3] = 255;

        let mut pin = PuppetPin::new_position("p1", [5.0, 5.0]);
        pin.current_position = [6.0, 5.0];

        let warped = apply_puppet_mesh_warp(&src, 10, 10, &[pin]);
        assert_eq!(warped.len(), src.len());
        let dst_idx = (5 * 10 + 6) * 4;
        assert!(warped[dst_idx] > 0);
    }

    #[test]
    fn test_delaunay_triangulation_four_corners() {
        let pts = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let tris = delaunay_triangulate_2d(&pts);
        // A rectangle should triangulate into exactly 2 triangles
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn test_generate_puppet_mesh_contains_vertices_and_triangles() {
        let pins = vec![PuppetPin::new_position("p1", [50.0, 50.0])];
        let mesh = generate_puppet_mesh(100.0, 100.0, 50.0, &pins);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.triangles.is_empty());
    }
}
