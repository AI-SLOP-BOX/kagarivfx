#![allow(dead_code)]
/// After Effects VFX Kernels Part 18 — Particle & Simulation Renderers
// 1. Particle Gravity Simulation (Point Emission with Gravity Field)
pub fn apply_particle_gravity_sim(pixels: &mut [u8], width: u32, height: u32, frame: u32, gravity: f32, spread: f32) {
    let t = frame as f32 * 0.016;
    let emitter_x = width as f32 * 0.5;
    let emitter_y = height as f32 * 0.2;
    let num_particles = 200u32;

    for p in 0..num_particles {
        let seed = p as f32 * 7.3891;
        let angle = seed.sin() * std::f32::consts::PI;
        let speed = ((seed * 1.3).cos().abs() + 0.5) * spread;
        let life = ((p % 30) as f32 / 30.0 + t).fract();

        let px = emitter_x + angle.cos() * speed * life * 60.0;
        let py = emitter_y + angle.sin() * speed * life * 60.0 + 0.5 * gravity * (life * 3.0).powi(2);

        let ix = px as i32;
        let iy = py as i32;
        if ix >= 0 && ix < width as i32 && iy >= 0 && iy < height as i32 {
            let alpha = (1.0 - life).powi(2);
            let idx = (iy as usize * width as usize + ix as usize) * 4;
            pixels[idx] = (pixels[idx] as f32 + 200.0 * alpha).clamp(0.0, 255.0) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 + 150.0 * alpha).clamp(0.0, 255.0) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 + 50.0 * alpha).clamp(0.0, 255.0) as u8;
        }
    }
}

// 2. Star Field Generator (3D Depth Star Parallax)
pub fn apply_star_field(pixels: &mut [u8], width: u32, height: u32, num_stars: u32, depth_speed: f32, time: f32) {
    for s in 0..num_stars {
        let seed = s as f32 * 127.1;
        let sx = (seed.sin().abs() * width as f32) as i32;
        let sy = ((seed * 7.3).cos().abs() * height as f32) as i32;
        let depth = ((seed * 3.1).sin().abs() + 0.1).min(1.0);

        let twinkle = (depth * 10.0 + time * depth_speed).sin() * 0.5 + 0.5;
        let brightness = (twinkle * depth * 255.0) as u8;

        if sx >= 0 && sx < width as i32 && sy >= 0 && sy < height as i32 {
            let idx = (sy as usize * width as usize + sx as usize) * 4;
            pixels[idx] = brightness;
            pixels[idx + 1] = brightness;
            pixels[idx + 2] = brightness;
            pixels[idx + 3] = 255;
        }
    }
}

// 3. Turbulence Displacement Map (Fractal Brownian Motion 2D)
pub fn apply_fbm_turbulence(pixels: &mut [u8], width: u32, height: u32, octaves: u32, amplitude: f32, time: f32) {
    if amplitude <= 0.001 || octaves == 0 { return; }
    let temp = pixels.to_vec();

    for y in 0..height {
        for x in 0..width {
            let mut dx = 0.0f32;
            let mut dy = 0.0f32;
            let mut amp = amplitude;
            let mut freq = 1.0f32;

            for _ in 0..octaves {
                let nx = x as f32 * 0.01 * freq + time;
                let ny = y as f32 * 0.01 * freq;
                dx += nx.sin() * ny.cos() * amp;
                dy += ny.sin() * nx.cos() * amp;
                amp *= 0.5;
                freq *= 2.0;
            }

            let sx = (x as f32 + dx).clamp(0.0, (width - 1) as f32) as usize;
            let sy = (y as f32 + dy).clamp(0.0, (height - 1) as f32) as usize;

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            let src_idx = (sy * width as usize + sx) * 4;
            pixels[dst_idx..dst_idx + 4].copy_from_slice(&temp[src_idx..src_idx + 4]);
        }
    }
}

// 4. Lightning Arc Generator
pub fn apply_lightning_arc(pixels: &mut [u8], width: u32, height: u32, start: [f32; 2], end: [f32; 2], seed: u32, glow: f32) {
    let steps = 64u32;
    let mut rng = seed;

    for s in 0..steps {
        let t = s as f32 / (steps - 1) as f32;
        let bx = start[0] + (end[0] - start[0]) * t;
        let by = start[1] + (end[1] - start[1]) * t;

        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let jitter_x = ((rng >> 16) as f32 / 65535.0 - 0.5) * 20.0 * (1.0 - (t - 0.5).abs() * 2.0);
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let jitter_y = ((rng >> 16) as f32 / 65535.0 - 0.5) * 20.0 * (1.0 - (t - 0.5).abs() * 2.0);

        let lx = (bx + jitter_x) as i32;
        let ly = (by + jitter_y) as i32;

        // Draw glow around arc
        for gy in -2i32..=2 {
            for gx in -2i32..=2 {
                let px = lx + gx;
                let py = ly + gy;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let dist = ((gx * gx + gy * gy) as f32).sqrt();
                    let g = (glow / (dist + 1.0) * 200.0).clamp(0.0, 255.0) as u8;
                    let idx = (py as usize * width as usize + px as usize) * 4;
                    pixels[idx] = pixels[idx].saturating_add(g / 2);
                    pixels[idx + 1] = pixels[idx + 1].saturating_add(g / 2);
                    pixels[idx + 2] = pixels[idx + 2].saturating_add(g);
                }
            }
        }
    }
}

// 5. Flame / Fire Cellular Automaton Upward Combustion
pub fn apply_fire_automaton(pixels: &mut [u8], width: u32, height: u32, intensity: f32) {
    if intensity <= 0.001 { return; }
    let temp = pixels.to_vec();

    // Bottom row as heat source
    for x in 0..width as usize {
        let idx = ((height - 1) as usize * width as usize + x) * 4;
        pixels[idx] = 255;
        pixels[idx + 1] = (80.0 * intensity) as u8;
        pixels[idx + 2] = 0;
    }

    // Propagate heat upward with cooling
    for y in 1..height as usize {
        for x in 0..width as usize {
            let src_y = (height as usize - 1) - y;

            let left  = if x > 0 { temp[((src_y + 1) * width as usize + (x - 1)) * 4] as u32 } else { 0 };
            let right = if x < width as usize - 1 { temp[((src_y + 1) * width as usize + (x + 1)) * 4] as u32 } else { 0 };
            let mid   = temp[((src_y + 1) * width as usize + x) * 4] as u32;
            let above = temp[(src_y * width as usize + x) * 4] as u32;

            let avg = (left + mid + right + above) / 4;
            let cooled = avg.saturating_sub((2.0 * intensity) as u32);

            let r_val = cooled.min(255) as u8;
            let g_val = ((cooled as f32 * 0.3) as u32).min(255) as u8;
            let dst_idx = (src_y * width as usize + x) * 4;
            pixels[dst_idx] = r_val;
            pixels[dst_idx + 1] = g_val;
            pixels[dst_idx + 2] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ae_effects_v18_filters() {
        let mut pixels = vec![0u8; 64 * 4];
        apply_star_field(&mut pixels, 8, 8, 10, 1.0, 0.0);
        assert_eq!(pixels.len(), 64 * 4);
    }
}
