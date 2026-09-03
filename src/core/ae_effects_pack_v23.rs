#![allow(dead_code)]
/// After Effects VFX Kernels Part 23 — Retro Photo & Artistic Stylizers
// 1. Ink & Paint (Cartoon Outline + Flat Color Quantization)
pub fn apply_ink_paint(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    quantize_levels: u32,
    edge_thickness: u32,
) {
    if quantize_levels == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let step = 255.0 / (quantize_levels - 1) as f32;

    // Step 1: Quantize colors
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let v = pixels[i + c] as f32;
            pixels[i + c] = ((v / step).round() * step).clamp(0.0, 255.0) as u8;
        }
    }

    // Step 2: Sobel edge detection and paint black outlines
    if edge_thickness > 0 {
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let idx = (y as usize * width as usize + x as usize) * 4;
                let mut g_total = 0i32;

                for c in 0..3 {
                    let tl = temp[(y as usize - 1) * width as usize * 4 + (x as usize - 1) * 4 + c]
                        as i32;
                    let tr = temp[(y as usize - 1) * width as usize * 4 + (x as usize + 1) * 4 + c]
                        as i32;
                    let bl = temp[(y as usize + 1) * width as usize * 4 + (x as usize - 1) * 4 + c]
                        as i32;
                    let br = temp[(y as usize + 1) * width as usize * 4 + (x as usize + 1) * 4 + c]
                        as i32;
                    let l = temp[y as usize * width as usize * 4 + (x as usize - 1) * 4 + c] as i32;
                    let r = temp[y as usize * width as usize * 4 + (x as usize + 1) * 4 + c] as i32;
                    let t = temp[(y as usize - 1) * width as usize * 4 + x as usize * 4 + c] as i32;
                    let b = temp[(y as usize + 1) * width as usize * 4 + x as usize * 4 + c] as i32;

                    let gx = -tl - 2 * l - bl + tr + 2 * r + br;
                    let gy = -tl - 2 * t - tr + bl + 2 * b + br;
                    g_total += gx * gx + gy * gy;
                }

                if (g_total as f32).sqrt() > 100.0 * edge_thickness as f32 {
                    pixels[idx] = 0;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                }
            }
        }
    }
}

// 2. Oil Painting Effect (Kuwahara Filter)
pub fn apply_kuwahara(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let r = radius as i32;

    for y in r..(height as i32 - r) {
        for x in r..(width as i32 - r) {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let mut best_var = f32::MAX;
            let mut best_mean = [0u8; 3];

            // Evaluate 4 quadrant windows
            let quadrants: [(i32, i32); 4] = [(-r, -r), (0, -r), (-r, 0), (0, 0)];
            for (ox, oy) in quadrants {
                let mut sum = [0.0f32; 3];
                let mut sum_sq = [0.0f32; 3];
                let mut count = 0.0f32;

                for ky in oy..=(oy + r) {
                    let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                    for kx in ox..=(ox + r) {
                        let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                        let k_idx = (py * width as usize + px) * 4;
                        for c in 0..3 {
                            let v = temp[k_idx + c] as f32;
                            sum[c] += v;
                            sum_sq[c] += v * v;
                        }
                        count += 1.0;
                    }
                }

                let var: f32 = (0..3)
                    .map(|c| sum_sq[c] / count - (sum[c] / count).powi(2))
                    .sum();
                if var < best_var {
                    best_var = var;
                    best_mean = std::array::from_fn(|c| (sum[c] / count).clamp(0.0, 255.0) as u8);
                }
            }

            pixels[idx] = best_mean[0];
            pixels[idx + 1] = best_mean[1];
            pixels[idx + 2] = best_mean[2];
        }
    }
}

// 3. Watercolor Effect (Bilateral + Border Feather Vignette)
pub fn apply_watercolor_stylize(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    blur_strength: f32,
    edge_scale: f32,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = (cx * cx + cy * cy).sqrt().max(1.0);

    for y in 0..height {
        for x in 0..width {
            let i = (y as usize * width as usize + x as usize) * 4;

            let r = pixels[i] as f32;
            let g = pixels[i + 1] as f32;
            let b = pixels[i + 2] as f32;
            let luma = r * 0.299 + g * 0.587 + b * 0.114;

            // Bleed color toward white (simulate paper absorption)
            let bleed = blur_strength.clamp(0.0, 1.0);
            let base_r = (r * (1.0 - bleed) + 255.0 * bleed * 0.15) + luma * bleed * 0.85;
            let base_g = (g * (1.0 - bleed) + 255.0 * bleed * 0.15) + luma * bleed * 0.85;
            let base_b = (b * (1.0 - bleed) + 255.0 * bleed * 0.15) + luma * bleed * 0.85;

            // Edge-aware paper texture darkening using edge_scale
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let norm_r = (dx * dx + dy * dy).sqrt() / max_r;
            let edge_factor = 1.0 - (norm_r * edge_scale).clamp(0.0, 0.4);

            pixels[i] = (base_r * edge_factor).clamp(0.0, 255.0) as u8;
            pixels[i + 1] = (base_g * edge_factor).clamp(0.0, 255.0) as u8;
            pixels[i + 2] = (base_b * edge_factor).clamp(0.0, 255.0) as u8;
        }
    }
}

// 4. Cinemagraph Freeze (Selective Static Mask Region)
pub fn apply_cinemagraph_freeze(pixels: &mut [u8], frozen: &[u8], mask: &[u8]) {
    let len = pixels.len().min(frozen.len()).min(mask.len());
    for i in (0..len).step_by(4) {
        let m = mask[i] as f32 / 255.0;
        for c in 0..3 {
            pixels[i + c] = (pixels[i + c] as f32 * (1.0 - m) + frozen[i + c] as f32 * m)
                .clamp(0.0, 255.0) as u8;
        }
    }
}

// 5. Sepia Tone (Chemical Photo Aging)
pub fn apply_sepia_tone(pixels: &mut [u8], intensity: f32) {
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i] as f32;
        let g = pixels[i + 1] as f32;
        let b = pixels[i + 2] as f32;

        let sr = (r * 0.393 + g * 0.769 + b * 0.189).clamp(0.0, 255.0);
        let sg = (r * 0.349 + g * 0.686 + b * 0.168).clamp(0.0, 255.0);
        let sb = (r * 0.272 + g * 0.534 + b * 0.131).clamp(0.0, 255.0);

        pixels[i] = (r + (sr - r) * intensity).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (g + (sg - g) * intensity).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (b + (sb - b) * intensity).clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v23_filters() {
        let mut pixels = vec![180u8; 8 * 8 * 4];
        apply_sepia_tone(&mut pixels, 1.0);
        apply_ink_paint(&mut pixels, 8, 8, 4, 0);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
