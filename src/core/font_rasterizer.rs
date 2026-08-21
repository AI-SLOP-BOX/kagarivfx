/// Font rasterizer using ab_glyph for text layer rendering.
///
/// Loads system fonts (or bundled fonts) and rasterizes individual glyphs
/// into RGBA pixel buffers that can be composited by the software renderer.
use std::collections::HashMap;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

/// Parses raw font bytes, supporting TrueType Collections (.ttc) via index fallback.
fn parse_font(data: &[u8]) -> Option<FontRef<'_>> {
    if let Ok(f) = FontRef::try_from_slice(data) {
        return Some(f);
    }
    for idx in 0..8u32 {
        if let Ok(f) = FontRef::try_from_slice_and_index(data, idx) {
            return Some(f);
        }
    }
    None
}

/// A rasterized glyph with its position and pixel data.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Left offset in pixels from the origin
    pub left: i32,
    /// Top offset in pixels from the baseline
    pub top: i32,
    /// Width of the glyph bitmap in pixels
    pub width: u32,
    /// Height of the glyph bitmap in pixels
    pub height: u32,
    /// RGBA pixel data (premultiplied alpha)
    pub pixels: Vec<u8>,
}

/// Font rasterizer that caches loaded fonts and rasterized glyphs.
pub struct FontRasterizer {
    /// Loaded fonts keyed by family name
    fonts: HashMap<String, Vec<u8>>,
}

impl Default for FontRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl FontRasterizer {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }

    /// Load a font from bytes. Returns true if successful.
    pub fn load_font(&mut self, family_name: &str, font_data: Vec<u8>) -> bool {
        self.fonts.insert(family_name.to_string(), font_data);
        true
    }

    /// Load a system font by name. Searches common system font paths.
    pub fn load_system_font(&mut self, family_name: &str) -> bool {
        if self.fonts.contains_key(family_name) {
            return true;
        }

        let paths = Self::system_font_paths(family_name);
        for path in &paths {
            if path.exists() {
                if let Ok(data) = std::fs::read(path) {
                    // Reject data we cannot actually parse (e.g. unsupported collection)
                    if parse_font(&data).is_some() {
                        self.fonts.insert(family_name.to_string(), data);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a font is loaded.
    pub fn has_font(&self, family_name: &str) -> bool {
        self.fonts.contains_key(family_name)
    }

    /// Returns the requested family if available, otherwise the first loaded font.
    /// Falls back to "Helvetica" only when no fonts are loaded at all.
    pub fn resolve_family(&self, family_name: &str) -> String {
        if self.fonts.contains_key(family_name) {
            family_name.to_string()
        } else if let Some(first) = self.fonts.keys().next() {
            first.clone()
        } else {
            "Helvetica".to_string()
        }
    }

    /// Get the default font (first available system font, or embed a fallback).
    pub fn get_default_font_data(&self) -> &[u8] {
        // Try to find a system font
        for name in &["Helvetica", "Arial", "DejaVu Sans", "Liberation Sans", "sans-serif"] {
            if let Some(data) = self.fonts.get(*name) {
                return data;
            }
        }
        // Return empty slice if no font loaded
        &[]
    }

    /// Rasterize a single glyph at the given font size.
    pub fn rasterize_glyph(
        &self,
        family_name: &str,
        ch: char,
        font_size: f32,
    ) -> Option<RasterizedGlyph> {
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scale = PxScale::from(font_size);
        let _scaled_font = font.as_scaled(scale);

        let glyph_id = font.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(PxScale::from(font_size), ab_glyph::point(0.0, 0.0));
        let outlined = font.outline_glyph(glyph)?;

        let bounds = outlined.px_bounds();
        let width = (bounds.max.x - bounds.min.x).max(0.0) as u32;
        let height = (bounds.max.y - bounds.min.y).max(0.0) as u32;

        if width == 0 || height == 0 {
            return Some(RasterizedGlyph {
                left: bounds.min.x as i32,
                top: bounds.min.y as i32,
                width: 0,
                height: 0,
                pixels: vec![],
            });
        }

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        outlined.draw(|x, y, coverage| {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 < pixels.len() {
                let a = (coverage * 255.0) as u8;
                pixels[idx] = 255;     // R
                pixels[idx + 1] = 255; // G
                pixels[idx + 2] = 255; // B
                pixels[idx + 3] = a;   // A
            }
        });

        Some(RasterizedGlyph {
            left: bounds.min.x as i32,
            top: bounds.min.y as i32,
            width,
            height,
            pixels,
        })
    }

    /// Rasterize a full text string into a single RGBA buffer, using the text_layout engine for multi-line paragraph rendering and alignment.
    pub fn rasterize_text(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        self.rasterize_text_formatted(family_name, text, font_size, color, tracking, 1.2, 0.0, crate::core::text_layout::TextAlign::Left)
    }

    /// Rasterize text with full paragraph formatting (leading, box_width, alignment).
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_formatted(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scale = PxScale::from(font_size);
        let scaled_font = font.as_scaled(scale);

        let layout = crate::core::text_layout::layout_text(text, font_size, tracking, leading, box_width, alignment);
        if layout.lines.is_empty() {
            return None;
        }

        let buf_w = (layout.total_width.ceil() as u32).max(1);
        let line_height = font_size * leading;
        let buf_h = (layout.total_height.ceil() as u32).max(line_height.ceil() as u32).max(1);

        let mut pixels = vec![0u8; (buf_w * buf_h * 4) as usize];
        let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;

        let max_top = font_size * 0.8; // baseline offset estimate

        for line in &layout.lines {
            let align_x = crate::core::text_layout::get_alignment_offset(line.width, box_width, alignment);
            let mut cursor_x = align_x;
            let cursor_y = line.y_offset;

            for ch in line.text.chars() {
                let glyph_id = font.glyph_id(ch);
                let h_advance = scaled_font.h_advance(glyph_id);

                if ch != ' ' {
                    let glyph = glyph_id.with_scale_and_position(PxScale::from(font_size), ab_glyph::point(0.0, 0.0));
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds = outlined.px_bounds();
                        let glyph_left = bounds.min.x as i32;
                        let glyph_top = bounds.min.y as i32;

                        outlined.draw(|x, y, coverage| {
                            let dest_x = cursor_x as i32 + glyph_left + x as i32;
                            let dest_y = cursor_y as i32 + max_top as i32 + glyph_top + y as i32;
                            if dest_x >= 0 && dest_y >= 0 && (dest_x as u32) < buf_w && (dest_y as u32) < buf_h {
                                let idx = ((dest_y as u32 * buf_w + dest_x as u32) * 4) as usize;
                                if idx + 3 < pixels.len() {
                                    let a = (coverage * 255.0) as u8;
                                    pixels[idx] = r;
                                    pixels[idx + 1] = g;
                                    pixels[idx + 2] = b;
                                    pixels[idx + 3] = a;
                                }
                            }
                        });
                    }
                }

                cursor_x += h_advance + tracking;
            }
        }

        Some((buf_w, buf_h, pixels))
    }

    /// System font directory paths per platform.
    fn system_font_paths(family_name: &str) -> Vec<std::path::PathBuf> {
        let lower = family_name.to_lowercase().replace(' ', "");
        let _candidates = [
            format!("{}.ttf", lower),
            format!("{}.otf", lower),
            format!("{}.ttc", lower),
            format!("{}.TTF", lower),
            format!("{}.OTF", lower),
        ];

        let mut paths = Vec::new();

        #[cfg(target_os = "macos")]
        {
            paths.push(std::path::PathBuf::from(format!(
                "/Library/Fonts/{}.ttf",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/System/Library/Fonts/{}.ttf",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/System/Library/Fonts/Supplemental/{}.ttf",
                family_name
            )));
            // TrueType Collections (e.g. /System/Library/Fonts/Helvetica.ttc)
            paths.push(std::path::PathBuf::from(format!(
                "/System/Library/Fonts/{}.ttc",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/System/Library/Fonts/Supplemental/{}.ttc",
                family_name
            )));
            // Also check ~/Library/Fonts
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(std::path::PathBuf::from(home).join(format!("Library/Fonts/{}.ttf", family_name)));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(font_dir) = std::env::var_os("WINDIR") {
                paths.push(std::path::PathBuf::from(font_dir).join(format!("Fonts/{}.ttf", family_name)));
            }
        }

        #[cfg(target_os = "linux")]
        {
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/{}.ttf",
                lower
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/{}/{}.ttf",
                lower, family_name
            )));
        }

        paths
    }
}

/// Global font rasterizer instance (thread-safe via once_cell).
use std::sync::OnceLock;

pub static GLOBAL_FONT_RASTERIZER: OnceLock<std::sync::Mutex<FontRasterizer>> = OnceLock::new();

/// Initialize the global font rasterizer and load default system fonts.
pub fn init_font_rasterizer() {
    let _ = GLOBAL_FONT_RASTERIZER.set(std::sync::Mutex::new({
        let mut rasterizer = FontRasterizer::new();
        // Try to load common system fonts
        for name in &["Helvetica", "Arial", "DejaVu Sans", "Liberation Sans", "Inter", "Roboto"] {
            rasterizer.load_system_font(name);
        }
        rasterizer
    }));
}

/// Access the global font rasterizer.
pub fn with_font_rasterizer<F, R>(f: F) -> R
where
    F: FnOnce(&FontRasterizer) -> R,
{
    let rasterizer = GLOBAL_FONT_RASTERIZER.get_or_init(|| {
        let mut r = FontRasterizer::new();
        for name in &["Helvetica", "Arial", "DejaVu Sans", "Liberation Sans", "Inter", "Roboto"] {
            r.load_system_font(name);
        }
        std::sync::Mutex::new(r)
    });
    let lock = rasterizer.lock().unwrap_or_else(|e| e.into_inner());
    f(&lock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_rasterizer_creation() {
        let r = FontRasterizer::new();
        assert!(r.fonts.is_empty());
    }

    #[test]
    fn test_load_nonexistent_font() {
        let mut r = FontRasterizer::new();
        assert!(!r.load_system_font("NonExistentFont12345"));
    }
}
