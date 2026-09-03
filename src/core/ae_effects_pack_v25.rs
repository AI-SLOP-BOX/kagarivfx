#![allow(dead_code)]
/// After Effects VFX Kernels Part 25 — 3D Space & Environment Rendering
// 1. Environment Map / Sphere Mapping (Reflection Probe Lookups)
pub fn apply_sphere_env_map(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    env_map: &[u8],
    env_width: u32,
    env_height: u32,
) {
    let temp = pixels.to_vec();
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = cx.min(cy);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / max_r;
            let dy = (y as f32 - cy) / max_r;
            let r2 = dx * dx + dy * dy;

            if r2 <= 1.0 {
                let dz = (1.0 - r2).sqrt();
                // Reflect view vector (0,0,1) about sphere normal (dx,dy,dz)
                let rx = 2.0 * dz * dx;
                let ry = 2.0 * dz * dy;

                let env_u = ((rx * 0.5 + 0.5) * (env_width - 1) as f32) as usize;
                let env_v = ((ry * 0.5 + 0.5) * (env_height - 1) as f32) as usize;
                let env_idx = (env_v * env_width as usize + env_u) * 4;

                let dst_idx = (y as usize * width as usize + x as usize) * 4;
                if env_idx + 3 < env_map.len() {
                    for c in 0..4 {
                        pixels[dst_idx + c] =
                            ((temp[dst_idx + c] as u16 + env_map[env_idx + c] as u16) / 2) as u8;
                    }
                }
            }
        }
    }
}

// 2. Z-Depth Fog (Atmospheric Depth Haze)
pub fn apply_z_depth_fog(
    pixels: &mut [u8],
    depth_map: &[u8],
    fog_color: [u8; 3],
    fog_near: f32,
    fog_far: f32,
) {
    for i in (0..pixels.len()).step_by(4) {
        let depth = if i < depth_map.len() {
            depth_map[i] as f32 / 255.0
        } else {
            1.0
        };

        let fog_factor = ((depth - fog_near) / (fog_far - fog_near).max(0.001)).clamp(0.0, 1.0);

        for c in 0..3 {
            pixels[i + c] = (pixels[i + c] as f32 * (1.0 - fog_factor)
                + fog_color[c] as f32 * fog_factor)
                .clamp(0.0, 255.0) as u8;
        }
    }
}

// 3. God Rays (Volumetric Radial Light Scattering)
pub fn apply_god_rays(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    sun_pos: [f32; 2],
    num_samples: u32,
    decay: f32,
    weight: f32,
) {
    if num_samples == 0 {
        return;
    }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let mut col = [0.0f32; 3];
            let mut illumination_decay = 1.0f32;

            let step_x = (sun_pos[0] - x as f32) / num_samples as f32;
            let step_y = (sun_pos[1] - y as f32) / num_samples as f32;

            let mut sx = x as f32;
            let mut sy = y as f32;

            for _ in 0..num_samples {
                let px = sx.clamp(0.0, (width - 1) as f32) as usize;
                let py = sy.clamp(0.0, (height - 1) as f32) as usize;
                let s_idx = (py * width as usize + px) * 4;

                col[0] += temp[s_idx] as f32 * illumination_decay * weight;
                col[1] += temp[s_idx + 1] as f32 * illumination_decay * weight;
                col[2] += temp[s_idx + 2] as f32 * illumination_decay * weight;

                illumination_decay *= decay;
                sx += step_x;
                sy += step_y;
            }

            let idx = (y as usize * width as usize + x as usize) * 4;
            let n = num_samples as f32;
            pixels[idx] = (temp[idx] as f32 + col[0] / n).clamp(0.0, 255.0) as u8;
            pixels[idx + 1] = (temp[idx + 1] as f32 + col[1] / n).clamp(0.0, 255.0) as u8;
            pixels[idx + 2] = (temp[idx + 2] as f32 + col[2] / n).clamp(0.0, 255.0) as u8;
        }
    }
}

// 4. Screen Space Ambient Occlusion (Simple 2D AO Approximation)
pub fn apply_ssao_approx(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    depth_map: &[u8],
    radius: u32,
    strength: f32,
) {
    if radius == 0 {
        return;
    }
    let temp_depth = depth_map.to_vec();
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let center_depth = if idx < temp_depth.len() {
                temp_depth[idx] as f32
            } else {
                128.0
            };

            let mut occlusion = 0.0f32;
            let mut count = 0u32;

            for ky in -r..=r {
                let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -r..=r {
                    if kx == 0 && ky == 0 {
                        continue;
                    }
                    let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                    let n_idx = (py * width as usize + px) * 4;
                    let neighbor_depth = if n_idx < temp_depth.len() {
                        temp_depth[n_idx] as f32
                    } else {
                        128.0
                    };
                    let depth_diff = (neighbor_depth - center_depth).max(0.0);
                    occlusion += depth_diff / 255.0;
                    count += 1;
                }
            }

            let ao = 1.0 - (occlusion / count.max(1) as f32 * strength).clamp(0.0, 1.0);
            pixels[idx] = (pixels[idx] as f32 * ao) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 * ao) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 * ao) as u8;
        }
    }
}

// 5. Gobo / Cookie Projection (Mask-Based Shadow Projection)
pub fn apply_gobo_projection(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    cookie: &[u8],
    light_pos: [f32; 2],
    falloff: f32,
) {
    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let cookie_idx = idx.min(cookie.len().saturating_sub(4));

            let dist = {
                let dx = x as f32 - light_pos[0];
                let dy = y as f32 - light_pos[1];
                (dx * dx + dy * dy).sqrt()
            };

            let shadow = cookie[cookie_idx] as f32 / 255.0;
            let attenuation = (1.0 / (1.0 + dist * falloff)).clamp(0.0, 1.0);
            let total = shadow * attenuation;

            for c in 0..3 {
                pixels[idx + c] = (pixels[idx + c] as f32 * total).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v25_filters() {
        let mut pixels = vec![100u8; 8 * 8 * 4];
        let depth = vec![128u8; 8 * 8 * 4];
        apply_z_depth_fog(&mut pixels, &depth, [20, 30, 50], 0.3, 0.9);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
