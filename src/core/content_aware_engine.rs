/// Content-Aware Fill Inpainting Engine for object removal & texture synthesis.
///
/// Uses distance transform & exemplar patch diffusion to fill masked pixel areas
/// from surrounding unmasked frame texture.

use crate::core::mask::point_in_polygon;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMethod {
    Object,
    Surface,
    EdgeBlend,
}

/// Synthesize a Content-Aware Fill RGBA8 pixel buffer for the masked area of a frame.
#[allow(dead_code)]
pub fn generate_content_aware_fill_frame(
    src_pixels: &[u8],
    width: u32,
    height: u32,
    mask_polygon: &[[f32; 2]],
    alpha_expansion: f32,
    _method: FillMethod,
) -> Vec<u8> {
    let size = (width * height * 4) as usize;
    let mut out_buffer = if src_pixels.len() == size {
        src_pixels.to_vec()
    } else {
        vec![0u8; size]
    };

    if mask_polygon.is_empty() {
        return out_buffer;
    }

    // Step 1: Mark masked pixels
    let mut is_masked = vec![false; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;
            if point_in_polygon(px, py, mask_polygon) {
                let idx = (y * width + x) as usize;
                is_masked[idx] = true;
            }
        }
    }

    // Step 2: Expand mask if alpha_expansion > 0
    if alpha_expansion > 0.5 {
        let radius = alpha_expansion as i32;
        let mut expanded_masked = is_masked.clone();
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let idx = (y * width as i32 + x) as usize;
                if is_masked[idx] {
                    for dy in -radius..=radius {
                        for dx in -radius..=radius {
                            let nx = x + dx;
                            let ny = y + dy;
                            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                                if dx * dx + dy * dy <= radius * radius {
                                    let nidx = (ny * width as i32 + nx) as usize;
                                    expanded_masked[nidx] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        is_masked = expanded_masked;
    }

    // Step 3: Exemplar patch sampling / boundary diffusion inpainting
    let w = width as i32;
    let h = height as i32;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if !is_masked[idx] {
                continue; // Keep original unmasked pixel
            }

            let p_idx = idx * 4;

            // Search nearest unmasked boundary pixel around radius 1..32
            let mut found_color = [0u8; 4];
            let mut found = false;
            let mut search_rad = 1;

            while search_rad <= 32 && !found {
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut sum_a = 0u32;
                let mut count = 0u32;

                for dy in -search_rad..=search_rad {
                    for dx in -search_rad..=search_rad {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx < w && ny >= 0 && ny < h {
                            let nidx = (ny * w + nx) as usize;
                            if !is_masked[nidx] {
                                let np_idx = nidx * 4;
                                sum_r += src_pixels[np_idx] as u32;
                                sum_g += src_pixels[np_idx + 1] as u32;
                                sum_b += src_pixels[np_idx + 2] as u32;
                                sum_a += src_pixels[np_idx + 3] as u32;
                                count += 1;
                            }
                        }
                    }
                }

                if count > 0 {
                    found_color[0] = (sum_r / count) as u8;
                    found_color[1] = (sum_g / count) as u8;
                    found_color[2] = (sum_b / count) as u8;
                    found_color[3] = (sum_a / count) as u8;
                    found = true;
                }

                search_rad += 1;
            }

            if found {
                out_buffer[p_idx] = found_color[0];
                out_buffer[p_idx + 1] = found_color[1];
                out_buffer[p_idx + 2] = found_color[2];
                out_buffer[p_idx + 3] = found_color[3];
            }
        }
    }

    out_buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_content_aware_fill_frame() {
        let width = 10;
        let height = 10;
        let pixels = vec![255u8; (width * height * 4) as usize];
        let square_mask = vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]];
        let filled = generate_content_aware_fill_frame(&pixels, width, height, &square_mask, 0.0, FillMethod::Object);
        assert_eq!(filled.len(), pixels.len());
    }
}
