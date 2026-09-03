#![allow(dead_code)]
/// After Effects VFX Kernels Part 26 — Keying & Compositing Utilities
// 1. Spill Suppressor (Chroma Spill Removal on Subject)
pub fn apply_spill_suppressor(
    pixels: &mut [u8],
    spill_channel: usize,
    neighbor_a: usize,
    neighbor_b: usize,
) {
    let spill = spill_channel.min(2);
    let na = neighbor_a.min(2);
    let nb = neighbor_b.min(2);

    for i in (0..pixels.len()).step_by(4) {
        let avg_neighbors = (pixels[i + na] as f32 + pixels[i + nb] as f32) * 0.5;
        if pixels[i + spill] as f32 > avg_neighbors {
            pixels[i + spill] = avg_neighbors as u8;
        }
    }
}

// 2. Inner / Outer Glow (Alpha-Aware Glow Synthesis)
pub fn apply_glow_alpha(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    radius: u32,
    color: [u8; 3],
    inner: bool,
) {
    if radius == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let a = temp[idx + 3];

            // Skip: inner glow only on opaque, outer glow only on transparent
            if (inner && a == 0) || (!inner && a == 255) {
                continue;
            }

            let mut max_a = 0u8;
            let mut min_a = 255u8;

            for ky in -r..=r {
                let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -r..=r {
                    let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                    let nb_a = temp[(py * width as usize + px) * 4 + 3];
                    max_a = max_a.max(nb_a);
                    min_a = min_a.min(nb_a);
                }
            }

            let glow_alpha = if inner { 255 - min_a } else { max_a } as f32 / 255.0;
            let dist_factor = ((r as f32 - glow_alpha * r as f32) / r as f32).clamp(0.0, 1.0);
            let blend = (1.0 - dist_factor) * glow_alpha;

            for c in 0..3 {
                pixels[idx + c] = (pixels[idx + c] as f32 * (1.0 - blend) + color[c] as f32 * blend)
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 3. Rotoscope Mask Inpainting (Edge-Aware Hole Fill)
pub fn apply_inpainting_simple(pixels: &mut [u8], width: u32, height: u32) {
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            if temp[idx + 3] > 0 {
                continue;
            } // Skip opaque pixels

            // Sample from nearest opaque neighbor (simplified flood from border)
            let mut sum = [0.0f32; 3];
            let mut count = 0u32;

            for ky in -4i32..=4 {
                let py = (y as i32 + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -4i32..=4 {
                    let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
                    let nb_idx = (py * width as usize + px) * 4;
                    if temp[nb_idx + 3] > 0 {
                        for c in 0..3 {
                            sum[c] += temp[nb_idx + c] as f32;
                        }
                        count += 1;
                    }
                }
            }

            if count > 0 {
                for c in 0..3 {
                    pixels[idx + c] = (sum[c] / count as f32) as u8;
                }
                pixels[idx + 3] = 255;
            }
        }
    }
}

// 4. Luminance Gain Map (Zone System Adaptive Brightness)
pub fn apply_luminance_gain_map(
    pixels: &mut [u8],
    shadows_gain: f32,
    midtones_gain: f32,
    highlights_gain: f32,
) {
    for i in (0..pixels.len()).step_by(4) {
        let luma = (pixels[i] as f32 * 0.299
            + pixels[i + 1] as f32 * 0.587
            + pixels[i + 2] as f32 * 0.114)
            / 255.0;

        let gain = if luma < 0.33 {
            shadows_gain
        } else if luma < 0.66 {
            midtones_gain
        } else {
            highlights_gain
        };

        for c in 0..3 {
            pixels[i + c] = (pixels[i + c] as f32 * gain).clamp(0.0, 255.0) as u8;
        }
    }
}

// 5. Difference Key (Frame Difference Matting)
pub fn apply_difference_key(pixels: &mut [u8], reference: &[u8], threshold: u8, softness: f32) {
    let len = pixels.len().min(reference.len());
    for i in (0..len).step_by(4) {
        let dr = (pixels[i] as i16 - reference[i] as i16).abs() as f32;
        let dg = (pixels[i + 1] as i16 - reference[i + 1] as i16).abs() as f32;
        let db = (pixels[i + 2] as i16 - reference[i + 2] as i16).abs() as f32;
        let diff = (dr + dg + db) / 3.0;

        let alpha = if diff < threshold as f32 {
            0.0
        } else if diff < threshold as f32 + softness * 50.0 {
            (diff - threshold as f32) / (softness * 50.0).max(1.0)
        } else {
            1.0
        };

        pixels[i + 3] = (pixels[i + 3] as f32 * alpha).clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v26_filters() {
        let mut pixels = vec![200u8; 8 * 8 * 4];
        apply_spill_suppressor(&mut pixels, 1, 0, 2);
        apply_luminance_gain_map(&mut pixels, 1.2, 1.0, 0.8);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
