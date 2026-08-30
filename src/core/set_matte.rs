//! Set Matte Channel Effect Engine (AE Parity).
//!
//! Replaces or composites the current layer's alpha channel with any channel
//! (Alpha, Luminance, Red, Green, Blue, Hue, Saturation, Lightness) extracted
//! from another specified layer in the composition, with optional inversion.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MatteSourceChannel {
    #[default]
    Alpha,
    Luminance,
    Red,
    Green,
    Blue,
    Lightness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MatteCompositeMode {
    #[default]
    Replace,
    Intersect,
    Add,
    Subtract,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetMatteParams {
    pub source_layer_idx: usize,
    pub source_channel: MatteSourceChannel,
    pub invert_matte: bool,
    pub composite_mode: MatteCompositeMode,
}

impl Default for SetMatteParams {
    fn default() -> Self {
        Self {
            source_layer_idx: 0,
            source_channel: MatteSourceChannel::Alpha,
            invert_matte: false,
            composite_mode: MatteCompositeMode::Replace,
        }
    }
}

/// Applies Set Matte channel extraction and alpha channel composition onto an RGBA pixel buffer.
pub fn apply_set_matte(
    target_pixels: &mut [u8],
    target_w: u32,
    target_h: u32,
    source_pixels: &[u8],
    source_w: u32,
    source_h: u32,
    params: &SetMatteParams,
) {
    let target_len = (target_w as usize)
        .checked_mul(target_h as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    let source_len = (source_w as usize)
        .checked_mul(source_h as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    if target_len != Some(target_pixels.len())
        || source_len != Some(source_pixels.len())
        || target_w == 0
        || target_h == 0
        || source_w == 0
        || source_h == 0
    {
        return;
    }

    let scale_x = source_w as f32 / target_w as f32;
    let scale_y = source_h as f32 / target_h as f32;

    for y in 0..target_h {
        let sy = ((y as f32 * scale_y).floor() as u32).min(source_h - 1);
        for x in 0..target_w {
            let sx = ((x as f32 * scale_x).floor() as u32).min(source_w - 1);

            let s_idx = ((sy * source_w + sx) * 4) as usize;
            let t_idx = ((y * target_w + x) * 4) as usize;

            let sr = source_pixels[s_idx] as f32 / 255.0;
            let sg = source_pixels[s_idx + 1] as f32 / 255.0;
            let sb = source_pixels[s_idx + 2] as f32 / 255.0;
            let sa = source_pixels[s_idx + 3] as f32 / 255.0;

            let mut extracted_alpha = match params.source_channel {
                MatteSourceChannel::Alpha => sa,
                MatteSourceChannel::Luminance => 0.2126 * sr + 0.7152 * sg + 0.0722 * sb,
                MatteSourceChannel::Red => sr,
                MatteSourceChannel::Green => sg,
                MatteSourceChannel::Blue => sb,
                MatteSourceChannel::Lightness => {
                    let max = sr.max(sg).max(sb);
                    let min = sr.min(sg).min(sb);
                    (max + min) * 0.5
                }
            };

            if params.invert_matte {
                extracted_alpha = 1.0 - extracted_alpha;
            }

            let cur_alpha = target_pixels[t_idx + 3] as f32 / 255.0;

            let final_alpha = match params.composite_mode {
                MatteCompositeMode::Replace => extracted_alpha,
                MatteCompositeMode::Intersect => cur_alpha * extracted_alpha,
                MatteCompositeMode::Add => (cur_alpha + extracted_alpha).min(1.0),
                MatteCompositeMode::Subtract => (cur_alpha - extracted_alpha).max(0.0),
            };

            target_pixels[t_idx + 3] = (final_alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_matte_alpha_replace() {
        let mut tgt = vec![255u8; 16]; // 2x2 opaque
        let src = vec![
            255, 0, 0, 128, 255, 0, 0, 128, 255, 0, 0, 128, 255, 0, 0, 128,
        ]; // 2x2 with alpha 128

        let params = SetMatteParams {
            source_layer_idx: 0,
            source_channel: MatteSourceChannel::Alpha,
            invert_matte: false,
            composite_mode: MatteCompositeMode::Replace,
        };

        apply_set_matte(&mut tgt, 2, 2, &src, 2, 2, &params);
        assert_eq!(tgt[3], 128);
    }

    #[test]
    fn test_set_matte_rejects_dimension_overflow() {
        let mut target = vec![255u8; 4];
        let source = vec![255u8; 4];
        let params = SetMatteParams::default();
        apply_set_matte(
            &mut target,
            u32::MAX,
            u32::MAX,
            &source,
            1,
            1,
            &params,
        );
        assert_eq!(target, vec![255u8; 4]);
    }

    #[test]
    fn test_set_matte_luminance_invert() {
        let mut tgt = vec![255u8; 16];
        let src = vec![
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ]; // pure white

        let params = SetMatteParams {
            source_layer_idx: 0,
            source_channel: MatteSourceChannel::Luminance,
            invert_matte: true,
            composite_mode: MatteCompositeMode::Replace,
        };

        apply_set_matte(&mut tgt, 2, 2, &src, 2, 2, &params);
        assert_eq!(tgt[3], 0); // Inverted luminance of white is 0
    }
}
