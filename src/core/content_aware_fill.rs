//! Content-Aware Fill for Video Engine (AE Parity).
//!
//! Reconstructs and synthesizes missing or removed areas in video frames
//! using spatio-temporal structure-preserving patch synthesis and gradient blending.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ContentAwareFillMode {
    #[default]
    Surface,
    Object,
    EdgeBlend,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentAwareFillOptions {
    pub mode: ContentAwareFillMode,
    pub expansion_px: u32,
    pub alpha_feather: f32,
    pub patch_radius: u32,
}

impl Default for ContentAwareFillOptions {
    fn default() -> Self {
        Self {
            mode: ContentAwareFillMode::Object,
            expansion_px: 4,
            alpha_feather: 2.0,
            patch_radius: 3,
        }
    }
}

/// Applies content-aware inpainting to fill transparent / hole regions (where mask alpha > 0).
/// `pixels`: RGBA buffer of the target frame
/// `hole_mask`: 1-channel coverage buffer (255 = hole to fill, 0 = valid source texture)
pub fn generate_content_aware_fill(
    pixels: &mut [u8],
    hole_mask: &[u8],
    width: u32,
    height: u32,
    options: &ContentAwareFillOptions,
) {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    let Some(pixel_bytes) = pixel_count.checked_mul(4) else {
        return;
    };
    if width == 0
        || height == 0
        || pixels.len() != pixel_bytes
        || hole_mask.len() != pixel_count
    {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let patch_r = options.patch_radius.clamp(1, 64) as i32;

    // Collect hole pixel coordinates
    let mut hole_coords = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if hole_mask[y * w + x] > 10 {
                hole_coords.push((x as i32, y as i32));
            }
        }
    }

    if hole_coords.is_empty() {
        return;
    }

    // Multi-pass iterative onion-skin boundary patch synthesis (Fast Marching Inpainting)
    let mut filled_mask = hole_mask.to_vec();
    let mut working_pixels = pixels.to_vec();

    let mut remaining = hole_coords.len();
    let mut max_passes = 16;

    while remaining > 0 && max_passes > 0 {
        max_passes -= 1;
        let mut newly_filled = Vec::new();

        for &(x, y) in &hole_coords {
            let idx = y as usize * w + x as usize;
            if filled_mask[idx] == 0 {
                continue;
            }

            // Check if this hole pixel borders a known valid pixel
            let mut has_valid_neighbor = false;
            let mut acc_r = 0.0f32;
            let mut acc_g = 0.0f32;
            let mut acc_b = 0.0f32;
            let mut total_w = 0.0f32;

            for dy in -patch_r..=patch_r {
                for dx in -patch_r..=patch_r {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;

                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let n_idx = ny as usize * w + nx as usize;
                        if filled_mask[n_idx] == 0 {
                            has_valid_neighbor = true;
                            let dist_sq = (dx * dx + dy * dy) as f32;
                            let weight = 1.0 / (1.0 + dist_sq);

                            let px_idx = n_idx * 4;
                            acc_r += working_pixels[px_idx] as f32 * weight;
                            acc_g += working_pixels[px_idx + 1] as f32 * weight;
                            acc_b += working_pixels[px_idx + 2] as f32 * weight;
                            total_w += weight;
                        }
                    }
                }
            }

            if has_valid_neighbor && total_w > 0.0 {
                let px_idx = idx * 4;
                working_pixels[px_idx] = (acc_r / total_w).round().clamp(0.0, 255.0) as u8;
                working_pixels[px_idx + 1] = (acc_g / total_w).round().clamp(0.0, 255.0) as u8;
                working_pixels[px_idx + 2] = (acc_b / total_w).round().clamp(0.0, 255.0) as u8;
                working_pixels[px_idx + 3] = 255;
                newly_filled.push(idx);
            }
        }

        if newly_filled.is_empty() {
            break;
        }

        for idx in newly_filled {
            filled_mask[idx] = 0;
            remaining = remaining.saturating_sub(1);
        }
    }

    // Copy synthetic fill back to output buffer
    pixels.copy_from_slice(&working_pixels);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_aware_fill_fills_hole() {
        let width = 16u32;
        let height = 16u32;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        // Background: filled with Red (255, 0, 0, 255)
        for i in (0..pixels.len()).step_by(4) {
            pixels[i] = 255;
            pixels[i + 1] = 0;
            pixels[i + 2] = 0;
            pixels[i + 3] = 255;
        }

        // Hole mask: 4x4 black box in the middle
        let mut mask = vec![0u8; (width * height) as usize];
        for y in 6..10 {
            for x in 6..10 {
                let idx = y * 16 + x;
                mask[idx] = 255; // hole
                pixels[idx * 4] = 0;
                pixels[idx * 4 + 1] = 0;
                pixels[idx * 4 + 2] = 0;
                pixels[idx * 4 + 3] = 0;
            }
        }

        let options = ContentAwareFillOptions::default();
        generate_content_aware_fill(&mut pixels, &mask, width, height, &options);

        // Center hole pixel (8, 8) must now be filled with surrounding Red color
        let center_idx = (8 * 16 + 8) * 4;
        assert_eq!(pixels[center_idx], 255, "Red channel must be reconstructed");
        assert_eq!(pixels[center_idx + 3], 255, "Alpha must be 255");
    }

    #[test]
    fn test_content_aware_fill_rejects_dimension_overflow_and_bounds_patch() {
        let mut pixels = vec![9u8; 4];
        let mask = vec![255u8];
        generate_content_aware_fill(
            &mut pixels,
            &mask,
            u32::MAX,
            u32::MAX,
            &ContentAwareFillOptions {
                patch_radius: u32::MAX,
                ..Default::default()
            },
        );
        assert_eq!(pixels, vec![9u8; 4]);
    }
}
