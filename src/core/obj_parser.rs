//! Wavefront OBJ 3D Mesh Parser & Importer.
//!
//! Loads 3D meshes with vertices, normals, and texture coordinates for 3D compositing layers.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh3DVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh3DTriangle {
    pub vertices: [Mesh3DVertex; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Mesh3DModel {
    pub name: String,
    pub triangles: Vec<Mesh3DTriangle>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

impl Mesh3DModel {
    /// Parses a Wavefront OBJ string format.
    pub fn parse_obj(obj_str: &str) -> Result<Self, String> {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut triangles: Vec<Mesh3DTriangle> = Vec::new();

        let mut bounds_min = [f32::INFINITY; 3];
        let mut bounds_max = [f32::NEG_INFINITY; 3];

        for line in obj_str.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().map_err(|e| format!("Invalid v.x: {e}"))?;
                        let y: f32 = parts[2].parse().map_err(|e| format!("Invalid v.y: {e}"))?;
                        let z: f32 = parts[3].parse().map_err(|e| format!("Invalid v.z: {e}"))?;
                        positions.push([x, y, z]);

                        bounds_min[0] = bounds_min[0].min(x);
                        bounds_min[1] = bounds_min[1].min(y);
                        bounds_min[2] = bounds_min[2].min(z);

                        bounds_max[0] = bounds_max[0].max(x);
                        bounds_max[1] = bounds_max[1].max(y);
                        bounds_max[2] = bounds_max[2].max(z);
                    }
                }
                "vn" => {
                    if parts.len() >= 4 {
                        let nx: f32 = parts[1].parse().map_err(|e| format!("Invalid vn.x: {e}"))?;
                        let ny: f32 = parts[2].parse().map_err(|e| format!("Invalid vn.y: {e}"))?;
                        let nz: f32 = parts[3].parse().map_err(|e| format!("Invalid vn.z: {e}"))?;
                        normals.push([nx, ny, nz]);
                    }
                }
                "vt" => {
                    if parts.len() >= 3 {
                        let u: f32 = parts[1].parse().map_err(|e| format!("Invalid vt.u: {e}"))?;
                        let v: f32 = parts[2].parse().map_err(|e| format!("Invalid vt.v: {e}"))?;
                        uvs.push([u, v]);
                    }
                }
                "f" => {
                    if parts.len() >= 4 {
                        let parse_vertex = |spec: &str| -> Result<Mesh3DVertex, String> {
                            let tokens: Vec<&str> = spec.split('/').collect();
                            let pos_idx: usize =
                                tokens[0].parse().map_err(|e| format!("Face idx: {e}"))?;
                            let pos = *positions
                                .get(pos_idx.checked_sub(1).ok_or("0-index")?)
                                .ok_or("Vertex position index out of range")?;

                            let uv = if tokens.len() > 1 && !tokens[1].is_empty() {
                                let uv_idx: usize =
                                    tokens[1].parse().map_err(|e| format!("UV idx: {e}"))?;
                                *uvs.get(uv_idx.checked_sub(1).ok_or("0-index")?)
                                    .unwrap_or(&[0.0, 0.0])
                            } else {
                                [0.0, 0.0]
                            };

                            let normal = if tokens.len() > 2 && !tokens[2].is_empty() {
                                let norm_idx: usize =
                                    tokens[2].parse().map_err(|e| format!("Norm idx: {e}"))?;
                                *normals
                                    .get(norm_idx.checked_sub(1).ok_or("0-index")?)
                                    .unwrap_or(&[0.0, 0.0, 1.0])
                            } else {
                                [0.0, 0.0, 1.0]
                            };

                            Ok(Mesh3DVertex {
                                position: pos,
                                normal,
                                uv,
                            })
                        };

                        let v0 = parse_vertex(parts[1])?;
                        let v1 = parse_vertex(parts[2])?;
                        let v2 = parse_vertex(parts[3])?;

                        triangles.push(Mesh3DTriangle {
                            vertices: [v0, v1, v2],
                        });

                        // Quad fan polygon triangulation
                        if parts.len() >= 5 {
                            let v3 = parse_vertex(parts[4])?;
                            let v0_clone = parse_vertex(parts[1])?;
                            let v2_clone = parse_vertex(parts[3])?;
                            triangles.push(Mesh3DTriangle {
                                vertices: [v0_clone, v2_clone, v3],
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if positions.is_empty() {
            bounds_min = [0.0, 0.0, 0.0];
            bounds_max = [0.0, 0.0, 0.0];
        }

        Ok(Self {
            name: "Imported Mesh".into(),
            triangles,
            bounds_min,
            bounds_max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_obj_triangle() {
        let obj_data = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
f 1/1/1 2/2/1 3/3/1
";
        let mesh = Mesh3DModel::parse_obj(obj_data).expect("Parsing OBJ succeeds");
        assert_eq!(mesh.triangles.len(), 1);
        assert_eq!(mesh.triangles[0].vertices[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(mesh.bounds_max[0], 1.0);
    }
}
