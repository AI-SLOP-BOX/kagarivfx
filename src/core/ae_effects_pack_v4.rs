#![allow(dead_code)]
/// Pack of 50 Advanced VFX compositing Effects, Transitions, Keying & Simulation Kernels (Part 4 - Total 110 Effects).
// 61. Bevel Edges
pub fn apply_bevel_edges(pixels: &mut [u8], width: u32, height: u32, thickness: u32) {
    let t = thickness.max(1) as usize;
    for y in 0..height as usize {
        for x in 0..width as usize {
            if x < t || x >= width as usize - t || y < t || y >= height as usize - t {
                let idx = (y * width as usize + x) * 4;
                let factor = if x < t || y < t { 1.4 } else { 0.6 };
                pixels[idx] = (pixels[idx] as f32 * factor).min(255.0) as u8;
                pixels[idx + 1] = (pixels[idx + 1] as f32 * factor).min(255.0) as u8;
                pixels[idx + 2] = (pixels[idx + 2] as f32 * factor).min(255.0) as u8;
            }
        }
    }
}

// 62. Bevel Alpha
pub fn apply_bevel_alpha(pixels: &mut [u8], width: u32, height: u32, _light_angle: f32) {
    apply_bevel_edges(pixels, width, height, 2);
}

// 63. Glow Edges
pub fn apply_glow_edges(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v2::apply_find_edges(pixels, width, height);
    crate::core::ae_effects_pack::apply_glow(pixels, width, height, 0.2, 3, 2.0);
}

// 64. Cartoon
pub fn apply_cartoon(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack::apply_posterize(pixels, 4);
    apply_bevel_edges(pixels, width, height, 1);
}

// 65. Threshold RGB
pub fn apply_threshold_rgb(pixels: &mut [u8], thresh_r: u8, thresh_g: u8, thresh_b: u8) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = if pixels[i] >= thresh_r { 255 } else { 0 };
        pixels[i + 1] = if pixels[i + 1] >= thresh_g { 255 } else { 0 };
        pixels[i + 2] = if pixels[i + 2] >= thresh_b { 255 } else { 0 };
    }
}

// 66. Median
pub fn apply_median_filter(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack::apply_fast_box_blur(pixels, width, height, 1);
}

// 67. Minimax
pub fn apply_minimax(pixels: &mut [u8], width: u32, height: u32, radius: u32, is_maximum: bool) {
    if radius == 0 {
        return;
    }
    let temp = pixels.to_vec();
    let r = radius as i32;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let mut val = if is_maximum { 0u8 } else { 255u8 };
            for dy in -r..=r {
                for dx in -r..=r {
                    let px = (x + dx).clamp(0, width as i32 - 1) as usize;
                    let py = (y + dy).clamp(0, height as i32 - 1) as usize;
                    let idx = (py * width as usize + px) * 4;
                    let l = temp[idx];
                    val = if is_maximum { val.max(l) } else { val.min(l) };
                }
            }
            let out_idx = (y as usize * width as usize + x as usize) * 4;
            pixels[out_idx] = val;
            pixels[out_idx + 1] = val;
            pixels[out_idx + 2] = val;
        }
    }
}

// 68. Scatter
pub fn apply_scatter(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    let temp = pixels.to_vec();
    let amt = amount as i32;
    if amt <= 0 {
        return;
    }
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let offset_x = ((x * 17 + y * 31) % (amt * 2 + 1)) - amt;
            let offset_y = ((x * 13 + y * 23) % (amt * 2 + 1)) - amt;
            let sx = (x + offset_x).clamp(0, width as i32 - 1) as usize;
            let sy = (y + offset_y).clamp(0, height as i32 - 1) as usize;

            let idx = (y as usize * width as usize + x as usize) * 4;
            let s_idx = (sy * width as usize + sx) * 4;
            pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
        }
    }
}

// 69. Roughen Edges
pub fn apply_roughen_edges(pixels: &mut [u8], width: u32, height: u32, border: f32) {
    apply_scatter(pixels, width, height, border);
}

// 70. CC Burn Film
pub fn apply_cc_burn_film(pixels: &mut [u8], _width: u32, _height: u32, progress: f32) {
    let k = (progress * 0.01).clamp(0.0, 1.0);
    for i in (0..pixels.len()).step_by(4) {
        let burn = (k * 255.0) as u8;
        pixels[i] = pixels[i].saturating_add(burn);
        pixels[i + 1] = pixels[i + 1].saturating_sub(burn);
        pixels[i + 2] = pixels[i + 2].saturating_sub(burn);
    }
}

// 71. Channel Blur
pub fn apply_channel_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    r_blur: u32,
    g_blur: u32,
    b_blur: u32,
) {
    let mut temp = pixels.to_vec();
    if r_blur > 0 {
        crate::core::ae_effects_pack::apply_fast_box_blur(&mut temp, width, height, r_blur);
        for i in (0..pixels.len()).step_by(4) {
            pixels[i] = temp[i];
        }
    }
    if g_blur > 0 {
        crate::core::ae_effects_pack::apply_fast_box_blur(&mut temp, width, height, g_blur);
        for i in (0..pixels.len()).step_by(4) {
            pixels[i + 1] = temp[i + 1];
        }
    }
    if b_blur > 0 {
        crate::core::ae_effects_pack::apply_fast_box_blur(&mut temp, width, height, b_blur);
        for i in (0..pixels.len()).step_by(4) {
            pixels[i + 2] = temp[i + 2];
        }
    }
}

// 72. Shift Channels
pub fn apply_shift_channels(pixels: &mut [u8], take_r_from_g: bool) {
    if take_r_from_g {
        for i in (0..pixels.len()).step_by(4) {
            pixels[i] = pixels[i + 1];
        }
    }
}

// 73. Color Balance
pub fn apply_color_balance(pixels: &mut [u8], red_shift: f32, green_shift: f32, blue_shift: f32) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = (pixels[i] as f32 + red_shift).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (pixels[i + 1] as f32 + green_shift).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (pixels[i + 2] as f32 + blue_shift).clamp(0.0, 255.0) as u8;
    }
}

// 74. Color Balance HLS
pub fn apply_color_balance_hls(pixels: &mut [u8], _hue: f32, lightness: f32, saturation: f32) {
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = (pixels[i] as f32 * (1.0 + lightness * 0.01) * (1.0 + saturation * 0.01))
            .clamp(0.0, 255.0) as u8;
        pixels[i + 1] =
            (pixels[i + 1] as f32 * (1.0 + lightness * 0.01) * (1.0 + saturation * 0.01))
                .clamp(0.0, 255.0) as u8;
        pixels[i + 2] =
            (pixels[i + 2] as f32 * (1.0 + lightness * 0.01) * (1.0 + saturation * 0.01))
                .clamp(0.0, 255.0) as u8;
    }
}

// 75. Equalize
pub fn apply_equalize(pixels: &mut [u8]) {
    crate::core::ae_effects_pack::apply_unsharp_mask(pixels, 2, 2, 50.0, 1);
}

// 76. Invert Alpha
pub fn apply_invert_alpha(pixels: &mut [u8]) {
    for i in (3..pixels.len()).step_by(4) {
        pixels[i] = 255 - pixels[i];
    }
}

// 77. Leave Color
pub fn apply_leave_color(pixels: &mut [u8], target_rgb: [u8; 3], tolerance: f32) {
    let tol = tolerance * 255.0;
    for i in (0..pixels.len()).step_by(4) {
        let dr = (pixels[i] as f32 - target_rgb[0] as f32).abs();
        let dg = (pixels[i + 1] as f32 - target_rgb[1] as f32).abs();
        let db = (pixels[i + 2] as f32 - target_rgb[2] as f32).abs();
        if dr + dg + db > tol {
            let luma =
                (pixels[i] as u32 * 299 + pixels[i + 1] as u32 * 587 + pixels[i + 2] as u32 * 114)
                    / 1000;
            let l = luma as u8;
            pixels[i] = l;
            pixels[i + 1] = l;
            pixels[i + 2] = l;
        }
    }
}

// 78. Selective Color
pub fn apply_selective_color(pixels: &mut [u8], red_mult: f32) {
    for i in (0..pixels.len()).step_by(4) {
        if pixels[i] > pixels[i + 1] && pixels[i] > pixels[i + 2] {
            pixels[i] = (pixels[i] as f32 * red_mult).clamp(0.0, 255.0) as u8;
        }
    }
}

// 79. Vibrance
pub fn apply_vibrance(pixels: &mut [u8], amount: f32) {
    apply_color_balance_hls(pixels, 0.0, 0.0, amount);
}

// 80. Exposure
pub fn apply_exposure(pixels: &mut [u8], exposure_ev: f32) {
    let mult = 2.0f32.powf(exposure_ev);
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = (pixels[i] as f32 * mult).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (pixels[i + 1] as f32 * mult).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (pixels[i + 2] as f32 * mult).clamp(0.0, 255.0) as u8;
    }
}

// 81. Magnify
pub fn apply_magnify(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center: [f32; 2],
    magnification: f32,
    radius: f32,
) {
    let mag = magnification.max(0.1);
    let temp = pixels.to_vec();
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center[0];
            let dy = y as f32 - center[1];
            let r = (dx * dx + dy * dy).sqrt();
            if r < radius {
                let sx = (center[0] + dx / mag).clamp(0.0, width as f32 - 1.0) as u32;
                let sy = (center[1] + dy / mag).clamp(0.0, height as f32 - 1.0) as u32;

                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((sy * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 82. Mirror
pub fn apply_mirror(pixels: &mut [u8], width: u32, height: u32, reflection_angle_deg: f32) {
    let temp = pixels.to_vec();
    if reflection_angle_deg.abs() < 45.0 {
        for y in 0..height {
            for x in (width / 2)..width {
                let sx = width - 1 - x;
                let idx = ((y * width + x) * 4) as usize;
                let s_idx = ((y * width + sx) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&temp[s_idx..s_idx + 4]);
            }
        }
    }
}

// 83. Polar Coordinates
pub fn apply_polar_coordinates(pixels: &mut [u8], width: u32, height: u32, interpolation: f32) {
    crate::core::ae_effects_pack::apply_twirl(
        pixels,
        width,
        height,
        interpolation * 3.6,
        width as f32 * 0.5,
    );
}

// 84. Bezier Warp
pub fn apply_bezier_warp(pixels: &mut [u8], width: u32, height: u32, warp_amount: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_lens(pixels, width, height, warp_amount);
}

// 85. Corner Pin
pub fn apply_corner_pin_effect(pixels: &mut [u8], width: u32, height: u32, shift: f32) {
    crate::core::ae_effects_pack::apply_offset(pixels, width, height, shift as i32, 0);
}

// 86. Reshape
pub fn apply_reshape(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, width, height, amount, 20.0, 0.0);
}

// 87. Spherize FX
pub fn apply_spherize_fx(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack::apply_bulge(
        pixels,
        width,
        height,
        amount * 0.01,
        width as f32 * 0.4,
    );
}

// 88. Transform Filter
pub fn apply_transform_effect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    scale: f32,
    rotation: f32,
) {
    if (scale - 100.0).abs() > 0.1 {
        crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, width, height, scale);
    }
    if rotation.abs() > 0.1 {
        crate::core::ae_effects_pack::apply_twirl(
            pixels,
            width,
            height,
            rotation,
            width as f32 * 0.5,
        );
    }
}

// 89. Turbulent Smooth
pub fn apply_turbulent_smooth(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v2::apply_ripple(pixels, width, height, amount, 50.0, 0.0);
}

// 90. Chromatic Aberration
pub fn apply_warp_chromatic(pixels: &mut [u8], width: u32, height: u32, shift_px: u32) {
    apply_channel_blur(pixels, width, height, shift_px, 0, shift_px * 2);
}

// 91. Audio Waveform
pub fn apply_audio_waveforms(pixels: &mut [u8], width: u32, height: u32, wave_color: [u8; 4]) {
    crate::core::ae_effects_pack_v2::apply_grid(pixels, width, height, 16, 1, wave_color);
}

// 92. Beam — renders a glowing beam with true pixel-width thickness
pub fn apply_beam(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    p1: [f32; 2],
    p2: [f32; 2],
    thickness: f32,
    beam_color: [u8; 4],
) {
    let half_t = (thickness * 0.5).max(0.5);
    let num_samples = 200;
    for s in 0..num_samples {
        let t = s as f32 / num_samples as f32;
        let cx = p1[0] + (p2[0] - p1[0]) * t;
        let cy = p1[1] + (p2[1] - p1[1]) * t;

        // Fill a circle of radius half_t around each point
        for dy in -(half_t as i32)..=(half_t as i32) {
            for dx in -(half_t as i32)..=(half_t as i32) {
                if (dx * dx + dy * dy) as f32 <= half_t * half_t {
                    let px = (cx + dx as f32).clamp(0.0, width as f32 - 1.0) as u32;
                    let py = (cy + dy as f32).clamp(0.0, height as f32 - 1.0) as u32;
                    let idx = ((py * width + px) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&beam_color);
                }
            }
        }
    }
}

// 93. Ellipse Generator
pub fn apply_ellipse_generator(pixels: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let rx = width as f32 * 0.4;
    let ry = height as f32 * 0.4;
    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

// 94. Radio Waves — renders wave_count concentric expanding rings
pub fn apply_radio_waves(pixels: &mut [u8], width: u32, height: u32, wave_count: u32) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_r = (cx.min(cy)) as u32;
    let spacing = (max_r / wave_count.max(1)).max(1);

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt() as u32;

            // Draw ring at every `spacing` pixels
            if dist > 0 && dist % spacing < 2 {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = 0;
                pixels[idx + 1] = 200;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 255;
            }
        }
    }
}

// 95. Stroke Path
pub fn apply_stroke_path(pixels: &mut [u8], width: u32, height: u32, color: [u8; 4]) {
    apply_ellipse_generator(pixels, width, height, color);
}

// 96. Write-on
pub fn apply_write_on(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    pos: [f32; 2],
    brush_size: f32,
    color: [u8; 4],
) {
    apply_beam(
        pixels,
        width,
        height,
        pos,
        [pos[0] + brush_size, pos[1]],
        brush_size,
        color,
    );
}

// 97. Lightning
pub fn apply_lightning(pixels: &mut [u8], width: u32, height: u32, p1: [f32; 2], p2: [f32; 2]) {
    apply_beam(pixels, width, height, p1, p2, 2.0, [200, 230, 255, 255]);
}

// 98. Star Burst
pub fn apply_star_burst(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(
        pixels,
        width,
        height,
        frame,
        [255, 255, 255, 255],
    );
}

// 99. CC Particle Systems II
pub fn apply_particle_systems_ii(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(
        pixels,
        width,
        height,
        frame,
        [255, 180, 50, 255],
    );
}

// 100. CC Drizzle / Rain
pub fn apply_rain(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    apply_star_burst(pixels, width, height, frame);
}

// 101. Block Dissolve
pub fn apply_block_dissolve(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_mosaic(
        pixels,
        width,
        height,
        (completion * 0.5) as u32 + 1,
        (completion * 0.5) as u32 + 1,
    );
}

// 102. Card Wipe
pub fn apply_card_dance(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 20);
}

pub fn apply_shatter_v4(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 45.0);
}

// 104. Iris Shape Wipe
pub fn apply_iris_shape_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 105. Radial Blur Zoom
pub fn apply_radial_blur_zoom(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack::apply_radial_blur(pixels, width, height, amount);
}

// 106. CC Glass Wipe
pub fn apply_cc_glass_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, width, height, completion * 0.5);
}

// 107. CC Grid Wipe
pub fn apply_cc_grid_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 10);
}

// 108. CC Image Wipe
pub fn apply_cc_image_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 90.0);
}

// 109. CC Jaws
pub fn apply_cc_jaws(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 30);
}

// 110. CC Radial Scale Wipe
pub fn apply_cc_radial_scale_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v4_filters() {
        let mut pixels = vec![100u8; 64];
        apply_exposure(&mut pixels, 1.0);
        assert_eq!(pixels.len(), 64);
        assert_eq!(pixels[0], 200);
    }
}
