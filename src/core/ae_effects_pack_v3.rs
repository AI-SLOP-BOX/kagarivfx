#![allow(dead_code)]
/// Pack of 20 Additional Advanced Adobe After Effects Effects & Simulation Kernels (Part 3 - Total 60 Effects).
// 41. Time Difference
pub fn apply_time_difference(pixels: &mut [u8], prev_pixels: &[u8]) {
    if pixels.len() != prev_pixels.len() { return; }
    for i in (0..pixels.len()).step_by(4) {
        let dr = (pixels[i] as i16 - prev_pixels[i] as i16).unsigned_abs() as u8;
        let dg = (pixels[i + 1] as i16 - prev_pixels[i + 1] as i16).unsigned_abs() as u8;
        let db = (pixels[i + 2] as i16 - prev_pixels[i + 2] as i16).unsigned_abs() as u8;
        pixels[i] = dr; pixels[i + 1] = dg; pixels[i + 2] = db;
    }
}

// 42. Freeze Frame
pub fn apply_freeze_frame(pixels: &mut [u8], frozen_frame_pixels: &[u8]) {
    if pixels.len() == frozen_frame_pixels.len() {
        pixels.copy_from_slice(frozen_frame_pixels);
    }
}

// 43. Time Reverse
pub fn apply_time_reverse(pixels: &mut [u8], reversed_frame_pixels: &[u8]) {
    apply_freeze_frame(pixels, reversed_frame_pixels);
}

// 44. Timewarp
pub fn apply_timewarp(pixels: &mut [u8], frame_a: &[u8], frame_b: &[u8], factor: f32) {
    if pixels.len() != frame_a.len() || pixels.len() != frame_b.len() { return; }
    let k = factor.clamp(0.0, 1.0);
    for i in 0..pixels.len() {
        let val = frame_a[i] as f32 * (1.0 - k) + frame_b[i] as f32 * k;
        pixels[i] = val.round() as u8;
    }
}

// 45. Strobe Light
pub fn apply_strobe_light(pixels: &mut [u8], frame: u32, interval_frames: u32, strobe_color: [u8; 4]) {
    if interval_frames > 0 && (frame / interval_frames) % 2 == 1 {
        for i in (0..pixels.len()).step_by(4) {
            pixels[i..i + 4].copy_from_slice(&strobe_color);
        }
    }
}

// 46. CC Particle World
pub fn apply_cc_particle_world(pixels: &mut [u8], width: u32, height: u32, frame: u32, particle_color: [u8; 4]) {
    let num_particles = 30;
    let seed = frame as f32;
    for p in 0..num_particles {
        let angle = p as f32 * 0.2 + seed * 0.1;
        let dist = (p as f32 * 3.0 + seed * 2.0) % (width as f32 * 0.4);
        let px = (width as f32 * 0.5 + angle.cos() * dist).clamp(0.0, width as f32 - 1.0) as u32;
        let py = (height as f32 * 0.5 + angle.sin() * dist).clamp(0.0, height as f32 - 1.0) as u32;

        let idx = ((py * width + px) * 4) as usize;
        pixels[idx..idx + 4].copy_from_slice(&particle_color);
    }
}

// 47. CC Ball Action
pub fn apply_cc_ball_action(pixels: &mut [u8], width: u32, height: u32, grid_spacing: u32, ball_size: f32) {
    let temp = pixels.to_vec();
    pixels.fill(0);

    let step = grid_spacing.max(2) as usize;
    let r_max = (grid_spacing as f32 * 0.5 * ball_size).max(1.0);

    for y in (0..height as usize).step_by(step) {
        for x in (0..width as usize).step_by(step) {
            let s_idx = (y * width as usize + x) * 4;
            let color = [temp[s_idx], temp[s_idx + 1], temp[s_idx + 2], temp[s_idx + 3]];

            for dy in 0..step {
                for dx in 0..step {
                    let px = x + dx;
                    let py = y + dy;
                    if px < width as usize && py < height as usize {
                        let drx = dx as f32 - step as f32 * 0.5;
                        let dry = dy as f32 - step as f32 * 0.5;
                        if (drx * drx + dry * dry).sqrt() <= r_max {
                            let idx = (py * width as usize + px) * 4;
                            pixels[idx..idx + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
    }
}

// 48. CC Cylinder
pub fn apply_cc_cylinder(pixels: &mut [u8], width: u32, height: u32, radius: f32) {
    let cx = width as f32 * 0.5;
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let rx = x as f32 - cx;
            if rx.abs() < radius {
                let theta = (rx / radius).asin();
                let sx = (cx + theta * radius).clamp(0.0, width as f32 - 1.0) as u32;

                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((y * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 49. CC Sphere
pub fn apply_cc_sphere(pixels: &mut [u8], width: u32, height: u32, radius: f32) {
    crate::core::ae_effects_pack::apply_bulge(pixels, width, height, 0.8, radius);
}

// 50. CC Page Turn
pub fn apply_cc_page_turn(pixels: &mut [u8], width: u32, height: u32, fold_progress: f32) {
    if fold_progress <= 0.0 { return; }
    let fold_x = width as f32 * (1.0 - fold_progress * 0.01);
    for y in 0..height {
        for x in 0..width {
            if x as f32 > fold_x {
                let idx = ((y * width + x) * 4 + 3) as usize;
                pixels[idx] = 0;
            }
        }
    }
}

// 51. CC Repeat Tile / CC Repetile
pub fn apply_cc_repetile(pixels: &mut [u8], width: u32, height: u32, expand_percent: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, width, height, 100.0 + expand_percent);
}

// 52. CC Split
pub fn apply_cc_split(pixels: &mut [u8], width: u32, height: u32, split_amount: f32) {
    let cy = height / 2;
    let shift = (split_amount * 0.5) as u32;
    let temp = pixels.to_vec();

    for y in 0..height {
        let sy = if y < cy { y.saturating_sub(shift) } else { (y + shift).min(height - 1) };
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let s_idx = ((sy * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 53. CC Pixel Polly
pub fn apply_cc_pixel_polly(pixels: &mut [u8], width: u32, height: u32, shatter_progress: f32) {
    if shatter_progress <= 0.0 { return; }
    apply_cc_ball_action(pixels, width, height, 8, 1.0 - shatter_progress * 0.01);
}

// 54. CC Light Sweep
pub fn apply_cc_light_sweep(pixels: &mut [u8], width: u32, height: u32, progress: f32, sweep_width: u32) {
    let sweep_x = (width as f32 * (progress * 0.01)) as i32;
    let w = sweep_width as i32;

    for y in 0..height {
        for x in 0..width {
            if (x as i32 - sweep_x).abs() < w {
                let idx = ((y * width + x) * 4) as usize;
                for c in 0..3 {
                    pixels[idx + c] = (pixels[idx + c] as u16 + 100).min(255) as u8;
                }
            }
        }
    }
}

// 55. CC Light Burst 2.5
pub fn apply_cc_light_burst(pixels: &mut [u8], width: u32, height: u32, ray_length: f32) {
    crate::core::ae_effects_pack::apply_radial_blur(pixels, width, height, ray_length * 2.0);
}

// 56. Lens Flare
pub fn apply_lens_flare(pixels: &mut [u8], width: u32, height: u32, flare_center: [f32; 2], brightness: f32) {
    let radius = (width as f32 * 0.2 * brightness).max(1.0);
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - flare_center[0];
            let dy = y as f32 - flare_center[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < radius {
                let intensity = (1.0 - dist / radius).powi(2) * brightness;
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = (pixels[idx] as f32 + 255.0 * intensity).clamp(0.0, 255.0) as u8;
                pixels[idx + 1] = (pixels[idx + 1] as f32 + 200.0 * intensity).clamp(0.0, 255.0) as u8;
                pixels[idx + 2] = (pixels[idx + 2] as f32 + 150.0 * intensity).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 57. CC Light Rays
pub fn apply_cc_light_rays(pixels: &mut [u8], width: u32, height: u32, intensity: f32) {
    apply_cc_light_burst(pixels, width, height, intensity * 5.0);
}

// 58. Fractal Noise
pub fn apply_fractal_noise(pixels: &mut [u8], width: u32, height: u32, scale: f32) {
    let sc = scale.max(1.0);
    for y in 0..height {
        for x in 0..width {
            let noise = ((x as f32 / sc).sin() * (y as f32 / sc).cos() * 0.5 + 0.5) * 255.0;
            let val = noise as u8;
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = val; pixels[idx + 1] = val; pixels[idx + 2] = val; pixels[idx + 3] = 255;
        }
    }
}

// 59. Cell Pattern
pub fn apply_cell_pattern(pixels: &mut [u8], width: u32, height: u32, cell_size: u32) {
    let sz = cell_size.max(2);
    for y in 0..height {
        for x in 0..width {
            let cx = (x / sz) * sz + sz / 2;
            let cy = (y / sz) * sz + sz / 2;
            let dx = x as f32 - cx as f32;
            let dy = y as f32 - cy as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let val = (dist * 10.0).clamp(0.0, 255.0) as u8;

            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = val; pixels[idx + 1] = val; pixels[idx + 2] = val; pixels[idx + 3] = 255;
        }
    }
}

// 60. Turbulent Displace Filter
pub fn apply_turbulent_displace_filter(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, width, height, amount, 40.0, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v3_filters() {
        let mut pixels = vec![0u8; 64];
        apply_fractal_noise(&mut pixels, 4, 4, 10.0);
        assert_eq!(pixels.len(), 64);
        assert_eq!(pixels[3], 255);
    }
}
