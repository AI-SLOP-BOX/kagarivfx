#![allow(dead_code)]
/// Matte Channel source types matching After Effects Set Matte effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatteChannelSource {
    Alpha,
    Luminance,
    Red,
    Green,
    Blue,
}

/// Applies external layer Matte masking (Set Matte) onto target pixel buffer.
pub fn apply_set_matte(
    target_pixels: &mut [u8],
    matte_pixels: &[u8],
    width: u32,
    height: u32,
    source: MatteChannelSource,
    invert_matte: bool,
) {
    let num_pixels = (width * height) as usize;
    if target_pixels.len() != num_pixels * 4 || matte_pixels.len() != num_pixels * 4 {
        return;
    }

    for i in 0..num_pixels {
        let idx = i * 4;
        let m_r = matte_pixels[idx] as f32 / 255.0;
        let m_g = matte_pixels[idx + 1] as f32 / 255.0;
        let m_b = matte_pixels[idx + 2] as f32 / 255.0;
        let m_a = matte_pixels[idx + 3] as f32 / 255.0;

        let raw_val = match source {
            MatteChannelSource::Alpha => m_a,
            MatteChannelSource::Luminance => 0.2126 * m_r + 0.7152 * m_g + 0.0722 * m_b,
            MatteChannelSource::Red => m_r,
            MatteChannelSource::Green => m_g,
            MatteChannelSource::Blue => m_b,
        };

        let mut final_factor = if invert_matte { 1.0 - raw_val } else { raw_val };
        final_factor = final_factor.clamp(0.0, 1.0);

        // Multiply existing target alpha by matte factor
        let current_a = target_pixels[idx + 3] as f32 / 255.0;
        target_pixels[idx + 3] = (current_a * final_factor * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_matte_alpha_mask() {
        let mut target = vec![255u8; 16]; // 2x2 solid white fully opaque
        let matte = vec![
            255, 255, 255, 128, // Pixel 0: Alpha = 128 (50%)
            255, 255, 255, 0,   // Pixel 1: Alpha = 0 (0%)
            255, 255, 255, 255, // Pixel 2: Alpha = 255 (100%)
            255, 255, 255, 255,
        ];

        apply_set_matte(&mut target, &matte, 2, 2, MatteChannelSource::Alpha, false);

        assert_eq!(target[3], 128);
        assert_eq!(target[7], 0);
        assert_eq!(target[11], 255);
    }
}
