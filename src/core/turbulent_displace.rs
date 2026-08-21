#![allow(dead_code)]
/// Turbulent Displace displacement types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurbulentDisplaceType {
    Turbulent,
    Bulge,
    Twist,
}

/// Turbulent Displace options matching After Effects Turbulent Displace effect.
#[derive(Debug, Clone)]
pub struct TurbulentDisplaceOptions {
    pub displace_type: TurbulentDisplaceType,
    pub amount: f32,       // Max pixel displacement offset
    pub size: f32,         // Noise fractal size scale
    pub evolution_deg: f32,// Evolution angle animation
    pub complexity: u32,   // Octaves of fractal noise
}

impl Default for TurbulentDisplaceOptions {
    fn default() -> Self {
        Self {
            displace_type: TurbulentDisplaceType::Turbulent,
            amount: 25.0,
            size: 100.0,
            evolution_deg: 0.0,
            complexity: 1,
        }
    }
}

// 256 Permutation table for Perlin Noise
const PERMUTATION: [u8; 256] = [
    151,160,137,91,90,15,131,13,201,95,96,53,194,233,7,225,140,36,103,30,69,142,
    8,99,37,240,21,10,23,190,6,148,247,120,234,75,0,26,197,62,94,252,219,203,117,
    35,11,32,57,177,33,88,237,149,56,87,174,20,125,136,171,168,68,175,74,165,71,
    134,139,48,27,166,77,146,158,231,83,111,229,122,60,211,133,230,220,105,92,41,
    55,46,245,40,244,102,143,54,65,25,63,161,1,216,80,73,209,76,132,187,208,89,
    18,169,200,196,135,130,116,188,159,86,164,100,109,198,173,186,3,64,52,217,226,
    250,124,123,5,202,38,147,118,126,255,82,85,212,207,206,59,227,47,16,58,17,182,
    189,28,42,223,183,170,213,119,248,152,2,44,154,163,70,221,153,101,155,167,43,
    172,9,129,22,39,253,19,98,108,110,79,113,224,232,178,185,112,104,218,246,97,
    228,251,34,242,193,238,210,144,12,191,179,162,241,81,51,145,235,249,14,239,
    107,49,192,214,31,181,199,106,157,184,84,204,176,115,121,50,45,127,4,150,254,
    138,236,205,93,222,114,67,29,24,72,243,141,128,195,78,66,215,61,156,180
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

fn grad(hash: u8, x: f64, y: f64) -> f64 {
    let h = hash & 7;
    let u = if h < 4 { x } else { y };
    let v = if h < 4 { y } else { x };
    (if (h & 1) == 0 { u } else { -u }) + (if (h & 2) == 0 { v } else { -v })
}

/// Standard 2D Perlin Gradient Noise evaluation
fn perlin_noise_2d(x: f64, y: f64) -> f64 {
    let xi = (x.floor() as i32 & 255) as usize;
    let yi = (y.floor() as i32 & 255) as usize;

    let xf = x - x.floor();
    let yf = y - y.floor();

    let u = fade(xf);
    let v = fade(yf);

    let p = &PERMUTATION;
    let aa = p[(p[xi] as usize + yi) & 255];
    let ab = p[(p[xi] as usize + yi + 1) & 255];
    let ba = p[(p[(xi + 1) & 255] as usize + yi) & 255];
    let bb = p[(p[(xi + 1) & 255] as usize + yi + 1) & 255];

    let x1 = lerp(u, grad(aa, xf, yf), grad(ba, xf - 1.0, yf));
    let x2 = lerp(u, grad(ab, xf, yf - 1.0), grad(bb, xf - 1.0, yf - 1.0));

    lerp(v, x1, x2)
}

/// Computes multi-octave 2D Perlin Fractal Noise vector for displacement.
fn compute_turbulent_offset(x: f32, y: f32, options: &TurbulentDisplaceOptions) -> (f32, f32) {
    let scale = (options.size.max(1.0) as f64) * 0.005;
    let evol_rad = (options.evolution_deg as f64).to_radians();

    let mut dx = 0.0f64;
    let mut dy = 0.0f64;
    let mut amp = 1.0f64;
    let mut freq = 1.0f64;

    let max_octaves = options.complexity.clamp(1, 6) as usize;

    for _ in 0..max_octaves {
        let nx = perlin_noise_2d(x as f64 * scale * freq + evol_rad.cos(), y as f64 * scale * freq);
        let ny = perlin_noise_2d(x as f64 * scale * freq + 100.0, y as f64 * scale * freq + evol_rad.sin() + 100.0);

        dx += nx * amp;
        dy += ny * amp;

        amp *= 0.5;
        freq *= 2.0;
    }

    ( (dx * options.amount as f64) as f32, (dy * options.amount as f64) as f32 )
}

/// Applies real-time Perlin Fractal Noise Turbulent Displacement to RGBA buffer.
pub fn apply_turbulent_displace(
    pixels: &[u8],
    width: u32,
    height: u32,
    options: &TurbulentDisplaceOptions,
) -> Vec<u8> {
    let num_pixels = (width * height) as usize;
    if pixels.len() != num_pixels * 4 || options.amount.abs() < 0.001 {
        return pixels.to_vec();
    }

    let mut out_pixels = vec![0u8; num_pixels * 4];
    let w_f32 = width as f32;
    let h_f32 = height as f32;

    for y in 0..height {
        for x in 0..width {
            let (dx, dy) = compute_turbulent_offset(x as f32, y as f32, options);

            let src_x = (x as f32 + dx).clamp(0.0, w_f32 - 1.0);
            let src_y = (y as f32 + dy).clamp(0.0, h_f32 - 1.0);

            let x0 = src_x.floor() as u32;
            let y0 = src_y.floor() as u32;
            let x1 = (x0 + 1).min(width - 1);
            let y1 = (y0 + 1).min(height - 1);

            let tx = src_x - x0 as f32;
            let ty = src_y - y0 as f32;

            let idx00 = ((y0 * width + x0) * 4) as usize;
            let idx10 = ((y0 * width + x1) * 4) as usize;
            let idx01 = ((y1 * width + x0) * 4) as usize;
            let idx11 = ((y1 * width + x1) * 4) as usize;
            let out_idx = ((y * width + x) * 4) as usize;

            for c in 0..4 {
                let p00 = pixels[idx00 + c] as f32;
                let p10 = pixels[idx10 + c] as f32;
                let p01 = pixels[idx01 + c] as f32;
                let p11 = pixels[idx11 + c] as f32;

                let top = p00 + (p10 - p00) * tx;
                let bottom = p01 + (p11 - p01) * tx;
                let val = top + (bottom - top) * ty;

                out_pixels[out_idx + c] = val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    out_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin_turbulent_displace_buffer_size() {
        let pixels = vec![255u8; 64]; // 4x4 RGBA
        let options = TurbulentDisplaceOptions::default();
        let out = apply_turbulent_displace(&pixels, 4, 4, &options);
        assert_eq!(out.len(), 64);
    }
}
