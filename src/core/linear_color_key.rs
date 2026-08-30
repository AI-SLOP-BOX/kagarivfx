//! Linear Color Key Effect Engine (AE Parity - Keying > Linear Color Key).
//!
//! Keys out a specific color or range of colors with precise Euclidean / Delta-E distance,
//! tolerance window, and softness ramp in RGB, Chroma, or Hue color spaces.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ColorMatchMode {
    #[default]
    UsingRGB,
    UsingHue,
    UsingChroma,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinearColorKeyParams {
    pub key_color: [f32; 3], // RGB in 0.0..1.0
    pub match_mode: ColorMatchMode,
    pub tolerance: f32, // 0.0..100.0%
    pub softness: f32,  // 0.0..100.0%
}

impl Default for LinearColorKeyParams {
    fn default() -> Self {
        Self {
            key_color: [0.0, 1.0, 0.0], // Default green screen
            match_mode: ColorMatchMode::UsingRGB,
            tolerance: 15.0,
            softness: 10.0,
        }
    }
}

/// Applies Linear Color Key to an RGBA pixel buffer.
pub fn apply_linear_color_key(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    params: &LinearColorKeyParams,
) {
    let Some(pixel_count) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return;
    };
    if width == 0 || height == 0 || pixels.len() != pixel_count * 4 {
        return;
    }

    let safe_color = |value: f32| {
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let kr = safe_color(params.key_color[0]);
    let kg = safe_color(params.key_color[1]);
    let kb = safe_color(params.key_color[2]);

    let tol = if params.tolerance.is_finite() {
        (params.tolerance / 100.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let soft = if params.softness.is_finite() {
        (params.softness / 100.0).clamp(0.001, 1.0)
    } else {
        0.001
    };

    let rgb_to_hsl = |r: f32, g: f32, b: f32| -> (f32, f32, f32) {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let l = (max + min) * 0.5;
        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        let h = if delta == 0.0 {
            0.0
        } else if (max - r).abs() < 1e-5 {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if (max - g).abs() < 1e-5 {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };

        (h, s, l)
    };

    let (kh, ks, _) = rgb_to_hsl(kr, kg, kb);

    for chunk in pixels.chunks_exact_mut(4) {
        let pr = chunk[0] as f32 / 255.0;
        let pg = chunk[1] as f32 / 255.0;
        let pb = chunk[2] as f32 / 255.0;
        let pa = chunk[3] as f32 / 255.0;

        let dist = match params.match_mode {
            ColorMatchMode::UsingRGB => {
                let dr = pr - kr;
                let dg = pg - kg;
                let db = pb - kb;
                (dr * dr + dg * dg + db * db).sqrt() / 1.73205 // Normalize sqrt(3) -> 1.0
            }
            ColorMatchMode::UsingHue => {
                let (ph, _, _) = rgb_to_hsl(pr, pg, pb);
                let mut dh = (ph - kh).abs();
                if dh > 0.5 {
                    dh = 1.0 - dh;
                }
                dh * 2.0
            }
            ColorMatchMode::UsingChroma => {
                let (ph, ps, _) = rgb_to_hsl(pr, pg, pb);
                let x1 = ks * (kh * std::f32::consts::TAU).cos();
                let y1 = ks * (kh * std::f32::consts::TAU).sin();
                let x2 = ps * (ph * std::f32::consts::TAU).cos();
                let y2 = ps * (ph * std::f32::consts::TAU).sin();
                let dx = x2 - x1;
                let dy = y2 - y1;
                (dx * dx + dy * dy).sqrt() / 1.4142
            }
        };

        let matte = if dist <= tol {
            0.0
        } else if dist >= tol + soft {
            1.0
        } else {
            (dist - tol) / soft
        };

        chunk[3] = ((pa * matte).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_color_key_green_screen() {
        let mut pixels = vec![
            0, 255, 0, 255, // Pure green -> should become transparent
            255, 0, 0, 255, // Pure red -> should stay opaque
        ];

        let params = LinearColorKeyParams {
            key_color: [0.0, 1.0, 0.0],
            match_mode: ColorMatchMode::UsingRGB,
            tolerance: 10.0,
            softness: 5.0,
        };

        apply_linear_color_key(&mut pixels, 2, 1, &params);
        assert_eq!(pixels[3], 0); // Green keyed out
        assert_eq!(pixels[7], 255); // Red kept
    }

    #[test]
    fn test_linear_color_key_rejects_invalid_buffer_and_parameters() {
        let original = vec![0, 255, 0, 255];
        let mut pixels = original.clone();
        apply_linear_color_key(
            &mut pixels,
            u32::MAX,
            u32::MAX,
            &LinearColorKeyParams {
                key_color: [f32::NAN, f32::INFINITY, -f32::INFINITY],
                tolerance: f32::NAN,
                softness: f32::INFINITY,
                ..Default::default()
            },
        );
        assert_eq!(pixels, original);
    }
}
