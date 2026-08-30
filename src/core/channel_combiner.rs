//! Channel Combiner Effect Engine (AE Parity - Channel > Channel Combiner).
//!
//! Extracts channels from Red, Green, Blue, Alpha, Luminance, Hue, Lightness,
//! Saturation, Min RGB, or Max RGB and routes/bakes them into target channels
//! (Red, Green, Blue, Alpha, RGB Only, RGBA, Lightness, etc.) with optional inversion.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ChannelCombinerFrom {
    Red,
    Green,
    Blue,
    Alpha,
    #[default]
    Luminance,
    Hue,
    Lightness,
    Saturation,
    MinRGB,
    MaxRGB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ChannelCombinerTo {
    Red,
    Green,
    Blue,
    #[default]
    Alpha,
    RGBOnly,
    RGBA,
    Lightness,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelCombinerParams {
    pub from_channel: ChannelCombinerFrom,
    pub to_target: ChannelCombinerTo,
    pub invert: bool,
}

impl Default for ChannelCombinerParams {
    fn default() -> Self {
        Self {
            from_channel: ChannelCombinerFrom::Luminance,
            to_target: ChannelCombinerTo::Alpha,
            invert: false,
        }
    }
}

/// Applies Channel Combiner transformation to an RGBA pixel buffer.
pub fn apply_channel_combiner(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    params: &ChannelCombinerParams,
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

    let hsl_to_rgb = |h: f32, s: f32, l: f32| -> (f32, f32, f32) {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - (((h * 6.0).rem_euclid(2.0)) - 1.0).abs());
        let m = l - c * 0.5;

        let (r1, g1, b1) = match (h * 6.0).floor() as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        (r1 + m, g1 + m, b1 + m)
    };

    for chunk in pixels.chunks_exact_mut(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;
        let a = chunk[3] as f32 / 255.0;

        let mut val = match params.from_channel {
            ChannelCombinerFrom::Red => r,
            ChannelCombinerFrom::Green => g,
            ChannelCombinerFrom::Blue => b,
            ChannelCombinerFrom::Alpha => a,
            ChannelCombinerFrom::Luminance => 0.299 * r + 0.587 * g + 0.114 * b,
            ChannelCombinerFrom::Hue => rgb_to_hsl(r, g, b).0,
            ChannelCombinerFrom::Lightness => rgb_to_hsl(r, g, b).2,
            ChannelCombinerFrom::Saturation => rgb_to_hsl(r, g, b).1,
            ChannelCombinerFrom::MinRGB => r.min(g).min(b),
            ChannelCombinerFrom::MaxRGB => r.max(g).max(b),
        };

        if params.invert {
            val = 1.0 - val;
        }
        val = val.clamp(0.0, 1.0);

        let byte_val = (val * 255.0).round() as u8;

        match params.to_target {
            ChannelCombinerTo::Red => chunk[0] = byte_val,
            ChannelCombinerTo::Green => chunk[1] = byte_val,
            ChannelCombinerTo::Blue => chunk[2] = byte_val,
            ChannelCombinerTo::Alpha => chunk[3] = byte_val,
            ChannelCombinerTo::RGBOnly => {
                chunk[0] = byte_val;
                chunk[1] = byte_val;
                chunk[2] = byte_val;
            }
            ChannelCombinerTo::RGBA => {
                chunk[0] = byte_val;
                chunk[1] = byte_val;
                chunk[2] = byte_val;
                chunk[3] = byte_val;
            }
            ChannelCombinerTo::Lightness => {
                let (h, s, _) = rgb_to_hsl(r, g, b);
                let (nr, ng, nb) = hsl_to_rgb(h, s, val);
                chunk[0] = (nr.clamp(0.0, 1.0) * 255.0).round() as u8;
                chunk[1] = (ng.clamp(0.0, 1.0) * 255.0).round() as u8;
                chunk[2] = (nb.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_combiner_rejects_dimension_overflow() {
        let original = vec![42u8; 4];
        let mut pixels = original.clone();
        apply_channel_combiner(
            &mut pixels,
            u32::MAX,
            u32::MAX,
            &ChannelCombinerParams::default(),
        );
        assert_eq!(pixels, original);
    }

    #[test]
    fn test_channel_combiner_luma_to_alpha() {
        let mut pixels = vec![255, 255, 255, 0]; // White with 0 alpha
        let params = ChannelCombinerParams {
            from_channel: ChannelCombinerFrom::Luminance,
            to_target: ChannelCombinerTo::Alpha,
            invert: false,
        };

        apply_channel_combiner(&mut pixels, 1, 1, &params);
        assert_eq!(pixels[3], 255); // Alpha should now be full white
    }

    #[test]
    fn test_channel_combiner_invert_alpha() {
        let mut pixels = vec![0, 0, 0, 255]; // Black with full alpha
        let params = ChannelCombinerParams {
            from_channel: ChannelCombinerFrom::Alpha,
            to_target: ChannelCombinerTo::Alpha,
            invert: true,
        };

        apply_channel_combiner(&mut pixels, 1, 1, &params);
        assert_eq!(pixels[3], 0); // Inverted alpha -> 0
    }
}
