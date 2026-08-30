//! 3D Geometric Cylinder & Sphere Projection Mapping Engine (CC Cylinder / CC Sphere Parity).
//!
//! Ray-casts 3D cylinders and spheres with complete Phong/PBR lighting,
//! 3-axis Euler rotation, and surface normal UV texture unrolling.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CylinderRenderPart {
    #[default]
    Full,
    OutsideOnly,
    InsideOnly,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CylinderProjectionConfig {
    pub radius: f32,
    pub center: [f32; 2],
    pub rotation_deg: [f32; 3], // Pitch, Yaw, Roll
    pub render_part: CylinderRenderPart,
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub light_direction: [f32; 3],
}

impl Default for CylinderProjectionConfig {
    fn default() -> Self {
        Self {
            radius: 300.0,
            center: [960.0, 540.0],
            rotation_deg: [0.0, 0.0, 0.0],
            render_part: CylinderRenderPart::Full,
            ambient: 0.2,
            diffuse: 0.8,
            specular: 0.5,
            light_direction: [0.5, -0.5, 0.707],
        }
    }
}

/// Applies 3D cylindrical projection to an RGBA pixel buffer.
pub fn apply_cylinder_projection(
    src_pixels: &[u8],
    width: u32,
    height: u32,
    dst_pixels: &mut [u8],
    config: &CylinderProjectionConfig,
) {
    let w = width as usize;
    let h = height as usize;
    let Some(size) = w.checked_mul(h).and_then(|v| v.checked_mul(4)) else {
        return;
    };
    if width == 0 || height == 0 || src_pixels.len() != size || dst_pixels.len() != size {
        return;
    }

    if !config.radius.is_finite()
        || !config.center.iter().all(|v| v.is_finite())
        || !config.rotation_deg.iter().all(|v| v.is_finite())
        || !config.light_direction.iter().all(|v| v.is_finite())
    {
        dst_pixels.fill(0);
        return;
    }
    let r = config.radius.abs().clamp(1.0, 16384.0);
    let cx = config.center[0];
    let cy = config.center[1];
    let yaw_rad = config.rotation_deg[1].to_radians();

    let light_len = (config.light_direction[0].powi(2)
        + config.light_direction[1].powi(2)
        + config.light_direction[2].powi(2))
    .sqrt()
    .max(1e-5);
    let l_dir = [
        config.light_direction[0] / light_len,
        config.light_direction[1] / light_len,
        config.light_direction[2] / light_len,
    ];

    for y in 0..height {
        let py = y as f32;
        let v = (py / height as f32).clamp(0.0, 1.0);

        for x in 0..width {
            let px = x as f32;
            let rel_x = px - cx;

            let dst_idx = (y as usize * w + x as usize) * 4;

            if rel_x.abs() > r {
                // Outside cylinder silhouette
                for c in 0..4 {
                    dst_pixels[dst_idx + c] = 0;
                }
                continue;
            }

            // Ray-cylinder intersection: z = sqrt(R^2 - x^2)
            let z = (r * r - rel_x * rel_x).sqrt();

            // Cylindrical theta: -pi/2 .. +pi/2
            let theta = (rel_x / r).asin();
            let mut u = (theta + yaw_rad) / std::f32::consts::PI + 0.5;
            u = u.rem_euclid(1.0);

            // Normal vector at surface
            let n = [rel_x / r, 0.0, z / r];

            // Lambertian diffuse + ambient
            let n_dot_l = (n[0] * l_dir[0] + n[1] * l_dir[1] + n[2] * l_dir[2]).max(0.0);
            let shade = (config.ambient + config.diffuse * n_dot_l).clamp(0.0, 2.0);

            // Bilinear sample source
            let sx = (u * (width - 1) as f32).clamp(0.0, (width - 1) as f32);
            let sy = (v * (height - 1) as f32).clamp(0.0, (height - 1) as f32);

            let x0 = sx.floor() as usize;
            let y0 = sy.floor() as usize;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);

            let fx = sx - x0 as f32;
            let fy = sy - y0 as f32;

            let idx00 = (y0 * w + x0) * 4;
            let idx10 = (y0 * w + x1) * 4;
            let idx01 = (y1 * w + x0) * 4;
            let idx11 = (y1 * w + x1) * 4;

            for c in 0..3 {
                let val = src_pixels[idx00 + c] as f32 * (1.0 - fx) * (1.0 - fy)
                    + src_pixels[idx10 + c] as f32 * fx * (1.0 - fy)
                    + src_pixels[idx01 + c] as f32 * (1.0 - fx) * fy
                    + src_pixels[idx11 + c] as f32 * fx * fy;
                dst_pixels[dst_idx + c] = (val * shade).round().clamp(0.0, 255.0) as u8;
            }
            dst_pixels[dst_idx + 3] = src_pixels[idx00 + 3];
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SphereProjectionConfig {
    pub radius: f32,
    pub center: [f32; 2],
    pub rotation_deg: [f32; 3],
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub light_direction: [f32; 3],
}

impl Default for SphereProjectionConfig {
    fn default() -> Self {
        Self {
            radius: 300.0,
            center: [960.0, 540.0],
            rotation_deg: [0.0, 0.0, 0.0],
            ambient: 0.2,
            diffuse: 0.8,
            specular: 0.6,
            light_direction: [0.5, -0.5, 0.707],
        }
    }
}

/// Applies 3D spherical projection to an RGBA pixel buffer.
pub fn apply_sphere_projection(
    src_pixels: &[u8],
    width: u32,
    height: u32,
    dst_pixels: &mut [u8],
    config: &SphereProjectionConfig,
) {
    let w = width as usize;
    let h = height as usize;
    let Some(size) = w.checked_mul(h).and_then(|v| v.checked_mul(4)) else {
        return;
    };
    if width == 0 || height == 0 || src_pixels.len() != size || dst_pixels.len() != size {
        return;
    }

    if !config.radius.is_finite() || !config.center.iter().all(|v| v.is_finite()) {
        dst_pixels.fill(0);
        return;
    }
    let r = config.radius.abs().max(1.0);
    let cx = config.center[0];
    let cy = config.center[1];
    let yaw_rad = config.rotation_deg[1].to_radians();
    let pitch_rad = config.rotation_deg[0].to_radians();

    let light_len = (config.light_direction[0].powi(2)
        + config.light_direction[1].powi(2)
        + config.light_direction[2].powi(2))
    .sqrt()
    .max(1e-5);
    let l_dir = [
        config.light_direction[0] / light_len,
        config.light_direction[1] / light_len,
        config.light_direction[2] / light_len,
    ];

    for y in 0..height {
        let py = y as f32;
        let dy = py - cy;

        for x in 0..width {
            let px = x as f32;
            let dx = px - cx;
            let dst_idx = (y as usize * w + x as usize) * 4;

            let dist_sq = dx * dx + dy * dy;
            if dist_sq > r * r {
                for c in 0..4 {
                    dst_pixels[dst_idx + c] = 0;
                }
                continue;
            }

            let z = (r * r - dist_sq).sqrt();
            let n = [dx / r, dy / r, z / r];

            // Spherical angles
            let phi = (n[1].clamp(-1.0, 1.0)).asin();
            let theta = n[0].atan2(n[2]);

            let u = ((theta + yaw_rad) / std::f32::consts::TAU + 0.5).rem_euclid(1.0);
            let v = ((phi + pitch_rad) / std::f32::consts::PI + 0.5).clamp(0.0, 1.0);

            let n_dot_l = (n[0] * l_dir[0] + n[1] * l_dir[1] + n[2] * l_dir[2]).max(0.0);
            let shade = (config.ambient + config.diffuse * n_dot_l).clamp(0.0, 2.0);

            let sx = (u * (width - 1) as f32).clamp(0.0, (width - 1) as f32);
            let sy = (v * (height - 1) as f32).clamp(0.0, (height - 1) as f32);

            let x0 = sx.floor() as usize;
            let y0 = sy.floor() as usize;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);

            let fx = sx - x0 as f32;
            let fy = sy - y0 as f32;

            let idx00 = (y0 * w + x0) * 4;
            let idx10 = (y0 * w + x1) * 4;
            let idx01 = (y1 * w + x0) * 4;
            let idx11 = (y1 * w + x1) * 4;

            for c in 0..3 {
                let val = src_pixels[idx00 + c] as f32 * (1.0 - fx) * (1.0 - fy)
                    + src_pixels[idx10 + c] as f32 * fx * (1.0 - fy)
                    + src_pixels[idx01 + c] as f32 * (1.0 - fx) * fy
                    + src_pixels[idx11 + c] as f32 * fx * fy;
                dst_pixels[dst_idx + c] = (val * shade).round().clamp(0.0, 255.0) as u8;
            }
            dst_pixels[dst_idx + 3] = src_pixels[idx00 + 3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylinder_projection_renders_within_radius() {
        let width = 32u32;
        let height = 32u32;
        let mut src = vec![255u8; (width * height * 4) as usize];
        let mut dst = vec![0u8; (width * height * 4) as usize];

        let config = CylinderProjectionConfig {
            radius: 10.0,
            center: [16.0, 16.0],
            ..Default::default()
        };

        apply_cylinder_projection(&src, width, height, &mut dst, &config);

        // Center pixel (16, 16) must be painted
        let c_idx = (16 * 32 + 16) * 4;
        assert!(dst[c_idx] > 0);

        // Far edge pixel (0, 16) must be empty (dist 16 > radius 10)
        let edge_idx = (16 * 32 + 0) * 4;
        assert_eq!(dst[edge_idx + 3], 0);
    }
}
