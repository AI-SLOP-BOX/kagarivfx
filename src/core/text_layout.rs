//! Text layout engine for paragraph formatting, line wrapping, and alignment.
//!
//! Handles:
//! - Line wrapping at word boundaries when text exceeds box_width
//! - Paragraph alignment (Left, Center, Right, Justify)
//! - Leading (line-height) multiplier
//! - Vertical overflow clipping

/// Text alignment modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum TextAlign {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
    Justify = 3,
}


/// A single laid-out line of text.
#[derive(Debug, Clone)]
pub struct TextLine {
    pub text: String,
    pub width: f32,
    pub y_offset: f32,
}

/// Layout result containing all lines and total dimensions.
#[derive(Debug, Clone)]
pub struct TextLayout {
    pub lines: Vec<TextLine>,
    pub total_width: f32,
    pub total_height: f32,
}

/// Compute the width of a text string given font size and tracking.
/// Uses approximate character width ratio (0.6 * font_size for average character).
pub fn measure_text_width(text: &str, font_size: f32, tracking: f32) -> f32 {
    let char_count = text.chars().count() as f32;
    if char_count == 0.0 {
        return 0.0;
    }
    let char_width = font_size * 0.6;
    char_count * char_width + (char_count - 1.0).max(0.0) * tracking * 0.1
}

/// Lay out text with line wrapping and alignment.
///
/// # Arguments
/// * `text` - The text content (may contain newlines)
/// * `font_size` - Font size in pixels
/// * `tracking` - Letter spacing in AE units
/// * `leading` - Line height multiplier (1.0 = normal)
/// * `box_width` - Maximum line width for wrapping (0 = no wrapping)
/// * `alignment` - Text alignment
pub fn layout_text(
    text: &str,
    font_size: f32,
    tracking: f32,
    leading: f32,
    box_width: f32,
    _alignment: TextAlign,
) -> TextLayout {
    let line_height = font_size * leading;
    let mut all_lines: Vec<TextLine> = Vec::new();

    // Split text into paragraphs by newlines
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            all_lines.push(TextLine {
                text: String::new(),
                width: 0.0,
                y_offset: all_lines.len() as f32 * line_height,
            });
            continue;
        }

        if box_width <= 0.0 {
            // No wrapping — single line
            let w = measure_text_width(paragraph, font_size, tracking);
            all_lines.push(TextLine {
                text: paragraph.to_string(),
                width: w,
                y_offset: all_lines.len() as f32 * line_height,
            });
        } else {
            // Word-wrap
            let words: Vec<&str> = paragraph.split_whitespace().collect();
            let mut current_line = String::new();
            let mut current_width = 0.0f32;

            for word in &words {
                let word_width = measure_text_width(word, font_size, tracking);
                let space_width = if current_line.is_empty() { 0.0 } else { font_size * 0.25 + tracking * 0.1 };

                if current_width + space_width + word_width > box_width && !current_line.is_empty() {
                    // Flush current line
                    all_lines.push(TextLine {
                        text: current_line.clone(),
                        width: current_width,
                        y_offset: all_lines.len() as f32 * line_height,
                    });
                    current_line = word.to_string();
                    current_width = word_width;
                } else {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                        current_width += space_width;
                    }
                    current_line.push_str(word);
                    current_width += word_width;
                }
            }

            if !current_line.is_empty() {
                all_lines.push(TextLine {
                    text: current_line,
                    width: current_width,
                    y_offset: all_lines.len() as f32 * line_height,
                });
            }
        }
    }

    let total_width = all_lines.iter().map(|l| l.width).fold(0.0f32, f32::max);
    let total_height = if all_lines.is_empty() {
        0.0
    } else {
        all_lines.last().unwrap().y_offset + line_height
    };

    TextLayout {
        lines: all_lines,
        total_width,
        total_height,
    }
}

/// Get the x-offset for a line based on alignment and box width.
pub fn get_alignment_offset(line_width: f32, box_width: f32, alignment: TextAlign) -> f32 {
    if box_width <= 0.0 {
        return 0.0;
    }
    match alignment {
        TextAlign::Left => 0.0,
        TextAlign::Center => (box_width - line_width) * 0.5,
        TextAlign::Right => box_width - line_width,
        TextAlign::Justify => 0.0, // Justify handled per-word during rasterization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measure_text_width() {
        let w = measure_text_width("Hello", 20.0, 0.0);
        assert!(w > 0.0);
        assert!((w - 60.0).abs() < 1.0); // 5 chars * 20 * 0.6 = 60
    }

    #[test]
    fn test_layout_no_wrapping() {
        let layout = layout_text("Hello World", 20.0, 0.0, 1.2, 0.0, TextAlign::Left);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "Hello World");
    }

    #[test]
    fn test_layout_with_wrapping() {
        let layout = layout_text("Hello World Foo Bar", 20.0, 0.0, 1.2, 100.0, TextAlign::Left);
        assert!(layout.lines.len() > 1);
    }

    #[test]
    fn test_layout_empty_text() {
        let layout = layout_text("", 20.0, 0.0, 1.2, 0.0, TextAlign::Left);
        // Empty string splits to one empty paragraph, which produces one empty line
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "");
    }

    #[test]
    fn test_alignment_offset() {
        assert_eq!(get_alignment_offset(50.0, 100.0, TextAlign::Left), 0.0);
        assert_eq!(get_alignment_offset(50.0, 100.0, TextAlign::Center), 25.0);
        assert_eq!(get_alignment_offset(50.0, 100.0, TextAlign::Right), 50.0);
    }
}
