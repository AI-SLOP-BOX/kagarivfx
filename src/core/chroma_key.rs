#![allow(dead_code)]
/// Chroma Key options matching After Effects Keylight effect.
#[derive(Debug, Clone)]
pub struct ChromaKeyOptions {
    pub screen_color: [f32; 3], // Primary Key Color [R, G, B] in range 0.0 .. 1.0 (Default Green: [0.0, 1.0, 0.0])
    pub screen_gain: f32,       // Key Sensitivity Gain (1.0 .. 2.0)
    pub screen_balance: f32,    // Relative balance between R, G, B channels
    pub despill_strength: f32,  // Green/Blue spill suppression factor (0.0 .. 1.0)
    pub clip_black: f32,        // Black level matte threshold (0.0 .. 1.0)
    pub clip_white: f32,        // White level matte threshold (0.0 .. 1.0)
}

impl Default for ChromaKeyOptions {
    fn default() -> Self {
        Self {
            screen_color: [0.0, 1.0, 0.0], // Standard Chroma Green
            screen_gain: 1.0,
            screen_balance: 0.5,
            despill_strength: 0.8,
            clip_black: 0.0,
            clip_white: 1.0,
        }
    }
}

/// Converts linear RGB (0.0..1.0) into full-range YCbCr chroma components.
fn rgb_to_chroma(r: f32, g: f32, b: f32) -> (f32, f32) {
    let cb = 0.5 - 0.168_735_89 * r - 0.331_264_1 * g + 0.5 * b;
    let cr = 0.5 + 0.5 * r - 0.418_687_6 * g - 0.081_312_41 * b;
    (cb, cr)
}

/// Computes the chroma distance between a pixel and the key color.
/// `balance` (0.0..1.0) emphasizes the Cb axis over the Cr axis; 0.5 is neutral.
fn chroma_distance(cb: f32, cr: f32, k_cb: f32, k_cr: f32, balance: f32) -> f32 {
    let w = balance.clamp(0.0, 1.0);
    let d_cb = (cb - k_cb) * (0.5 + w);
    let d_cr = (cr - k_cr) * (1.5 - w);
    (d_cb * d_cb + d_cr * d_cr).sqrt()
}

/// Applies professional Chroma Keying and Spill Suppression onto RGBA pixel buffer.
///
/// The matte is computed from the YCbCr chroma distance to the key color, which
/// separates luminance from hue and produces soft, feathered edges. Spill
/// suppression works for both green and blue screens by clamping the dominant
/// channel against the other two channels.
pub fn apply_chroma_key(pixels: &mut [u8], width: u32, height: u32, options: &ChromaKeyOptions) {
    let Some(num_pixels) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return;
    };
    if width == 0 || height == 0 || pixels.len() != num_pixels * 4 {
        return;
    }

    let finite_clamped = |value: f32, default: f32| {
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            default
        }
    };
    let k_r = finite_clamped(options.screen_color[0], 0.0);
    let k_g = finite_clamped(options.screen_color[1], 1.0);
    let k_b = finite_clamped(options.screen_color[2], 0.0);
    let gain = if options.screen_gain.is_finite() {
        options.screen_gain.max(0.0)
    } else {
        1.0
    };
    let balance = finite_clamped(options.screen_balance, 0.5);
    let clip_black = finite_clamped(options.clip_black, 0.0);
    let clip_white = finite_clamped(options.clip_white, 1.0);
    let despill_strength = finite_clamped(options.despill_strength, 0.0);

    let (k_cb, k_cr) = rgb_to_chroma(k_r, k_g, k_b);

    let is_green_screen = k_g >= k_r && k_g >= k_b;

    for i in 0..num_pixels {
        let idx = i * 4;
        let r = pixels[idx] as f32 / 255.0;
        let g = pixels[idx + 1] as f32 / 255.0;
        let b = pixels[idx + 2] as f32 / 255.0;

        // Chroma-distance based matte: pixels near the key chroma are fully
        // transparent, distances beyond the tolerance ramp up smoothly to
        // opaque, producing natural soft edge feathering.
        let (cb, cr) = rgb_to_chroma(r, g, b);
        const CORE_TOLERANCE: f32 = 0.15;
        const MATTE_RAMP: f32 = 3.0;
        let raw_matte = ((chroma_distance(cb, cr, k_cb, k_cr, balance)
            - CORE_TOLERANCE)
            * MATTE_RAMP
            * gain)
            .clamp(0.0, 1.0);

        // Apply Clip Black / Clip White thresholds
        let matte = if clip_white > clip_black + 0.001 {
            ((raw_matte - clip_black) / (clip_white - clip_black))
                .clamp(0.0, 1.0)
        } else {
            raw_matte.clamp(0.0, 1.0)
        };

        // Multiply existing alpha channel
        let current_a = pixels[idx + 3] as f32 / 255.0;
        pixels[idx + 3] = (current_a * matte * 255.0).round().clamp(0.0, 255.0) as u8;

        // Apply Spill Suppression (Despill): clamp the dominant screen channel
        // (G for green screens, B for blue screens) against the other channels.
        let strength = despill_strength;
        if strength > 0.001 {
            let ch_idx = if is_green_screen { idx + 1 } else { idx + 2 };
            let primary_ch = pixels[ch_idx] as f32 / 255.0;
            let max_allowed = if is_green_screen { r.max(b) } else { r.max(g) };
            if primary_ch > max_allowed {
                let despilled = primary_ch - (primary_ch - max_allowed) * strength;
                pixels[ch_idx] = (despilled * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(pixels: &[u8], i: usize) -> [u8; 4] {
        [
            pixels[i * 4],
            pixels[i * 4 + 1],
            pixels[i * 4 + 2],
            pixels[i * 4 + 3],
        ]
    }

    #[test]
    fn test_chroma_key_green_screen_removal() {
        let mut pixels = vec![
            0, 255, 0, 255, // Pure Green Pixel -> Should become transparent
            255, 0, 0, 255, // Pure Red Pixel -> Should stay opaque
        ];

        let options = ChromaKeyOptions::default();
        apply_chroma_key(&mut pixels, 2, 1, &options);

        assert_eq!(pixels[3], 0); // Green pixel alpha removed
        assert_eq!(pixels[7], 255); // Red pixel alpha preserved
    }

    #[test]
    fn test_foreground_background_separation() {
        // 4x4: green background with a centered 2x2 red square
        let mut pixels = vec![0u8; 64];
        for i in 0..16 {
            let x = i % 4;
            let y = i / 4;
            if (1..=2).contains(&x) && (1..=2).contains(&y) {
                pixels[i * 4..i * 4 + 4].copy_from_slice(&[220, 30, 30, 255]);
            } else {
                pixels[i * 4..i * 4 + 4].copy_from_slice(&[10, 235, 30, 255]);
            }
        }

        apply_chroma_key(&mut pixels, 4, 4, &ChromaKeyOptions::default());

        for i in 0..16 {
            let x = i % 4;
            let y = i / 4;
            let a = px(&pixels, i)[3];
            if (1..=2).contains(&x) && (1..=2).contains(&y) {
                assert!(a >= 240, "foreground pixel {i} should be opaque, got {a}");
            } else {
                assert!(
                    a <= 10,
                    "background pixel {i} should be transparent, got {a}"
                );
            }
        }
    }

    #[test]
    fn test_despill_reduces_green_spill() {
        // Gray-green contaminated pixel (green channel exceeds R/B)
        let mut pixels = vec![80, 120, 70, 255];
        let options = ChromaKeyOptions {
            screen_gain: 2.0,
            despill_strength: 1.0,
            ..Default::default()
        };
        apply_chroma_key(&mut pixels, 1, 1, &options);

        let [r, g, b, _a] = px(&pixels, 0);
        assert!(
            g <= 84,
            "spill should be suppressed toward max(R,B), got G={g} R={r} B={b}"
        );
        assert_eq!(g, 80);
    }

    #[test]
    fn test_despill_blue_screen() {
        // Blue screen: dominant B channel must also be despilled
        let mut pixels = vec![70, 70, 130, 255];
        let options = ChromaKeyOptions {
            screen_color: [0.0, 0.0, 1.0],
            screen_gain: 2.0,
            despill_strength: 1.0,
            ..Default::default()
        };
        apply_chroma_key(&mut pixels, 1, 1, &options);

        let [_, _, b, _] = px(&pixels, 0);
        assert!(b <= 74, "blue spill should be suppressed, got B={b}");
    }

    #[test]
    fn test_blue_screen_keying() {
        let mut pixels = vec![
            0, 0, 255, 255, // Blue background -> transparent
            255, 165, 0, 255, // Orange foreground -> opaque
        ];
        let options = ChromaKeyOptions {
            screen_color: [0.0, 0.0, 1.0],
            ..Default::default()
        };
        apply_chroma_key(&mut pixels, 2, 1, &options);

        assert!(pixels[3] <= 10);
        assert!(pixels[7] >= 240);
    }

    #[test]
    fn test_edge_feathering_produces_partial_alpha() {
        // A pixel blending green screen and foreground should get intermediate alpha
        let mut solid = vec![255, 0, 0, 255];
        let mut blended = vec![89, 166, 32, 255]; // ~65% green / 35% red mix
        let options = ChromaKeyOptions::default();
        apply_chroma_key(&mut solid, 1, 1, &options);
        apply_chroma_key(&mut blended, 1, 1, &options);

        let a_blend = blended[3] as i32;
        assert!(
            a_blend > 10 && a_blend < 250,
            "edge pixel should be partially transparent: {a_blend}"
        );
    }

    #[test]
    fn test_clip_black_white_tightens_matte() {
        let mut pixels = vec![128, 128, 32, 255]; // partial matte pixel
        let options = ChromaKeyOptions {
            clip_black: 0.5,
            clip_white: 0.75,
            ..Default::default()
        };
        apply_chroma_key(&mut pixels, 1, 1, &options);
        assert_eq!(
            pixels[3], 255,
            "matte above clip range should be forced opaque"
        );
    }

    #[test]
    fn test_invalid_buffer_is_noop() {
        let mut pixels = vec![0u8; 7];
        apply_chroma_key(&mut pixels, 2, 2, &ChromaKeyOptions::default());
        assert!(pixels.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_skin_tone_preserved() {
        let mut pixels = vec![230, 180, 140, 255];
        apply_chroma_key(&mut pixels, 1, 1, &ChromaKeyOptions::default());
        assert!(px(&pixels, 0)[3] >= 240, "skin tone must not be keyed out");
    }
}
