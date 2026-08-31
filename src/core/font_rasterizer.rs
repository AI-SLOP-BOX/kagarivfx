use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
/// Font rasterizer using ab_glyph for text layer rendering.
///
/// Loads system fonts (or bundled fonts) and rasterizes individual glyphs
/// into RGBA pixel buffers that can be composited by the software renderer.
use std::collections::HashMap;

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

    /// Sorted list of successfully loaded font families.
    pub fn available_families(&self) -> Vec<String> {
        let mut names: Vec<String> = self.fonts.keys().cloned().collect();
        names.sort();
        names
    }

    /// Dynamically discover installed system fonts by scanning font directories.
    /// Returns a Vec of (family_name, file_path) pairs.
    pub fn discover_system_fonts() -> Vec<(String, String)> {
        let mut fonts = Vec::new();

        #[cfg(target_os = "macos")]
        {
            let dirs = [
                "/Library/Fonts",
                "/System/Library/Fonts",
                "/System/Library/Fonts/Supplemental",
            ];
            if let Some(home) = std::env::var_os("HOME") {
                let home = std::path::PathBuf::from(home);
                let user_dirs = [home.join("Library/Fonts")];
                for dir in &user_dirs {
                    if dir.is_dir() {
                        if let Ok(entries) = std::fs::read_dir(dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if matches!(ext, "ttf" | "otf" | "ttc") {
                                    let name = path
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("Unknown")
                                        .to_string();
                                    fonts.push((name, path.to_string_lossy().to_string()));
                                }
                            }
                        }
                    }
                }
            }
            for dir in &dirs {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "ttf" | "otf" | "ttc") {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            fonts.push((name, path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
            for dir in &dirs {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                                for sub_entry in sub_entries.flatten() {
                                    let sub_path = sub_entry.path();
                                    let ext =
                                        sub_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                    if matches!(ext, "ttf" | "otf") {
                                        let name = sub_path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("Unknown")
                                            .to_string();
                                        fonts.push((name, sub_path.to_string_lossy().to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(windir) = std::env::var("WINDIR") {
                let fonts_dir = std::path::PathBuf::from(windir).join("Fonts");
                if let Ok(entries) = std::fs::read_dir(&fonts_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "ttf" | "otf" | "ttc") {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            fonts.push((name, path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }

        fonts
    }

    /// Try to load every plausible system font family (best effort).
    /// Uses dynamic font discovery with a hardcoded fallback list.
    pub fn load_all_system_fonts(&mut self) {
        let discovered = Self::discover_system_fonts();
        for (name, path) in discovered {
            if let Ok(data) = std::fs::read(&path) {
                if parse_font(&data).is_some() {
                    self.fonts.insert(name, data);
                }
            }
        }

        const CANDIDATES: &[&str] = &[
            "Helvetica",
            "Helvetica Neue",
            "Arial",
            "Inter",
            "Roboto",
            "DejaVu Sans",
            "Liberation Sans",
            "Times New Roman",
            "Georgia",
            "Courier New",
            "Menlo",
            "Monaco",
            "Verdana",
            "Tahoma",
            "Trebuchet MS",
            "Futura",
            "Gill Sans",
            "Avenir",
            "Optima",
            "Hiragino Sans",
            "Yu Gothic",
            "Noto Sans CJK JP",
            "Osaka",
        ];
        for name in CANDIDATES {
            if !self.fonts.contains_key(*name) {
                self.load_system_font(name);
            }
        }
    }

    /// Get the default font (first available system font, or embed a fallback).
    pub fn get_default_font_data(&self) -> &[u8] {
        // Try to find a system font
        for name in &[
            "Helvetica",
            "Arial",
            "DejaVu Sans",
            "Liberation Sans",
            "sans-serif",
        ] {
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
        if !font_size.is_finite() || !(0.1..=8192.0).contains(&font_size) {
            return None;
        }
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scale = PxScale::from(font_size);
        let _scaled_font = font.as_scaled(scale);

        let glyph_id = font.glyph_id(ch);
        let glyph =
            glyph_id.with_scale_and_position(PxScale::from(font_size), ab_glyph::point(0.0, 0.0));
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

        let pixel_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|count| count.checked_mul(4))?;
        let mut pixels = vec![0u8; pixel_len];
        outlined.draw(|x, y, coverage| {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 < pixels.len() {
                let a = (coverage * 255.0) as u8;
                pixels[idx] = 255; // R
                pixels[idx + 1] = 255; // G
                pixels[idx + 2] = 255; // B
                pixels[idx + 3] = a; // A
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
        if !font_size.is_finite()
            || !(0.1..=8192.0).contains(&font_size)
            || !tracking.is_finite()
            || !color.iter().all(|value| value.is_finite())
        {
            return None;
        }
        self.rasterize_text_formatted(
            family_name,
            text,
            font_size,
            color,
            tracking,
            1.2,
            0.0,
            crate::core::text_layout::TextAlign::Left,
        )
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
        if !font_size.is_finite()
            || !(0.1..=8192.0).contains(&font_size)
            || !tracking.is_finite()
            || !leading.is_finite()
            || leading <= 0.0
            || leading > 100.0
            || !box_width.is_finite()
            || box_width < 0.0
            || !color.iter().all(|value| value.is_finite())
        {
            return None;
        }
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scale = PxScale::from(font_size);
        let scaled_font = font.as_scaled(scale);

        let layout = crate::core::text_layout::layout_text(
            text, font_size, tracking, leading, box_width, alignment,
        );
        if layout.lines.is_empty() {
            return None;
        }

        let buf_w = (layout.total_width.ceil() as u32).max(1);
        let line_height = font_size * leading;
        let buf_h = (layout.total_height.ceil() as u32)
            .max(line_height.ceil() as u32)
            .max(1);

        let mut pixels = vec![0u8; (buf_w * buf_h * 4) as usize];
        let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;

        let max_top = font_size * 0.8; // baseline offset estimate

        for line in &layout.lines {
            let align_x =
                crate::core::text_layout::get_alignment_offset(line.width, box_width, alignment);
            let mut cursor_x = align_x;
            let cursor_y = line.y_offset;

            for ch in line.text.chars() {
                let glyph_id = font.glyph_id(ch);
                let h_advance = scaled_font.h_advance(glyph_id);

                if ch != ' ' {
                    let glyph = glyph_id.with_scale_and_position(
                        PxScale::from(font_size),
                        ab_glyph::point(0.0, 0.0),
                    );
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds = outlined.px_bounds();
                        let glyph_left = bounds.min.x as i32;
                        let glyph_top = bounds.min.y as i32;

                        outlined.draw(|x, y, coverage| {
                            let dest_x = cursor_x as i32 + glyph_left + x as i32;
                            let dest_y = cursor_y as i32 + max_top as i32 + glyph_top + y as i32;
                            if dest_x >= 0
                                && dest_y >= 0
                                && (dest_x as u32) < buf_w
                                && (dest_y as u32) < buf_h
                            {
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

    /// Rasterize text with per-character animator transforms (AE Text Animator).
    /// Each glyph gets the selector-driven position/scale/rotation/opacity/
    /// tracking/blur from `animator`, composited into a padded canvas.
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize_text_animated(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        animator: &crate::core::text_animator::TextAnimatorSettings,
        time: f32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        if !font_size.is_finite()
            || !(0.1..=8192.0).contains(&font_size)
            || !tracking.is_finite()
            || !leading.is_finite()
            || leading <= 0.0
            || leading > 100.0
            || !box_width.is_finite()
            || box_width < 0.0
            || !time.is_finite()
            || !color.iter().all(|value| value.is_finite())
        {
            return None;
        }
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scaled_font = font.as_scaled(PxScale::from(font_size));

        // Per-character transforms (amount already baked in)
        let flat_text: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
        if flat_text.is_empty() {
            return None;
        }
        let xforms =
            crate::core::text_animator::TextAnimatorEngine::eval_character_transforms_extended(
                &flat_text,
                &animator.selector,
                animator.position_offset,
                animator.scale,
                animator.opacity,
                animator.tracking,
                animator.rotation,
                animator.blur_amount,
                false,
                time,
            );

        let layout = crate::core::text_layout::layout_text(
            text, font_size, tracking, leading, box_width, alignment,
        );
        if layout.lines.is_empty() {
            return None;
        }

        // Padding so offsets / blur / rotation have room
        let pad = (font_size.ceil() as u32 + 96).max(128);
        let buf_w = (layout.total_width.ceil() as u32).max(1) + pad * 2;
        let line_height = font_size * leading;
        let buf_h = ((layout.total_height.ceil() as u32) + pad * 2)
            .max(line_height.ceil() as u32 + pad * 2);
        let mut pixels = vec![0u8; (buf_w * buf_h * 4) as usize];
        let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
        let max_top = font_size * 0.8;

        let mut char_idx: usize = 0;
        for line in &layout.lines {
            let align_x =
                crate::core::text_layout::get_alignment_offset(line.width, box_width, alignment);
            let mut cursor_x = align_x;
            let cursor_y = line.y_offset;

            for ch in line.text.chars() {
                let c = xforms
                    .get(char_idx)
                    .cloned()
                    .unwrap_or_else(crate::core::text_animator::CharacterTransform::default);
                let glyph_id = font.glyph_id(ch);
                let h_advance = scaled_font.h_advance(glyph_id);

                if ch != ' ' && (c.opacity_multiplier > 0.01) {
                    let glyph = glyph_id.with_scale_and_position(
                        PxScale::from(font_size),
                        ab_glyph::point(0.0, 0.0),
                    );
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds = outlined.px_bounds();
                        let bw = bounds.width().ceil() as usize;
                        let bh = bounds.height().ceil() as usize;
                        if bw > 0 && bh > 0 {
                            // Capture coverage grid for transform blitting
                            let mut cov = vec![0f32; bw * bh];
                            {
                                let cov_ref = &mut cov;
                                outlined.draw(|x, y, coverage| {
                                    let xi = x as usize;
                                    let yi = y as usize;
                                    if xi < bw && yi < bh {
                                        cov_ref[yi * bw + xi] = coverage;
                                    }
                                });
                            }

                            // Optional per-char blur on an RGBA scratch buffer
                            let mut src_rgba = vec![0u8; bw * bh * 4];
                            for (i, cv) in cov.iter().enumerate() {
                                src_rgba[i * 4] = r;
                                src_rgba[i * 4 + 1] = g;
                                src_rgba[i * 4 + 2] = b;
                                src_rgba[i * 4 + 3] = (cv * 255.0) as u8;
                            }
                            let blur_px = c.blur.round().max(0.0) as u32;
                            if blur_px >= 1 {
                                crate::core::ae_effects_pack::apply_gaussian_blur(
                                    &mut src_rgba,
                                    bw as u32,
                                    bh as u32,
                                    blur_px.min(16),
                                );
                            }

                            // Destination placement: layout position + animator offset
                            let dest_x = pad as f32
                                + cursor_x
                                + c.position_offset[0]
                                + c.tracking_offset
                                + bounds.min.x;
                            let dest_y = pad as f32
                                + cursor_y
                                + max_top
                                + bounds.min.y
                                + c.position_offset[1];

                            // Transform-blit with scale + rotation about glyph center
                            let sx = c.scale_multiplier[0].max(0.01);
                            let sy = c.scale_multiplier[1].max(0.01);
                            let rad = c.rotation_deg.to_radians();
                            let (cos_r, sin_r) = (rad.cos(), rad.sin());
                            let dst_w = (bounds.width() * sx).ceil() as i32 + 4;
                            let dst_h = (bounds.height() * sy).ceil() as i32 + 4;
                            let cx_dst = dest_x + dst_w as f32 * 0.5;
                            let cy_dst = dest_y + dst_h as f32 * 0.5;
                            for dy in -2..dst_h {
                                for dx in -2..dst_w {
                                    // destination pixel relative to rotated center
                                    let rx = dx as f32 - dst_w as f32 * 0.5;
                                    let ry = dy as f32 - dst_h as f32 * 0.5;
                                    // inverse rotate + inverse scale → source coords
                                    let ux = (rx * cos_r + ry * sin_r) / sx + bounds.width() * 0.5;
                                    let uy =
                                        (-rx * sin_r + ry * cos_r) / sy + bounds.height() * 0.5;
                                    let sxi = ux.floor() as isize;
                                    let syi = uy.floor() as isize;
                                    if sxi < 0
                                        || syi < 0
                                        || sxi >= bw as isize
                                        || syi >= bh as isize
                                    {
                                        continue;
                                    }
                                    let a =
                                        src_rgba[((syi as usize) * bw + (sxi as usize)) * 4 + 3];
                                    if a == 0 {
                                        continue;
                                    }
                                    let pxi = (cx_dst + rx).round() as isize;
                                    let pyi = (cy_dst + ry).round() as isize;
                                    if pxi < 0
                                        || pyi < 0
                                        || pxi >= buf_w as isize
                                        || pyi >= buf_h as isize
                                    {
                                        continue;
                                    }
                                    let out_a =
                                        (a as f32 * c.opacity_multiplier).round().clamp(0.0, 255.0)
                                            as u8;
                                    if out_a == 0 {
                                        continue;
                                    }
                                    let idx = ((pyi as u32 * buf_w + pxi as u32) * 4) as usize;
                                    // Overwrite mode: glyphs don't self-overlap
                                    pixels[idx] = r;
                                    pixels[idx + 1] = g;
                                    pixels[idx + 2] = b;
                                    pixels[idx + 3] = out_a;
                                }
                            }
                        }
                    }
                }

                cursor_x += h_advance + tracking + c.tracking_offset;
                char_idx += 1;
            }
        }

        Some((buf_w, buf_h, pixels))
    }

    #[allow(clippy::too_many_arguments)]
    /// Rasterize text with AnimatorStack: multiple animators composed additively/multiplicatively.
    pub fn rasterize_text_animated_stack(
        &self,
        family_name: &str,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        tracking: f32,
        leading: f32,
        box_width: f32,
        alignment: crate::core::text_layout::TextAlign,
        stack: &crate::core::text_animator_advanced::AnimatorStack,
        _time: f32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let font_data = self.fonts.get(family_name)?;
        let font = parse_font(font_data)?;
        let scaled_font = font.as_scaled(PxScale::from(font_size));

        let flat_text: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
        if flat_text.is_empty() {
            return None;
        }

        let advanced = stack.compose(&flat_text);
        let xforms: Vec<crate::core::text_animator::CharacterTransform> =
            advanced.iter().map(|a| a.base).collect();

        let layout = crate::core::text_layout::layout_text(
            text, font_size, tracking, leading, box_width, alignment,
        );
        if layout.lines.is_empty() {
            return None;
        }

        let pad = (font_size.ceil() as u32 + 96).max(128);
        let buf_w = (layout.total_width.ceil() as u32).max(1) + pad * 2;
        let line_height = font_size * leading;
        let buf_h = ((layout.total_height.ceil() as u32) + pad * 2)
            .max(line_height.ceil() as u32 + pad * 2);
        let mut pixels = vec![0u8; (buf_w * buf_h * 4) as usize];
        let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
        let max_top = font_size * 0.8;

        let mut char_idx: usize = 0;
        for line in &layout.lines {
            let align_x =
                crate::core::text_layout::get_alignment_offset(line.width, box_width, alignment);
            let mut cursor_x = align_x;
            let cursor_y = line.y_offset;

            for ch in line.text.chars() {
                let c = xforms
                    .get(char_idx)
                    .cloned()
                    .unwrap_or_else(crate::core::text_animator::CharacterTransform::default);
                let glyph_id = font.glyph_id(ch);
                let h_advance = scaled_font.h_advance(glyph_id);

                if ch != ' ' && (c.opacity_multiplier > 0.01) {
                    let glyph = glyph_id.with_scale_and_position(
                        PxScale::from(font_size),
                        ab_glyph::point(0.0, 0.0),
                    );
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds = outlined.px_bounds();
                        let bw = bounds.width().ceil() as usize;
                        let bh = bounds.height().ceil() as usize;
                        if bw > 0 && bh > 0 {
                            let mut cov = vec![0f32; bw * bh];
                            outlined.draw(|x, y, coverage| {
                                let xi = x as usize;
                                let yi = y as usize;
                                if xi < bw && yi < bh {
                                    cov[yi * bw + xi] = coverage;
                                }
                            });
                            let (char_r, char_g, char_b) = if let Some(fc) = advanced.get(char_idx).and_then(|a| a.fill_color) {
                                (
                                    (fc[0].clamp(0.0, 1.0) * 255.0) as u8,
                                    (fc[1].clamp(0.0, 1.0) * 255.0) as u8,
                                    (fc[2].clamp(0.0, 1.0) * 255.0) as u8,
                                )
                            } else {
                                (r, g, b)
                            };
                            let mut src_rgba = vec![0u8; bw * bh * 4];
                            for (i, cv) in cov.iter().enumerate() {
                                src_rgba[i * 4] = char_r;
                                src_rgba[i * 4 + 1] = char_g;
                                src_rgba[i * 4 + 2] = char_b;
                                src_rgba[i * 4 + 3] = (cv * 255.0) as u8;
                            }
                            let blur_px = c.blur.round().max(0.0) as u32;
                            if blur_px >= 1 {
                                crate::core::ae_effects_pack::apply_gaussian_blur(
                                    &mut src_rgba,
                                    bw as u32,
                                    bh as u32,
                                    blur_px.min(16),
                                );
                            }

                            let dest_x = pad as f32
                                + cursor_x
                                + c.position_offset[0]
                                + c.tracking_offset
                                + bounds.min.x;
                            let dest_y = pad as f32
                                + cursor_y
                                + max_top
                                + bounds.min.y
                                + c.position_offset[1];

                            let sx = c.scale_multiplier[0].max(0.01);
                            let sy = c.scale_multiplier[1].max(0.01);
                            let rad = c.rotation_deg.to_radians();
                            let (cos_r, sin_r) = (rad.cos(), rad.sin());
                            let dst_w = (bounds.width() * sx).ceil() as i32 + 4;
                            let dst_h = (bounds.height() * sy).ceil() as i32 + 4;
                            let cx_dst = dest_x + dst_w as f32 * 0.5;
                            let cy_dst = dest_y + dst_h as f32 * 0.5;
                            for dy in -2..dst_h {
                                for dx in -2..dst_w {
                                    let rx = dx as f32 - dst_w as f32 * 0.5;
                                    let ry = dy as f32 - dst_h as f32 * 0.5;
                                    let ux = (rx * cos_r + ry * sin_r) / sx + bounds.width() * 0.5;
                                    let uy =
                                        (-rx * sin_r + ry * cos_r) / sy + bounds.height() * 0.5;
                                    let sxi = ux.floor() as isize;
                                    let syi = uy.floor() as isize;
                                    if sxi < 0
                                        || syi < 0
                                        || sxi >= bw as isize
                                        || syi >= bh as isize
                                    {
                                        continue;
                                    }
                                    let sa = cov[(syi as usize) * bw + (sxi as usize)];
                                    if sa < 0.005 {
                                        continue;
                                    }
                                    let pxi = (cx_dst + rx).round() as isize;
                                    let pyi = (cy_dst + ry).round() as isize;
                                    if pxi < 0
                                        || pyi < 0
                                        || pxi >= buf_w as isize
                                        || pyi >= buf_h as isize
                                    {
                                        continue;
                                    }
                                    let out_a = (sa * 255.0 * c.opacity_multiplier)
                                        .round()
                                        .clamp(0.0, 255.0)
                                        as u8;
                                    if out_a == 0 {
                                        continue;
                                    }
                                    let idx = ((pyi as u32 * buf_w + pxi as u32) * 4) as usize;
                                    pixels[idx] = r;
                                    pixels[idx + 1] = g;
                                    pixels[idx + 2] = b;
                                    pixels[idx + 3] = out_a;
                                }
                            }
                        }
                    }
                    cursor_x += h_advance + c.tracking_offset;
                } else {
                    cursor_x += h_advance;
                }
                char_idx += 1;
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
                paths.push(
                    std::path::PathBuf::from(home)
                        .join(format!("Library/Fonts/{}.ttf", family_name)),
                );
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(font_dir) = std::env::var_os("WINDIR") {
                paths.push(
                    std::path::PathBuf::from(font_dir).join(format!("Fonts/{}.ttf", family_name)),
                );
            }
        }

        #[cfg(target_os = "linux")]
        {
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/{}/{}.ttf",
                lower, family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/dejavu/{}.ttf",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/ubuntu/{}.ttf",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/noto/{}.ttf",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/liberation/{}.ttf",
                family_name
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/truetype/{}.ttf",
                lower
            )));
            paths.push(std::path::PathBuf::from(format!(
                "/usr/share/fonts/{}/{}.ttf",
                lower, family_name
            )));
            if let Some(home) = std::env::var_os("HOME") {
                let home_path = std::path::PathBuf::from(home);
                paths.push(home_path.join(format!(".local/share/fonts/{}.ttf", family_name)));
                paths.push(home_path.join(format!(".fonts/{}.ttf", family_name)));
            }
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
        rasterizer.load_all_system_fonts();
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
        for name in &[
            "Helvetica",
            "Arial",
            "DejaVu Sans",
            "Liberation Sans",
            "Inter",
            "Roboto",
        ] {
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
