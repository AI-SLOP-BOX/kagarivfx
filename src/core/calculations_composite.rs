//! Channel Calculations & Combiner Composite Engine (AE Calculations Parity).
//!
//! Blends isolated color/alpha/luminance/HSL channels between two layers
//! with mathematical transfer modes and opacity routing.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ChannelExtractSource {
    #[default]
    Red,
    Green,
    Blue,
    Alpha,
    Luminance,
    Hue,
    Saturation,
    Lightness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CalcTransferMode {
    #[default]
    Normal,
    Add,
    Subtract,
    Multiply,
    Screen,
    Overlay,
    Difference,
    Exclusion,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalculationsConfig {
    pub input_channel: ChannelExtractSource,
    pub second_channel: ChannelExtractSource,
    pub transfer_mode: CalcTransferMode,
    pub second_layer_opacity: f32, // 0.0 .. 1.0
    pub preserve_transparency: bool,
}

impl Default for CalculationsConfig {
    fn default() -> Self {
        Self {
            input_channel: ChannelExtractSource::Red,
            second_channel: ChannelExtractSource::Green,
            transfer_mode: CalcTransferMode::Multiply,
            second_layer_opacity: 1.0,
            preserve_transparency: true,
        }
    }
}

/// Extracts a scalar (0.0 .. 1.0) value from a pixel according to the selected channel source.
#[inline]
pub fn extract_channel_scalar(rgba: [u8; 4], source: ChannelExtractSource) -> f32 {
    let r = rgba[0] as f32 / 255.0;
    let g = rgba[1] as f32 / 255.0;
    let b = rgba[2] as f32 / 255.0;
    let a = rgba[3] as f32 / 255.0;

    match source {
        ChannelExtractSource::Red => r,
        ChannelExtractSource::Green => g,
        ChannelExtractSource::Blue => b,
        ChannelExtractSource::Alpha => a,
        ChannelExtractSource::Luminance => 0.299 * r + 0.587 * g + 0.114 * b,
        ChannelExtractSource::Lightness => (r.max(g).max(b) + r.min(g).min(b)) * 0.5,
        ChannelExtractSource::Saturation => {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            if max > 1e-4 {
                (max - min) / max
            } else {
                0.0
            }
        }
        ChannelExtractSource::Hue => {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let d = max - min;
            if d < 1e-4 {
                0.0
            } else {
                let h = if (max - r).abs() < 1e-4 {
                    ((g - b) / d).rem_euclid(6.0)
                } else if (max - g).abs() < 1e-4 {
                    (b - r) / d + 2.0
                } else {
                    (r - g) / d + 4.0
                };
                h / 6.0
            }
        }
    }
}

/// Applies AE Calculations channel transfer blending onto an RGBA pixel buffer.
pub fn apply_calculations_composite(
    primary_rgba: &mut [u8],
    secondary_rgba: Option<&[u8]>,
    width: u32,
    height: u32,
    config: &CalculationsConfig,
) {
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return;
    };
    if primary_rgba.len() != size {
        return;
    }

    let opacity = if config.second_layer_opacity.is_finite() {
        config.second_layer_opacity.clamp(0.0, 1.0)
    } else {
        0.0
    };

    for i in 0..size / 4 {
        let idx = i * 4;
        let p_pixel = [
            primary_rgba[idx],
            primary_rgba[idx + 1],
            primary_rgba[idx + 2],
            primary_rgba[idx + 3],
        ];

        let s_pixel = if let Some(sec) = secondary_rgba {
            if idx + 3 < sec.len() {
                [sec[idx], sec[idx + 1], sec[idx + 2], sec[idx + 3]]
            } else {
                p_pixel
            }
        } else {
            p_pixel
        };

        let val_a = extract_channel_scalar(p_pixel, config.input_channel);
        let val_b = extract_channel_scalar(s_pixel, config.second_channel);

        let blended_val = match config.transfer_mode {
            CalcTransferMode::Normal => val_b,
            CalcTransferMode::Add => (val_a + val_b).min(1.0),
            CalcTransferMode::Subtract => (val_a - val_b).max(0.0),
            CalcTransferMode::Multiply => val_a * val_b,
            CalcTransferMode::Screen => 1.0 - (1.0 - val_a) * (1.0 - val_b),
            CalcTransferMode::Overlay => {
                if val_a < 0.5 {
                    2.0 * val_a * val_b
                } else {
                    1.0 - 2.0 * (1.0 - val_a) * (1.0 - val_b)
                }
            }
            CalcTransferMode::Difference => (val_a - val_b).abs(),
            CalcTransferMode::Exclusion => val_a + val_b - 2.0 * val_a * val_b,
        };

        let final_val = val_a * (1.0 - opacity) + blended_val * opacity;
        let out_byte = (final_val * 255.0).round().clamp(0.0, 255.0) as u8;

        // Route result back into primary color channels
        primary_rgba[idx] = out_byte;
        primary_rgba[idx + 1] = out_byte;
        primary_rgba[idx + 2] = out_byte;

        if !config.preserve_transparency {
            primary_rgba[idx + 3] = (final_val * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculations_multiply_channels() {
        let mut primary = vec![200u8, 0, 0, 255]; // Red channel = 200/255
        let secondary = vec![0u8, 128, 0, 255]; // Green channel = 128/255

        let config = CalculationsConfig {
            input_channel: ChannelExtractSource::Red,
            second_channel: ChannelExtractSource::Green,
            transfer_mode: CalcTransferMode::Multiply,
            second_layer_opacity: 1.0,
            preserve_transparency: true,
        };

        apply_calculations_composite(&mut primary, Some(&secondary), 1, 1, &config);

        // Expected: (200/255) * (128/255) * 255 ≈ 100
        assert!((primary[0] as i32 - 100).abs() <= 2);
    }
}
