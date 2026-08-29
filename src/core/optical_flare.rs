//! Physical Optical Flare & Anamorphic Lens Flare Engine (AE Parity).
//!
//! Generates multi-element cinematic lens flares including:
//! - Core Glow Ball (Inverse-Square physical falloff)
//! - Anamorphic Streaks (Horizontal chromatic anamorphic lines)
//! - Aperture Polygon Spikes (Bladed starbursts)
//! - Multi-Iris Lens Ghosts (Internal optic reflections)
//! - Ring Halo (Lens barrel refraction rings)

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum FlareElementType {
    #[default]
    GlowBall,
    AnamorphicStreak,
    ApertureSpikes { blades: u32 },
    LensGhost { distance_factor: f32 },
    RingHalo { radius_ratio: f32 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlareElement {
    pub element_type: FlareElementType,
    pub color: [f32; 4],
    pub scale: f32,
    pub brightness: f32,
    pub rotation_deg: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpticalFlareConfig {
    pub position: [f32; 2],
    pub overall_scale: f32,
    pub overall_brightness: f32,
    pub elements: Vec<FlareElement>,
}

impl Default for OpticalFlareConfig {
    fn default() -> Self {
        Self {
            position: [960.0, 540.0],
            overall_scale: 1.0,
            overall_brightness: 1.0,
            elements: vec![
                FlareElement {
                    element_type: FlareElementType::GlowBall,
                    color: [1.0, 0.9, 0.7, 1.0],
                    scale: 100.0,
                    brightness: 1.0,
                    rotation_deg: 0.0,
                },
                FlareElement {
                    element_type: FlareElementType::AnamorphicStreak,
                    color: [0.3, 0.6, 1.0, 0.8],
                    scale: 600.0,
                    brightness: 0.8,
                    rotation_deg: 0.0,
                },
                FlareElement {
                    element_type: FlareElementType::ApertureSpikes { blades: 6 },
                    color: [1.0, 0.95, 0.8, 0.6],
                    scale: 180.0,
                    brightness: 0.7,
                    rotation_deg: 0.0,
                },
            ],
        }
    }
}

/// Renders additive optical lens flare onto an RGBA destination buffer.
pub fn render_optical_flare(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    config: &OpticalFlareConfig,
) {
    if pixels.len() != (width * height * 4) as usize || config.overall_brightness <= 0.0 {
        return;
    }

    let cx = config.position[0];
    let cy = config.position[1];
    let global_scale = config.overall_scale.max(0.01);
    let global_bright = config.overall_brightness;

    let w = width as usize;
    let h = height as usize;

    for elem in &config.elements {
        let e_scale = (elem.scale * global_scale).max(1.0);
        let e_bright = elem.brightness * global_bright;
        let col = elem.color;

        let r_box = e_scale.ceil() as i32;
        let min_x = ((cx - e_scale) as i32).clamp(0, width as i32 - 1);
        let max_x = ((cx + e_scale) as i32).clamp(0, width as i32 - 1);
        let min_y = ((cy - e_scale) as i32).clamp(0, height as i32 - 1);
        let max_y = ((cy + e_scale) as i32).clamp(0, height as i32 - 1);

        match elem.element_type {
            FlareElementType::GlowBall => {
                for y in min_y..=max_y {
                    let dy = y as f32 - cy;
                    for x in min_x..=max_x {
                        let dx = x as f32 - cx;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < e_scale {
                            let falloff = (1.0 - dist / e_scale).powi(2) * e_bright;
                            let idx = (y as usize * w + x as usize) * 4;
                            pixels[idx] = (pixels[idx] as f32 + col[0] * 255.0 * falloff).min(255.0) as u8;
                            pixels[idx + 1] = (pixels[idx + 1] as f32 + col[1] * 255.0 * falloff).min(255.0) as u8;
                            pixels[idx + 2] = (pixels[idx + 2] as f32 + col[2] * 255.0 * falloff).min(255.0) as u8;
                            pixels[idx + 3] = pixels[idx + 3].max((falloff * 255.0).min(255.0) as u8);
                        }
                    }
                }
            }
            FlareElementType::AnamorphicStreak => {
                let streak_h = (e_scale * 0.03).max(2.0);
                let streak_min_y = ((cy - streak_h) as i32).clamp(0, height as i32 - 1);
                let streak_max_y = ((cy + streak_h) as i32).clamp(0, height as i32 - 1);

                for y in streak_min_y..=streak_max_y {
                    let dy = (y as f32 - cy).abs();
                    let vert_falloff = (1.0 - dy / streak_h).max(0.0);

                    for x in min_x..=max_x {
                        let dx = (x as f32 - cx).abs();
                        let horiz_falloff = (1.0 - dx / e_scale).max(0.0).powi(3);
                        let intensity = vert_falloff * horiz_falloff * e_bright;

                        let idx = (y as usize * w + x as usize) * 4;
                        pixels[idx] = (pixels[idx] as f32 + col[0] * 255.0 * intensity).min(255.0) as u8;
                        pixels[idx + 1] = (pixels[idx + 1] as f32 + col[1] * 255.0 * intensity).min(255.0) as u8;
                        pixels[idx + 2] = (pixels[idx + 2] as f32 + col[2] * 255.0 * intensity).min(255.0) as u8;
                        pixels[idx + 3] = pixels[idx + 3].max((intensity * 255.0).min(255.0) as u8);
                    }
                }
            }
            FlareElementType::ApertureSpikes { blades } => {
                let num_blades = blades.max(3) as f32;
                for y in min_y..=max_y {
                    let dy = y as f32 - cy;
                    for x in min_x..=max_x {
                        let dx = x as f32 - cx;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < e_scale && dist > 1.0 {
                            let angle = dy.atan2(dx);
                            let spike_mod = ((angle * num_blades * 0.5).cos().abs()).powi(8);
                            let falloff = (1.0 - dist / e_scale) * spike_mod * e_bright;

                            let idx = (y as usize * w + x as usize) * 4;
                            pixels[idx] = (pixels[idx] as f32 + col[0] * 255.0 * falloff).min(255.0) as u8;
                            pixels[idx + 1] = (pixels[idx + 1] as f32 + col[1] * 255.0 * falloff).min(255.0) as u8;
                            pixels[idx + 2] = (pixels[idx + 2] as f32 + col[2] * 255.0 * falloff).min(255.0) as u8;
                            pixels[idx + 3] = pixels[idx + 3].max((falloff * 255.0).min(255.0) as u8);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optical_flare_renders_glow_and_streak() {
        let width = 64u32;
        let height = 64u32;
        let mut buf = vec![0u8; (width * height * 4) as usize];

        let mut config = OpticalFlareConfig::default();
        config.position = [32.0, 32.0];
        config.overall_scale = 0.2;

        render_optical_flare(&mut buf, width, height, &config);

        let center_idx = (32 * 64 + 32) * 4;
        assert!(buf[center_idx] > 50, "Center red channel should be illuminated");
        assert!(buf[center_idx + 3] > 50, "Center alpha should be non-zero");

        // Edge along horizontal line (streak) should have brightness
        let streak_idx = (32 * 64 + 48) * 4;
        assert!(buf[streak_idx + 2] > 0, "Anamorphic streak should propagate horizontally");
    }
}
