#![allow(clippy::too_many_arguments)]

use crate::core::text_animator::TextAnimatorEngine;
use crate::core::timeline::{Layer, LayerType};
use crate::ui::theme::colors;
use crate::AfterEffectsApp;
use eframe::egui;

pub fn draw_software_canvas(
    ui: &mut egui::Ui,
    app: &AfterEffectsApp,
    current_frame: u32,
    draw_rect: egui::Rect,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
    clip_rect: Option<egui::Rect>,
) {
    let prev_clip = ui.clip_rect();
    if let Some(clip) = clip_rect {
        ui.set_clip_rect(prev_clip.intersect(clip));
    }

    ui.painter()
        .rect_filled(draw_rect, 0.0, egui::Color32::BLACK);
    let comp = app.history.current().active_composition();

    let has_solo = comp
        .layers
        .iter()
        .any(|l: &Layer| l.is_active(current_frame) && l.solo);

    for (li, layer) in comp.layers.iter().enumerate() {
        let l: &Layer = layer;
        if l.is_active(current_frame) {
            if has_solo && !l.solo {
                continue;
            }

            let (pos, scale, rotation, opacity) = comp.resolve_world_transform(l, current_frame);

            let rx = origin_x + (pos[0] / comp_w) * draw_w;
            let ry = origin_y + (pos[1] / comp_h) * draw_h;

            let base_color = match &l.layer_type {
                LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
                LayerType::Shape { color, .. } => *color,
                LayerType::Image { .. } => [0.2, 0.6, 0.9, 0.9],
                LayerType::PreComp { .. } => [0.8, 0.3, 0.8, 0.9],
                _ => [0.5, 0.5, 0.5, 0.5],
            };

            let alpha_mult = match l.blend_mode {
                crate::core::timeline::BlendMode::Normal => 1.0,
                crate::core::timeline::BlendMode::Multiply => 0.85,
                crate::core::timeline::BlendMode::Screen => 0.7,
                crate::core::timeline::BlendMode::Overlay => 0.8,
                crate::core::timeline::BlendMode::Add => 0.9,
                crate::core::timeline::BlendMode::Darken => 0.8,
                crate::core::timeline::BlendMode::Lighten => 0.8,
                crate::core::timeline::BlendMode::SoftLight => 0.8,
                crate::core::timeline::BlendMode::HardLight => 0.8,
                crate::core::timeline::BlendMode::Difference => 0.7,
                crate::core::timeline::BlendMode::Exclusion => 0.7,
                crate::core::timeline::BlendMode::Divide => 0.8,
                crate::core::timeline::BlendMode::Subtract => 0.8,
                _ => 0.8,
            };

            let layer_color = egui::Color32::from_rgba_unmultiplied(
                (base_color[0] * 255.0) as u8,
                (base_color[1] * 255.0) as u8,
                (base_color[2] * 255.0) as u8,
                (opacity / 100.0 * alpha_mult * 255.0) as u8,
            );

            match &l.layer_type {
                LayerType::Solid { .. } | LayerType::Shape { .. } => {
                    let mut pts_to_draw = None;
                    for mask in &l.masks {
                        if mask.enabled && mask.mode != crate::core::mask::MaskMode::None {
                            let points = mask.path.to_polygon(current_frame, 16);
                            if points.len() >= 3 {
                                pts_to_draw = Some(points);
                                break;
                            }
                        }
                    }

                    let raw_pts: Vec<[f32; 2]> = if let Some(dpts) = pts_to_draw {
                        dpts
                    } else {
                        let w = (scale[0] / 100.0) * 100.0;
                        let h = (scale[1] / 100.0) * 100.0;
                        let rad = rotation.to_radians();
                        let cos_r = rad.cos();
                        let sin_r = rad.sin();
                        let local = [
                            (-w * 0.5, -h * 0.5),
                            (w * 0.5, -h * 0.5),
                            (w * 0.5, h * 0.5),
                            (-w * 0.5, h * 0.5),
                            (-w * 0.5, -h * 0.5),
                        ];
                        local
                            .iter()
                            .map(|(px, py)| {
                                [
                                    pos[0] + px * cos_r - py * sin_r,
                                    pos[1] + px * sin_r + py * cos_r,
                                ]
                            })
                            .collect()
                    };

                    let final_raw = if let Some(ref trim) = l.trim_paths {
                        trim.trim_polygon(&raw_pts, current_frame)
                    } else {
                        raw_pts
                    };

                    if final_raw.len() >= 2 {
                        let pts: Vec<egui::Pos2> = final_raw
                            .iter()
                            .map(|pt| {
                                let mx = origin_x + (pt[0] / comp_w) * draw_w;
                                let my = origin_y + (pt[1] / comp_h) * draw_h;
                                egui::pos2(mx, my)
                            })
                            .collect();

                        if l.trim_paths.is_some() {
                            for window in pts.windows(2) {
                                ui.painter().line_segment(
                                    [window[0], window[1]],
                                    egui::Stroke::new(3.0, layer_color),
                                );
                            }
                        } else {
                            ui.painter().add(egui::Shape::convex_polygon(
                                pts,
                                layer_color,
                                egui::Stroke::NONE,
                            ));
                        }
                    }
                }
                LayerType::Image { path } => {
                    let w = (scale[0] / 100.0) * 160.0 * (draw_w / comp_w);
                    let h = (scale[1] / 100.0) * 120.0 * (draw_h / comp_h);
                    let img_rect =
                        egui::Rect::from_center_size(egui::pos2(rx, ry), egui::vec2(w, h));
                    ui.painter().rect_filled(img_rect, 6.0, layer_color);
                    ui.painter().rect_stroke(
                        img_rect,
                        6.0,
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    );
                    let filename = path.split('/').next_back().unwrap_or(path);
                    ui.painter().text(
                        img_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("IMG :: {}", filename),
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                }
                LayerType::PreComp { comp_id } => {
                    let w = (scale[0] / 100.0) * 200.0 * (draw_w / comp_w);
                    let h = (scale[1] / 100.0) * 140.0 * (draw_h / comp_h);
                    let comp_rect =
                        egui::Rect::from_center_size(egui::pos2(rx, ry), egui::vec2(w, h));
                    ui.painter().rect_filled(comp_rect, 6.0, layer_color);
                    ui.painter().rect_stroke(
                        comp_rect,
                        6.0,
                        egui::Stroke::new(2.0, colors::ACCENT_PURPLE),
                    );
                    ui.painter().text(
                        comp_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("COMP :: {}", comp_id),
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                }
                LayerType::Text {
                    text,
                    font_size,
                    color,
                    text_on_path,
                    ..
                } => {
                    let font_px = *font_size as f32 * (scale[1] / 100.0) * (draw_h / comp_h);
                    let char_w = font_px * 0.55;

                    if *text_on_path && !l.masks.is_empty() {
                        // Text on Path: use first mask as path
                        use crate::core::mask::MaskVertex;
                        use crate::core::path_text::{layout_text_along_path, PathTextOptions};
                        let mask = &l.masks[0];
                        let path_points = mask.path.to_polygon(current_frame, 12);
                        let verts: Vec<MaskVertex> = path_points
                            .windows(2)
                            .map(|w| MaskVertex {
                                position: w[0],
                                tangent_in: [0.0; 2],
                                tangent_out: [w[1][0] - w[0][0], w[1][1] - w[0][1]],
                            })
                            .chain(std::iter::once(MaskVertex {
                                position: *path_points.last().unwrap_or(&[0.0; 2]),
                                tangent_in: [0.0; 2],
                                tangent_out: [0.0; 2],
                            }))
                            .collect();
                        let opts = PathTextOptions::default();
                        let glyphs = layout_text_along_path(
                            text,
                            font_px,
                            &verts,
                            mask.path.is_closed,
                            &opts,
                        );
                        for g in &glyphs {
                            let gx = origin_x + (g.position[0] / comp_w) * draw_w;
                            let gy = origin_y + (g.position[1] / comp_h) * draw_h;
                            ui.painter().text(
                                egui::pos2(gx, gy),
                                egui::Align2::LEFT_CENTER,
                                g.char_code.to_string(),
                                egui::FontId::proportional(font_px),
                                layer_color,
                            );
                        }
                    } else if let Some(anim) = &l.text_animator {
                        if anim.enabled {
                            let transforms = TextAnimatorEngine::eval_character_transforms(
                                text,
                                &anim.selector,
                                anim.position_offset,
                                anim.scale,
                                anim.opacity,
                                anim.tracking,
                                anim.rotation,
                                0.0,
                            );
                            let total_w = char_w * text.chars().count() as f32;
                            let mut cx = rx - total_w * 0.5;

                            for (ch, ct) in text.chars().zip(transforms.iter()) {
                                let px = cx + ct.position_offset[0] * (draw_w / comp_w);
                                let py = ry + ct.position_offset[1] * (draw_h / comp_h);
                                let char_font = font_px * ct.scale_multiplier[1].max(0.1);
                                let char_color = egui::Color32::from_rgba_unmultiplied(
                                    (color[0] * 255.0) as u8,
                                    (color[1] * 255.0) as u8,
                                    (color[2] * 255.0) as u8,
                                    (layer_color.a() as f32 * ct.opacity_multiplier) as u8,
                                );
                                ui.painter().text(
                                    egui::pos2(px, py),
                                    egui::Align2::LEFT_CENTER,
                                    ch.to_string(),
                                    egui::FontId::proportional(char_font),
                                    char_color,
                                );
                                cx += char_w + ct.tracking_offset * (draw_w / comp_w);
                            }
                        } else {
                            ui.painter().text(
                                egui::pos2(rx, ry),
                                egui::Align2::CENTER_CENTER,
                                text,
                                egui::FontId::proportional(font_px),
                                layer_color,
                            );
                        }
                    } else {
                        ui.painter().text(
                            egui::pos2(rx, ry),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(font_px),
                            layer_color,
                        );
                    }
                }
                _ => {}
            }

            for mask in &l.masks {
                if !mask.enabled {
                    continue;
                }
                let points = mask.path.to_polygon(current_frame, 12);
                if points.len() >= 2 {
                    let mut draw_points = Vec::with_capacity(points.len());
                    for pt in &points {
                        let mx = origin_x + (pt[0] / comp_w) * draw_w;
                        let my = origin_y + (pt[1] / comp_h) * draw_h;
                        draw_points.push(egui::pos2(mx, my));
                    }

                    let is_selected_layer = Some(li) == app.selection.selected_layer_idx;
                    let line_color = if is_selected_layer {
                        colors::ACCENT_YELLOW
                    } else {
                        egui::Color32::from_rgba_unmultiplied(255, 180, 50, 100)
                    };

                    for w in draw_points.windows(2) {
                        ui.painter()
                            .line_segment([w[0], w[1]], egui::Stroke::new(1.2, line_color));
                    }
                    if mask.path.is_closed {
                        ui.painter().line_segment(
                            [draw_points[draw_points.len() - 1], draw_points[0]],
                            egui::Stroke::new(1.2, line_color),
                        );
                    }

                    if is_selected_layer {
                        let anchor_verts = mask.path.vertices_at_frame(current_frame);
                        for (v_idx, pt) in anchor_verts.iter().enumerate() {
                            let mx = origin_x + (pt[0] / comp_w) * draw_w;
                            let my = origin_y + (pt[1] / comp_h) * draw_h;
                            let screen_pt = egui::pos2(mx, my);
                            let v_rect =
                                egui::Rect::from_center_size(screen_pt, egui::vec2(8.0, 8.0));
                            let is_hovered = ui.rect_contains_pointer(v_rect);
                            let handle_color = if is_hovered {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::WHITE
                            };
                            ui.painter().rect_filled(v_rect, 1.0, handle_color);
                            ui.painter().rect_stroke(
                                v_rect,
                                1.0,
                                egui::Stroke::new(1.2, colors::HANDLE_HOVER_STROKE),
                            );
                            ui.painter().text(
                                egui::pos2(screen_pt.x + 8.0, screen_pt.y - 8.0),
                                egui::Align2::LEFT_BOTTOM,
                                format!("V{}", v_idx + 1),
                                egui::FontId::proportional(10.0),
                                colors::HANDLE_HOVER_FILL,
                            );
                        }
                    }
                }
            }

            if l.blend_mode != crate::core::timeline::BlendMode::Normal {
                ui.painter().text(
                    egui::pos2(rx, ry - 14.0),
                    egui::Align2::CENTER_CENTER,
                    format!("[{:?}]", l.blend_mode),
                    egui::FontId::proportional(10.0),
                    egui::Color32::YELLOW,
                );
            }
        }
    }

    ui.set_clip_rect(prev_clip);
}
