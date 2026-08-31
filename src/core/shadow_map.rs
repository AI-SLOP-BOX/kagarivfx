//! 3D Lighting, Shadow Mapping & Material Shading Engine (AE Parity).
//!
//! Provides:
//! - Depth shadow mapping with Percentage-Closer Filtering (PCF) soft shadows
//! - Phong / Blinn-Phong shading with diffuse, specular, and ambient components
//! - Support for Point, Spot, Directional, and Ambient 3D lights

#![allow(dead_code)]

use crate::core::timeline::{Light3D, LightType, MaterialOptions};

#[derive(Debug, Clone)]
pub struct ShadowMap {
    pub width: u32,
    pub height: u32,
    pub depth_buffer: Vec<f32>,
    pub light_view_proj: [[f32; 4]; 4],
}

impl ShadowMap {
    pub fn new(width: u32, height: u32) -> Self {
        const MAX_SHADOW_PIXELS: usize = 16_777_216;
        let size = (width as usize)
            .checked_mul(height as usize)
            .filter(|&count| count <= MAX_SHADOW_PIXELS)
            .unwrap_or(0);
        Self {
            width,
            height,
            depth_buffer: vec![f32::INFINITY; size],
            light_view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Clears the depth buffer to infinity.
    pub fn clear(&mut self) {
        self.depth_buffer.fill(f32::INFINITY);
    }

    /// Records depth for a sample point in shadow map space.
    pub fn set_depth(&mut self, x: u32, y: u32, depth: f32) {
        if x < self.width && y < self.height {
            let idx = y as usize * self.width as usize + x as usize;
            if let Some(slot) = self.depth_buffer.get_mut(idx) {
                if depth.is_finite() && depth < *slot {
                    *slot = depth;
                }
            }
        }
    }

    /// Evaluates shadow visibility (0.0 = fully in shadow, 1.0 = fully lit)
    /// using 3x3 Percentage-Closer Filtering (PCF) for smooth soft shadow edges.
    pub fn sample_shadow_pcf(&self, sm_x: f32, sm_y: f32, current_depth: f32, bias: f32) -> f32 {
        if self.width == 0
            || self.height == 0
            || self.depth_buffer.len() != self.width as usize * self.height as usize
            || !sm_x.is_finite()
            || !sm_y.is_finite()
            || !current_depth.is_finite()
            || !bias.is_finite()
        {
            return 1.0;
        }
        let ix = sm_x.round() as i32;
        let iy = sm_y.round() as i32;

        let mut lit_samples = 0.0f32;
        let mut total_samples = 0.0f32;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let sx = (ix + dx).clamp(0, self.width as i32 - 1) as usize;
                let sy = (iy + dy).clamp(0, self.height as i32 - 1) as usize;
                let map_depth = self.depth_buffer[sy * self.width as usize + sx];

                if current_depth - bias <= map_depth {
                    lit_samples += 1.0;
                }
                total_samples += 1.0;
            }
        }

        lit_samples / total_samples
    }
}

/// Shading calculation for a 3D surface point interacting with lights and materials.
pub fn calculate_surface_shading(
    world_pos: [f32; 3],
    world_normal: [f32; 3],
    camera_pos: [f32; 3],
    base_color: [f32; 4],
    material: &MaterialOptions,
    lights: &[Light3D],
    shadow_map: Option<&ShadowMap>,
    current_frame: u32,
) -> [f32; 4] {
    let mut diffuse_acc = [0.0f32; 3];
    let mut specular_acc = [0.0f32; 3];
    let mut ambient_acc = [material.ambient * 0.01; 3];

    // View direction
    let vx = camera_pos[0] - world_pos[0];
    let vy = camera_pos[1] - world_pos[1];
    let vz = camera_pos[2] - world_pos[2];
    let v_len = (vx * vx + vy * vy + vz * vz).sqrt().max(1e-5);
    let view_dir = [vx / v_len, vy / v_len, vz / v_len];

    let n = world_normal;

    for light in lights {
        let (light_dir, attenuation) = match &light.light_type {
            LightType::Ambient => {
                let col = light.color;
                let intensity = light.intensity * 0.01;
                ambient_acc[0] += col[0] * intensity;
                ambient_acc[1] += col[1] * intensity;
                ambient_acc[2] += col[2] * intensity;
                continue;
            }
            LightType::Parallel => {
                let lx = -1.0f32;
                let ly = -1.0f32;
                let lz = 1.0f32;
                let len = (lx * lx + ly * ly + lz * lz).sqrt();
                ([lx / len, ly / len, lz / len], light.intensity * 0.01)
            }
            LightType::Point => {
                let lpos = light.position.evaluate(current_frame);
                let lx = lpos[0] - world_pos[0];
                let ly = lpos[1] - world_pos[1];
                let lz = lpos[2] - world_pos[2];
                let dist = (lx * lx + ly * ly + lz * lz).sqrt().max(1.0);
                let l_dir = [lx / dist, ly / dist, lz / dist];

                let falloff = 1.0 / (1.0 + 0.0001 * dist * dist);
                let atten = light.intensity * 0.01 * falloff;
                (l_dir, atten)
            }
            LightType::Spot { cone_angle_deg, .. } => {
                let lpos = light.position.evaluate(current_frame);
                let lx = lpos[0] - world_pos[0];
                let ly = lpos[1] - world_pos[1];
                let lz = lpos[2] - world_pos[2];
                let dist = (lx * lx + ly * ly + lz * lz).sqrt().max(1.0);
                let l_dir = [lx / dist, ly / dist, lz / dist];

                let falloff = 1.0 / (1.0 + 0.0001 * dist * dist);
                let mut atten = light.intensity * 0.01 * falloff;

                let cone_angle = (cone_angle_deg * 0.5).to_radians().cos();
                let spot_dir = [0.0f32, 0.0f32, -1.0f32];
                let dot_spot =
                    -(l_dir[0] * spot_dir[0] + l_dir[1] * spot_dir[1] + l_dir[2] * spot_dir[2]);
                if dot_spot < cone_angle {
                    atten = 0.0;
                }
                (l_dir, atten)
            }
        };

        if attenuation <= 0.0 {
            continue;
        }

        // Shadow factor
        let shadow = if material.cast_shadows && light.casts_shadows {
            if let Some(sm) = shadow_map {
                sm.sample_shadow_pcf(world_pos[0], world_pos[1], world_pos[2], 0.05)
            } else {
                1.0
            }
        } else {
            1.0
        };

        let l_col = light.color;

        // Diffuse (Lambert)
        let n_dot_l = (n[0] * light_dir[0] + n[1] * light_dir[1] + n[2] * light_dir[2]).max(0.0);
        let diff_factor = n_dot_l * (material.diffuse * 0.01) * attenuation * shadow;
        diffuse_acc[0] += l_col[0] * diff_factor;
        diffuse_acc[1] += l_col[1] * diff_factor;
        diffuse_acc[2] += l_col[2] * diff_factor;

        // Specular (Blinn-Phong)
        let hx = light_dir[0] + view_dir[0];
        let hy = light_dir[1] + view_dir[1];
        let hz = light_dir[2] + view_dir[2];
        let h_len = (hx * hx + hy * hy + hz * hz).sqrt().max(1e-5);
        let half_vec = [hx / h_len, hy / h_len, hz / h_len];

        let n_dot_h = (n[0] * half_vec[0] + n[1] * half_vec[1] + n[2] * half_vec[2]).max(0.0);
        let spec_power = material.specular_exponent.max(1.0);
        let spec_factor =
            n_dot_h.powf(spec_power) * (material.specular * 0.01) * attenuation * shadow;

        specular_acc[0] += l_col[0] * spec_factor;
        specular_acc[1] += l_col[1] * spec_factor;
        specular_acc[2] += l_col[2] * spec_factor;
    }

    let final_r =
        (base_color[0] * (ambient_acc[0] + diffuse_acc[0]) + specular_acc[0]).clamp(0.0, 1.0);
    let final_g =
        (base_color[1] * (ambient_acc[1] + diffuse_acc[1]) + specular_acc[1]).clamp(0.0, 1.0);
    let final_b =
        (base_color[2] * (ambient_acc[2] + diffuse_acc[2]) + specular_acc[2]).clamp(0.0, 1.0);

    [final_r, final_g, final_b, base_color[3]]
}

/// Calculates Fresnel reflectance using Schlick's approximation for dielectric surfaces.
pub fn calculate_fresnel_reflectance(view_dir: [f32; 3], normal: [f32; 3], ior: f32) -> f32 {
    let cos_theta = (view_dir[0] * normal[0] + view_dir[1] * normal[1] + view_dir[2] * normal[2])
        .abs()
        .clamp(0.0, 1.0);
    let n1 = 1.0f32; // air
    let n2 = ior.max(1.0);
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    (r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5)).clamp(0.0, 1.0)
}

/// Samples an equirectangular 360° HDRI environment map from a 3D reflection or view direction.
pub fn sample_equirectangular_env_map(
    dir: [f32; 3],
    env_pixels: &[u8],
    env_width: u32,
    env_height: u32,
) -> [f32; 3] {
    if env_pixels.is_empty() || env_width == 0 || env_height == 0 {
        return [0.0, 0.0, 0.0];
    }

    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-6);
    let dx = dir[0] / len;
    let dy = dir[1] / len;
    let dz = dir[2] / len;

    // Convert direction vector to spherical coordinates (u, v in 0..1)
    let u = (dz.atan2(dx) / (2.0 * std::f32::consts::PI) + 0.5).fract();
    let v = ((-dy).asin() / std::f32::consts::PI + 0.5).clamp(0.0, 1.0);

    let x = (u * (env_width - 1) as f32).clamp(0.0, (env_width - 1) as f32) as usize;
    let y = (v * (env_height - 1) as f32).clamp(0.0, (env_height - 1) as f32) as usize;

    let idx = (y * env_width as usize + x) * 4;
    if idx + 2 < env_pixels.len() {
        [
            env_pixels[idx] as f32 / 255.0,
            env_pixels[idx + 1] as f32 / 255.0,
            env_pixels[idx + 2] as f32 / 255.0,
        ]
    } else {
        [0.0, 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_map_bounds_allocation_and_invalid_samples() {
        let map = ShadowMap::new(u32::MAX, u32::MAX);
        assert!(map.depth_buffer.is_empty());
        assert_eq!(map.sample_shadow_pcf(0.0, 0.0, 0.0, 0.0), 1.0);

        let map = ShadowMap::new(2, 2);
        assert_eq!(map.sample_shadow_pcf(f32::NAN, 0.0, 0.0, 0.0), 1.0);
        assert_eq!(map.sample_shadow_pcf(0.0, 0.0, f32::INFINITY, 0.0), 1.0);
    }
    use crate::core::timeline::LightType;

    #[test]
    fn test_shadow_map_pcf_sampling() {
        let mut sm = ShadowMap::new(10, 10);
        sm.set_depth(5, 5, 10.0);

        let shadow_val = sm.sample_shadow_pcf(5.0, 5.0, 12.0, 0.01);
        assert!(
            shadow_val < 1.0,
            "Point behind occluder must receive shadow"
        );

        let lit_val = sm.sample_shadow_pcf(5.0, 5.0, 8.0, 0.01);
        assert_eq!(lit_val, 1.0, "Point in front must be 100% lit");
    }

    #[test]
    fn test_surface_shading_with_ambient_and_point_light() {
        let mat = MaterialOptions {
            ambient: 50.0,
            diffuse: 100.0,
            specular: 50.0,
            specular_exponent: 32.0,
            cast_shadows: true,
            ..Default::default()
        };

        let light = Light3D {
            id: "l1".into(),
            name: "Key Light".into(),
            light_type: LightType::Point,
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 100.0,
            casts_shadows: true,
            ..Default::default()
        };

        let shaded = calculate_surface_shading(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 100.0],
            [1.0, 0.5, 0.2, 1.0],
            &mat,
            &[light],
            None,
            0,
        );

        assert!(shaded[0] > 0.0 && shaded[0] <= 1.0);
        assert_eq!(shaded[3], 1.0);
    }

    #[test]
    fn test_fresnel_and_env_map_sampling() {
        let f = calculate_fresnel_reflectance([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], 1.5);
        assert!(f >= 0.0 && f <= 1.0);

        let env_pixels = vec![128u8; 4 * 4 * 4];
        let rgb = sample_equirectangular_env_map([0.0, 0.0, 1.0], &env_pixels, 4, 4);
        assert!((rgb[0] - 0.5).abs() < 0.1);
    }
}

