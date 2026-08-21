use crate::core::timeline::{Composition, LayerType, BlendMode, ShapeType, TrimPaths, TrackMatteMode};
use crate::core::mask::point_in_polygon;
use rayon::prelude::*;

/// Render a sub-composition into a pixel buffer (for PreComp nesting).
pub fn render_sub_comp(_comp: &Composition, sub_comp_id: &str, _frame: u32, _width: u32, _height: u32, time_remapped_frame: u32) -> Option<Vec<u8>> {
    // Find the sub-comp by id in the project (we need to search all compositions)
    // Since Composition doesn't store a reference to other comps, we use a flat lookup approach:
    // The sub-comp is expected to be passed via the project's compositions list.
    // For now, we render a copy of the current comp with only the sub-comp's layers.
    let _ = (sub_comp_id, time_remapped_frame);
    // In a full implementation, this would find the sub-comp and render it recursively.
    // For now, return None to signal that pre-comp nesting is not yet fully wired.
    // The composition system needs a project-level composition registry.
    None
}

/// Render a pre-comp by recursively rendering its layers into a pixel buffer.
/// This is the core of pre-comp nesting support.
pub fn render_precomp_layers(_comp: &Composition, precomp_comp: &Composition, frame: u32, width: u32, height: u32) -> Vec<u8> {
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 { return Vec::new(); }

    let mut buffer = vec![0u8; size];
    // Fill with transparent black
    for p in (0..size).step_by(4) {
        buffer[p] = 0; buffer[p+1] = 0; buffer[p+2] = 0; buffer[p+3] = 0;
    }

    let has_solo = precomp_comp.layers.iter().any(|l| l.is_active(frame) && l.solo);

    for layer in &precomp_comp.layers {
        if !layer.is_active(frame) || !layer.visible { continue; }
        if has_solo && !layer.solo { continue; }

        let effective_frame = layer.remap_frame(frame);
        let (pos, scale, rotation, opacity) = precomp_comp.resolve_world_transform(layer, effective_frame);
        let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
        if l_opacity < 0.001 { continue; }

        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            if !layer.effects.is_empty() {
                crate::core::cpu_effects::apply_layer_effects(&mut buffer, width, height, &layer.effects, effective_frame);
            }
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => (precomp_comp.width as f32, precomp_comp.height as f32),
            LayerType::Text { font_size, text, .. } => (
                (text.chars().count().max(1) as f32 * *font_size as f32 * 0.6).max(*font_size as f32),
                *font_size as f32 * 1.2,
            ),
            LayerType::Shape { .. } | LayerType::Image { .. } => (precomp_comp.width as f32, precomp_comp.height as f32),
            _ => continue,
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Image { .. } => [0.2, 0.6, 0.9, 1.0],
            LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
            _ => continue,
        };

        let rad = rotation.to_radians();
        let (cos_r, sin_r) = rad.sin_cos();
        let cx = pos[0];
        let cy = pos[1];
        let bounds_x = w * 0.5;
        let bounds_y = h * 0.5;

        let min_x = (cx - bounds_x - 2.0).max(0.0) as u32;
        let max_x = (cx + bounds_x + 2.0).min(width as f32) as u32;
        let min_y = (cy - bounds_y - 2.0).max(0.0) as u32;
        let max_y = (cy + bounds_y + 2.0).min(height as f32) as u32;
        let bw = max_x.saturating_sub(min_x);
        let bh = max_y.saturating_sub(min_y);
        if bw == 0 || bh == 0 { continue; }

        let buf_size = (bw * bh * 4) as usize;
        let mut layer_buf = vec![0u8; buf_size];

        match &layer.layer_type {
            LayerType::Shape { shape_type, color, stroke_color, stroke_width } => {
                // SDF shape rendering (same path as the main renderer)
                rasterize_shape_sdf(
                    &mut layer_buf, bw, bh, min_x, min_y,
                    cx, cy, bounds_x, bounds_y,
                    *color, *stroke_color, *stroke_width, l_opacity,
                    shape_type, effective_frame, layer.trim_paths.as_ref(),
                );
            }
            LayerType::Image { path } => {
                // Texture sampling via image cache
                use crate::core::image_cache::with_image_cache;
                let img_path = path.clone();
                with_image_cache(|cache| {
                    if let Some(img) = cache.load_image(&img_path) {
                        let img_w = img.width as f32;
                        let img_h = img.height as f32;
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let dx = px as f32 - cx;
                                let dy = py as f32 - cy;
                                let lx = dx * cos_r + dy * sin_r;
                                let ly = -dx * sin_r + dy * cos_r;
                                let u = (lx / bounds_x + 1.0) * 0.5;
                                let v = (ly / bounds_y + 1.0) * 0.5;
                                #[allow(clippy::manual_range_contains)]
            if u < 0.0 || 1.0 < u || v < 0.0 || 1.0 < v { continue; }
                                let tex_x = ((u * (img_w - 1.0)).round() as u32).min(img.width - 1);
                                let tex_y = ((v * (img_h - 1.0)).round() as u32).min(img.height - 1);
                                let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
                                if tidx + 3 >= img.pixels.len() { continue; }
                                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                if lidx + 3 < layer_buf.len() {
                                    let src_a = (img.pixels[tidx + 3] as f32 / 255.0) * l_opacity;
                                    layer_buf[lidx] = img.pixels[tidx];
                                    layer_buf[lidx+1] = img.pixels[tidx+1];
                                    layer_buf[lidx+2] = img.pixels[tidx+2];
                                    layer_buf[lidx+3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                });
            }
            LayerType::Text { text, font_size, color, font_family, tracking, .. } => {
                // Glyph rendering via font rasterizer
                use crate::core::font_rasterizer::with_font_rasterizer;
                let text_color = *color;
                let text_str = text.clone();
                let fs = *font_size as f32;
                let tk = *tracking;
                let family = font_family.clone();
                with_font_rasterizer(|rasterizer| {
                    let family_name = rasterizer.resolve_family(&family);
                    if let Some((tw, th, text_pixels)) = rasterizer.rasterize_text(&family_name, &text_str, fs, text_color, tk) {
                        let origin_x = (cx - tw as f32 * 0.5) as i32;
                        let origin_y = (cy - th as f32 * 0.5) as i32;
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let tx = px as i32 - origin_x;
                                let ty = py as i32 - origin_y;
                                if tx < 0 || ty < 0 || (tx as u32) >= tw || (ty as u32) >= th { continue; }
                                let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                if tidx + 3 >= text_pixels.len() { continue; }
                                let glyph_a = text_pixels[tidx + 3] as f32 / 255.0;
                                if glyph_a <= 0.001 { continue; }
                                let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                if lidx + 3 < layer_buf.len() {
                                    let src_a = glyph_a * l_opacity;
                                    layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                    layer_buf[lidx+1] = (text_color[1] * 255.0) as u8;
                                    layer_buf[lidx+2] = (text_color[2] * 255.0) as u8;
                                    layer_buf[lidx+3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                });
            }
            _ => {
                // Flat fill for Solid / PreComp / others
                for py in min_y..max_y {
                    for px in min_x..max_x {
                        let dx = px as f32 - cx;
                        let dy = py as f32 - cy;
                        let lx = dx * cos_r + dy * sin_r;
                        let ly = -dx * sin_r + dy * cos_r;
                        if lx >= -bounds_x && lx <= bounds_x && ly >= -bounds_y && ly <= bounds_y {
                            let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                            if lidx + 3 < layer_buf.len() {
                                let src_a = base_color[3] * l_opacity;
                                layer_buf[lidx] = (base_color[0] * 255.0) as u8;
                                layer_buf[lidx+1] = (base_color[1] * 255.0) as u8;
                                layer_buf[lidx+2] = (base_color[2] * 255.0) as u8;
                                layer_buf[lidx+3] = (src_a * 255.0) as u8;
                            }
                        }
                    }
                }
            }
        }

        crate::core::cpu_effects::apply_layer_effects(&mut layer_buf, bw, bh, &layer.effects, effective_frame);

        // Composite onto buffer
        for ly in 0..bh {
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                let src_a = layer_buf[lidx+3] as f32 / 255.0;
                if src_a <= 0.001 { continue; }
                let px = min_x + lx;
                let py = min_y + ly;
                let idx = ((py * width + px) * 4) as usize;
                if idx+3 >= buffer.len() { continue; }
                let src_r = layer_buf[lidx] as f32 / 255.0;
                let src_g = layer_buf[lidx+1] as f32 / 255.0;
                let src_b = layer_buf[lidx+2] as f32 / 255.0;
                let dst_r = buffer[idx] as f32 / 255.0;
                let dst_g = buffer[idx+1] as f32 / 255.0;
                let dst_b = buffer[idx+2] as f32 / 255.0;
                let dst_a = buffer[idx+3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);
                let out_r = if out_a > 0.0 { (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                let out_g = if out_a > 0.0 { (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                let out_b = if out_a > 0.0 { (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                buffer[idx] = (out_r * 255.0) as u8;
                buffer[idx+1] = (out_g * 255.0) as u8;
                buffer[idx+2] = (out_b * 255.0) as u8;
                buffer[idx+3] = (out_a * 255.0) as u8;
            }
        }
    }

    buffer
}

/// Safe buffer size calculation: returns None on overflow or zero dimensions.
pub fn rgba_buffer_size(width: u32, height: u32) -> Option<usize> {
    let w = width as usize;
    let h = height as usize;
    w.checked_mul(h)?.checked_mul(4)
}

// ─── Shape SDF Rasterization ─────────────────────────────────────────────

/// SDF for axis-aligned rectangle centered at origin with half-extents (hx, hy).
fn sdf_rectangle(x: f32, y: f32, hx: f32, hy: f32) -> f32 {
    let dx = x.abs() - hx;
    let dy = y.abs() - hy;
    let outside = (dx.max(0.0), dy.max(0.0));
    let inside = dx.min(0.0).max(dy.min(0.0));
    (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside
}

/// SDF for axis-aligned ellipse centered at origin with radii (rx, ry).
/// Returns negative inside, positive outside.
fn sdf_ellipse(x: f32, y: f32, rx: f32, ry: f32) -> f32 {
    // Normalized distance from center
    let nx = x / rx;
    let ny = y / ry;
    let dist = (nx * nx + ny * ny).sqrt() - 1.0;
    // Scale back to pixel space (approximate)
    dist * rx.min(ry)
}

/// SDF for a star with n points, outer radius, and inner radius.
/// Returns negative inside, positive outside.
fn sdf_star(x: f32, y: f32, points: u32, outer_r: f32, inner_r: f32) -> f32 {
    let angle = y.atan2(x);
    let radius = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (points as f32 * 2.0);
    let half = segment * 0.5;
    let a = ((angle + half) % segment - half).abs();
    let r = inner_r + (outer_r - inner_r) * (a / half).min(1.0);
    radius - r
}

/// SDF for a regular polygon with n sides and radius r.
/// Returns negative inside, positive outside.
fn sdf_polygon(x: f32, y: f32, sides: u32, radius: f32) -> f32 {
    let angle = y.atan2(x);
    let radius_point = (x * x + y * y).sqrt();
    let segment = 2.0 * std::f32::consts::PI / (sides as f32);
    let a = (angle % segment + segment) % segment - segment * 0.5;
    let s = radius * (a.cos() / (std::f32::consts::PI / sides as f32).cos());
    radius_point - s
}

/// Rasterize a shape layer into the layer buffer using SDF.
/// Returns true if any pixels were written.
#[allow(clippy::too_many_arguments)]
fn rasterize_shape_sdf(
    layer_buf: &mut [u8],
    bw: u32,
    bh: u32,
    min_x: u32,
    min_y: u32,
    cx: f32,
    cy: f32,
    bounds_x: f32,
    bounds_y: f32,
    base_color: [f32; 4],
    stroke_color: [f32; 4],
    stroke_width: f32,
    l_opacity: f32,
    shape_type: &ShapeType,
    frame: u32,
    trim_paths: Option<&TrimPaths>,
) -> bool {
    let mut any_written = false;
    for py in 0..bh {
        for px in 0..bw {
            let world_x = min_x + px;
            let world_y = min_y + py;
            let local_x = world_x as f32 - cx;
            let local_y = world_y as f32 - cy;
            // Normalize to [-1, 1]
            let nx = local_x / bounds_x;
            let ny = local_y / bounds_y;

            let dist = match shape_type {
                ShapeType::Rectangle { width, height, corner_radius } => {
                    let w = width.evaluate(frame) / 100.0;
                    let h = height.evaluate(frame) / 100.0;
                    let cr = corner_radius.evaluate(frame) / 100.0;
                    let hx = w * 0.5;
                    let hy = h * 0.5;
                    if cr > 0.01 {
                        // Rounded rectangle SDF (Inigo Quilez formula)
                        let dx = nx.abs() - hx + cr;
                        let dy = ny.abs() - hy + cr;
                        let outside = (dx.max(0.0), dy.max(0.0));
                        let inside = dx.min(0.0).max(dy.min(0.0));
                        (outside.0 * outside.0 + outside.1 * outside.1).sqrt() + inside - cr
                    } else {
                        sdf_rectangle(nx, ny, hx, hy)
                    }
                }
                ShapeType::Ellipse { width, height } => {
                    let w = width.evaluate(frame) / 100.0;
                    let h = height.evaluate(frame) / 100.0;
                    sdf_ellipse(nx, ny, w * 0.5, h * 0.5)
                }
                ShapeType::Star { points, inner_radius, outer_radius } => {
                    let pts = (points.evaluate(frame) as u32).max(3);
                    let ir = inner_radius.evaluate(frame) / 100.0;
                    let or = outer_radius.evaluate(frame) / 100.0;
                    sdf_star(nx, ny, pts, or, ir)
                }
                ShapeType::Polygon { sides, radius } => {
                    let s = (sides.evaluate(frame) as u32).max(3);
                    let r = radius.evaluate(frame) / 100.0;
                    sdf_polygon(nx, ny, s, r)
                }
            };

            // Smooth anti-aa edge: ~4 pixel width in normalized coordinate space
            let pixel_width = 4.0 / bounds_x;
            let mut alpha = (1.0 - (dist / pixel_width).clamp(0.0, 1.0)) * l_opacity;

            // Apply trim paths: angular trim for SDF shapes
            if alpha > 0.001 {
                if let Some(tp) = trim_paths {
                    let angle = ny.atan2(nx); // -PI to PI
                    let angle_norm = (angle / (2.0 * std::f32::consts::PI) + 1.0).fract(); // 0..1
                    let start_pct = tp.start.evaluate(frame).clamp(0.0, 100.0) / 100.0;
                    let end_pct = tp.end.evaluate(frame).clamp(0.0, 100.0) / 100.0;
                    let offset_pct = (tp.offset.evaluate(frame) / 360.0).fract();
                    let s = (start_pct + offset_pct).fract();
                    let e = (end_pct + offset_pct).fract();
                    let in_trim = if s < e {
                        angle_norm >= s && angle_norm <= e
                    } else {
                        angle_norm >= s || angle_norm <= e
                    };
                    if !in_trim {
                        alpha = 0.0;
                    }
                }
            }

            if alpha > 0.001 {
                let lidx = ((py * bw + px) * 4) as usize;
                if lidx + 3 < layer_buf.len() {
                    // Determine fill vs stroke color
                    let (r, g, b, a) = if stroke_width > 0.5 && dist.abs() < stroke_width / bounds_x {
                        // Stroke: render stroke color where dist is near the edge
                        (stroke_color[0], stroke_color[1], stroke_color[2], stroke_color[3] * alpha)
                    } else {
                        // Fill: render fill color inside the shape
                        (base_color[0], base_color[1], base_color[2], base_color[3] * alpha)
                    };
                    layer_buf[lidx] = (r * 255.0) as u8;
                    layer_buf[lidx + 1] = (g * 255.0) as u8;
                    layer_buf[lidx + 2] = (b * 255.0) as u8;
                    layer_buf[lidx + 3] = (a * 255.0) as u8;
                    any_written = true;
                }
            }
        }
    }
    any_written
}

/// Professional CPU-based rasterizer to composite active composition layers
/// into a flat RGBA8 pixel buffer for preview rendering or FFmpeg export.
///
/// Each visible layer is first rasterized into its own straight-alpha RGBA
/// buffer, then its stack of effects (`EffectType`) is applied via the CPU
/// effect pipeline (`core::cpu_effects`), and finally the result is composited
/// over the frame using the layer's blend mode. This keeps spatial effects
/// (blur, twirl, bulge, wipe, ...) correct instead of flattening them away.
pub fn render_frame_to_pixels(comp: &Composition, frame: u32, width: u32, height: u32, exposure_ev: f32, lut_mode: u32) -> Vec<u8> {
    let size = rgba_buffer_size(width, height).unwrap_or(0);
    if size == 0 {
        return Vec::new();
    }
    // Base composition background: Dark gray
    let mut buffer = vec![0u8; size];
    for p in (0..size).step_by(4) {
        buffer[p] = 20;     // R
        buffer[p + 1] = 20; // G
        buffer[p + 2] = 25; // B
        buffer[p + 3] = 255; // A
    }

    let has_solo = comp.layers.iter().any(|l| l.is_active(frame) && l.solo);

    for layer in &comp.layers {
        if !layer.is_active(frame) {
            continue;
        }
        if has_solo && !layer.solo {
            continue;
        }
        if !layer.visible {
            continue;
        }

        // Apply time remap: the layer evaluates its properties at the remapped frame
        let effective_frame = layer.remap_frame(frame);

        // Get world transform properties
        let (pos, scale, rotation, opacity) = comp.resolve_world_transform(layer, effective_frame);

        let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
        if l_opacity < 0.001 {
            continue;
        }

        // Adjustment Layer: apply effects to the composite below
        if matches!(layer.layer_type, LayerType::AdjustmentLayer) {
            if !layer.effects.is_empty() {
                crate::core::cpu_effects::apply_layer_effects(&mut buffer, width, height, &layer.effects, effective_frame);
            }
            continue;
        }

        // Evaluate Vector Mask geometry (if any) — used by all rasterization paths
        let mut mask_vertices = Vec::new();
        let mut mask_feather = 0.0;
        let mut mask_inverted = false;
        for mask in &layer.masks {
            if mask.enabled && mask.mode != crate::core::mask::MaskMode::None {
                mask_vertices = mask.path.to_polygon(frame, 16);
                mask_feather = mask.feather.evaluate(frame);
                mask_inverted = mask.inverted;
                break; // Use the first active mask for clipping
            }
        }

        // PreComp: recursively render the sub-composition, then composite it
        // through the layer's transform (position / scale / rotation / opacity).
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            if let Some(sub_comp) = comp.find_sub_comp(comp_id) {
                let sub_pixels = render_precomp_layers(comp, sub_comp, effective_frame, width, height);
                if !sub_pixels.is_empty() {
                    // Treat the rendered sub-comp as a full-frame texture and
                    // sample it through the inverse layer transform.
                    let pc_rad = rotation.to_radians();
                    let pc_cos = pc_rad.cos();
                    let pc_sin = pc_rad.sin();
                    let pc_cx = pos[0];
                    let pc_cy = pos[1];

                    let pc_base_w = comp.width as f32;
                    let pc_base_h = comp.height as f32;
                    let pc_w = (scale[0].abs() / 100.0) * pc_base_w;
                    let pc_h = (scale[1].abs() / 100.0) * pc_base_h;
                    let pc_bx = pc_w * 0.5;
                    let pc_by = pc_h * 0.5;

                    let lo_x = (pc_cx - pc_bx - 2.0).floor().max(0.0) as u32;
                    let hi_x = (pc_cx + pc_bx + 2.0).ceil().min(width as f32 - 1.0) as u32;
                    let lo_y = (pc_cy - pc_by - 2.0).floor().max(0.0) as u32;
                    let hi_y = (pc_cy + pc_by + 2.0).ceil().min(height as f32 - 1.0) as u32;

                    for py in lo_y..=hi_y {
                        for px in lo_x..=hi_x {
                            // Vector mask check
                            let mut mask_alpha = 1.0;
                            if !mask_vertices.is_empty() {
                                let inside = point_in_polygon(px as f32, py as f32, &mask_vertices);
                                let actual_inside = if mask_inverted { !inside } else { inside };
                                if mask_feather > 0.1 {
                                    let dist = distance_to_polygon(px as f32, py as f32, &mask_vertices);
                                    mask_alpha = if actual_inside {
                                        (dist / mask_feather).clamp(0.0, 1.0)
                                    } else {
                                        (1.0 - dist / mask_feather).clamp(0.0, 1.0)
                                    };
                                } else if !actual_inside {
                                    continue;
                                }
                            }
                            if mask_alpha <= 0.001 { continue; }

                            // Inverse rotation into local space, then UV over sub-comp frame
                            let dx = px as f32 - pc_cx;
                            let dy = py as f32 - pc_cy;
                            let lx = dx * pc_cos + dy * pc_sin;
                            let ly = -dx * pc_sin + dy * pc_cos;
                            let u = (lx / pc_bx + 1.0) * 0.5;
                            let v = (ly / pc_by + 1.0) * 0.5;
                            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) { continue; }

                            let sw = width.max(1);
                            let sh = height.max(1);
                            let sx = ((u * (sw - 1) as f32).round() as u32).min(sw - 1);
                            let sy = ((v * (sh - 1) as f32).round() as u32).min(sh - 1);
                            let src_idx = ((sy * sw + sx) * 4) as usize;
                            if src_idx + 3 >= sub_pixels.len() { continue; }

                            let src_a = sub_pixels[src_idx + 3] as f32 / 255.0 * l_opacity * mask_alpha;
                            if src_a <= 0.001 { continue; }
                            let dst_idx = ((py * width + px) * 4) as usize;
                            if dst_idx + 3 >= buffer.len() { continue; }
                            let src_r = sub_pixels[src_idx] as f32 / 255.0;
                            let src_g = sub_pixels[src_idx + 1] as f32 / 255.0;
                            let src_b = sub_pixels[src_idx + 2] as f32 / 255.0;
                            let dst_r = buffer[dst_idx] as f32 / 255.0;
                            let dst_g = buffer[dst_idx + 1] as f32 / 255.0;
                            let dst_b = buffer[dst_idx + 2] as f32 / 255.0;
                            let dst_a = buffer[dst_idx + 3] as f32 / 255.0;
                            let out_a = src_a + dst_a * (1.0 - src_a);
                            let out_r = if out_a > 0.0 { (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                            let out_g = if out_a > 0.0 { (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                            let out_b = if out_a > 0.0 { (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                            buffer[dst_idx] = (out_r * 255.0) as u8;
                            buffer[dst_idx + 1] = (out_g * 255.0) as u8;
                            buffer[dst_idx + 2] = (out_b * 255.0) as u8;
                            buffer[dst_idx + 3] = (out_a * 255.0) as u8;
                        }
                    }
                }
            }
            continue;
        }

        // Particle layer: deterministic simulation from frame 0 to current frame,
        // then render particles directly into the composite buffer.
        if let LayerType::Particle { emitter } = &layer.layer_type {
            let mut em = emitter.clone();
            // Bake layer opacity into particle colors
            em.color_start[3] *= l_opacity;
            em.color_end[3] *= l_opacity;

            let mut ps = crate::core::particle_system::ParticleSystem::new(em);
            let dt = 1.0 / comp.fps.max(1) as f32;
            // Cap simulation length for performance on very long compositions
            let sim_frames = effective_frame.min(2000);
            for _ in 0..=sim_frames {
                ps.update(dt, pos[0], pos[1]);
            }
            ps.render(&mut buffer, width, height, effective_frame as f32 * dt);

            // Apply the layer's CPU effect stack to the full frame
            crate::core::cpu_effects::apply_layer_effects(&mut buffer, width, height, &layer.effects, effective_frame);
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
            LayerType::Text { font_size, text, .. } => (
                (text.chars().count().max(1) as f32 * *font_size as f32 * 0.6).max(*font_size as f32),
                *font_size as f32 * 1.2,
            ),
            LayerType::Shape { .. } | LayerType::Image { .. } => (comp.width as f32, comp.height as f32),
            _ => continue, // Null or audio layers don't output visual pixels
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Image { .. } => [0.2, 0.6, 0.9, 1.0], // fallback image color
            LayerType::PreComp { .. } => [1.0, 1.0, 1.0, 1.0],
            _ => continue,
        };

        // Extract layer transform matrix metrics for pixel boundaries
        let rad = rotation.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();

        let cx = pos[0];
        let cy = pos[1];

        // Draw each pixel inside the layer bounding area
        // A simple inverse transform to check pixel coverage
        let bounds_x = w * 0.5;
        let bounds_y = h * 0.5;

        // Render loop over the target bounding box
        let min_x = (cx - bounds_x.max(bounds_y) * 1.5).max(0.0) as u32;
        let max_x = (cx + bounds_x.max(bounds_y) * 1.5).min(width as f32) as u32;
        let min_y = (cy - bounds_x.max(bounds_y) * 1.5).max(0.0) as u32;
        let max_y = (cy + bounds_x.max(bounds_y) * 1.5).min(height as f32) as u32;

        let bw = (max_x - min_x).max(1);
        let bh = (max_y - min_y).max(1);

        // Phase 1: rasterize the layer into a local buffer.
        let mut layer_buf = vec![0u8; (bw * bh * 4) as usize];

        // Shape layers: use SDF rasterization instead of flat fill
        if let LayerType::Shape { shape_type, stroke_color, stroke_width, .. } = &layer.layer_type {
            let sc = *stroke_color;
            let sw = *stroke_width;
            if let Some(repeater) = &layer.shape_repeater {
                // Repeater: render shape multiple times with transforms
                use crate::core::shape_repeater::evaluate_shape_repeater;
                let instances = evaluate_shape_repeater(repeater);
                for instance in &instances {
                    // Apply repeater transform to center position
                    let m = &instance.transform_matrix;
                    let rx = cx * m[0][0] + cy * m[0][1] + m[0][2];
                    let ry = cx * m[1][0] + cy * m[1][1] + m[1][2];
                    let mut copy_color = base_color;
                    copy_color[3] *= instance.opacity;
                    rasterize_shape_sdf(
                        &mut layer_buf, bw, bh, min_x, min_y,
                        rx, ry, bounds_x, bounds_y, copy_color, sc, sw, l_opacity,
                        shape_type, effective_frame, layer.trim_paths.as_ref(),
                    );
                }
            } else {
                rasterize_shape_sdf(
                    &mut layer_buf, bw, bh, min_x, min_y,
                    cx, cy, bounds_x, bounds_y, base_color, sc, sw, l_opacity,
                    shape_type, effective_frame, layer.trim_paths.as_ref(),
                );
            }
        } else if let LayerType::Text { text, font_size, color, font_family, tracking, stroke_color, stroke_width, leading, align, text_on_path, .. } = &layer.layer_type {
            // Text layers: rasterize glyphs via ab_glyph and composite
            use crate::core::font_rasterizer::with_font_rasterizer;
            use crate::core::text_layout::TextAlign;

            let text_color = *color;
            let stroke_c = *stroke_color;
            let stroke_w = *stroke_width;
            let text_str = text.clone();
            let fs = *font_size as f32;
            let tk = *tracking;
            let ld = *leading;
            let alignment = match align {
                1 => TextAlign::Center,
                2 => TextAlign::Right,
                _ => TextAlign::Left,
            };
            let family = font_family.clone();

            with_font_rasterizer(|rasterizer| {
                let family_name = rasterizer.resolve_family(&family);

                // ── Text on Path: lay glyphs out along the first mask path ──
                if *text_on_path && !layer.masks.is_empty() {
                    use crate::core::path_text::{layout_text_along_path, PathTextOptions};
                    use crate::core::mask::MaskVertex;

                    let mask = &layer.masks[0];
                    let path_points = mask.path.to_polygon(effective_frame, 12);
                    if path_points.len() >= 2 {
                        let verts: Vec<MaskVertex> = path_points.windows(2).map(|w| MaskVertex {
                            position: w[0],
                            tangent_in: [0.0; 2],
                            tangent_out: [w[1][0] - w[0][0], w[1][1] - w[0][1]],
                        }).collect();

                        let glyphs = layout_text_along_path(&text_str, fs, &verts, mask.path.is_closed, &PathTextOptions::default());
                        for g in &glyphs {
                            let Some(rg) = rasterizer.rasterize_glyph(&family_name, g.char_code, fs) else { continue };
                            if rg.width == 0 || rg.height == 0 { continue; }

                            // Glyph center in comp coordinates, rotated by path tangent
                            let angle = g.rotation_deg.to_radians();
                            let cos_a = angle.cos();
                            let sin_a = angle.sin();
                            let gcx = g.position[0];
                            let gcy = g.position[1] + fs * 0.35; // approximate baseline centering

                            // Rotated blit over the bounding box of the rotated glyph
                            let half_diag = (rg.width.max(rg.height) as f32 * 0.5 * std::f32::consts::SQRT_2).ceil();
                            let gx0 = (gcx - half_diag).floor().max(0.0) as u32;
                            let gy0 = (gcy - half_diag).floor().max(0.0) as u32;
                            let gx1 = (gcx + half_diag).ceil().min(width as f32) as u32;
                            let gy1 = (gcy + half_diag).ceil().min(height as f32) as u32;

                            for py in gy0..gy1 {
                                for px in gx0..gx1 {
                                    // Inverse-rotate the destination point into glyph space
                                    let dx = px as f32 + 0.5 - gcx;
                                    let dy = py as f32 + 0.5 - gcy;
                                    let lx = dx * cos_a + dy * sin_a;
                                    let ly = -dx * sin_a + dy * cos_a;
                                    // Glyph local coords: center the bitmap around the path point
                                    let tx = lx + rg.width as f32 * 0.5 + rg.left as f32;
                                    let ty = ly + rg.height as f32 * 0.5 + rg.top as f32;
                                    if tx < 0.0 || ty < 0.0 || tx >= rg.width as f32 || ty >= rg.height as f32 { continue; }
                                    let tidx = ((ty as u32 * rg.width + tx as u32) * 4) as usize;
                                    if tidx + 3 >= rg.pixels.len() { continue; }
                                    let cov = rg.pixels[tidx + 3] as f32 / 255.0;
                                    if cov <= 0.001 { continue; }

                                    let src_a = cov * l_opacity;
                                    if src_a <= 0.001 { continue; }
                                    let didx = (((py * width) + px) * 4) as usize;
                                    if didx + 3 >= buffer.len() { continue; }
                                    // Straight-alpha over using glyph coverage tinted with text color
                                    let inv = 1.0 - src_a;
                                    buffer[didx]     = (text_color[0] * 255.0 * src_a + buffer[didx] as f32 * inv) as u8;
                                    buffer[didx + 1] = (text_color[1] * 255.0 * src_a + buffer[didx + 1] as f32 * inv) as u8;
                                    buffer[didx + 2] = (text_color[2] * 255.0 * src_a + buffer[didx + 2] as f32 * inv) as u8;
                                    buffer[didx + 3] = ((src_a + buffer[didx + 3] as f32 / 255.0 * inv) * 255.0) as u8;
                                }
                            }
                        }
                    }
                    return;
                }

                if let Some((tw, th, text_pixels)) = rasterizer.rasterize_text_formatted(&family_name, &text_str, fs, text_color, tk, ld, 0.0, alignment) {
                    let text_w = tw as i32;
                    let text_h = th as i32;
                    let origin_x = (cx - tw as f32 * 0.5) as i32;
                    let origin_y = (cy - th as f32 * 0.5) as i32;
                    let stroke_radius = (stroke_w * 0.5).ceil() as i32;

                    for py in min_y..max_y {
                        for px in min_x..max_x {
                            // Vector mask check
                            let mut mask_alpha = 1.0;
                            if !mask_vertices.is_empty() {
                                let is_inside = point_in_polygon(px as f32, py as f32, &mask_vertices);
                                let actual_inside = if mask_inverted { !is_inside } else { is_inside };
                                if mask_feather > 0.1 {
                                    let dist = distance_to_polygon(px as f32, py as f32, &mask_vertices);
                                    mask_alpha = if actual_inside {
                                        (dist / mask_feather).clamp(0.0, 1.0)
                                    } else {
                                        (1.0 - (dist / mask_feather)).clamp(0.0, 1.0)
                                    };
                                } else if !actual_inside {
                                    continue;
                                }
                            }
                            if mask_alpha <= 0.001 { continue; }

                            let tx = px as i32 - origin_x;
                            let ty = py as i32 - origin_y;

                            // Sample fill alpha
                            let fill_alpha = if tx >= 0 && ty >= 0 && tx < text_w && ty < text_h {
                                let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                if tidx + 3 < text_pixels.len() {
                                    text_pixels[tidx + 3] as f32 / 255.0
                                } else { 0.0 }
                            } else { 0.0 };

                            // Sample stroke alpha: check if any neighbor within radius has fill
                            let mut stroke_alpha = 0.0f32;
                            if stroke_w > 0.1 && fill_alpha < 0.001 {
                                for dy in -stroke_radius..=stroke_radius {
                                    for dx in -stroke_radius..=stroke_radius {
                                        let nx = tx + dx;
                                        let ny = ty + dy;
                                        if nx >= 0 && ny >= 0 && nx < text_w && ny < text_h {
                                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                            if dist <= stroke_w * 0.5 {
                                                let nidx = ((ny as u32 * tw + nx as u32) * 4) as usize;
                                                if nidx + 3 < text_pixels.len() {
                                                    let n_alpha = text_pixels[nidx + 3] as f32 / 255.0;
                                                    if n_alpha > 0.001 {
                                                        let edge_dist = stroke_w * 0.5 - dist;
                                                        stroke_alpha = stroke_alpha.max((edge_dist / (stroke_w * 0.25)).clamp(0.0, 1.0));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Composite: stroke behind fill
                            let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                            if lidx + 3 < layer_buf.len() {
                                if stroke_alpha > 0.001 {
                                    let src_a = stroke_alpha * l_opacity * mask_alpha;
                                    layer_buf[lidx] = (stroke_c[0] * 255.0) as u8;
                                    layer_buf[lidx + 1] = (stroke_c[1] * 255.0) as u8;
                                    layer_buf[lidx + 2] = (stroke_c[2] * 255.0) as u8;
                                    layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                }
                                if fill_alpha > 0.001 {
                                    let src_a = fill_alpha * l_opacity * mask_alpha;
                                    layer_buf[lidx] = (text_color[0] * 255.0) as u8;
                                    layer_buf[lidx + 1] = (text_color[1] * 255.0) as u8;
                                    layer_buf[lidx + 2] = (text_color[2] * 255.0) as u8;
                                    layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                }
            });
        } else if let LayerType::Image { path } = &layer.layer_type {
            // Image layers: load from disk and sample pixels
            use crate::core::image_cache::with_image_cache;

            let img_path = path.clone();
            with_image_cache(|cache| {
                if let Some(img) = cache.load_image(&img_path) {
                    let img_w = img.width as f32;
                    let img_h = img.height as f32;

                    for py in min_y..max_y {
                        for px in min_x..max_x {
                            // Vector mask check
                            let mut mask_alpha = 1.0;
                            if !mask_vertices.is_empty() {
                                let is_inside = point_in_polygon(px as f32, py as f32, &mask_vertices);
                                let actual_inside = if mask_inverted { !is_inside } else { is_inside };
                                if mask_feather > 0.1 {
                                    let dist = distance_to_polygon(px as f32, py as f32, &mask_vertices);
                                    mask_alpha = if actual_inside {
                                        (dist / mask_feather).clamp(0.0, 1.0)
                                    } else {
                                        (1.0 - (dist / mask_feather)).clamp(0.0, 1.0)
                                    };
                                } else if !actual_inside {
                                    continue;
                                }
                            }
                            if mask_alpha <= 0.001 { continue; }

                            // Map pixel to image texture coordinates [0, 1]
                            let dx = px as f32 - cx;
                            let dy = py as f32 - cy;
                            let lx = dx * cos_r + dy * sin_r;
                            let ly = -dx * sin_r + dy * cos_r;
                            let u = (lx / bounds_x + 1.0) * 0.5;
                            let v = (ly / bounds_y + 1.0) * 0.5;

                            if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                                let tex_x = ((u * (img_w - 1.0)).round() as u32).min(img_w as u32 - 1);
                                let tex_y = ((v * (img_h - 1.0)).round() as u32).min(img_h as u32 - 1);
                                let tidx = ((tex_y * img.width + tex_x) * 4) as usize;
                                if tidx + 3 < img.pixels.len() {
                                    let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                                    if lidx + 3 < layer_buf.len() {
                                        let src_a = (img.pixels[tidx + 3] as f32 / 255.0) * l_opacity * mask_alpha;
                                        layer_buf[lidx] = img.pixels[tidx];
                                        layer_buf[lidx + 1] = img.pixels[tidx + 1];
                                        layer_buf[lidx + 2] = img.pixels[tidx + 2];
                                        layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        } else {
            // Other non-shape layers: flat rasterization with mask support
            for py in min_y..max_y {
                for px in min_x..max_x {
                    // Vector mask check with feathering support
                    let mut mask_alpha = 1.0;
                    if !mask_vertices.is_empty() {
                        let is_inside = point_in_polygon(px as f32, py as f32, &mask_vertices);
                        let actual_inside = if mask_inverted { !is_inside } else { is_inside };

                        if mask_feather > 0.1 {
                            let dist = distance_to_polygon(px as f32, py as f32, &mask_vertices);
                            if actual_inside {
                                mask_alpha = (dist / mask_feather).clamp(0.0, 1.0);
                            } else {
                                mask_alpha = (1.0 - (dist / mask_feather)).clamp(0.0, 1.0);
                            }
                        } else if !actual_inside {
                            continue;
                        }
                    }

                    if mask_alpha <= 0.001 {
                        continue; // fully masked out pixel
                    }

                    // Inverse rotation & scale transform to local space
                    let dx = px as f32 - cx;
                    let dy = py as f32 - cy;
                    let lx = dx * cos_r + dy * sin_r;
                    let ly = -dx * sin_r + dy * cos_r;

                    if lx >= -bounds_x && lx <= bounds_x && ly >= -bounds_y && ly <= bounds_y {
                        let lidx = (((py - min_y) * bw + (px - min_x)) * 4) as usize;
                        if lidx + 3 >= layer_buf.len() {
                            continue;
                        }
                        let src_a = base_color[3] * l_opacity * mask_alpha;
                        layer_buf[lidx] = (base_color[0] * 255.0) as u8;
                        layer_buf[lidx + 1] = (base_color[1] * 255.0) as u8;
                        layer_buf[lidx + 2] = (base_color[2] * 255.0) as u8;
                        layer_buf[lidx + 3] = (src_a * 255.0) as u8;
                    }
                }
            }
        }

        // Phase 2: apply the layer's CPU effect stack.
        crate::core::cpu_effects::apply_layer_effects(&mut layer_buf, bw, bh, &layer.effects, effective_frame);

        // Phase 2.5: velocity-based motion blur (AE-style shutter angle).
        // Computes the layer's positional velocity across neighboring frames and
        // smears the layer buffer along the motion vector.
        if layer.motion_blur && comp.motion_blur_shutter_angle > 0.0 {
            let fps = comp.fps.max(1);
            let f_prev = effective_frame.saturating_sub(1);
            let f_next = effective_frame.saturating_add(1);
            let (p_prev, _, _, _) = comp.resolve_world_transform(layer, f_prev);
            let (p_next, _, _, _) = comp.resolve_world_transform(layer, f_next);
            let vel_x = (p_next[0] - p_prev[0]) * 0.5;
            let vel_y = (p_next[1] - p_prev[1]) * 0.5;
            let speed = (vel_x * vel_x + vel_y * vel_y).sqrt();
            if speed > 0.05 {
                let shutter = (comp.motion_blur_shutter_angle / 360.0).clamp(0.0, 1.0);
                let samples = ((speed * shutter).ceil() as u32).clamp(2, 32);
                crate::core::ae_effects_pack_v17::apply_motion_blur_vector(
                    &mut layer_buf, bw, bh,
                    vel_x * shutter * fps as f32 / 24.0, vel_y * shutter * fps as f32 / 24.0,
                    samples,
                );
            }
        }

        // Phase 2.6: 3D light shading for 3D layers.
        // Lambertian diffuse from each active light in the composition.
        if layer.is_3d {
            let mut shade = 0.35f32; // ambient floor
            for light in &comp.lights {
                let lpos = light.position.evaluate(effective_frame);
                let lx = cx - lpos[0];
                let ly = cy - lpos[1];
                let lz = 0.0 - lpos[2]; // layer plane sits at z=0
                let dist = (lx * lx + ly * ly + lz * lz).sqrt().max(1.0);
                // N·L with flat normal facing the camera (+z)
                let ndotl = (lz / dist).max(0.0);
                let attenuation = (light.intensity / 100.0) / (1.0 + dist / 1000.0);
                shade += ndotl * attenuation;
            }
            shade = shade.clamp(0.0, 2.0);
            if (shade - 1.0).abs() > 0.01 {
                for px_chunk in layer_buf.chunks_exact_mut(4) {
                    px_chunk[0] = ((px_chunk[0] as f32 * shade).min(255.0)) as u8;
                    px_chunk[1] = ((px_chunk[1] as f32 * shade).min(255.0)) as u8;
                    px_chunk[2] = ((px_chunk[2] as f32 * shade).min(255.0)) as u8;
                }
            }
        }

        // Phase 3: composite the (effect-processed) buffer over the frame.
        // First, check if this layer uses a track matte from the layer below.
        let matte_pixels = if layer.track_matte != TrackMatteMode::None {
            // Find the layer below this one (the matte source)
            let layer_idx = comp.layers.iter().position(|l| l.id == layer.id);
            if let Some(idx) = layer_idx {
                if idx > 0 {
                    let matte_layer = &comp.layers[idx - 1];
                    if matte_layer.is_active(frame) && matte_layer.visible {
                        let m_frame = matte_layer.remap_frame(frame);
                        let (m_pos, _m_scale, m_rot, m_opa) = comp.resolve_world_transform(matte_layer, m_frame);
                        let m_opacity = (m_opa / 100.0).clamp(0.0, 1.0);
                        let m_rad = m_rot.to_radians();
                        let _m_cos = m_rad.cos();
                        let _m_sin = m_rad.sin();
                        let m_cx = m_pos[0];
                        let m_cy = m_pos[1];
                        let m_bw = width;
                        let m_bh = height;
                        let mut m_buf = vec![0u8; (m_bw * m_bh * 4) as usize];

                        // Render matte layer to get its alpha/luma
                        match &matte_layer.layer_type {
                            LayerType::Solid { color } => {
                                for py in 0..m_bh {
                                    for px in 0..m_bw {
                                        let idx = ((py * m_bw + px) * 4) as usize;
                                        if idx + 3 < m_buf.len() {
                                            m_buf[idx] = (color[0] * 255.0) as u8;
                                            m_buf[idx+1] = (color[1] * 255.0) as u8;
                                            m_buf[idx+2] = (color[2] * 255.0) as u8;
                                            m_buf[idx+3] = (color[3] * m_opacity * 255.0) as u8;
                                        }
                                    }
                                }
                            }
                            LayerType::Text { text, font_size, color, font_family, tracking, .. } => {
                                use crate::core::font_rasterizer::with_font_rasterizer;
                                let text_color = *color;
                                let text_str = text.clone();
                                let fs = *font_size as f32;
                                let tk = *tracking;
                                let family = font_family.clone();
                                with_font_rasterizer(|rasterizer| {
                                    let family_name = rasterizer.resolve_family(&family);
                                    if let Some((tw, th, text_pixels)) = rasterizer.rasterize_text(&family_name, &text_str, fs, text_color, tk) {
                                        let origin_x = (m_cx - tw as f32 * 0.5) as i32;
                                        let origin_y = (m_cy - th as f32 * 0.5) as i32;
                                        for py in 0..m_bh {
                                            for px in 0..m_bw {
                                                let tx = px as i32 - origin_x;
                                                let ty = py as i32 - origin_y;
                                                if tx >= 0 && ty >= 0 && (tx as u32) < tw && (ty as u32) < th {
                                                    let tidx = ((ty as u32 * tw + tx as u32) * 4) as usize;
                                                    let lidx = ((py * m_bw + px) * 4) as usize;
                                                    if tidx + 3 < text_pixels.len() && lidx + 3 < m_buf.len() {
                                                        m_buf[lidx] = text_pixels[tidx];
                                                        m_buf[lidx+1] = text_pixels[tidx+1];
                                                        m_buf[lidx+2] = text_pixels[tidx+2];
                                                        m_buf[lidx+3] = (text_pixels[tidx+3] as f32 * m_opacity) as u8;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                            _ => {
                                // For other matte layer types, render as solid white (full matte)
                                for py in 0..m_bh {
                                    for px in 0..m_bw {
                                        let idx = ((py * m_bw + px) * 4) as usize;
                                        if idx + 3 < m_buf.len() {
                                            m_buf[idx] = 255;
                                            m_buf[idx+1] = 255;
                                            m_buf[idx+2] = 255;
                                            m_buf[idx+3] = (m_opacity * 255.0) as u8;
                                        }
                                    }
                                }
                            }
                        }
                        Some(m_buf)
                    } else { None }
                } else { None }
            } else { None }
        } else { None };

        for ly in 0..bh {
            for lx in 0..bw {
                let lidx = ((ly * bw + lx) * 4) as usize;
                let mut src_a = layer_buf[lidx + 3] as f32 / 255.0;
                if src_a <= 0.001 {
                    continue;
                }
                let px = min_x + lx;
                let py = min_y + ly;
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 >= buffer.len() {
                    continue;
                }

                // Apply track matte masking
                if let Some(ref matte_buf) = matte_pixels {
                    let mx = px.min(width - 1);
                    let my = py.min(height - 1);
                    let midx = ((my * width + mx) * 4) as usize;
                    if midx + 3 < matte_buf.len() {
                        let matte_a = matte_buf[midx + 3] as f32 / 255.0;
                        let matte_luma = (matte_buf[midx] as f32 * 0.299 + matte_buf[midx+1] as f32 * 0.587 + matte_buf[midx+2] as f32 * 0.114) / 255.0;
                        src_a = match layer.track_matte {
                            TrackMatteMode::AlphaMatte => src_a * matte_a,
                            TrackMatteMode::AlphaMatteInverted => src_a * (1.0 - matte_a),
                            TrackMatteMode::LumaMatte => src_a * matte_luma,
                            TrackMatteMode::LumaMatteInverted => src_a * (1.0 - matte_luma),
                            _ => src_a,
                        };
                        if src_a <= 0.001 { continue; }
                    }
                }

                let src_r = layer_buf[lidx] as f32 / 255.0;
                let src_g = layer_buf[lidx + 1] as f32 / 255.0;
                let src_b = layer_buf[lidx + 2] as f32 / 255.0;

                let dst_r = buffer[idx] as f32 / 255.0;
                let dst_g = buffer[idx + 1] as f32 / 255.0;
                let dst_b = buffer[idx + 2] as f32 / 255.0;
                let dst_a = buffer[idx + 3] as f32 / 255.0;

                // Compute BlendMode calculations
                let (blended_r, blended_g, blended_b) = match layer.blend_mode {
                    BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
                    BlendMode::Screen => (1.0 - (1.0 - src_r) * (1.0 - dst_r), 1.0 - (1.0 - src_g) * (1.0 - dst_g), 1.0 - (1.0 - src_b) * (1.0 - dst_b)),
                    BlendMode::Overlay => (
                        if dst_r < 0.5 { 2.0 * src_r * dst_r } else { 1.0 - 2.0 * (1.0 - src_r) * (1.0 - dst_r) },
                        if dst_g < 0.5 { 2.0 * src_g * dst_g } else { 1.0 - 2.0 * (1.0 - src_g) * (1.0 - dst_g) },
                        if dst_b < 0.5 { 2.0 * src_b * dst_b } else { 1.0 - 2.0 * (1.0 - src_b) * (1.0 - dst_b) },
                    ),
                    BlendMode::Add => ((src_r + dst_r).min(1.0), (src_g + dst_g).min(1.0), (src_b + dst_b).min(1.0)),
                    BlendMode::Darken => (src_r.min(dst_r), src_g.min(dst_g), src_b.min(dst_b)),
                    BlendMode::Lighten => (src_r.max(dst_r), src_g.max(dst_g), src_b.max(dst_b)),
                    BlendMode::SoftLight => {
                        let f = |s: f32, d: f32| {
                            if s <= 0.5 {
                                d - (1.0 - 2.0 * s) * d * (1.0 - d)
                            } else {
                                let a = if d <= 0.25 {
                                    ((16.0 * d - 12.0) * d + 4.0) * d
                                } else {
                                    d.sqrt()
                                };
                                d + (2.0 * s - 1.0) * (a - d)
                            }
                        };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::HardLight => {
                        let f = |s: f32, d: f32| if s <= 0.5 { 2.0 * s * d } else { 1.0 - 2.0 * (1.0 - s) * (1.0 - d) };
                        (f(src_r, dst_r), f(src_g, dst_g), f(src_b, dst_b))
                    }
                    BlendMode::Difference => ((dst_r - src_r).abs(), (dst_g - src_g).abs(), (dst_b - src_b).abs()),
                    BlendMode::Exclusion => (src_r + dst_r - 2.0 * src_r * dst_r, src_g + dst_g - 2.0 * src_g * dst_g, src_b + dst_b - 2.0 * src_b * dst_b),
                    BlendMode::Divide => ((src_r / dst_r.max(1e-6)).clamp(0.0, 1.0), (src_g / dst_g.max(1e-6)).clamp(0.0, 1.0), (src_b / dst_b.max(1e-6)).clamp(0.0, 1.0)),
                    BlendMode::Subtract => ((src_r - dst_r).max(0.0), (src_g - dst_g).max(0.0), (src_b - dst_b).max(0.0)),
                    BlendMode::Normal => (src_r, src_g, src_b),
                };

                // Alpha blending formula: Standard Source-Over
                let out_a = src_a + dst_a * (1.0 - src_a);
                let out_r = if out_a > 0.0 { (blended_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                let out_g = if out_a > 0.0 { (blended_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };
                let out_b = if out_a > 0.0 { (blended_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a } else { 0.0 };

                buffer[idx] = (out_r * 255.0) as u8;
                buffer[idx + 1] = (out_g * 255.0) as u8;
                buffer[idx + 2] = (out_b * 255.0) as u8;
                buffer[idx + 3] = (out_a * 255.0) as u8;
            }
        }
    }

    // Apply exposure EV shift and LUT color mapping in parallel across CPU cores
    let mult = 2.0f32.powf(exposure_ev);
    buffer.par_chunks_exact_mut(4).for_each(|p| {
        let mut r = p[0] as f32 / 255.0 * mult;
        let mut g = p[1] as f32 / 255.0 * mult;
        let mut b = p[2] as f32 / 255.0 * mult;

        if lut_mode == 1 {
            // Linear sRGB conversion (2.2 Gamma linearize)
            r = r.powf(2.2);
            g = g.powf(2.2);
            b = b.powf(2.2);
        } else if lut_mode == 2 {
            // ACEScg filmic tone mapping curve
            let a = 2.51f32;
            let b_val = 0.03f32;
            let c = 2.43f32;
            let d = 0.59f32;
            let e = 0.14f32;
            let denom_r = (r * (c * r + d) + e).max(1e-6);
            let denom_g = (g * (c * g + d) + e).max(1e-6);
            let denom_b = (b * (c * b + d) + e).max(1e-6);
            r = (r * (a * r + b_val)) / denom_r;
            g = (g * (a * g + b_val)) / denom_g;
            b = (b * (a * b + b_val)) / denom_b;
        }

        p[0] = (r.clamp(0.0, 1.0) * 255.0) as u8;
        p[1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
        p[2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
    });

    buffer
}

/// Calculate the shortest distance from point (px, py) to the polygon boundary.
fn distance_to_polygon(px: f32, py: f32, verts: &[[f32; 2]]) -> f32 {
    let mut min_dist = f32::INFINITY;
    let n = verts.len();
    if n < 2 { return 0.0; }
    for i in 0..n {
        let p1 = verts[i];
        let p2 = verts[(i + 1) % n];

        let dx = p2[0] - p1[0];
        let dy = p2[1] - p1[1];
        let l2 = dx * dx + dy * dy;
        if l2 < 1e-5 {
            let d = ((px - p1[0]).powi(2) + (py - p1[1]).powi(2)).sqrt();
            min_dist = min_dist.min(d);
            continue;
        }
        let t = ((px - p1[0]) * dx + (py - p1[1]) * dy) / l2;
        let t = t.clamp(0.0, 1.0);
        let proj_x = p1[0] + t * dx;
        let proj_y = p1[1] + t * dy;
        let d = ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt();
        min_dist = min_dist.min(d);
    }
    min_dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType, BlendMode, Effect, EffectType};
    use crate::core::property::Animatable;
    

    #[test]
    fn test_software_render_frame_to_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        layer.blend_mode = BlendMode::Multiply;
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 100, 100, 0.0, 0);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_particle_layer_renders() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new("p1".to_string(), "Particles".to_string(), LayerType::Particle {
            emitter: crate::core::particle_system::ParticleEmitter {
                rate: 500.0,
                lifetime: 1.0,
                speed: 50.0,
                size_start: 6.0,
                size_end: 3.0,
                ..Default::default()
            },
        }, 30);
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        // Frame 10 should have particles simulated and rendered
        let pixels = render_frame_to_pixels(&comp, 10, 64, 64, 0.0, 0);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // Some pixels should be brighter than the dark background (20,20,25)
        let bright = pixels.chunks_exact(4).filter(|p| p[0] > 40).count();
        assert!(bright > 0, "Particle layer should produce visible pixels");
    }

    #[test]
    fn test_particle_layer_deterministic() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new("p1".to_string(), "Particles".to_string(), LayerType::Particle {
            emitter: crate::core::particle_system::ParticleEmitter::default(),
        }, 30);
        layer.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let p1 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
        let p2 = render_frame_to_pixels(&comp, 5, 64, 64, 0.0, 0);
        assert_eq!(p1, p2, "Particle simulation must be deterministic per frame");
    }

    #[test]
    fn test_motion_blur_smears_moving_layer() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 100, 100, 30, 30);
        let mut layer = Layer::new("m1".to_string(), "Moving Solid".to_string(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        layer.motion_blur = true;
        // Move horizontally: position keyframes at x=20 (frame 5) → x=80 (frame 15)
        layer.transform.position = Animatable::new_animated(vec![
            crate::core::keyframe::Keyframe::new(5, [20.0, 50.0], crate::core::keyframe::InterpolationType::Linear),
            crate::core::keyframe::Keyframe::new(15, [80.0, 50.0], crate::core::keyframe::InterpolationType::Linear),
        ]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 10, 100, 100, 0.0, 0);
        assert_eq!(pixels.len(), 100 * 100 * 4);
        // Motion-blurred moving layer should still render without panic
    }

    #[test]
    fn test_3d_light_shading_applied() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new("l3d".to_string(), "3D Solid".to_string(), LayerType::Solid { color: [1.0, 1.0, 1.0, 1.0] }, 30);
        layer.is_3d = true;
        comp.layers.push(layer);

        // Strong light near the layer center should brighten it
        comp.lights[0].intensity = 200.0;
        comp.lights[0].position = Animatable::new_constant([16.0, 16.0, -300.0]);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        // Center pixel should be lit (brighter than ambient-only floor)
        let center_idx = ((16 * 32 + 16) * 4) as usize;
        assert!(pixels[center_idx] > 60, "3D layer should be shaded by light, got {}", pixels[center_idx]);
    }


    #[test]
    fn test_text_on_path_renders_glyphs() {
        use crate::core::mask::{Mask, MaskPath};
        use crate::core::property::Animatable as PAnimatable;

        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 128, 64, 30, 30);
        let mut layer = Layer::new(
            "tp".to_string(), "Path Text".to_string(),
            LayerType::Text {
                text: "AB".to_string(),
                font_size: 24,
                color: [1.0, 1.0, 1.0, 1.0],
                font_family: "Helvetica".to_string(),
                tracking: 0.0,
                leading: 1.2,
                align: 0,
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                text_on_path: true,
            },
            30,
        );
        // Gentle horizontal wave path across the comp
        let path = MaskPath {
            vertices: PAnimatable::new_constant(vec![[16.0, 32.0], [48.0, 20.0], [80.0, 44.0], [112.0, 32.0]]),
            tangents: None,
            is_closed: false,
        };
        layer.masks.push(Mask {
            path,
            ..Mask::new_rect("m1".to_string(), "Path".to_string(), 0.0, 0.0, 10.0, 10.0)
        });
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 128, 64, 0.0, 0);
        let bright = (0..pixels.len()).step_by(4)
            .filter(|&i| pixels[i] > 200 && pixels[i + 1] > 200 && pixels[i + 2] > 200)
            .count();
        assert!(bright > 20, "text-on-path glyphs should render bright pixels, got {}", bright);
    }

    #[test]
    fn test_gpu_parent_transform_resolution() {
        // Parented layer should resolve through resolve_world_transform without panicking
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut parent = Layer::new("par".to_string(), "Parent".to_string(), LayerType::Null, 30);
        parent.transform.position = Animatable::new_constant([32.0, 32.0]);
        parent.transform.rotation = Animatable::new_constant(45.0);
        comp.layers.push(parent);
        let mut child = Layer::new("chi".to_string(), "Child".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        child.parent_id = Some("par".to_string());
        child.transform.position = Animatable::new_constant([10.0, 0.0]);
        let child_pos = child.transform.position.evaluate(0);
        comp.layers.push(child);

        let (pos, _scale, _rot, _opa) = comp.resolve_world_transform(comp.layers.last().unwrap(), 0);
        let _ = child_pos;
        // Child at local (10,0) rotated 45deg around parent origin (32,32):
        // offset = (10*cos45, 10*sin45) ≈ (7.07, 7.07) → world ≈ (39.07, 39.07)
        assert!((pos[0] - 39.07).abs() < 0.5, "unexpected world x: {}", pos[0]);
        assert!((pos[1] - 39.07).abs() < 0.5, "unexpected world y: {}", pos[1]);
    }

    #[test]
    fn test_precomp_nested_rendering() {
        // Sub-comp: red ellipse at its own center
        let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
        let mut shape = Layer::new("s1".to_string(), "Dot".to_string(), LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(40.0),
                height: Animatable::new_constant(40.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 0.0,
        }, 30);
        shape.transform.position = Animatable::new_constant([32.0, 32.0]);
        sub.layers.push(shape);

        // Main comp: pre-comp layer referencing the sub-comp
        let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
        comp.sub_compositions.push(sub);
        let mut pc = Layer::new("pc".to_string(), "Nested".to_string(), LayerType::PreComp { comp_id: "sub".to_string() }, 30);
        pc.transform.position = Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(pc);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(pixels[center] > 180, "PreComp should render nested shape (R={})", pixels[center]);
    }

    #[test]
    fn test_precomp_respects_layer_scale() {
        use crate::core::timeline::ShapeType;

        // Sub-comp: full-frame red solid
        let mut sub = Composition::new("sub".to_string(), "Sub".to_string(), 64, 64, 30, 30);
        let solid = Layer::new("bg".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        sub.layers.push(solid);

        // Main comp: pre-comp scaled to 50% — corners should stay background
        let mut comp = Composition::new("main".to_string(), "Main".to_string(), 64, 64, 30, 30);
        comp.sub_compositions.push(sub);
        let mut pc = Layer::new("pc".to_string(), "Half".to_string(), LayerType::PreComp { comp_id: "sub".to_string() }, 30);
        pc.transform.position = Animatable::new_constant([32.0, 32.0]);
        pc.transform.scale = Animatable::new_constant([50.0, 50.0]);
        comp.layers.push(pc);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let corner = 0usize;
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(pixels[corner] < 60, "Corner should be background when pre-comp is scaled down (R={})", pixels[corner]);
        assert!(pixels[center] > 180, "Center should show scaled pre-comp content (R={})", pixels[center]);
    }

    #[test]
    fn test_visual_headless_pixel_comparison() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        let layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] }, 30);
        comp.layers.push(layer);

        let p1 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        let p2 = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);

        let mut mse = 0.0f32;
        for i in 0..p1.len() {
            let diff = (p1[i] as f32 - p2[i] as f32) / 255.0;
            mse += diff * diff;
        }
        mse /= p1.len() as f32;

        assert!(
            mse < 1e-5,
            "Visual regression test failed! Pixel MSE ({}) exceeds threshold 1e-5",
            mse
        );
    }

    #[test]
    fn test_invert_effect_changes_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 20, 20, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [0.8, 0.2, 0.4, 1.0] }, 30);
        layer.effects.push(Effect {
            id: "e1".to_string(),
            name: "Invert".to_string(),
            effect_type: EffectType::Invert { invert_alpha: false },
            enabled: true,
        });
        comp.layers.push(layer);

        let inverted = render_frame_to_pixels(&comp, 0, 20, 20, 0.0, 0);
        let ci = ((10 * 20 + 10) * 4) as usize;
        // 0.8 -> inverted ~0.2 (255*0.2=51); allow small tolerance.
        assert!((inverted[ci] as i32 - (255 - (0.8 * 255.0) as u8) as i32).abs() <= 3,
            "invert effect did not apply: got {}", inverted[ci]);
    }

    #[test]
    fn test_twirl_effect_shifts_pixels() {
        use crate::core::cpu_effects;

        // Place a red pixel slightly off-center so the twirl displaces it.
        // (The exact center pixel at r=0 is the twirl axis and doesn't move.)
        let mut buf = vec![0u8; 16 * 16 * 4];
        let px_x = 8;
        let px_y = 10; // 2 pixels below center — r=2, within radius=30
        let idx = ((px_y * 16 + px_x) * 4) as usize;
        buf[idx] = 255; buf[idx + 1] = 40; buf[idx + 2] = 40; buf[idx + 3] = 255;

        let effects = vec![Effect {
            id: "e2".to_string(),
            name: "Twirl".to_string(),
            effect_type: EffectType::Twirl {
                angle: Animatable::new_constant(90.0),
                radius: Animatable::new_constant(30.0),
            },
            enabled: true,
        }];

        cpu_effects::apply_layer_effects(&mut buf, 16, 16, &effects, 0);

        // The twirl should have moved the red pixel away from (8,10).
        let orig_val = ((px_y * 16 + px_x) * 4) as usize;
        assert_ne!(buf[orig_val], 255, "red pixel should have moved from original position");

        // A nearby pixel should now be red (displaced).
        let mut found = false;
        for y in 0..16u32 {
            for x in 0..16u32 {
                let i = ((y * 16 + x) * 4) as usize;
                if buf[i] > 200 && buf[i + 1] < 80 && buf[i + 3] > 200 {
                    found = true;
                    break;
                }
            }
            if found { break; }
        }
        assert!(found, "displaced red pixel not found in buffer after twirl");
    }

    #[test]
    fn test_adjustment_layer_applies_effects_to_composite() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        // Bottom layer: red solid
        comp.layers.push(Layer::new("l1".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30));
        // Top layer: adjustment layer with Invert effect
        let mut adj = Layer::new_adjustment("adj1".to_string(), "Adj Invert".to_string(), 30);
        adj.effects.push(Effect {
            id: "inv".to_string(),
            name: "Invert".to_string(),
            effect_type: EffectType::Invert { invert_alpha: false },
            enabled: true,
        });
        comp.layers.push(adj);

        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        // Red (255,0,0) inverted should be (0,255,255)
        let idx = (5 * 10 + 5) * 4;
        assert_eq!(pixels[idx], 0, "R should be inverted");
        assert_eq!(pixels[idx + 1], 255, "G should be 255");
        assert_eq!(pixels[idx + 2], 255, "B should be 255");
    }

    #[test]
    fn test_fractal_noise_generates_output() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 32, 32, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Solid".to_string(), LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] }, 30);
        layer.effects.push(Effect {
            id: "fn1".to_string(),
            name: "FractalNoise".to_string(),
            effect_type: EffectType::FractalNoise {
                fractal_type: Animatable::new_constant(0.0),
                contrast: Animatable::new_constant(100.0),
                brightness: Animatable::new_constant(0.0),
                complexity: Animatable::new_constant(3.0),
                evolution: Animatable::new_constant(0.0),
            },
            enabled: true,
        });
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
        assert_eq!(pixels.len(), 32 * 32 * 4);

        // At least some pixels should differ from the original gray
        let mut non_gray = 0;
        for p in pixels.chunks_exact(4) {
            if (p[0] as i32 - p[1] as i32).abs() > 5 || (p[1] as i32 - p[2] as i32).abs() > 5 || (p[0] as i32 - 128).abs() > 20 {
                non_gray += 1;
            }
        }
        assert!(non_gray > 0, "FractalNoise should produce varied pixel values");
    }

    #[test]
    fn test_time_remap_shifts_layer_time() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        let mut layer = Layer::new("l1".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        // Set time remap: at frame 0, remap to frame 10
        layer.time_remap = Some(crate::core::property::Animatable::new_constant(10.0));
        comp.layers.push(layer);

        // At frame 0, the layer should evaluate at frame 10 (still active, since duration=30)
        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        // Should still have red pixels since layer is active at frame 10
        let idx = (5 * 10 + 5) * 4;
        assert!(pixels[idx] > 200, "Time remapped layer should still render at frame 0");
    }

    #[test]
    fn test_softlight_blend_uses_both_src_and_dst() {
        // Create a red layer over a green layer with SoftLight blend
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 10, 10, 30, 30);
        // Bottom: green solid
        let mut bottom = Layer::new("l1".to_string(), "Green".to_string(), LayerType::Solid { color: [0.0, 1.0, 0.0, 1.0] }, 30);
        bottom.blend_mode = BlendMode::Normal;
        comp.layers.push(bottom);
        // Top: red solid with SoftLight
        let mut top = Layer::new("l2".to_string(), "Red".to_string(), LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 30);
        top.blend_mode = BlendMode::SoftLight;
        comp.layers.push(top);

        let pixels = render_frame_to_pixels(&comp, 0, 10, 10, 0.0, 0);
        let idx = (5 * 10 + 5) * 4;
        // SoftLight with src=red(1,0,0) over dst=green(0,1,0):
        // Red channel: s=1.0 > 0.5, d=0.0, a=0.0 → d + (2*s-1)*(a-d) = 0 + 1*(0-0) = 0
        // Green channel: s=0.0 <= 0.5, d=1.0 → d - (1-2*s)*d*(1-d) = 1 - 1*1*0 = 1
        // Blue channel: both 0 → 0
        // Result should be (0, 1, 0) which is green (unchanged since src red doesn't affect green via SoftLight)
        assert!(pixels[idx + 1] > 200, "Green channel should be preserved by SoftLight");
    }

    #[test]
    fn test_shape_ellipse_renders_pixels() {
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(), "Ellipse".to_string(),
            LayerType::Shape { shape_type: crate::core::timeline::ShapeType::Ellipse {
                width: crate::core::property::Animatable::new_constant(100.0),
                height: crate::core::property::Animatable::new_constant(100.0),
            }, color: [1.0, 0.0, 0.0, 1.0], stroke_color: [0.0, 0.0, 0.0, 1.0], stroke_width: 0.0 },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        // Center pixel should be red (shape color)
        let center = (32 * 64 + 32) * 4;
        assert!(pixels[center] > 200, "Center of ellipse should be red (R={})", pixels[center]);
        // Corner pixel should be background color, not red
        let corner = 0;
        assert!(pixels[corner] < 50, "Corner should be background color (R={})", pixels[corner]);
    }

    #[test]
    fn test_shape_rectangle_renders_pixels() {
        use crate::core::timeline::ShapeType;
        let mut comp = Composition::new("c1".to_string(), "Comp".to_string(), 64, 64, 30, 30);
        let mut layer = Layer::new(
            "l1".to_string(), "Rect".to_string(),
            LayerType::Shape { shape_type: ShapeType::Rectangle {
                width: crate::core::property::Animatable::new_constant(100.0),
                height: crate::core::property::Animatable::new_constant(100.0),
                corner_radius: crate::core::property::Animatable::new_constant(0.0),
            }, color: [0.0, 1.0, 0.0, 1.0], stroke_color: [0.0, 0.0, 0.0, 1.0], stroke_width: 0.0 },
            30,
        );
        layer.transform.position = crate::core::property::Animatable::new_constant([32.0, 32.0]);
        comp.layers.push(layer);

        let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
        let center = (32 * 64 + 32) * 4;
        assert!(pixels[center + 1] > 200, "Center of rectangle should be green");
    }
}
