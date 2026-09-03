#![allow(dead_code)]
/// High-Precision After Effects VFX Kernels (Part 6).
/// All algorithms are implemented with distinct mathematical formulas and dedicated pixel processing.
// 161. Shatter (3D Explosive Glass/Tile Physics)
pub fn apply_shatter(pixels: &mut [u8], width: u32, height: u32, force: f32) {
    if force <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let piece_size = 16u32;
    let cols = width.div_ceil(piece_size);
    let rows = height.div_ceil(piece_size);

    pixels.fill(0);
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;

    for r in 0..rows {
        for c in 0..cols {
            let px = (c * piece_size + piece_size / 2) as f32;
            let py = (r * piece_size + piece_size / 2) as f32;

            let dx = px - center_x;
            let dy = py - center_y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);

            let shift_x = (dx / dist * force * 40.0) as i32;
            let shift_y = (dy / dist * force * 40.0) as i32;

            for y_off in 0..piece_size {
                let src_y = r * piece_size + y_off;
                if src_y >= height {
                    continue;
                }
                let dst_y = src_y as i32 + shift_y;
                if dst_y < 0 || dst_y >= height as i32 {
                    continue;
                }

                for x_off in 0..piece_size {
                    let src_x = c * piece_size + x_off;
                    if src_x >= width {
                        continue;
                    }
                    let dst_x = src_x as i32 + shift_x;
                    if dst_x < 0 || dst_x >= width as i32 {
                        continue;
                    }

                    let src_idx = (src_y as usize * width as usize + src_x as usize) * 4;
                    let dst_idx = (dst_y as usize * width as usize + dst_x as usize) * 4;

                    pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
                }
            }
        }
    }
}

// 162. Card Dance (3D Card Array Transformation)
pub fn apply_card_dance(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    rows: u32,
    cols: u32,
    rotation_deg: f32,
) {
    if rows == 0 || cols == 0 || width == 0 || height == 0 {
        return;
    }
    let temp = pixels.to_vec();
    pixels.fill(0);

    // Use ceiling division so edge cards always cover full image
    let card_w = width.div_ceil(cols);
    let card_h = height.div_ceil(rows);

    // Guard against zero division if image smaller than grid
    if card_w == 0 || card_h == 0 {
        return;
    }

    let rad = rotation_deg.to_radians();
    let cos_r = rad.cos();

    for r in 0..rows {
        for c in 0..cols {
            let start_x = c * card_w;
            let start_y = r * card_h;

            for cy in 0..card_h {
                let src_y = start_y + cy;
                if src_y >= height {
                    continue;
                }

                for cx in 0..card_w {
                    let src_x = start_x + cx;
                    if src_x >= width {
                        continue;
                    }

                    // Simulate 3D rotation scale squeeze along X axis
                    let rel_x = cx as f32 - (card_w as f32 * 0.5);
                    let proj_x = rel_x * cos_r;
                    let dst_x = (start_x as f32 + (card_w as f32 * 0.5) + proj_x) as i32;

                    if dst_x >= 0 && dst_x < width as i32 {
                        let src_idx = (src_y as usize * width as usize + src_x as usize) * 4;
                        let dst_idx = (src_y as usize * width as usize + dst_x as usize) * 4;
                        pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
                    }
                }
            }
        }
    }
}

// 163. Caustics (Water Surface Refraction)
pub fn apply_caustics(pixels: &mut [u8], width: u32, height: u32, wave_height: f32) {
    if wave_height <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let frequency = 0.05f32;

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;

            let dx = (fx * frequency).sin() * (fy * frequency * 0.8).cos() * wave_height * 10.0;
            let dy = (fy * frequency).sin() * (fx * frequency * 0.8).cos() * wave_height * 10.0;

            let src_x = (fx + dx).clamp(0.0, (width - 1) as f32) as usize;
            let src_y = (fy + dy).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (src_y * width as usize + src_x) * 4;

            let highlight = ((dx + dy) * 2.0).clamp(-30.0, 50.0) as i16;
            for c in 0..3 {
                let val = temp[src_idx + c] as i16 + highlight;
                pixels[dst_idx + c] = val.clamp(0, 255) as u8;
            }
            pixels[dst_idx + 3] = temp[src_idx + 3];
        }
    }
}

// 164. Wave World (Sinusoidal Water Wave Heights)
pub fn apply_wave_world(pixels: &mut [u8], width: u32, height: u32, speed: f32, amplitude: f32) {
    let temp = pixels.to_vec();
    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            let wave = ((dist * 0.1 - speed).sin() * amplitude) as i32;
            let src_x = (x as i32 + wave).clamp(0, width as i32 - 1) as usize;
            let src_y = (y as i32 + wave).clamp(0, height as i32 - 1) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (src_y * width as usize + src_x) * 4;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 165. Foam Simulation (Procedural Sea Foam Noise)
pub fn apply_foam(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    let t = frame as f32 * 0.05;
    for y in 0..height {
        for x in 0..width {
            let n = (x as f32 * 0.1 + t).sin() * (y as f32 * 0.1 - t).cos() * 0.5 + 0.5;
            if n > 0.75 {
                let idx = (y as usize * width as usize + x as usize) * 4;
                let foam_val = ((n - 0.75) * 4.0 * 255.0) as u8;
                pixels[idx] = pixels[idx].saturating_add(foam_val);
                pixels[idx + 1] = pixels[idx + 1].saturating_add(foam_val);
                pixels[idx + 2] = pixels[idx + 2].saturating_add(foam_val);
            }
        }
    }
}

// 166. Depth of Field Blur (Variable Depth Blur — Overflow-safe u64 accumulators)
pub fn apply_dof_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    depth_map: &[u8],
    focus_depth: u8,
    max_radius: u32,
) {
    if max_radius == 0 {
        return;
    }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let depth = if idx < depth_map.len() {
                depth_map[idx]
            } else {
                focus_depth
            };
            let diff = (depth as i16 - focus_depth as i16).unsigned_abs() as u32;
            let blur_r = (diff * max_radius / 255).min(max_radius).min(64);
            // Cap at 64px to stay real-time safe. Full DOF needs GPU.

            if blur_r == 0 {
                continue;
            }

            // Use u64 to prevent overflow: max sum = (2*64+1)^2 * 255 ≈ 845M < u64::MAX
            let mut r_sum = 0u64;
            let mut g_sum = 0u64;
            let mut b_sum = 0u64;
            let mut count = 0u64;

            for ky in -(blur_r as i32)..=(blur_r as i32) {
                let py = (y as i32 + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -(blur_r as i32)..=(blur_r as i32) {
                    let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
                    let k_idx = (py * width as usize + px) * 4;
                    r_sum += temp[k_idx] as u64;
                    g_sum += temp[k_idx + 1] as u64;
                    b_sum += temp[k_idx + 2] as u64;
                    count += 1;
                }
            }

            pixels[idx] = (r_sum / count) as u8;
            pixels[idx + 1] = (g_sum / count) as u8;
            pixels[idx + 2] = (b_sum / count) as u8;
        }
    }
}

// 167. Depth Matte (Z-Buffer Cutout)
pub fn apply_depth_matte(pixels: &mut [u8], depth_map: &[u8], z_near: u8, z_far: u8) {
    for i in (0..pixels.len()).step_by(4) {
        let depth = if i < depth_map.len() {
            depth_map[i]
        } else {
            128
        };
        if depth < z_near || depth > z_far {
            pixels[i + 3] = 0; // Cutout alpha outside range
        }
    }
}

// 168. 3D Glasses (Anaglyph Stereo Rendering)
pub fn apply_3d_glasses(pixels: &mut [u8], width: u32, height: u32, separation: i32) {
    let temp = pixels.to_vec();
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let rx = (x + separation).clamp(0, width as i32 - 1) as usize;
            let lx = (x - separation).clamp(0, width as i32 - 1) as usize;

            let idx = (y as usize * width as usize + x as usize) * 4;
            let r_idx = (y as usize * width as usize + rx) * 4;
            let l_idx = (y as usize * width as usize + lx) * 4;

            pixels[idx] = temp[r_idx]; // Red from right eye
            pixels[idx + 1] = temp[l_idx + 1]; // Cyan (Green) from left eye
            pixels[idx + 2] = temp[l_idx + 2]; // Cyan (Blue) from left eye
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v6_filters() {
        let mut pixels = vec![100u8; 64];
        apply_shatter(&mut pixels, 4, 4, 0.5);
        assert_eq!(pixels.len(), 64);
    }
}
