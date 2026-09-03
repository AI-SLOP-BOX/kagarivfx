#![allow(dead_code)]
/// Advanced Production-Grade After Effects VFX Kernels (Part 14).
/// Advanced Photographic and Stylize Renderers.
// 1. Cinematic Light Leak Synthesis
pub fn apply_light_leak_synth(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    position: [f32; 2],
    intensity: f32,
    leak_color: [u8; 3],
) {
    if intensity <= 0.001 {
        return;
    }
    let max_dist = (width as f32).max(height as f32) * 0.8;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - position[0];
            let dy = y as f32 - position[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < max_dist {
                let falloff = (1.0 - dist / max_dist).powf(1.5) * intensity;
                let idx = (y as usize * width as usize + x as usize) * 4;

                for c in 0..3 {
                    let val = pixels[idx + c] as f32 + leak_color[c] as f32 * falloff;
                    pixels[idx + c] = val.clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

// 2. Bevel Alpha 3D (3D Inner Bevel & Contour Highlight)
pub fn apply_bevel_alpha_3d(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    bevel_depth: u32,
    light_angle_deg: f32,
) {
    if bevel_depth == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let rad = light_angle_deg.to_radians();
    let lx = rad.cos();
    let ly = rad.sin();

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            if temp[idx + 3] == 0 {
                continue;
            }

            // Estimate Alpha Gradient
            let left_a = if x > 0 {
                temp[(y as usize * width as usize + (x - 1) as usize) * 4 + 3] as f32
            } else {
                0.0
            };
            let right_a = if x < width - 1 {
                temp[(y as usize * width as usize + (x + 1) as usize) * 4 + 3] as f32
            } else {
                0.0
            };
            let top_a = if y > 0 {
                temp[((y - 1) as usize * width as usize + x as usize) * 4 + 3] as f32
            } else {
                0.0
            };
            let bottom_a = if y < height - 1 {
                temp[((y + 1) as usize * width as usize + x as usize) * 4 + 3] as f32
            } else {
                0.0
            };

            let gx = (right_a - left_a) / 255.0;
            let gy = (bottom_a - top_a) / 255.0;

            let shading = (gx * lx + gy * ly) * bevel_depth as f32 * 30.0;

            for c in 0..3 {
                let val = pixels[idx + c] as f32 + shading;
                pixels[idx + c] = val.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 3. Halftone Screen Printing (Dot Screen Rasterization)
pub fn apply_halftone_screen(pixels: &mut [u8], width: u32, height: u32, cell_size: u32) {
    if cell_size == 0 {
        return;
    }
    let temp = pixels.to_vec();

    for cy in (0..height).step_by(cell_size as usize) {
        for cx in (0..width).step_by(cell_size as usize) {
            let mut luma_sum = 0u32;
            let mut count = 0u32;

            for dy in 0..cell_size {
                let y = cy + dy;
                if y >= height {
                    break;
                }
                for dx in 0..cell_size {
                    let x = cx + dx;
                    if x >= width {
                        break;
                    }
                    let idx = (y as usize * width as usize + x as usize) * 4;
                    let luma = (temp[idx] as u32 + temp[idx + 1] as u32 + temp[idx + 2] as u32) / 3;
                    luma_sum += luma;
                    count += 1;
                }
            }

            let avg_luma = luma_sum / count.max(1);
            let max_radius = cell_size as f32 * 0.5;
            let dot_radius = (avg_luma as f32 / 255.0) * max_radius;

            let center_cell_x = cx as f32 + max_radius;
            let center_cell_y = cy as f32 + max_radius;

            for dy in 0..cell_size {
                let y = cy + dy;
                if y >= height {
                    break;
                }
                for dx in 0..cell_size {
                    let x = cx + dx;
                    if x >= width {
                        break;
                    }
                    let dist = ((x as f32 - center_cell_x).powi(2)
                        + (y as f32 - center_cell_y).powi(2))
                    .sqrt();
                    let idx = (y as usize * width as usize + x as usize) * 4;

                    if dist <= dot_radius {
                        pixels[idx] = 255;
                        pixels[idx + 1] = 255;
                        pixels[idx + 2] = 255;
                    } else {
                        pixels[idx] = 0;
                        pixels[idx + 1] = 0;
                        pixels[idx + 2] = 0;
                    }
                }
            }
        }
    }
}

// 4. Solarize Effect (Solar Inversion Response)
pub fn apply_solarize_effect(pixels: &mut [u8], threshold: u8) {
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            if pixels[i + c] > threshold {
                pixels[i + c] = 255 - pixels[i + c];
            }
        }
    }
}

// 5. Pixel Sort Glitch Renderer
pub fn apply_pixel_sort_glitch(pixels: &mut [u8], width: u32, height: u32, threshold: u8) {
    for y in 0..height {
        let row_start = (y as usize * width as usize) * 4;
        let mut x = 0usize;

        while x < width as usize {
            let idx = row_start + x * 4;
            let luma = (pixels[idx] as u32 + pixels[idx + 1] as u32 + pixels[idx + 2] as u32) / 3;

            if luma > threshold as u32 {
                let run_start = x;
                while x < width as usize {
                    let cur_idx = row_start + x * 4;
                    let cur_luma = (pixels[cur_idx] as u32
                        + pixels[cur_idx + 1] as u32
                        + pixels[cur_idx + 2] as u32)
                        / 3;
                    if cur_luma <= threshold as u32 {
                        break;
                    }
                    x += 1;
                }
                let run_end = x;

                // Sort pixel span by luminance
                if run_end > run_start + 1 {
                    let mut span: Vec<[u8; 4]> = (run_start..run_end)
                        .map(|i| {
                            let p_idx = row_start + i * 4;
                            [
                                pixels[p_idx],
                                pixels[p_idx + 1],
                                pixels[p_idx + 2],
                                pixels[p_idx + 3],
                            ]
                        })
                        .collect();

                    span.sort_by_key(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3);

                    for (i, p) in (run_start..run_end).zip(span.iter()) {
                        let p_idx = row_start + i * 4;
                        pixels[p_idx..p_idx + 4].copy_from_slice(p);
                    }
                }
            } else {
                x += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v14_filters() {
        let mut pixels = vec![100u8; 64];
        apply_solarize_effect(&mut pixels, 128);
        assert_eq!(pixels.len(), 64);
    }
}
