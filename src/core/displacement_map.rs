#![allow(dead_code)]
/// Source channel selection for Displacement Mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplacementChannel {
    Red,
    Green,
    Blue,
    Alpha,
    Luminance,
}

/// Configuration settings for Displacement Map effect.
#[derive(Debug, Clone)]
pub struct DisplacementMapOptions {
    pub max_horizontal_displacement: f32,
    pub max_vertical_displacement: f32,
    pub horizontal_channel: DisplacementChannel,
    pub vertical_channel: DisplacementChannel,
    pub wrap_pixels: bool,
}

impl Default for DisplacementMapOptions {
    fn default() -> Self {
        Self {
            max_horizontal_displacement: 20.0,
            max_vertical_displacement: 20.0,
            horizontal_channel: DisplacementChannel::Red,
            vertical_channel: DisplacementChannel::Green,
            wrap_pixels: false,
        }
    }
}

/// Extracts normalized displacement factor (-0.5 .. +0.5) from a RGBA pixel buffer based on channel type.
fn sample_channel_value(rgba: [u8; 4], channel: DisplacementChannel) -> f32 {
    let norm = match channel {
        DisplacementChannel::Red => rgba[0] as f32 / 255.0,
        DisplacementChannel::Green => rgba[1] as f32 / 255.0,
        DisplacementChannel::Blue => rgba[2] as f32 / 255.0,
        DisplacementChannel::Alpha => rgba[3] as f32 / 255.0,
        DisplacementChannel::Luminance => {
            (0.2126 * rgba[0] as f32 + 0.7152 * rgba[1] as f32 + 0.0722 * rgba[2] as f32) / 255.0
        }
    };
    norm - 0.5
}

/// Core Displacement Map CPU Buffer Processing Kernel:
/// Warps target RGBA buffer pixels based on reference displacement map channel intensities.
pub fn apply_displacement_map(
    target_pixels: &[u8],
    ref_map_pixels: &[u8],
    width: u32,
    height: u32,
    options: &DisplacementMapOptions,
) -> Vec<u8> {
    let num_pixels = (width * height) as usize;
    if target_pixels.len() != num_pixels * 4 || ref_map_pixels.len() != num_pixels * 4 {
        return target_pixels.to_vec();
    }

    let mut out_pixels = vec![0u8; num_pixels * 4];
    let w_f32 = width as f32;
    let h_f32 = height as f32;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let ref_rgba = [
                ref_map_pixels[idx],
                ref_map_pixels[idx + 1],
                ref_map_pixels[idx + 2],
                ref_map_pixels[idx + 3],
            ];

            let dx = sample_channel_value(ref_rgba, options.horizontal_channel) * options.max_horizontal_displacement;
            let dy = sample_channel_value(ref_rgba, options.vertical_channel) * options.max_vertical_displacement;

            let mut src_x = x as f32 + dx;
            let mut src_y = y as f32 + dy;

            if options.wrap_pixels {
                src_x = (src_x % w_f32 + w_f32) % w_f32;
                src_y = (src_y % h_f32 + h_f32) % h_f32;
            } else {
                src_x = src_x.clamp(0.0, w_f32 - 1.0);
                src_y = src_y.clamp(0.0, h_f32 - 1.0);
            }

            // Bilinear sampling of source pixel
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

            for c in 0..4 {
                let p00 = target_pixels[idx00 + c] as f32;
                let p10 = target_pixels[idx10 + c] as f32;
                let p01 = target_pixels[idx01 + c] as f32;
                let p11 = target_pixels[idx11 + c] as f32;

                let top = p00 + (p10 - p00) * tx;
                let bottom = p01 + (p11 - p01) * tx;
                let val = top + (bottom - top) * ty;

                out_pixels[idx + c] = val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    out_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_displacement_map_identity_neutral() {
        let width = 4;
        let height = 4;
        let pixels = vec![128u8; (width * height * 4) as usize];
        let ref_neutral = vec![128u8; (width * height * 4) as usize]; // Midpoint 128 = 0 displacement

        let options = DisplacementMapOptions::default();
        let out = apply_displacement_map(&pixels, &ref_neutral, width, height, &options);

        assert_eq!(out.len(), pixels.len());
        assert_eq!(out[0], 128);
    }
}
