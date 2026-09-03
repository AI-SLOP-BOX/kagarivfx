#![allow(dead_code)]
/// After Effects VFX Kernels Part 22 — Matte Tools & Alpha Compositing
// 1. Simple Choke / Spread Matte (Morphological Erosion/Dilation)
pub fn apply_matte_choke(pixels: &mut [u8], width: u32, height: u32, radius: u32, expand: bool) {
    if radius == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let mut extreme = if expand { 0u8 } else { 255u8 };

            for ky in -r..=r {
                let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -r..=r {
                    let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                    let a = temp[(py * width as usize + px) * 4 + 3];
                    if expand {
                        extreme = extreme.max(a);
                    } else {
                        extreme = extreme.min(a);
                    }
                }
            }

            pixels[idx + 3] = extreme;
        }
    }
}

// 2. Soft Matte Edge Blur (Alpha Feather)
pub fn apply_alpha_feather(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    if radius == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let r = radius as i32;

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let mut sum = 0.0f32;
            let mut weight = 0.0f32;
            let sigma = radius as f32 * 0.5;

            for ky in -r..=r {
                let py = (y + ky).clamp(0, height as i32 - 1) as usize;
                for kx in -r..=r {
                    let px = (x + kx).clamp(0, width as i32 - 1) as usize;
                    let dist_sq = (kx * kx + ky * ky) as f32;
                    let w = (-dist_sq / (2.0 * sigma * sigma)).exp();
                    sum += temp[(py * width as usize + px) * 4 + 3] as f32 * w;
                    weight += w;
                }
            }

            pixels[idx + 3] = (sum / weight).clamp(0.0, 255.0) as u8;
        }
    }
}

// 3. Alpha From Luminance (Luminance to Alpha Conversion)
pub fn apply_alpha_from_luminance(pixels: &mut [u8], invert: bool) {
    for i in (0..pixels.len()).step_by(4) {
        let luma =
            (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                / 1000;
        pixels[i + 3] = if invert { 255 - luma as u8 } else { luma as u8 };
    }
}

// 4. Pre-multiply Alpha (Straight to Premul Conversion)
pub fn apply_premultiply_alpha(pixels: &mut [u8]) {
    for i in (0..pixels.len()).step_by(4) {
        let a = pixels[i + 3] as f32 / 255.0;
        pixels[i] = (pixels[i] as f32 * a) as u8;
        pixels[i + 1] = (pixels[i + 1] as f32 * a) as u8;
        pixels[i + 2] = (pixels[i + 2] as f32 * a) as u8;
    }
}

// 5. Un-Premultiply Alpha (Premul to Straight Conversion)
pub fn apply_unpremultiply_alpha(pixels: &mut [u8]) {
    for i in (0..pixels.len()).step_by(4) {
        let a = pixels[i + 3] as f32 / 255.0;
        if a > 0.001 {
            pixels[i] = (pixels[i] as f32 / a).clamp(0.0, 255.0) as u8;
            pixels[i + 1] = (pixels[i + 1] as f32 / a).clamp(0.0, 255.0) as u8;
            pixels[i + 2] = (pixels[i + 2] as f32 / a).clamp(0.0, 255.0) as u8;
        }
    }
}

// 6. Blend Mode: Screen
pub fn apply_blend_screen(base: &mut [u8], overlay: &[u8]) {
    let len = base.len().min(overlay.len());
    for i in (0..len).step_by(4) {
        for c in 0..3 {
            let b = base[i + c] as f32 / 255.0;
            let o = overlay[i + c] as f32 / 255.0;
            base[i + c] = ((1.0 - (1.0 - b) * (1.0 - o)) * 255.0) as u8;
        }
    }
}

// 7. Blend Mode: Multiply
pub fn apply_blend_multiply(base: &mut [u8], overlay: &[u8]) {
    let len = base.len().min(overlay.len());
    for i in (0..len).step_by(4) {
        for c in 0..3 {
            let b = base[i + c] as f32 / 255.0;
            let o = overlay[i + c] as f32 / 255.0;
            base[i + c] = (b * o * 255.0) as u8;
        }
    }
}

// 8. Blend Mode: Overlay
pub fn apply_blend_overlay(base: &mut [u8], overlay: &[u8]) {
    let len = base.len().min(overlay.len());
    for i in (0..len).step_by(4) {
        for c in 0..3 {
            let b = base[i + c] as f32 / 255.0;
            let o = overlay[i + c] as f32 / 255.0;
            let result = if b < 0.5 {
                2.0 * b * o
            } else {
                1.0 - 2.0 * (1.0 - b) * (1.0 - o)
            };
            base[i + c] = (result * 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v22_filters() {
        let mut pixels = vec![200u8; 8 * 8 * 4];
        apply_alpha_from_luminance(&mut pixels, false);
        apply_premultiply_alpha(&mut pixels);
        apply_unpremultiply_alpha(&mut pixels);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
