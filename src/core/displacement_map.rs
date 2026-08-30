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
    let Some(num_pixels) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return target_pixels.to_vec();
    };
    if width == 0
        || height == 0
        || target_pixels.len() != num_pixels * 4
        || ref_map_pixels.len() != num_pixels * 4
    {
        return target_pixels.to_vec();
    }

    let mut out_pixels = vec![0u8; num_pixels * 4];
    let w_f32 = width as f32;
    let h_f32 = height as f32;
    let max_x = if options.max_horizontal_displacement.is_finite() {
        options.max_horizontal_displacement.clamp(-4096.0, 4096.0)
    } else { 0.0 };
    let max_y = if options.max_vertical_displacement.is_finite() {
        options.max_vertical_displacement.clamp(-4096.0, 4096.0)
    } else { 0.0 };

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let ref_rgba = [
                ref_map_pixels[idx],
                ref_map_pixels[idx + 1],
                ref_map_pixels[idx + 2],
                ref_map_pixels[idx + 3],
            ];

            let dx = sample_channel_value(ref_rgba, options.horizontal_channel) * max_x;
            let dy = sample_channel_value(ref_rgba, options.vertical_channel) * max_y;

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

    /// 8x2 image: left half black column marker at x=0, right half white.
    fn marker_image() -> Vec<u8> {
        let mut v = Vec::new();
        for _y in 0..2u32 {
            for x in 0..8u32 {
                let c = if x == 0 { 0 } else { 255 };
                v.extend_from_slice(&[c, c, c, 255]);
            }
        }
        v
    }

    #[test]
    fn test_positive_red_shifts_samples_left() {
        // Red=255 → dx = +0.5*max → src = dst + max: destination pixel shows
        // content from further right (marker appears to move left).
        let img = marker_image();
        let mut refmap = vec![0u8; img.len()];
        for i in 0..16 {
            refmap[i * 4] = 255; // full positive horizontal displacement
            refmap[i * 4 + 3] = 255;
        }
        let options = DisplacementMapOptions {
            max_horizontal_displacement: 4.0,
            max_vertical_displacement: 0.0,
            horizontal_channel: DisplacementChannel::Red,
            vertical_channel: DisplacementChannel::Green,
            wrap_pixels: false,
        };
        let out = apply_displacement_map(&img, &refmap, 8, 2, &options);
        // Destination x=4 samples source x≈8 → clamped to edge (white).
        let px4 = &out[4 * 4..4 * 4 + 3];
        assert!(px4.iter().all(|&c| c == 255), "edge clamp fills with white");
        // Destination x=7 also samples the clamped right edge.
        let px7 = &out[7 * 4..7 * 4 + 3];
        assert!(px7.iter().all(|&c| c == 255));
    }

    #[test]
    fn test_wrap_mode_tiles_displaced_content() {
        let img = marker_image();
        let mut refmap = vec![0u8; img.len()];
        for i in 0..16 {
            refmap[i * 4] = 255; // shift by +4px horizontally
            refmap[i * 4 + 3] = 255;
        }
        // Channel factor is ±0.5, so max=8 yields a full +4px shift.
        let options = DisplacementMapOptions {
            max_horizontal_displacement: 8.0,
            max_vertical_displacement: 0.0,
            wrap_pixels: true,
            ..Default::default()
        };
        let out = apply_displacement_map(&img, &refmap, 8, 2, &options);
        // Destination x=4 wraps to source x=0 → black marker reappears.
        let px4 = out[4 * 4];
        assert_eq!(px4, 0, "wrapped sample must show the black marker");
    }

    #[test]
    fn test_luminance_and_alpha_channels_drive_displacement() {
        let img = vec![90u8; 8 * 2 * 4];
        // Luminance map: white → +max vertical shift.
        let mut lum_map = vec![255u8; 8 * 2 * 4];
        let opts_lum = DisplacementMapOptions {
            max_horizontal_displacement: 0.0,
            max_vertical_displacement: 2.0,
            horizontal_channel: DisplacementChannel::Luminance,
            vertical_channel: DisplacementChannel::Luminance,
            ..Default::default()
        };
        let out = apply_displacement_map(&img, &lum_map, 8, 2, &opts_lum);
        // Row 0 samples row ~2 → clamped to last row; uniform image stays uniform.
        assert!(out.chunks(4).all(|px| px[0] == 90));

        // Alpha channel of the MAP drives displacement when selected.
        lum_map.iter_mut().skip(3).step_by(4).for_each(|a| *a = 0);
        let opts_alpha = DisplacementMapOptions {
            max_vertical_displacement: 2.0,
            vertical_channel: DisplacementChannel::Alpha,
            ..Default::default()
        };
        let out2 = apply_displacement_map(&img, &lum_map, 8, 2, &opts_alpha);
        assert!(
            out2.chunks(4).all(|px| px[0] == 90),
            "uniform source stays uniform"
        );
    }

    #[test]
    fn test_mismatched_buffers_return_target_unchanged() {
        let target = vec![42u8; 32];
        let bad_ref = vec![7u8; 16];
        let out =
            apply_displacement_map(&target, &bad_ref, 4, 2, &DisplacementMapOptions::default());
        assert_eq!(out, target);
    }

    #[test]
    fn test_extreme_displacement_inputs_are_safe() {
        let target = vec![42u8; 4];
        let map = vec![255u8; 4];
        let out = apply_displacement_map(
            &target,
            &map,
            1,
            1,
            &DisplacementMapOptions {
                max_horizontal_displacement: f32::INFINITY,
                max_vertical_displacement: f32::NAN,
                ..Default::default()
            },
        );
        assert_eq!(out.len(), target.len());
        assert_eq!(
            apply_displacement_map(
                &target,
                &map,
                u32::MAX,
                u32::MAX,
                &DisplacementMapOptions::default()
            ),
            target
        );
    }

    #[test]
    fn test_deterministic_output() {
        let mut img = Vec::new();
        for i in 0..64u32 {
            let c = (i * 13 % 256) as u8;
            img.extend_from_slice(&[c, 255 - c, c / 2, 255]);
        }
        let refmap = img.clone();
        let a = apply_displacement_map(&img, &refmap, 8, 2, &DisplacementMapOptions::default());
        let b = apply_displacement_map(&img, &refmap, 8, 2, &DisplacementMapOptions::default());
        assert_eq!(a, b);
    }

    #[test]
    fn test_zero_displacement_preserves_image() {
        let mut img = Vec::new();
        for i in 0..64u32 {
            let c = (i * 29 % 256) as u8;
            img.extend_from_slice(&[c, c, c, 255]);
        }
        let neutral = vec![128u8; img.len()];
        let out = apply_displacement_map(&img, &neutral, 8, 2, &DisplacementMapOptions::default());
        assert_eq!(out, img, "neutral map must be an exact identity");
    }
}
