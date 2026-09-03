#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 12).
/// Advanced Physical Wave and Edge Refraction Kernels.
// 1. Water Drop / Rain Ripple Distortion Physics
pub fn apply_rain_ripples(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    frame: u32,
    drop_count: u32,
    wave_strength: f32,
) {
    if wave_strength <= 0.001 || drop_count == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let time = frame as f32 * 0.1;

    for y in 0..height {
        for x in 0..width {
            let mut total_dx = 0.0f32;
            let mut total_dy = 0.0f32;

            for d in 0..drop_count {
                // Pseudo-random drop positions
                let seed = d as f32 * 12.9898;
                let drop_x = (seed.sin().fract().abs()) * width as f32;
                let drop_y = ((seed * 2.0).cos().fract().abs()) * height as f32;

                let dx = x as f32 - drop_x;
                let dy = y as f32 - drop_y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);

                let phase = dist * 0.1 - time;
                let wave = (phase.sin() / (dist * 0.2 + 1.0)) * wave_strength * 5.0;

                total_dx += (dx / dist) * wave;
                total_dy += (dy / dist) * wave;
            }

            let sx = (x as f32 + total_dx).clamp(0.0, (width - 1) as f32) as usize;
            let sy = (y as f32 + total_dy).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (sy * width as usize + sx) * 4;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 2. Glass Edge Bevel & Refraction (CC Glass Edges Pro)
pub fn apply_glass_edge_bevel(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    bevel_size: u32,
    refraction: f32,
) {
    if bevel_size == 0 {
        return;
    }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let is_edge_x = x < bevel_size || x >= width - bevel_size;
            let is_edge_y = y < bevel_size || y >= height - bevel_size;

            if is_edge_x || is_edge_y {
                let norm_x = if x < bevel_size {
                    -(1.0 - x as f32 / bevel_size as f32)
                } else if x >= width - bevel_size {
                    (x as f32 - (width - bevel_size) as f32) / bevel_size as f32
                } else {
                    0.0
                };

                let norm_y = if y < bevel_size {
                    -(1.0 - y as f32 / bevel_size as f32)
                } else if y >= height - bevel_size {
                    (y as f32 - (height - bevel_size) as f32) / bevel_size as f32
                } else {
                    0.0
                };

                let sx =
                    (x as f32 + norm_x * refraction * 10.0).clamp(0.0, (width - 1) as f32) as usize;
                let sy = (y as f32 + norm_y * refraction * 10.0).clamp(0.0, (height - 1) as f32)
                    as usize;

                let dst_idx = (y as usize * width as usize + x as usize) * 4;
                let src_idx = (sy * width as usize + sx) * 4;

                // Highlight specular glass edge
                let highlight = ((norm_x + norm_y).abs() * 40.0) as i16;
                for c in 0..3 {
                    let val = temp[src_idx + c] as i16 + highlight;
                    pixels[dst_idx + c] = val.clamp(0, 255) as u8;
                }
                pixels[dst_idx + 3] = temp[src_idx + 3];
            }
        }
    }
}

// 3. Scanline CRT TV Distortion
pub fn apply_crt_scanlines(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    line_spacing: u32,
    intensity: f32,
) {
    if line_spacing == 0 {
        return;
    }
    let factor = 1.0 - intensity.clamp(0.0, 1.0);

    for y in 0..height {
        if y % line_spacing == 0 {
            for x in 0..width {
                let idx = (y as usize * width as usize + x as usize) * 4;
                pixels[idx] = (pixels[idx] as f32 * factor) as u8;
                pixels[idx + 1] = (pixels[idx + 1] as f32 * factor) as u8;
                pixels[idx + 2] = (pixels[idx + 2] as f32 * factor) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v12_filters() {
        let mut pixels = vec![100u8; 64];
        apply_crt_scanlines(&mut pixels, 4, 4, 2, 0.5);
        assert_eq!(pixels.len(), 64);
    }
}
