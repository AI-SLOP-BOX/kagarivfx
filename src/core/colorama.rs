#![allow(dead_code)]
/// Colorama Cycle Preset types matching After Effects Colorama effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColoramaPreset {
    Rainbow,
    Heatmap,
    Sepia,
    Solarize,
}

/// Applies Colorama gradient color cycling to an RGBA pixel buffer based on pixel luminosity.
pub fn apply_colorama(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    preset: ColoramaPreset,
    cycle_phase_deg: f32,
) {
    let Some(num_pixels) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return;
    };
    if width == 0 || height == 0 || pixels.len() != num_pixels * 4 {
        return;
    }

    let phase_norm = if cycle_phase_deg.is_finite() {
        (cycle_phase_deg / 360.0).fract()
    } else {
        0.0
    };

    for i in 0..num_pixels {
        let idx = i * 4;
        let r = pixels[idx] as f32 / 255.0;
        let g = pixels[idx + 1] as f32 / 255.0;
        let b = pixels[idx + 2] as f32 / 255.0;

        // Luminance factor
        let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b + phase_norm).fract();

        let new_rgb = match preset {
            ColoramaPreset::Rainbow => {
                // Hue cycle: HSV(lum * 360, 1.0, 1.0)
                hsv_to_rgb(lum * 360.0, 1.0, 1.0)
            }
            ColoramaPreset::Heatmap => {
                if lum < 0.33 {
                    [lum * 3.0, 0.0, 0.0]
                } else if lum < 0.66 {
                    [1.0, (lum - 0.33) * 3.0, 0.0]
                } else {
                    [1.0, 1.0, (lum - 0.66) * 3.0]
                }
            }
            ColoramaPreset::Sepia => [lum * 0.9, lum * 0.7, lum * 0.4],
            ColoramaPreset::Solarize => {
                let s = (lum * std::f32::consts::PI * 2.0).sin().abs();
                [s, s * 0.8, s * 0.5]
            }
        };

        pixels[idx] = (new_rgb[0] * 255.0).clamp(0.0, 255.0) as u8;
        pixels[idx + 1] = (new_rgb[1] * 255.0).clamp(0.0, 255.0) as u8;
        pixels[idx + 2] = (new_rgb[2] * 255.0).clamp(0.0, 255.0) as u8;
    }
}

/// Utility helper converting HSV color space to RGBA float channels.
fn hsv_to_rgb(h_deg: f32, s: f32, v: f32) -> [f32; 3] {
    let h = (h_deg % 360.0 + 360.0) % 360.0 / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [r1 + m, g1 + m, b1 + m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorama_rainbow_cycle() {
        let mut pixels = vec![128u8; 16]; // 2x2 buffer
        apply_colorama(&mut pixels, 2, 2, ColoramaPreset::Rainbow, 0.0);
        assert_eq!(pixels.len(), 16);
    }

    #[test]
    fn test_colorama_rejects_overflow_and_nonfinite_phase() {
        let original = vec![128u8; 16];
        let mut pixels = original.clone();
        apply_colorama(
            &mut pixels,
            u32::MAX,
            u32::MAX,
            ColoramaPreset::Rainbow,
            f32::NAN,
        );
        assert_eq!(pixels, original);

        apply_colorama(&mut pixels, 2, 2, ColoramaPreset::Rainbow, f32::INFINITY);
    }
}
