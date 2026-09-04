#![allow(dead_code)]

use crate::core::effect_utils;
use crate::core::software_renderer::rgba_buffer_size;
use rayon::prelude::*;

/// Bilinear interpolation sample — delegates to shared `effect_utils`.
#[inline]
pub fn sample_bilinear(pixels: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    effect_utils::sample_bilinear(pixels, width, height, x, y)
}

/// Apply chromatic aberration by shifting R and B channels.
/// shift_r/shift_b: pixel offset for red/blue channels (negative = left/up).
/// edge_falloff: 0.0=hard edge, 1.0=smooth falloff at borders.
pub fn apply_chromatic_aberration(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    shift_r: f32,
    shift_b: f32,
    edge_falloff: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let mut tmp = vec![0u8; pixels.len()];
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;

    // Per-pixel independent — parallelized across rows with rayon.
    tmp.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = (i % width as usize) as f32;
            let y = (i / width as usize) as f32;
            let dx = (x - cx) / cx;
            let dy = (y - cy) / cy;
            let dist = ((dx * dx + dy * dy).sqrt()).min(1.0);
            let falloff = if edge_falloff < 1.0 {
                dist.powf(1.0 - edge_falloff.clamp(0.0, 1.0))
            } else {
                dist
            };

            // Radial dispersion model (from NextVFX): channels shift ALONG the
            // vector from image center through this pixel — like real lens
            // chromatic aberration, which fringes radially, not horizontally.
            let dir_x = if dist > 1e-6 { dx / dist } else { 0.0 };
            let dir_y = if dist > 1e-6 { dy / dist } else { 0.0 };
            let r_sample = sample_bilinear(
                pixels,
                width,
                height,
                x + shift_r * falloff * dir_x,
                y + shift_r * falloff * dir_y,
            );
            let g_sample = sample_bilinear(pixels, width, height, x, y);
            let b_sample = sample_bilinear(
                pixels,
                width,
                height,
                x - shift_b * falloff * dir_x,
                y - shift_b * falloff * dir_y,
            );

            out[0] = r_sample[0];
            out[1] = g_sample[1];
            out[2] = b_sample[2];
            out[3] = g_sample[3];
        });
    pixels[..tmp.len()].copy_from_slice(&tmp);
}

/// Apply vignette darkening effect.
/// intensity: 0.0=no effect, 1.0=full black edges.
/// roundness: shape of vignette (0.0=circular, 1.0=elliptical).
/// feather: edge softness (0.0=hard, 1.0=very soft).
/// color: vignette color [r,g,b].
pub fn apply_vignette(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    intensity: f32,
    roundness: f32,
    feather: f32,
    color: [f32; 4],
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let aspect = width as f32 / height as f32;

    pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        if chunk.len() < 4 {
            return;
        }
        let px = (i as u32) % width;
        let py = (i as u32) / width;
        let dx = (px as f32 - cx) / cx;
        let dy = (py as f32 - cy) / cy;
        let rx = dx * aspect;
        let ry = dy;
        let norm_dist = ((rx * rx + ry * ry).powf(roundness.clamp(0.1, 10.0) * 0.5)).min(1.0);
        // feather: 0.0=hard edge (vignette only at border), 1.0=very soft (vignette reaches center)
        let soft_start = (1.0 - feather.clamp(0.0, 1.0)).max(0.001);
        let vignette = ((norm_dist - soft_start) / (1.0 - soft_start)).clamp(0.0, 1.0);
        let factor = vignette * intensity;

        for ch in 0..3 {
            let orig = chunk[ch] as f32 / 255.0;
            let blended = orig * (1.0 - factor) + color[ch] * factor;
            chunk[ch] = (blended.clamp(0.0, 1.0) * 255.0) as u8;
        }
    });
}

/// Apply levels adjustment (tone remapping).
/// input_black/input_white: input range [0-255] → output [0-255].
/// gamma: midtone correction (>1.0 brightens, <1.0 darkens).
/// output_black/output_white: output range [0-255].
#[allow(clippy::too_many_arguments)]
pub fn apply_levels(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    input_black: f32,
    input_white: f32,
    gamma: f32,
    output_black: f32,
    output_white: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let range = (input_white - input_black).max(0.001);
    let inv_gamma = if gamma > 0.0 { 1.0 / gamma } else { 1.0 };

    let mut table = [0u8; 256];
    for (i, entry) in table.iter_mut().enumerate() {
        let v = i as f32 / 255.0;
        let normalized = ((v - input_black) / range).clamp(0.0, 1.0);
        let corrected = normalized.powf(inv_gamma);
        let out = output_black + (output_white - output_black) * corrected;
        *entry = (out.clamp(0.0, 1.0) * 255.0) as u8;
    }

    pixels.par_chunks_mut(4).for_each(|chunk| {
        if chunk.len() < 4 {
            return;
        }
        chunk[0] = table[chunk[0] as usize];
        chunk[1] = table[chunk[1] as usize];
        chunk[2] = table[chunk[2] as usize];
    });
}

/// RGB to HSL — delegates to shared `effect_utils`.
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    effect_utils::rgb_to_hsl(r, g, b)
}

/// HSL to RGB — delegates to shared `effect_utils`.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    effect_utils::hsl_to_rgb(h, s, l)
}

/// Apply HSL (Hue/Saturation/Lightness) adjustment.
/// hue_shift: degrees to rotate hue (-180..+180).
/// saturation: 0.0=grayscale, 1.0=normal, >1.0=oversaturated.
/// lightness: -1.0=black, 0.0=normal, 1.0=white.
pub fn apply_hue_saturation(
    pixels: &mut [u8],
    _width: u32,
    _height: u32,
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
) {
    let shift_norm = hue_shift / 360.0;

    pixels.par_chunks_mut(4).for_each(|chunk| {
        if chunk.len() < 4 {
            return;
        }
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

        let (h, s, l) = rgb_to_hsl(r, g, b);
        let new_h = (h + shift_norm).fract();
        let new_h = if new_h < 0.0 { new_h + 1.0 } else { new_h };
        let new_s = (s * (1.0 + saturation)).clamp(0.0, 1.0);
        let new_l = (l + lightness).clamp(0.0, 1.0);

        let (rr, gg, bb) = hsl_to_rgb(new_h, new_s, new_l);
        chunk[0] = (rr.clamp(0.0, 1.0) * 255.0) as u8;
        chunk[1] = (gg.clamp(0.0, 1.0) * 255.0) as u8;
        chunk[2] = (bb.clamp(0.0, 1.0) * 255.0) as u8;
    });
}

/// Apply directional motion blur.
/// angle: blur direction in degrees (0=right, 90=down).
/// length: blur distance in pixels (0=no blur).
pub fn apply_motion_blur(pixels: &mut [u8], width: u32, height: u32, angle: f32, length: f32) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected || length < 1.0 {
        return;
    }
    let rad = angle.to_radians();
    let dir_x = rad.cos();
    let dir_y = rad.sin();
    let samples = (length * 2.0 + 1.0).ceil() as i32;
    let inv_samples = 1.0 / samples as f32;
    let mut tmp = vec![0u8; pixels.len()];

    // Directional blur is sample-heavy: parallelize per pixel with rayon.
    tmp.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let x = (i % width as usize) as f32;
            let y = (i / width as usize) as f32;
            let mut sum_r = 0f32;
            let mut sum_g = 0f32;
            let mut sum_b = 0f32;
            let mut sum_a = 0f32;

            for s in -samples / 2..=samples / 2 {
                let offset = s as f32;
                let sx = x + dir_x * offset;
                let sy = y + dir_y * offset;
                let ch = sample_bilinear(pixels, width, height, sx, sy);
                sum_r += ch[0] as u32 as f32;
                sum_g += ch[1] as u32 as f32;
                sum_b += ch[2] as u32 as f32;
                sum_a += ch[3] as u32 as f32;
            }

            out[0] = (sum_r * inv_samples) as u8;
            out[1] = (sum_g * inv_samples) as u8;
            out[2] = (sum_b * inv_samples) as u8;
            out[3] = (sum_a * inv_samples) as u8;
        });
    pixels[..tmp.len()].copy_from_slice(&tmp);
}

/// Apply 4-corner mesh warp displacement.
/// top_left/top_right/bottom_left/bottom_right: [x,y] displacement in pixels.
pub fn apply_mesh_warp(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    top_left: [f32; 2],
    top_right: [f32; 2],
    bottom_left: [f32; 2],
    bottom_right: [f32; 2],
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let mut tmp = vec![0u8; pixels.len()];
    let w = width as f32;
    let h = height as f32;

    // Bilinear homography warp is per-pixel independent — parallelize.
    tmp.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let out_x = i % width as usize;
            let out_y = i / width as usize;
            let u = out_x as f32 / w;
            let v = out_y as f32 / h;

            let tl_x = top_left[0] * w;
            let tl_y = top_left[1] * h;
            let tr_x = top_right[0] * w;
            let tr_y = top_right[1] * h;
            let bl_x = bottom_left[0] * w;
            let bl_y = bottom_left[1] * h;
            let br_x = bottom_right[0] * w;
            let br_y = bottom_right[1] * h;

            let src_x = tl_x * (1.0 - u) * (1.0 - v)
                + tr_x * u * (1.0 - v)
                + bl_x * (1.0 - u) * v
                + br_x * u * v;
            let src_y = tl_y * (1.0 - u) * (1.0 - v)
                + tr_y * u * (1.0 - v)
                + bl_y * (1.0 - u) * v
                + br_y * u * v;

            let ch = sample_bilinear(pixels, width, height, src_x, src_y);
            out.copy_from_slice(&ch);
        });
    pixels[..tmp.len()].copy_from_slice(&tmp);
}

/// Apply 3D LUT color grading.
/// lut_data: flattened 3D LUT as [r,g,b] triples.
/// lut_size: cube dimension (e.g. 17 for 17x17x17).
/// intensity: blend factor (0.0=bypass, 1.0=full LUT).
pub fn apply_color_grade_lut(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    lut_data: &[[f32; 3]],
    lut_size: usize,
    intensity: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected || lut_size < 2 || lut_data.len() < lut_size * lut_size * lut_size {
        return;
    }
    let ls = lut_size as f32 - 1.0;

    pixels.par_chunks_mut(4).for_each(|chunk| {
        if chunk.len() < 4 {
            return;
        }
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

        let fx = r * ls;
        let fy = g * ls;
        let fz = b * ls;

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let z0 = fz.floor() as usize;
        let x1 = (x0 + 1).min(lut_size - 1);
        let y1 = (y0 + 1).min(lut_size - 1);
        let z1 = (z0 + 1).min(lut_size - 1);
        let dx = fx - fx.floor();
        let dy = fy - fy.floor();
        let dz = fz - fz.floor();

        let idx3d = |xi: usize, yi: usize, zi: usize| -> usize {
            zi * lut_size * lut_size + yi * lut_size + xi
        };

        let fetch_lut =
            |xi: usize, yi: usize, zi: usize| -> [f32; 3] { lut_data[idx3d(xi, yi, zi)] };

        let c000 = fetch_lut(x0, y0, z0);
        let c100 = fetch_lut(x1, y0, z0);
        let c010 = fetch_lut(x0, y1, z0);
        let c110 = fetch_lut(x1, y1, z0);
        let c001 = fetch_lut(x0, y0, z1);
        let c101 = fetch_lut(x1, y0, z1);
        let c011 = fetch_lut(x0, y1, z1);
        let c111 = fetch_lut(x1, y1, z1);

        let mut result = [0.0f32; 3];
        for ch in 0..3 {
            let v = c000[ch] * (1.0 - dx) * (1.0 - dy) * (1.0 - dz)
                + c100[ch] * dx * (1.0 - dy) * (1.0 - dz)
                + c010[ch] * (1.0 - dx) * dy * (1.0 - dz)
                + c110[ch] * dx * dy * (1.0 - dz)
                + c001[ch] * (1.0 - dx) * (1.0 - dy) * dz
                + c101[ch] * dx * (1.0 - dy) * dz
                + c011[ch] * (1.0 - dx) * dy * dz
                + c111[ch] * dx * dy * dz;
            result[ch] = v;
        }

        for ch in 0..3 {
            let orig = chunk[ch] as f32 / 255.0;
            let graded = result[ch];
            let blended = orig + (graded - orig) * intensity;
            chunk[ch] = (blended.clamp(0.0, 1.0) * 255.0) as u8;
        }
    });
}

/// Apply color space conversion.
/// mode: 0=sRGB→Linear, 1=Linear→sRGB, 2=Grayscale.
pub fn apply_color_space_convert(pixels: &mut [u8], _width: u32, _height: u32, mode: u32) {
    match mode {
        0 => {
            pixels.par_chunks_mut(4).for_each(|chunk| {
                if chunk.len() < 4 {
                    return;
                }
                for ch in chunk.iter_mut().take(3) {
                    let v = *ch as f32 / 255.0;
                    let linear = if v <= 0.04045 {
                        v / 12.92
                    } else {
                        ((v + 0.055) / 1.055).powf(2.2)
                    };
                    *ch = (linear.clamp(0.0, 1.0) * 255.0) as u8;
                }
            });
        }
        1 => {
            pixels.par_chunks_mut(4).for_each(|chunk| {
                if chunk.len() < 4 {
                    return;
                }
                for ch in chunk.iter_mut().take(3) {
                    let v = *ch as f32 / 255.0;
                    let srgb = if v <= 0.0031308 {
                        v * 12.92
                    } else {
                        1.055 * v.powf(1.0 / 2.2) - 0.055
                    };
                    *ch = (srgb.clamp(0.0, 1.0) * 255.0) as u8;
                }
            });
        }
        2 => {
            pixels.par_chunks_mut(4).for_each(|chunk| {
                if chunk.len() < 4 {
                    return;
                }
                let r = chunk[0] as f32 / 255.0;
                let g = chunk[1] as f32 / 255.0;
                let b = chunk[2] as f32 / 255.0;
                let (_h, _s, l) = rgb_to_hsl(r, g, b);
                let v = (l * 255.0) as u8;
                chunk[0] = v;
                chunk[1] = v;
                chunk[2] = v;
            });
        }
        _ => {}
    }
}

fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747796405).wrapping_add(2891336453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277803737);
    (word >> 22) ^ word
}

/// Apply film grain noise.
/// intensity: grain strength (0.0=none, 1.0=heavy).
/// grain_size: block size in pixels (1=fine, 5=coarse).
/// seed: random seed (use frame number for temporal variation).
pub fn apply_film_grain(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    intensity: f32,
    grain_size: u32,
    seed: u32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected || grain_size == 0 {
        return;
    }
    let gs = grain_size.max(1);
    let blocks_w = width.div_ceil(gs);
    let blocks_h = height.div_ceil(gs);
    let mut noise_map = vec![0i16; (blocks_w * blocks_h) as usize];

    for by in 0..blocks_h {
        for bx in 0..blocks_w {
            let hash = pcg_hash(seed.wrapping_add(by * blocks_w + bx));
            let val = ((hash as i32) % 512) - 256;
            let idx = (by * blocks_w + bx) as usize;
            if idx < noise_map.len() {
                noise_map[idx] = val as i16;
            }
        }
    }

    pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        if chunk.len() < 4 {
            return;
        }
        let px = (i as u32) % width;
        let py = (i as u32) / width;
        let bx = (px / gs) as usize;
        let by = (py / gs) as usize;
        let block_w = width.div_ceil(gs) as usize;
        let idx = by * block_w + bx;
        if idx >= noise_map.len() {
            return;
        }
        let noise = noise_map[idx] as f32 * intensity;
        for ch in chunk.iter_mut().take(3) {
            let v = *ch as f32 + noise;
            *ch = v.clamp(0.0, 255.0) as u8;
        }
    });
}

// ──────────────────────────── FractalNoise ─────────────────────────────

/// Simple 2D value noise hash
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let mut n = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263))
        .wrapping_add(seed);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n as f32) / (u32::MAX as f32)
}

/// Smooth interpolation
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 2D interpolated value noise in [0,1]
fn noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = smooth(x - xi);
    let yf = smooth(y - yi);
    let n00 = value_noise(xi, yi, seed);
    let n10 = value_noise(xi + 1.0, yi, seed);
    let n01 = value_noise(xi, yi + 1.0, seed);
    let n11 = value_noise(xi + 1.0, yi + 1.0, seed);
    let nx0 = n00 + (n10 - n00) * xf;
    let nx1 = n01 + (n11 - n01) * xf;
    nx0 + (nx1 - nx0) * yf
}

/// Fractional Brownian Motion (fBm) - multiple octaves of noise
fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 0.5f32;
    let mut frequency = 1.0f32;
    for o in 0..octaves {
        value += noise_2d(x * frequency, y * frequency, seed.wrapping_add(o * 1000)) * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

/// Apply FractalNoise - generates procedural noise texture
/// fractal_type: 0=Basic fBm, 1=Turbulence (abs of each octave), 2=Dynamic (turbulence), 3=Crane (ridged)
#[allow(clippy::too_many_arguments)]
pub fn apply_fractal_noise(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    fractal_type: f32,
    contrast: f32,
    brightness: f32,
    complexity: f32,
    evolution: f32,
) {
    let Some(expected) = rgba_buffer_size(width, height) else {
        return;
    };
    if pixels.len() < expected {
        return;
    }
    let w = width as f32;
    let h = height as f32;
    let octaves = (complexity.clamp(1.0, 10.0)) as u32;
    let ft = fractal_type as u32;
    let scale = 3.0; // base frequency scale
    let evo = evolution * 0.1; // slow down evolution
    let seed = (evo * 1000.0) as u32;

    for py in 0..height {
        for px in 0..width {
            let nx = px as f32 / w * scale;
            let ny = py as f32 / h * scale;
            let mut n = 0.0f32;

            match ft {
                0 => {
                    // Basic fBm
                    n = fbm(nx + evo, ny + evo, octaves, seed);
                }
                1 => {
                    // Turbulence: abs of each octave
                    let mut amp = 0.5f32;
                    let mut freq = 1.0f32;
                    for o in 0..octaves {
                        n += noise_2d(
                            (nx + evo) * freq,
                            (ny + evo) * freq,
                            seed.wrapping_add(o * 1000),
                        )
                        .abs()
                            * amp;
                        amp *= 0.5;
                        freq *= 2.0;
                    }
                    n = n.clamp(0.0, 1.0);
                }
                2 => {
                    // Dynamic (evolving turbulence)
                    let mut amp = 0.5f32;
                    let mut freq = 1.0f32;
                    for o in 0..octaves {
                        let ev = evo * (o as f32 + 1.0);
                        n += noise_2d(
                            (nx + ev) * freq,
                            (ny + ev * 0.7) * freq,
                            seed.wrapping_add(o * 1000),
                        )
                        .abs()
                            * amp;
                        amp *= 0.5;
                        freq *= 2.0;
                    }
                    n = n.clamp(0.0, 1.0);
                }
                3 => {
                    // Crane (ridged noise)
                    let mut amp = 0.5f32;
                    let mut freq = 1.0f32;
                    for o in 0..octaves {
                        let val = noise_2d(
                            (nx + evo) * freq,
                            (ny + evo) * freq,
                            seed.wrapping_add(o * 1000),
                        );
                        n += (1.0 - (val * 2.0 - 1.0).abs()).powf(2.0) * amp;
                        amp *= 0.5;
                        freq *= 2.0;
                    }
                    n = n.clamp(0.0, 1.0);
                }
                _ => {
                    n = fbm(nx + evo, ny + evo, octaves, seed);
                }
            }

            // Apply contrast and brightness
            n = ((n - 0.5) * contrast + 0.5 + brightness - 0.5).clamp(0.0, 1.0);
            let gray = (n * 255.0) as u8;
            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 < pixels.len() {
                pixels[idx] = gray;
                pixels[idx + 1] = gray;
                pixels[idx + 2] = gray;
                pixels[idx + 3] = 255;
            }
        }
    }
}

// ──────────────────────────── Curves ─────────────────────────────

/// Simple 5-point bezier curve for Curves effect
/// Control points: 0%, 25%, 50%, 75%, 100%
/// channel: 0=Master, 1=Red, 2=Green, 3=Blue
/// Apply per-channel bezier spline tone curves.
/// channel: 0=Master, 1=Red, 2=Green, 3=Blue.
/// Uses 5-point Catmull-Rom spline for smooth S-curve.
pub fn apply_curves(pixels: &mut [u8], _width: u32, _height: u32, channel: f32) {
    let ch = channel as u32;
    let ch_idx = ch as usize;

    // Default S-curve control points (master)
    let points = [
        0.0f32, // 0%
        0.2,    // 25%
        0.5,    // 50%
        0.8,    // 75%
        1.0,    // 100%
    ];

    // Build lookup table with bezier interpolation
    let mut lut = [0u8; 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        let t = i as f32 / 255.0;
        // Simple catmull-rom through the 5 points
        let val = catmull_rom_spline(&points, t);
        *entry = (val.clamp(0.0, 1.0) * 255.0) as u8;
    }

    for px in pixels.chunks_exact_mut(4) {
        if ch == 0 {
            // Master: apply to all channels
            px[0] = lut[px[0] as usize];
            px[1] = lut[px[1] as usize];
            px[2] = lut[px[2] as usize];
        } else if ch_idx <= 3 {
            px[ch_idx - 1] = lut[px[ch_idx - 1] as usize];
        }
    }
}

fn catmull_rom_spline(points: &[f32], t: f32) -> f32 {
    let n = points.len() - 1;
    let x = t * n as f32;
    let i = (x.floor() as usize).min(n - 1);
    let frac = x - i as f32;
    let p0 = points[i.saturating_sub(1)];
    let p1 = points[i];
    let p2 = points[(i + 1).min(n)];
    let p3 = points[(i + 2).min(n)];
    // Catmull-Rom
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * frac
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * frac * frac
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * frac * frac * frac)
}

// ──────────────────────────── DisplacementMap ─────────────────────────────

/// Displace pixels using a source layer as the displacement map.
/// This is a placeholder - in production, source_layer would be a reference to another layer's rendered pixels.
/// Here we apply a simple horizontal/vertical displacement based on luminance.
/// Apply displacement map warp.
/// source_layer: layer ID used as displacement source (placeholder).
/// max_horizontal/max_vertical: max displacement in pixels.
pub fn apply_displacement_map(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    _source_layer: f32,
    max_horizontal: f32,
    max_vertical: f32,
) {
    // For now, use a procedural displacement pattern
    let orig = pixels.to_vec();
    let w = width as f32;
    let h = height as f32;

    for py in 0..height {
        for px in 0..width {
            let idx = ((py * width + px) * 4) as usize;
            if idx + 3 >= orig.len() {
                continue;
            }

            // Generate a simple displacement pattern (sine wave)
            let disp_x = ((px as f32 / w * std::f32::consts::TAU
                + py as f32 / h * std::f32::consts::PI)
                .sin()
                * 0.5
                + 0.5)
                * max_horizontal;
            let disp_y = ((py as f32 / h * std::f32::consts::TAU
                + px as f32 / w * std::f32::consts::PI)
                .cos()
                * 0.5
                + 0.5)
                * max_vertical;

            let src_x = (px as f32 - disp_x).round() as i32;
            let src_y = (py as f32 - disp_y).round() as i32;

            if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                let sidx = ((src_y as u32 * width + src_x as u32) * 4) as usize;
                if sidx + 3 < orig.len() {
                    pixels[idx] = orig[sidx];
                    pixels[idx + 1] = orig[sidx + 1];
                    pixels[idx + 2] = orig[sidx + 2];
                    pixels[idx + 3] = orig[sidx + 3];
                }
            }
        }
    }
}

// ──────────────────────────── CompoundBlur ─────────────────────────────

/// Apply variable blur using a luminance-based intensity map.
/// Bright areas in the source layer = more blur.
/// Apply variable compound blur using luminance as intensity map.
/// source_layer: layer ID for blur map (placeholder).
/// max_blur: maximum blur radius in pixels.
pub fn apply_compound_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    _source_layer: f32,
    max_blur: f32,
) {
    if max_blur < 0.5 {
        return;
    }
    let radius = max_blur.round() as u32;
    let orig = pixels.to_vec();

    // Parallel per-row processing
    pixels
        .par_chunks_mut((width * 4) as usize)
        .enumerate()
        .for_each(|(row, row_chunk)| {
            let py = row as u32;
            for px in 0..width {
                let idx = (px * 4) as usize;
                if idx + 3 >= row_chunk.len() {
                    continue;
                }

                // Use pixel brightness as blur intensity
                let lum = (orig[((py * width + px) * 4) as usize] as f32
                    + orig[((py * width + px) * 4 + 1) as usize] as f32
                    + orig[((py * width + px) * 4 + 2) as usize] as f32)
                    / 765.0;
                let r = (lum * radius as f32).round() as u32;
                if r < 1 {
                    continue;
                }

                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut count = 0.0f32;

                for dy in -(r as i32)..=r as i32 {
                    for dx in -(r as i32)..=r as i32 {
                        if dx * dx + dy * dy > (r as i32 * r as i32) {
                            continue;
                        }
                        let sx = (px as i32 + dx).max(0).min(width as i32 - 1) as u32;
                        let sy = (py as i32 + dy).max(0).min(height as i32 - 1) as u32;
                        let sidx = ((sy * width + sx) * 4) as usize;
                        if sidx + 3 < orig.len() {
                            sum_r += orig[sidx] as f32;
                            sum_g += orig[sidx + 1] as f32;
                            sum_b += orig[sidx + 2] as f32;
                            count += 1.0;
                        }
                    }
                }
                if count > 0.0 {
                    row_chunk[idx] = (sum_r / count) as u8;
                    row_chunk[idx + 1] = (sum_g / count) as u8;
                    row_chunk[idx + 2] = (sum_b / count) as u8;
                }
            }
        });
}

// ──────────────────────────── Minimax ─────────────────────────────

/// Minimax - dilate (max) or erode (min) based on luminance.
/// operation: 0=Min (erode), 1=Max (dilate)
/// Apply minimax (dilate/erode) matte operation.
/// operation: 0=Min (erode dark areas), 1=Max (dilate bright areas).
/// radius: operation radius in pixels.
pub fn apply_minimax(pixels: &mut [u8], width: u32, height: u32, operation: f32, radius: f32) {
    let r = radius.round() as u32;
    if r < 1 {
        return;
    }
    let orig = pixels.to_vec();
    let use_max = operation > 0.5;

    pixels
        .par_chunks_mut((width * 4) as usize)
        .enumerate()
        .for_each(|(row, row_chunk)| {
            let py = row as u32;
            for px in 0..width {
                let idx = (px * 4) as usize;
                if idx + 3 >= row_chunk.len() {
                    continue;
                }

                let mut best = orig[((py * width + px) * 4) as usize] as f32;
                for dy in -(r as i32)..=r as i32 {
                    for dx in -(r as i32)..=r as i32 {
                        if dx * dx + dy * dy > (r as i32 * r as i32) {
                            continue;
                        }
                        let sx = (px as i32 + dx).max(0).min(width as i32 - 1) as u32;
                        let sy = (py as i32 + dy).max(0).min(height as i32 - 1) as u32;
                        let sidx = ((sy * width + sx) * 4) as usize;
                        if sidx + 3 < orig.len() {
                            let lum =
                                (orig[sidx] as f32 + orig[sidx + 1] as f32 + orig[sidx + 2] as f32)
                                    / 3.0;
                            if use_max {
                                best = best.max(lum);
                            } else {
                                best = best.min(lum);
                            }
                        }
                    }
                }
                let v = best as u8;
                row_chunk[idx] = v;
                row_chunk[idx + 1] = v;
                row_chunk[idx + 2] = v;
                // Keep alpha
            }
        });
}

// ──────────────────────────── ShiftChannels ─────────────────────────────

/// Shift RGBA channels - remap each channel to a different source.
/// take_red/green/blue/alpha: 0=Red, 1=Green, 2=Blue, 3=Alpha, 4=Off(0), 5=On(1)
/// Shift RGBA channels - remap each channel to a different source.
/// take_red/green/blue/alpha: 0=Red, 1=Green, 2=Blue, 3=Alpha, 4=Off(0), 5=On(1).
pub fn apply_shift_channels(
    pixels: &mut [u8],
    _width: u32,
    _height: u32,
    take_red: f32,
    take_green: f32,
    take_blue: f32,
    take_alpha: f32,
) {
    let src_r = take_red as usize;
    let src_g = take_green as usize;
    let src_b = take_blue as usize;
    let src_a = take_alpha as usize;

    for px in pixels.chunks_exact_mut(4) {
        let orig = [px[0], px[1], px[2], px[3]];
        px[0] = match src_r {
            0..=3 => orig[src_r],
            4 => 0,
            5 => 255,
            _ => orig[0],
        };
        px[1] = match src_g {
            0..=3 => orig[src_g],
            4 => 0,
            5 => 255,
            _ => orig[1],
        };
        px[2] = match src_b {
            0..=3 => orig[src_b],
            4 => 0,
            5 => 255,
            _ => orig[2],
        };
        px[3] = match src_a {
            0..=3 => orig[src_a],
            4 => 0,
            5 => 255,
            _ => orig[3],
        };
    }
}

#[cfg(test)]
mod ca_radial_tests {
    use super::*;

    #[test]
    fn test_ca_center_pixel_unchanged() {
        // At the exact center, falloff direction is zero → no shift anywhere
        let w = 33u32;
        let h = 33u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        for i in (0..px.len()).step_by(4) {
            px[i] = 200;
            px[i + 1] = 100;
            px[i + 2] = 50;
            px[i + 3] = 255;
        }
        let before = px.clone();
        apply_chromatic_aberration(&mut px, w, h, 20.0, 20.0, 0.0);
        let center = ((16 * w + 16) * 4) as usize;
        assert_eq!(px[center], before[center], "center R unchanged");
        assert_eq!(px[center + 1], before[center + 1]);
    }

    #[test]
    fn test_ca_radial_fringes_left_and_right_symmetrically() {
        // A white block on black: radial fringing must push red outward on BOTH
        // sides of the block (left side of block gains red on its left, right
        // side loses red beyond its right edge).
        let w = 64u32;
        let h = 32u32;
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 12..20 {
            for x in 16..48 {
                let idx = ((y * w + x) * 4) as usize;
                px[idx] = 255;
                px[idx + 1] = 255;
                px[idx + 2] = 255;
                px[idx + 3] = 255;
            }
        }
        apply_chromatic_aberration(&mut px, w, h, 6.0, 6.0, 0.0);

        // Probe row through the block's vertical center.
        // Radial CA creates mirrored channel separation around the center:
        // whichever way each side fringes, the pattern must be point-symmetric.
        let row = 15usize;
        let separation_at = |x: usize| -> i32 {
            let idx = (row * w as usize + x) * 4;
            px[idx] as i32 - px[idx + 2] as i32 // R - B
        };
        // Sum |separation| near both edges of the block
        let sep_left: i32 = (14..20).map(|x| separation_at(x).abs()).sum();
        let sep_right: i32 = (44..50).map(|x| separation_at(x).abs()).sum();
        assert!(
            sep_left > 30,
            "channel separation expected at left edge, got {}",
            sep_left
        );
        assert!(
            sep_right > 30,
            "channel separation expected at right edge, got {}",
            sep_right
        );
    }
}
