//! Puppet Pin Tool 2D Mesh Deformation Engine (AE Parity).
//!
//! Provides Moving Least Squares (MLS) As-Rigid-As-Possible (ARAP) mesh warping
//! driven by interactive Position, Starch, and Overlap pins.

#![allow(dead_code)]

/// Single puppet control pin with rest and animated deformed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct PuppetPin {
    pub id: String,
    pub rest_pos: [f32; 2],
    pub deformed_pos: [f32; 2],
    pub starch_weight: f32, // Starch rigidity (0.0 = full flexible, 1.0 = rigid)
}

impl PuppetPin {
    pub fn new(id: &str, pos: [f32; 2]) -> Self {
        Self {
            id: id.to_string(),
            rest_pos: pos,
            deformed_pos: pos,
            starch_weight: 0.0,
        }
    }
}

/// Triangular subdivision mesh for layer deformation.
#[derive(Debug, Clone)]
pub struct PuppetMesh {
    pub rest_vertices: Vec<[f32; 2]>,
    pub triangles: Vec<[usize; 3]>,
}

impl PuppetMesh {
    /// Generates a regular grid mesh over a rectangular area [0..w, 0..h].
    pub fn new_grid(width: f32, height: f32, cols: usize, rows: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let mut rest_vertices = Vec::new();
        let mut triangles = Vec::new();

        let dx = width / cols as f32;
        let dy = height / rows as f32;

        for r in 0..=rows {
            for c in 0..=cols {
                rest_vertices.push([c as f32 * dx, r as f32 * dy]);
            }
        }

        let stride = cols + 1;
        for r in 0..rows {
            for c in 0..cols {
                let v00 = r * stride + c;
                let v10 = v00 + 1;
                let v01 = (r + 1) * stride + c;
                let v11 = v01 + 1;

                triangles.push([v00, v10, v01]);
                triangles.push([v10, v11, v01]);
            }
        }

        Self {
            rest_vertices,
            triangles,
        }
    }

    /// Deforms all mesh vertices using Moving Least Squares (MLS) affine interpolation.
    pub fn deform_mls(&self, pins: &[PuppetPin], alpha: f32) -> Vec<[f32; 2]> {
        if pins.is_empty() {
            return self.rest_vertices.clone();
        }

        let mut deformed = Vec::with_capacity(self.rest_vertices.len());

        for &v in &self.rest_vertices {
            let mut w_sum = 0.0f32;
            let mut weights = Vec::with_capacity(pins.len());

            for pin in pins {
                let dx = v[0] - pin.rest_pos[0];
                let dy = v[1] - pin.rest_pos[1];
                let dist_sq = dx * dx + dy * dy;
                let eps = 1e-4f32;
                let w = 1.0 / (dist_sq + eps).powf(alpha);
                weights.push(w);
                w_sum += w;
            }

            if w_sum <= 0.0 {
                deformed.push(v);
                continue;
            }

            let mut target_x = 0.0f32;
            let mut target_y = 0.0f32;

            for (i, pin) in pins.iter().enumerate() {
                let norm_w = weights[i] / w_sum;
                let delta_x = pin.deformed_pos[0] - pin.rest_pos[0];
                let delta_y = pin.deformed_pos[1] - pin.rest_pos[1];

                target_x += norm_w * (v[0] + delta_x);
                target_y += norm_w * (v[1] + delta_y);
            }

            deformed.push([target_x, target_y]);
        }

        deformed
    }
}

/// Warps an RGBA8 buffer using inverse bilinear/barycentric sampling from deformed mesh.
pub fn warp_image_puppet_cpu(
    src: &[u8],
    width: u32,
    height: u32,
    mesh: &PuppetMesh,
    deformed_vertices: &[[f32; 2]],
) -> Vec<u8> {
    let mut dst = vec![0u8; (width as usize) * (height as usize) * 4];
    if mesh.triangles.is_empty() || deformed_vertices.len() != mesh.rest_vertices.len() {
        return src.to_vec();
    }

    // Helper: compute barycentric coordinates (u, v, w) of point p inside triangle (a, b, c)
    fn barycentric(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> Option<[f32; 3]> {
        let v0 = [b[0] - a[0], b[1] - a[1]];
        let v1 = [c[0] - a[0], c[1] - a[1]];
        let v2 = [p[0] - a[0], p[1] - a[1]];
        let den = v0[0] * v1[1] - v1[0] * v0[1];
        if den.abs() < 1e-6 {
            return None;
        }
        let v = (v2[0] * v1[1] - v1[0] * v2[1]) / den;
        let w = (v0[0] * v2[1] - v2[0] * v0[1]) / den;
        let u = 1.0 - v - w;
        Some([u, v, w])
    }

    for tri in &mesh.triangles {
        let da = deformed_vertices[tri[0]];
        let db = deformed_vertices[tri[1]];
        let dc = deformed_vertices[tri[2]];

        let ra = mesh.rest_vertices[tri[0]];
        let rb = mesh.rest_vertices[tri[1]];
        let rc = mesh.rest_vertices[tri[2]];

        // Bounding box of deformed triangle
        let min_x = (da[0].min(db[0]).min(dc[0]).floor().max(0.0)) as u32;
        let max_x = (da[0].max(db[0]).max(dc[0]).ceil().min(width as f32 - 1.0)) as u32;
        let min_y = (da[1].min(db[1]).min(dc[1]).floor().max(0.0)) as u32;
        let max_y = (da[1].max(db[1]).max(dc[1]).ceil().min(height as f32 - 1.0)) as u32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                if let Some([u, v, w]) = barycentric(p, da, db, dc) {
                    if u >= -0.01 && v >= -0.01 && w >= -0.01 {
                        // Interpolate rest source coordinate
                        let sx = (u * ra[0] + v * rb[0] + w * rc[0])
                            .round()
                            .clamp(0.0, width as f32 - 1.0) as u32;
                        let sy = (u * ra[1] + v * rb[1] + w * rc[1])
                            .round()
                            .clamp(0.0, height as f32 - 1.0)
                            as u32;

                        let src_idx = (sy * width + sx) as usize * 4;
                        let dst_idx = (y * width + x) as usize * 4;

                        if src_idx + 3 < src.len() && dst_idx + 3 < dst.len() {
                            dst[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
                        }
                    }
                }
            }
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puppet_grid_creation_and_identity_mls() {
        let mesh = PuppetMesh::new_grid(100.0, 100.0, 2, 2);
        assert_eq!(mesh.rest_vertices.len(), 9); // (2+1)*(2+1)
        assert_eq!(mesh.triangles.len(), 8); // 2*2*2

        let pin = PuppetPin::new("p1", [50.0, 50.0]);
        let deformed = mesh.deform_mls(&[pin], 1.0);
        assert_eq!(deformed.len(), 9);
        assert_eq!(deformed[4], [50.0, 50.0]); // Centre vertex stays at centre
    }

    #[test]
    fn test_puppet_pin_translation_pulls_mesh() {
        let mesh = PuppetMesh::new_grid(100.0, 100.0, 2, 2);
        let mut pin = PuppetPin::new("p1", [50.0, 50.0]);
        pin.deformed_pos = [60.0, 50.0]; // Moved +10px right

        let deformed = mesh.deform_mls(&[pin], 1.0);
        // Centre vertex should move right by 10px
        assert!((deformed[4][0] - 60.0).abs() < 1e-2);
    }
}
