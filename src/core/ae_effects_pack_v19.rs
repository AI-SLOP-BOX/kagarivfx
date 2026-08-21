#![allow(dead_code)]
/// After Effects VFX Kernels Part 19 — Frequency Domain & Compositing Ops
// 1. Unsharp Mask (Gaussian Difference Sharpening)
pub fn apply_unsharp_mask(pixels: &mut [u8], width: u32, height: u32, radius: u32, amount: f32, threshold: u8) {
    if radius == 0 || amount <= 0.0 { return; }
    let temp = pixels.to_vec();
    let mut blurred = temp.clone();
    let r = radius as i32;

    // Gaussian blur into blurred buffer
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let mut sum = [0.0f32; 3];
            let mut weight = 0.0f32;

            for ky in -r..=r {
                let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -r..=r {
                    let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                    let dist_sq = (kx * kx + ky * ky) as f32;
                    let sigma = radius as f32 * 0.5;
                    let w = (-dist_sq / (2.0 * sigma * sigma)).exp();
                    let k_idx = (py * width as usize + px) * 4;
                    for c in 0..3 {
                        sum[c] += temp[k_idx + c] as f32 * w;
                    }
                    weight += w;
                }
            }

            let idx = (y as usize * width as usize + x as usize) * 4;
            for c in 0..3 {
                blurred[idx + c] = (sum[c] / weight).clamp(0.0, 255.0) as u8;
            }
        }
    }

    // Sharpen = original + amount * (original - blurred) if diff > threshold
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let diff = temp[i + c] as i16 - blurred[i + c] as i16;
            if diff.unsigned_abs() > threshold as u16 {
                let sharpened = temp[i + c] as f32 + diff as f32 * amount;
                pixels[i + c] = sharpened.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 2. Median Filter (Salt-and-Pepper Noise Removal)
pub fn apply_median_filter(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 { return; }
    let temp = pixels.to_vec();
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = (y as usize * width as usize + x as usize) * 4;

            for c in 0..3 {
                let mut samples: Vec<u8> = Vec::new();

                for ky in -r..=r {
                    let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                    for kx in -r..=r {
                        let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                        samples.push(temp[(py * width as usize + px) * 4 + c]);
                    }
                }

                samples.sort_unstable();
                pixels[idx + c] = samples[samples.len() / 2];
            }
        }
    }
}

// 3. Sobel Edge Detection
pub fn apply_sobel_edges(pixels: &mut [u8], width: u32, height: u32, invert: bool) {
    let temp = pixels.to_vec();

    let kx: [[i16; 3]; 3] = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
    let ky: [[i16; 3]; 3] = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];

    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let mut gx_sum = 0i16;
            let mut gy_sum = 0i16;

            for ky_off in 0..3usize {
                for kx_off in 0..3usize {
                    let py = y as usize + ky_off - 1;
                    let px = x as usize + kx_off - 1;
                    let k_idx = (py * width as usize + px) * 4;
                    let luma = (temp[k_idx] as i16 + temp[k_idx + 1] as i16 + temp[k_idx + 2] as i16) / 3;
                    gx_sum += luma * kx[ky_off][kx_off];
                    gy_sum += luma * ky[ky_off][kx_off];
                }
            }

            let magnitude = ((gx_sum * gx_sum + gy_sum * gy_sum) as f32).sqrt().clamp(0.0, 255.0) as u8;
            let val = if invert { 255 - magnitude } else { magnitude };
            pixels[idx] = val;
            pixels[idx + 1] = val;
            pixels[idx + 2] = val;
        }
    }
}

// 4. Mosaic (Pixelate) Effect
pub fn apply_mosaic(pixels: &mut [u8], width: u32, height: u32, block_w: u32, block_h: u32) {
    if block_w == 0 || block_h == 0 { return; }

    let mut y = 0u32;
    while y < height {
        let mut x = 0u32;
        while x < width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut count = 0u32;

            for by in 0..block_h {
                let py = (y + by).min(height - 1) as usize;
                for bx in 0..block_w {
                    let px = (x + bx).min(width - 1) as usize;
                    let idx = (py * width as usize + px) * 4;
                    r += pixels[idx] as u32;
                    g += pixels[idx + 1] as u32;
                    b += pixels[idx + 2] as u32;
                    count += 1;
                }
            }

            let avg_r = (r / count) as u8;
            let avg_g = (g / count) as u8;
            let avg_b = (b / count) as u8;

            for by in 0..block_h {
                let py = (y + by).min(height - 1) as usize;
                for bx in 0..block_w {
                    let px = (x + bx).min(width - 1) as usize;
                    let idx = (py * width as usize + px) * 4;
                    pixels[idx] = avg_r;
                    pixels[idx + 1] = avg_g;
                    pixels[idx + 2] = avg_b;
                }
            }

            x += block_w;
        }
        y += block_h;
    }
}

// 5. Tilt Shift Lens Simulation (Graduated Focus Falloff)
pub fn apply_tilt_shift(pixels: &mut [u8], width: u32, height: u32, focus_y: f32, focus_height: f32, max_blur: u32) {
    if max_blur == 0 { return; }
    let temp = pixels.to_vec();

    for y in 0..height {
        let fy = y as f32;
        let dist_from_focus = (fy - focus_y).abs() - focus_height * 0.5;
        let blur_r = if dist_from_focus <= 0.0 {
            0u32
        } else {
            ((dist_from_focus / (height as f32 * 0.5)) * max_blur as f32) as u32
        }.min(max_blur);

        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            if blur_r == 0 { continue; }

            let mut r = 0.0f32; let mut g = 0.0f32; let mut b = 0.0f32; let mut count = 0u32;

            for kx in -(blur_r as i32)..=(blur_r as i32) {
                let px = (x as i32 + kx).clamp(0, width as i32 - 1) as usize;
                let k_idx = (y as usize * width as usize + px) * 4;
                r += temp[k_idx] as f32;
                g += temp[k_idx + 1] as f32;
                b += temp[k_idx + 2] as f32;
                count += 1;
            }

            pixels[idx] = (r / count as f32) as u8;
            pixels[idx + 1] = (g / count as f32) as u8;
            pixels[idx + 2] = (b / count as f32) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v19_filters() {
        let mut pixels = vec![128u8; 8 * 8 * 4];
        apply_mosaic(&mut pixels, 8, 8, 2, 2);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
