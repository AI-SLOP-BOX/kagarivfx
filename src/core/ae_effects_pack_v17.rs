#![allow(dead_code)]
/// After Effects VFX Kernels Part 17 — Edge-Aware Filters & Motion Rendering
// 1. Bilateral Filter (Edge-Preserving Smoothing)
pub fn apply_bilateral_filter(pixels: &mut [u8], width: u32, height: u32, radius: u32, sigma_space: f32, sigma_color: f32) {
    if radius == 0 { return; }
    let temp = pixels.to_vec();
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let mut sum = [0.0f32; 3];
            let mut weight_total = 0.0f32;

            for ky in -r..=r {
                let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -r..=r {
                    let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                    let k_idx = (py * width as usize + px) * 4;

                    let space_dist_sq = (kx * kx + ky * ky) as f32;
                    let space_w = (-space_dist_sq / (2.0 * sigma_space * sigma_space)).exp();

                    let mut color_dist_sq = 0.0f32;
                    for c in 0..3 {
                        let d = temp[idx + c] as f32 - temp[k_idx + c] as f32;
                        color_dist_sq += d * d;
                    }
                    let color_w = (-color_dist_sq / (2.0 * sigma_color * sigma_color)).exp();

                    let w = space_w * color_w;
                    for c in 0..3 {
                        sum[c] += temp[k_idx + c] as f32 * w;
                    }
                    weight_total += w;
                }
            }

            for c in 0..3 {
                pixels[idx + c] = (sum[c] / weight_total).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 2. Motion Blur (Velocity Vector Based)
pub fn apply_motion_blur_vector(pixels: &mut [u8], width: u32, height: u32, vel_x: f32, vel_y: f32, samples: u32) {
    if vel_x.abs() < 0.001 && vel_y.abs() < 0.001 || samples == 0 { return; }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let mut r = 0.0f32; let mut g = 0.0f32; let mut b = 0.0f32;

            for s in 0..samples {
                let t = (s as f32 / (samples - 1) as f32) - 0.5;
                let sx = (x as f32 + vel_x * t).clamp(0.0, (width - 1) as f32) as usize;
                let sy = (y as f32 + vel_y * t).clamp(0.0, (height - 1) as f32) as usize;
                let s_idx = (sy * width as usize + sx) * 4;
                r += temp[s_idx] as f32;
                g += temp[s_idx + 1] as f32;
                b += temp[s_idx + 2] as f32;
            }

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx] = (r / samples as f32) as u8;
            pixels[idx + 1] = (g / samples as f32) as u8;
            pixels[idx + 2] = (b / samples as f32) as u8;
        }
    }
}

// 3. Emboss (Surface Relief Shader)
pub fn apply_emboss(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32, depth: f32) {
    let temp = pixels.to_vec();
    let rad = angle_deg.to_radians();
    let lx = rad.cos();
    let ly = rad.sin();

    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let mut gx = [0.0f32; 3];
            let mut gy = [0.0f32; 3];

            for c in 0..3 {
                let left = temp[(y as usize * width as usize + (x - 1) as usize) * 4 + c] as f32;
                let right = temp[(y as usize * width as usize + (x + 1) as usize) * 4 + c] as f32;
                let top = temp[((y - 1) as usize * width as usize + x as usize) * 4 + c] as f32;
                let bot = temp[((y + 1) as usize * width as usize + x as usize) * 4 + c] as f32;
                gx[c] = right - left;
                gy[c] = bot - top;
            }

            let luma = (gx[0] * lx + gy[0] * ly + gx[1] * lx + gy[1] * ly + gx[2] * lx + gy[2] * ly) / 3.0;
            let val = (128.0 + luma * depth).clamp(0.0, 255.0) as u8;
            pixels[idx] = val;
            pixels[idx + 1] = val;
            pixels[idx + 2] = val;
        }
    }
}

// 4. Anisotropic Diffusion (Perona-Malik Edge-Stopping Diffusion)
pub fn apply_anisotropic_diffusion(pixels: &mut [u8], width: u32, height: u32, iterations: u32, kappa: f32, delta_t: f32) {
    let mut buf: Vec<f32> = pixels.iter().map(|&v| v as f32).collect();

    for _ in 0..iterations {
        let prev = buf.clone();

        for y in 1..(height as usize - 1) {
            for x in 1..(width as usize - 1) {
                for c in 0..3 {
                    let i = (y * width as usize + x) * 4 + c;
                    let n = ((y - 1) * width as usize + x) * 4 + c;
                    let s = ((y + 1) * width as usize + x) * 4 + c;
                    let w = (y * width as usize + (x - 1)) * 4 + c;
                    let e = (y * width as usize + (x + 1)) * 4 + c;

                    let dn = prev[n] - prev[i];
                    let ds = prev[s] - prev[i];
                    let dw = prev[w] - prev[i];
                    let de = prev[e] - prev[i];

                    // Perona-Malik conductance function
                    let cn = (-(dn / kappa).powi(2)).exp();
                    let cs = (-(ds / kappa).powi(2)).exp();
                    let cw = (-(dw / kappa).powi(2)).exp();
                    let ce = (-(de / kappa).powi(2)).exp();

                    buf[i] += delta_t * (cn * dn + cs * ds + cw * dw + ce * de);
                }
            }
        }
    }

    for (i, v) in buf.iter().enumerate() {
        pixels[i] = v.clamp(0.0, 255.0) as u8;
    }
}

// 5. Cross Hatch Stylize (Ink Sketch Rendering)
pub fn apply_cross_hatch(pixels: &mut [u8], width: u32, height: u32, line_gap: u32, threshold: u8) {
    if line_gap == 0 { return; }
    let temp = pixels.to_vec();
    pixels.fill(255); // White background

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let luma = (temp[idx] as u32 * 299 + temp[idx + 1] as u32 * 587 + temp[idx + 2] as u32 * 114) / 1000;

            if luma < threshold as u32 {
                // Draw hatching lines (diagonal)
                if (x + y) % line_gap == 0 || (x as i32 - y as i32).unsigned_abs().is_multiple_of(line_gap) {
                    pixels[idx] = 0;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v17_filters() {
        let mut pixels = vec![100u8; 64 * 4];
        apply_emboss(&mut pixels, 8, 8, 45.0, 1.0);
        assert_eq!(pixels.len(), 64 * 4);
    }
}
