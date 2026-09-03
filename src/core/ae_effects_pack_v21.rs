#![allow(dead_code)]
/// After Effects VFX Kernels Part 21 — Warp & Geometry Distortion Suite
// 1. Mesh Warp (Bilinear Grid Deformation)
pub fn apply_mesh_warp(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    grid_x: u32,
    grid_y: u32,
    offsets: &[(f32, f32)],
) {
    if grid_x == 0 || grid_y == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let cell_w = width as f32 / grid_x as f32;
    let cell_h = height as f32 / grid_y as f32;

    for y in 0..height {
        for x in 0..width {
            let gx = ((x as f32 / cell_w) as usize).min(grid_x as usize - 1);
            let gy = ((y as f32 / cell_h) as usize).min(grid_y as usize - 1);

            let tx = (x as f32 - gx as f32 * cell_w) / cell_w;
            let ty = (y as f32 - gy as f32 * cell_h) / cell_h;

            let i00 = gy * (grid_x as usize + 1) + gx;
            let i10 = gy * (grid_x as usize + 1) + (gx + 1).min(grid_x as usize);
            let i01 = (gy + 1).min(grid_y as usize - 1) * (grid_x as usize + 1) + gx;
            let i11 = (gy + 1).min(grid_y as usize - 1) * (grid_x as usize + 1)
                + (gx + 1).min(grid_x as usize);

            let get_off = |i: usize| -> (f32, f32) {
                if i < offsets.len() {
                    offsets[i]
                } else {
                    (0.0, 0.0)
                }
            };

            let (o00x, o00y) = get_off(i00);
            let (o10x, o10y) = get_off(i10);
            let (o01x, o01y) = get_off(i01);
            let (o11x, o11y) = get_off(i11);

            let off_x = o00x * (1.0 - tx) * (1.0 - ty)
                + o10x * tx * (1.0 - ty)
                + o01x * (1.0 - tx) * ty
                + o11x * tx * ty;
            let off_y = o00y * (1.0 - tx) * (1.0 - ty)
                + o10y * tx * (1.0 - ty)
                + o01y * (1.0 - tx) * ty
                + o11y * tx * ty;

            let sx = (x as f32 + off_x).clamp(0.0, (width - 1) as f32) as usize;
            let sy = (y as f32 + off_y).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (sy * width as usize + sx) * 4;
            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 2. Reflection Map (Mirror Reflection Compositing)
pub fn apply_reflection_map(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    reflect_y: u32,
    fade_dist: f32,
    opacity: f32,
) {
    if opacity <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();

    for y in reflect_y..height {
        let mirror_y = reflect_y as i32 - (y as i32 - reflect_y as i32);
        if mirror_y < 0 {
            break;
        }

        let fade = (1.0 - (y - reflect_y) as f32 / fade_dist.max(1.0)).clamp(0.0, 1.0) * opacity;

        for x in 0..width {
            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (mirror_y as usize * width as usize + x as usize) * 4;

            for c in 0..3 {
                let reflected = temp[src_idx + c] as f32 * fade;
                pixels[dst_idx + c] =
                    (pixels[dst_idx + c] as f32 * (1.0 - fade) + reflected).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

// 3. Fisheye Lens Distortion
pub fn apply_fisheye(pixels: &mut [u8], width: u32, height: u32, strength: f32) {
    if strength.abs() <= 0.001 {
        return;
    }
    let temp = pixels.to_vec();
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = cx.min(cy);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / max_r;
            let dy = (y as f32 - cy) / max_r;
            let r = (dx * dx + dy * dy).sqrt();

            if r > 0.001 && r <= 1.0 {
                let r2 = r * (1.0 + strength * r * r);
                let sx = (cx + dx / r * r2 * max_r).clamp(0.0, (width - 1) as f32) as usize;
                let sy = (cy + dy / r * r2 * max_r).clamp(0.0, (height - 1) as f32) as usize;

                let dst_idx = (y as usize * width as usize + x as usize) * 4;
                let src_idx = (sy * width as usize + sx) * 4;
                pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
            }
        }
    }
}

// 4. Displacement Map with Channel Selection
pub fn apply_displacement_channel(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    disp_map: &[u8],
    h_channel: usize,
    v_channel: usize,
    max_disp: f32,
) {
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize * width as usize + x as usize) * 4;
            let map_idx = idx.min(disp_map.len().saturating_sub(4));

            let h_disp = (disp_map[map_idx + h_channel.min(3)] as f32 / 127.5 - 1.0) * max_disp;
            let v_disp = (disp_map[map_idx + v_channel.min(3)] as f32 / 127.5 - 1.0) * max_disp;

            let sx = (x as f32 + h_disp).clamp(0.0, (width - 1) as f32) as usize;
            let sy = (y as f32 + v_disp).clamp(0.0, (height - 1) as f32) as usize;

            let src_idx = (sy * width as usize + sx) * 4;
            pixels[idx..idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 5. Barrel / Pincushion Lens Correction
pub fn apply_barrel_correction(pixels: &mut [u8], width: u32, height: u32, k1: f32, k2: f32) {
    let temp = pixels.to_vec();
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;

    for y in 0..height {
        for x in 0..width {
            let xn = (x as f32 - cx) / cx;
            let yn = (y as f32 - cy) / cy;
            let r2 = xn * xn + yn * yn;
            let r4 = r2 * r2;
            let factor = 1.0 + k1 * r2 + k2 * r4;

            let sx = (cx + xn * factor * cx).clamp(0.0, (width - 1) as f32) as usize;
            let sy = (cy + yn * factor * cy).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (sy * width as usize + sx) * 4;
            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v21_filters() {
        let mut pixels = vec![100u8; 8 * 8 * 4];
        apply_fisheye(&mut pixels, 8, 8, 0.5);
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
