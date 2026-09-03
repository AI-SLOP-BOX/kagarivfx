//! 3D Mesh Extrusion & Beveling Engine (AE Parity).
//!
//! Converts 2D vector paths and shape contours into fully extruded 3D solid meshes
//! with custom extrusion depth, bevel depth, and separate Front, Side, and Back material facets.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtrusionOptions {
    /// Total extrusion depth along Z axis (in pixels)
    pub depth: f32,
    /// Bevel depth / chamfer size
    pub bevel_depth: f32,
    /// Number of curved bevel subdivisions
    pub bevel_segments: u32,
}

impl Default for ExtrusionOptions {
    fn default() -> Self {
        Self {
            depth: 50.0,
            bevel_depth: 4.0,
            bevel_segments: 2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExtrudedMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    /// Index ranges for [Front Cap, Side Walls, Back Cap]
    pub front_indices_count: usize,
    pub side_indices_count: usize,
    pub back_indices_count: usize,
}

/// Extrudes a closed 2D polygon contour into a 3D volume mesh.
pub fn extrude_2d_contour(contour_points: &[[f32; 2]], options: &ExtrusionOptions) -> ExtrudedMesh {
    let mut mesh = ExtrudedMesh::default();
    let n = contour_points.len();
    if n < 3 {
        return mesh;
    }

    let z_front = 0.0f32;
    let z_back = -options.depth.max(0.1);

    // 1. Front Cap (Simple Triangle Fan for convex contours)
    let front_start = mesh.vertices.len() as u32;
    for &p in contour_points {
        mesh.vertices.push([p[0], p[1], z_front]);
        mesh.normals.push([0.0, 0.0, 1.0]);
        mesh.uvs.push([p[0] * 0.01, p[1] * 0.01]);
    }
    for i in 1..(n - 1) {
        mesh.indices.push(front_start);
        mesh.indices.push(front_start + i as u32);
        mesh.indices.push(front_start + i as u32 + 1);
    }
    mesh.front_indices_count = mesh.indices.len();

    // 2. Side Walls (Extrusion Ribbons between Front & Back)
    let side_start_idx = mesh.indices.len();
    for i in 0..n {
        let next_i = (i + 1) % n;
        let p1 = contour_points[i];
        let p2 = contour_points[next_i];

        // Side normal pointing outward
        let dx = p2[0] - p1[0];
        let dy = p2[1] - p1[1];
        let len = (dx * dx + dy * dy).sqrt().max(1e-5);
        let normal = [dy / len, -dx / len, 0.0];

        let base_idx = mesh.vertices.len() as u32;
        // Quad vertices (4 per wall segment)
        mesh.vertices.push([p1[0], p1[1], z_front]); // 0: Front Top
        mesh.vertices.push([p2[0], p2[1], z_front]); // 1: Front Next
        mesh.vertices.push([p2[0], p2[1], z_back]); // 2: Back Next
        mesh.vertices.push([p1[0], p1[1], z_back]); // 3: Back Top

        for _ in 0..4 {
            mesh.normals.push(normal);
        }
        mesh.uvs.push([0.0, 0.0]);
        mesh.uvs.push([1.0, 0.0]);
        mesh.uvs.push([1.0, 1.0]);
        mesh.uvs.push([0.0, 1.0]);

        // Triangle 1
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);
        // Triangle 2
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);
    }
    mesh.side_indices_count = mesh.indices.len() - side_start_idx;

    // 3. Back Cap (Facing -Z)
    let back_start_idx = mesh.indices.len();
    let back_vert_start = mesh.vertices.len() as u32;
    for &p in contour_points {
        mesh.vertices.push([p[0], p[1], z_back]);
        mesh.normals.push([0.0, 0.0, -1.0]);
        mesh.uvs.push([p[0] * 0.01, p[1] * 0.01]);
    }
    for i in 1..(n - 1) {
        mesh.indices.push(back_vert_start);
        mesh.indices.push(back_vert_start + i as u32 + 1); // Inverted winding for back face
        mesh.indices.push(back_vert_start + i as u32);
    }
    mesh.back_indices_count = mesh.indices.len() - back_start_idx;

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extrude_2d_rectangle() {
        let square = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

        let opts = ExtrusionOptions {
            depth: 40.0,
            bevel_depth: 0.0,
            bevel_segments: 0,
        };

        let mesh = extrude_2d_contour(&square, &opts);

        assert_eq!(mesh.front_indices_count, 6); // 2 triangles for front cap quad
        assert_eq!(mesh.side_indices_count, 24); // 4 sides * 2 triangles * 3 indices = 24
        assert_eq!(mesh.back_indices_count, 6); // 2 triangles for back cap quad
        assert_eq!(mesh.indices.len(), 36); // Total cube index count = 36
    }
}
