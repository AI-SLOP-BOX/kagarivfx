#![allow(dead_code)]
/// Chroma Key options matching After Effects Keylight effect.
#[derive(Debug, Clone)]
pub struct ChromaKeyOptions {
    pub screen_color: [f32; 3],  // Primary Key Color [R, G, B] in range 0.0 .. 1.0 (Default Green: [0.0, 1.0, 0.0])
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

/// Applies professional Chroma Keying and Spill Suppression onto RGBA pixel buffer.
pub fn apply_chroma_key(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    options: &ChromaKeyOptions,
) {
    let num_pixels = (width * height) as usize;
    if pixels.len() != num_pixels * 4 {
        return;
    }

    let k_r = options.screen_color[0];
    let k_g = options.screen_color[1];
    let k_b = options.screen_color[2];

    let is_green_screen = k_g > k_r && k_g > k_b;

    for i in 0..num_pixels {
        let idx = i * 4;
        let r = pixels[idx] as f32 / 255.0;
        let g = pixels[idx + 1] as f32 / 255.0;
        let b = pixels[idx + 2] as f32 / 255.0;

        // Calculate screen matte difference distance
        let primary = if is_green_screen { g } else { b };
        let secondary = if is_green_screen {
            options.screen_balance * r + (1.0 - options.screen_balance) * b
        } else {
            options.screen_balance * g + (1.0 - options.screen_balance) * r
        };

        let raw_matte = 1.0 - ((primary - secondary) * options.screen_gain).max(0.0);

        // Apply Clip Black / Clip White thresholds
        let matte = if options.clip_white > options.clip_black + 0.001 {
            ((raw_matte - options.clip_black) / (options.clip_white - options.clip_black)).clamp(0.0, 1.0)
        } else {
            raw_matte.clamp(0.0, 1.0)
        };

        // Multiply existing alpha channel
        let current_a = pixels[idx + 3] as f32 / 255.0;
        pixels[idx + 3] = (current_a * matte * 255.0).round().clamp(0.0, 255.0) as u8;

        // Apply Spill Suppression (Despill) to remove green/blue fringe bounce
        if options.despill_strength > 0.001 && is_green_screen {
            let max_allowed_g = r.max(b);
            if g > max_allowed_g {
                let despilled_g = g - (g - max_allowed_g) * options.despill_strength;
                pixels[idx + 1] = (despilled_g * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chroma_key_green_screen_removal() {
        let mut pixels = vec![
            0, 255, 0, 255,   // Pure Green Pixel -> Should become transparent
            255, 0, 0, 255,   // Pure Red Pixel -> Should stay opaque
        ];

        let options = ChromaKeyOptions::default();
        apply_chroma_key(&mut pixels, 2, 1, &options);

        assert_eq!(pixels[3], 0);   // Green pixel alpha removed
        assert_eq!(pixels[7], 255); // Red pixel alpha preserved
    }
}
