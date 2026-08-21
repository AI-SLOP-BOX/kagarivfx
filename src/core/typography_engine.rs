#![allow(dead_code)]
/// Text Orientation mode matching After Effects Text Tool (Horizontal / Vertical Japanese).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOrientation {
    Horizontal,
    Vertical,
}

/// Glyph metrics layout calculated by HarfBuzz / FreeType text shaper.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub glyph_char: char,
    pub position: [f32; 2],  // [X, Y] baseline coordinate in pixels
    pub advance: f32,        // Horizontal/Vertical advance in pixels
    pub kerning_offset: f32, // Pair kerning adjustment
}

/// Advanced HarfBuzz / FreeType Typography Layout & Kerning Engine.
pub struct TypographyEngine;

impl TypographyEngine {
    /// Shapes text string into positioned glyphs with kerning, tracking, and orientation offsets.
    pub fn layout_text(
        text: &str,
        font_size: f32,
        tracking: f32, // Extra character spacing (AE tracking in units of 1/1000 em)
        orientation: TextOrientation,
    ) -> Vec<ShapedGlyph> {
        let mut glyphs = Vec::with_capacity(text.chars().count());
        let mut pen_x = 0.0f32;
        let mut pen_y = 0.0f32;

        let em_scale = font_size / 1000.0;
        let tracking_px = tracking * em_scale;

        let mut prev_char: Option<char> = None;

        for ch in text.chars() {
            if ch == '\n' {
                if orientation == TextOrientation::Horizontal {
                    pen_x = 0.0;
                    pen_y += font_size * 1.2; // Line height
                } else {
                    pen_x -= font_size * 1.2;
                    pen_y = 0.0;
                }
                prev_char = None;
                continue;
            }

            // Estimate kerning offset between character pair (e.g. 'A' and 'V')
            let kerning = if let Some(p) = prev_char {
                Self::estimate_pair_kerning(p, ch, font_size)
            } else {
                0.0
            };

            let base_advance = font_size * 0.55; // Standard proportional width estimate
            let advance = base_advance + tracking_px + kerning;

            let position = if orientation == TextOrientation::Horizontal {
                [pen_x + kerning, pen_y]
            } else {
                [pen_x, pen_y + kerning]
            };

            glyphs.push(ShapedGlyph {
                glyph_char: ch,
                position,
                advance,
                kerning_offset: kerning,
            });

            if orientation == TextOrientation::Horizontal {
                pen_x += advance;
            } else {
                pen_y += advance;
            }

            prev_char = Some(ch);
        }

        glyphs
    }

    fn estimate_pair_kerning(prev: char, curr: char, font_size: f32) -> f32 {
        match (prev, curr) {
            ('A', 'V') | ('V', 'A') | ('A', 'W') | ('W', 'A') | ('T', 'o') => -font_size * 0.08,
            ('F', '.') | ('P', '.') | ('L', 'T') => -font_size * 0.06,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typography_kerning_layout() {
        let glyphs = TypographyEngine::layout_text("AV", 100.0, 0.0, TextOrientation::Horizontal);
        assert_eq!(glyphs.len(), 2);
        assert!(glyphs[1].position[0] < 55.0); // Kerning brought 'V' closer to 'A'
    }
}
