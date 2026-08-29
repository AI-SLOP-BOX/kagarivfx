//! Professional 3D Engine: 2-Node Camera, Physical Lights (Inverse-Square Falloff),
//! Physical Depth of Field (CoC Bokeh), and HDRI Environment Map (IBL) (AE Parity).

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CameraType {
    OneNode,
    #[default]
    TwoNode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Camera3DProperties {
    pub camera_type: CameraType,
    pub position: [f32; 3],
    pub point_of_interest: [f32; 3],
    pub orientation_deg: [f32; 3],
    pub fov_deg: f32,
    pub focal_length_mm: f32,
    pub aperture_mm: f32,
    pub focus_distance: f32,
    pub blur_level_percent: f32,
}

impl Default for Camera3DProperties {
    fn default() -> Self {
        Self {
            camera_type: CameraType::TwoNode,
            position: [960.0, 540.0, -1500.0],
            point_of_interest: [960.0, 540.0, 0.0],
            orientation_deg: [0.0, 0.0, 0.0],
            fov_deg: 50.0,
            focal_length_mm: 50.0,
            aperture_mm: 20.0,
            focus_distance: 1500.0,
            blur_level_percent: 100.0,
        }
    }
}

impl Camera3DProperties {
    /// Computes View Matrix for 2-Node or 1-Node camera.
    pub fn compute_view_matrix(&self) -> [[f32; 4]; 4] {
        let eye = self.position;
        let target = if self.camera_type == CameraType::TwoNode {
            self.point_of_interest
        } else {
            // Forward from orientation
            [eye[0], eye[1], eye[2] + 1000.0]
        };

        let f = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
        let len = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt().max(1e-5);
        let f = [f[0] / len, f[1] / len, f[2] / len];

        let up_guide = [0.0f32, 1.0, 0.0];
        // Cross(f, up)
        let s = [
            f[1] * up_guide[2] - f[2] * up_guide[1],
            f[2] * up_guide[0] - f[0] * up_guide[2],
            f[0] * up_guide[1] - f[1] * up_guide[0],
        ];
        let s_len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt().max(1e-5);
        let s = [s[0] / s_len, s[1] / s_len, s[2] / s_len];

        // Cross(s, f)
        let u = [
            s[1] * f[2] - s[2] * f[1],
            s[2] * f[0] - s[0] * f[2],
            s[0] * f[1] - s[1] * f[0],
        ];

        [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [
                -(s[0] * eye[0] + s[1] * eye[1] + s[2] * eye[2]),
                -(u[0] * eye[0] + u[1] * eye[1] + u[2] * eye[2]),
                f[0] * eye[0] + f[1] * eye[1] + f[2] * eye[2],
                1.0,
            ],
        ]
    }

    /// Computes Circle of Confusion (CoC) diameter in pixels for a given depth Z.
    pub fn compute_circle_of_confusion(&self, z_depth: f32) -> f32 {
        let s = self.focus_distance.max(1.0);
        let z = z_depth.max(1.0);
        let a = self.aperture_mm;
        let f = self.focal_length_mm;

        if (s - f).abs() < 1e-3 || z <= f {
            return 0.0;
        }

        // Thin lens Circle of Confusion formula: c = |A * f * (s - z) / (z * (s - f))|
        let coc = (a * f * (s - z).abs() / (z * (s - f))).abs();
        coc * (self.blur_level_percent / 100.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum LightFalloff {
    None,
    Smooth,
    #[default]
    InverseSquare,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Advanced3DLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub falloff: LightFalloff,
    pub radius: f32,
}

impl Default for Advanced3DLight {
    fn default() -> Self {
        Self {
            position: [960.0, 200.0, -800.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            falloff: LightFalloff::InverseSquare,
            radius: 500.0,
        }
    }
}

impl Advanced3DLight {
    /// Computes illumination at surface point with realistic physical falloff.
    pub fn compute_irradiance(&self, surface_pos: [f32; 3]) -> [f32; 3] {
        let d = [
            self.position[0] - surface_pos[0],
            self.position[1] - surface_pos[1],
            self.position[2] - surface_pos[2],
        ];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1.0);

        let atten = match self.falloff {
            LightFalloff::None => 1.0,
            LightFalloff::Smooth => {
                let norm_d = (dist / self.radius.max(1.0)).clamp(0.0, 1.0);
                (1.0 - norm_d * norm_d).max(0.0)
            }
            LightFalloff::InverseSquare => {
                let r0 = self.radius.max(1.0);
                (r0 * r0) / (dist * dist + r0 * r0)
            }
        };

        let factor = self.intensity * atten;
        [
            self.color[0] * factor,
            self.color[1] * factor,
            self.color[2] * factor,
        ]
    }
}

/// Samples 360 HDRI Equirectangular environment map using surface normal vector.
pub fn sample_hdri_environment(
    env_hdr: &[f32], // Flat [R, G, B] floats
    width: u32,
    height: u32,
    normal: [f32; 3],
) -> [f32; 3] {
    if env_hdr.is_empty() || width == 0 || height == 0 {
        return [0.0, 0.0, 0.0];
    }

    let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
        .sqrt()
        .max(1e-5);
    let n = [normal[0] / len, normal[1] / len, normal[2] / len];

    // Spherical coordinates
    let u = (n[0].atan2(n[2]) / std::f32::consts::TAU + 0.5).clamp(0.0, 1.0);
    let v = ((-n[1]).asin() / std::f32::consts::PI + 0.5).clamp(0.0, 1.0);

    let px = ((u * (width - 1) as f32).round() as usize).min(width as usize - 1);
    let py = ((v * (height - 1) as f32).round() as usize).min(height as usize - 1);

    let idx = (py * width as usize + px) * 3;
    if idx + 2 < env_hdr.len() {
        [env_hdr[idx], env_hdr[idx + 1], env_hdr[idx + 2]]
    } else {
        [0.0, 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_node_camera_view_matrix_generation() {
        let cam = Camera3DProperties::default();
        let view = cam.compute_view_matrix();
        assert_eq!(view[3][3], 1.0);
        // Camera orientation along Z axis
        assert!(view[2][2].abs() > 0.5);
    }

    #[test]
    fn test_physical_dof_circle_of_confusion() {
        let cam = Camera3DProperties {
            focus_distance: 1000.0,
            focal_length_mm: 50.0,
            aperture_mm: 25.0,
            blur_level_percent: 100.0,
            ..Default::default()
        };

        // At exact focal distance, CoC is 0
        let coc_in_focus = cam.compute_circle_of_confusion(1000.0);
        assert!(coc_in_focus < 1e-4);

        // At foreground (e.g. 500mm), CoC > 0
        let coc_fg = cam.compute_circle_of_confusion(500.0);
        assert!(coc_fg > 1.0);
    }

    #[test]
    fn test_inverse_square_light_falloff() {
        let light = Advanced3DLight {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            falloff: LightFalloff::InverseSquare,
            radius: 100.0,
        };

        let close = light.compute_irradiance([0.0, 0.0, 50.0]);
        let far = light.compute_irradiance([0.0, 0.0, 200.0]);

        assert!(close[0] > far[0]);
    }
}
