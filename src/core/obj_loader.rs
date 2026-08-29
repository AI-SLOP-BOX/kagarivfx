//! Wavefront OBJ 3D Model Mesh Parser & PBR Material System (AE Parity).
//!
//! Enables importing 3D models (.obj) into compositions with automatic triangulation,
//! vertex normal generation, UV texture mapping, and PBR material properties.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PbrMaterial3D {
    pub name: String,
    pub albedo_color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
    pub emission: [f32; 3],
    pub albedo_texture: Option<String>,
    pub normal_map: Option<String>,
}

impl Default for PbrMaterial3D {
    fn default() -> Self {
        Self {
            name: "DefaultMaterial".to_string(),
            albedo_color: [0.8, 0.8, 0.8, 1.0],
            roughness: 0.5,
            metallic: 0.0,
            emission: [0.0, 0.0, 0.0],
            albedo_texture: None,
            normal_map: None,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Mesh3D {
    pub name: String,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub materials: Vec<PbrMaterial3D>,
    pub material_indices: Vec<usize>, // Material index per triangle
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

impl Mesh3D {
    pub fn new(name: String) -> Self {
        Self {
            name,
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            materials: vec![PbrMaterial3D::default()],
            material_indices: Vec::new(),
            bbox_min: [f32::INFINITY; 3],
            bbox_max: [f32::NEG_INFINITY; 3],
        }
    }

    /// Computes bounding box dimensions and center.
    pub fn update_bounds(&mut self) {
        let mut bmin = [f32::INFINITY; 3];
        let mut bmax = [f32::NEG_INFINITY; 3];

        for v in &self.vertices {
            bmin[0] = bmin[0].min(v[0]);
            bmin[1] = bmin[1].min(v[1]);
            bmin[2] = bmin[2].min(v[2]);

            bmax[0] = bmax[0].max(v[0]);
            bmax[1] = bmax[1].max(v[1]);
            bmax[2] = bmax[2].max(v[2]);
        }

        self.bbox_min = bmin;
        self.bbox_max = bmax;
    }

    /// Calculates smooth per-vertex normals from triangle face normals.
    pub fn compute_smooth_normals(&mut self) {
        if self.vertices.is_empty() || self.indices.is_empty() {
            return;
        }

        let mut normals = vec![[0.0f32; 3]; self.vertices.len()];

        for chunk in self.indices.chunks_exact(3) {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            if i0 < self.vertices.len() && i1 < self.vertices.len() && i2 < self.vertices.len() {
                let v0 = self.vertices[i0];
                let v1 = self.vertices[i1];
                let v2 = self.vertices[i2];

                let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

                // Cross product
                let fnorm = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];

                for &idx in &[i0, i1, i2] {
                    normals[idx][0] += fnorm[0];
                    normals[idx][1] += fnorm[1];
                    normals[idx][2] += fnorm[2];
                }
            }
        }

        for n in &mut normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-5);
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        }

        self.normals = normals;
    }

    /// Möller-Trumbore Ray-Triangle intersection test.
    pub fn ray_intersect(&self, ray_orig: [f32; 3], ray_dir: [f32; 3]) -> Option<(f32, usize)> {
        let mut closest_t = f32::INFINITY;
        let mut hit_tri = None;

        for (tri_idx, chunk) in self.indices.chunks_exact(3).enumerate() {
            let (Some(&v0), Some(&v1), Some(&v2)) = (
                self.vertices.get(chunk[0] as usize),
                self.vertices.get(chunk[1] as usize),
                self.vertices.get(chunk[2] as usize),
            ) else {
                continue;
            };

            if let Some(t) = ray_triangle_intersect(ray_orig, ray_dir, v0, v1, v2) {
                if t > 0.001 && t < closest_t {
                    closest_t = t;
                    hit_tri = Some(tri_idx);
                }
            }
        }

        hit_tri.map(|tri| (closest_t, tri))
    }
}

/// Parses Wavefront OBJ format text content.
pub fn parse_obj_str(content: &str) -> Result<Mesh3D, String> {
    let mut raw_positions: Vec<[f32; 3]> = Vec::new();
    let mut raw_uvs: Vec<[f32; 2]> = Vec::new();
    let mut raw_normals: Vec<[f32; 3]> = Vec::new();

    let mut mesh = Mesh3D::new("ImportedMesh".to_string());

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(tag) = parts.next() else { continue };

        match tag {
            "v" => {
                let x: f32 = parts.next().ok_or_else(|| "missing vertex x".to_string())?.parse().map_err(|_| "invalid vertex coordinate".to_string())?;
                let y: f32 = parts.next().ok_or_else(|| "missing vertex y".to_string())?.parse().map_err(|_| "invalid vertex coordinate".to_string())?;
                let z: f32 = parts.next().ok_or_else(|| "missing vertex z".to_string())?.parse().map_err(|_| "invalid vertex coordinate".to_string())?;
                raw_positions.push([x, y, z]);
            }
            "vt" => {
                let u: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let v: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                raw_uvs.push([u, v]);
            }
            "vn" => {
                let nx: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let ny: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let nz: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                raw_normals.push([nx, ny, nz]);
            }
            "f" => {
                let mut face_indices = Vec::new();
                for token in parts {
                    // Formats: v, v/vt, v/vt/vn, v//vn
                    let subparts: Vec<&str> = token.split('/').collect();
                    let raw = subparts
                        .first()
                        .ok_or_else(|| "missing face index".to_string())?;
                    let parsed: i32 = raw
                        .parse()
                        .map_err(|_| format!("invalid face index: {raw}"))?;
                    let v_idx = if parsed < 0 {
                        raw_positions
                            .len()
                            .checked_sub(parsed.unsigned_abs() as usize)
                    } else if parsed > 0 {
                        Some(parsed as usize - 1)
                    } else {
                        None
                    };
                    let Some(v_idx) = v_idx.filter(|&i| i < raw_positions.len()) else {
                        return Err(format!("face index out of range: {raw}"));
                    };
                    face_indices.push(v_idx);
                }

                // Triangulate n-gons into triangle fan
                if face_indices.len() >= 3 {
                    for i in 1..(face_indices.len() - 1) {
                        mesh.indices.push(face_indices[0] as u32);
                        mesh.indices.push(face_indices[i] as u32);
                        mesh.indices.push(face_indices[i + 1] as u32);
                    }
                }
            }
            _ => {}
        }
    }

    mesh.vertices = raw_positions;
    if !raw_normals.is_empty() && raw_normals.len() == mesh.vertices.len() {
        mesh.normals = raw_normals;
    } else {
        mesh.compute_smooth_normals();
    }
    mesh.uvs = raw_uvs;
    mesh.update_bounds();

    Ok(mesh)
}

fn ray_triangle_intersect(
    orig: [f32; 3],
    dir: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<f32> {
    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    let pvec = [
        dir[1] * e2[2] - dir[2] * e2[1],
        dir[2] * e2[0] - dir[0] * e2[2],
        dir[0] * e2[1] - dir[1] * e2[0],
    ];

    let det = e1[0] * pvec[0] + e1[1] * pvec[1] + e1[2] * pvec[2];
    if det.abs() < 1e-6 {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = [orig[0] - v0[0], orig[1] - v0[1], orig[2] - v0[2]];
    let u = (tvec[0] * pvec[0] + tvec[1] * pvec[1] + tvec[2] * pvec[2]) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let qvec = [
        tvec[1] * e1[2] - tvec[2] * e1[1],
        tvec[2] * e1[0] - tvec[0] * e1[2],
        tvec[0] * e1[1] - tvec[1] * e1[0],
    ];

    let v = (dir[0] * qvec[0] + dir[1] * qvec[1] + dir[2] * qvec[2]) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = (e2[0] * qvec[0] + e2[1] * qvec[1] + e2[2] * qvec[2]) * inv_det;
    if t > 1e-5 {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_obj_cube() {
        let obj_data = r#"
# Simple unit quad
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
f 1 2 3 4
"#;
        let mesh = parse_obj_str(obj_data).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6); // 2 triangles for quad
        assert_eq!(mesh.normals.len(), 4);
        assert_eq!(mesh.bbox_min, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.bbox_max, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn test_ray_mesh_intersection() {
        let obj_data = r#"
v -1.0 -1.0 0.0
v  1.0 -1.0 0.0
v  0.0  1.0 0.0
f 1 2 3
"#;
        let mesh = parse_obj_str(obj_data).unwrap();
        let hit = mesh.ray_intersect([0.0, 0.0, -5.0], [0.0, 0.0, 1.0]);
        assert!(hit.is_some());
        let (t, tri) = hit.unwrap();
        assert!((t - 5.0).abs() < 1e-3);
        assert_eq!(tri, 0);
    }

    #[test]
    fn test_obj_negative_indices_reference_previous_vertices() {
        let mesh = parse_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_obj_invalid_face_index_is_rejected() {
        assert!(parse_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 9\n").is_err());
    }
}
