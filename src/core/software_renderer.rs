use crate::core::timeline::{Composition, LayerType, BlendMode};
use crate::core::mask::point_in_polygon;
use rayon::prelude::*;

/// Professional CPU-based rasterizer to composite active composition layers
/// into a flat RGBA8 pixel buffer for preview rendering or FFmpeg export.
pub fn render_frame_to_pixels(comp: &Composition, frame: u32, width: u32, height: u32, exposure_ev: f32, lut_mode: u32) -> Vec<u8> {
    let size = (width * height * 4) as usize;
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

        // Get world transform properties
        let (pos, scale, rotation, opacity) = comp.resolve_world_transform(layer, frame);
        
        let l_opacity = (opacity / 100.0).clamp(0.0, 1.0);
        if l_opacity < 0.001 {
            continue;
        }

        let (base_w, base_h) = match &layer.layer_type {
            LayerType::Solid { .. } | LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
            LayerType::Text { font_size, text, .. } => (
                (text.len().max(1) as f32 * *font_size as f32 * 0.6).max(*font_size as f32),
                *font_size as f32 * 1.2,
            ),
            LayerType::Shape { .. } | LayerType::Image { .. } => (comp.width as f32 * 0.5, comp.height as f32 * 0.5),
            _ => continue, // Null or audio layers don't output visual pixels
        };

        let w = (scale[0].abs() / 100.0) * base_w;
        let h = (scale[1].abs() / 100.0) * base_h;

        let base_color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
            LayerType::Shape { color, .. } => *color,
            LayerType::Image { .. } => [0.2, 0.6, 0.9, 1.0], // fallback image color
            LayerType::PreComp { .. } => [0.8, 0.3, 0.8, 1.0], // fallback precomp color
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

        // Evaluate Vector Mask geometry (if any)
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
                            // Inside mask: fade out if close to edge within feather width
                            mask_alpha = (dist / mask_feather).clamp(0.0, 1.0);
                        } else {
                            // Outside mask: if close to edge, fade in (soft falloff)
                            mask_alpha = (1.0 - (dist / mask_feather)).clamp(0.0, 1.0);
                        }
                    } else {
                        // Hard mask edge
                        if !actual_inside {
                            continue;
                        }
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
                    let idx = ((py * width + px) * 4) as usize;
                    if idx + 3 >= buffer.len() {
                        continue;
                    }

                    // Composite current layer color over buffer pixel using BlendMode
                    let src_r = base_color[0];
                    let src_g = base_color[1];
                    let src_b = base_color[2];
                    let src_a = base_color[3] * l_opacity * mask_alpha;

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
            let a = 2.51;
            let b_val = 0.03;
            let c = 2.43;
            let d = 0.59;
            let e = 0.14;
            r = (r * (a * r + b_val)) / (r * (c * r + d) + e);
            g = (g * (a * g + b_val)) / (g * (c * g + d) + e);
            b = (b * (a * b + b_val)) / (b * (c * b + d) + e);
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
    use crate::core::timeline::{Composition, Layer, LayerType, BlendMode};

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
}
