use eframe::egui;
use crate::core::timeline::Layer;

/// A reusable module for rendering the After Effects keyframe Graph Editor.
///
/// Visualizes animatable property value curves over time, drawing interactive control
/// points and Bezier tangent handles.
#[allow(dead_code)]
pub fn draw_graph_editor(
    selected_property: &mut Option<String>,
    ui: &mut egui::Ui,
    duration_frames: u32,
    layer: &mut Layer,
    project_changed: &mut bool,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📈 Graph Editor").strong());
            let prop_name = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
            egui::ComboBox::from_id_source("graph_prop_select_module")
                .selected_text(&prop_name)
                .show_ui(ui, |ui| {
                    for p in ["Position X", "Position Y", "Scale X", "Scale Y", "Rotation", "Opacity"] {
                        if ui.selectable_label(prop_name == p, p).clicked() {
                            *selected_property = Some(p.to_string());
                        }
                    }
                });

            ui.add_space(8.0);
            if ui.button("⚡ Easy Ease (F9)").on_hover_text("Apply smooth cubic Bezier ease curve to all keyframes").clicked() {
                use crate::core::property::Animatable;
                use crate::core::keyframe::InterpolationType;

                macro_rules! apply_easy_ease {
                    ($kfs:expr) => {
                        for kf in $kfs.iter_mut() {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some([0.33, 0.0, 0.67, 1.0]),
                            };


                        }
                    };
                }

                let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                match active_prop.as_str() {
                    "Position X" | "Position Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.position { apply_easy_ease!(kfs); }
                    }
                    "Scale X" | "Scale Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.scale { apply_easy_ease!(kfs); }
                    }
                    "Rotation" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.rotation { apply_easy_ease!(kfs); }
                    }
                    "Opacity" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.opacity { apply_easy_ease!(kfs); }
                    }
                    _ => {}
                }
                *project_changed = true;
            }

            ui.add_space(4.0);
            if ui.button("⚡ Mirror Ease").on_hover_text("Symmetrically mirror Ease In / Ease Out handles").clicked() {
                use crate::core::property::Animatable;
                use crate::core::keyframe::InterpolationType;

                let mirror_custom_bezier = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier { custom_bezier: Some(ref mut pts), .. } = interpolation {
                        let mirrored = [1.0 - pts[2], 1.0 - pts[3], 1.0 - pts[0], 1.0 - pts[1]];
                        *pts = mirrored;
                    }
                };

                let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                match active_prop.as_str() {
                    "Position X" | "Position Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    "Scale X" | "Scale Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.scale {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    "Rotation" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.rotation {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    "Opacity" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.opacity {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    _ => {}
                }
                *project_changed = true;
            }

        });

        let graph_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
        let total_f = duration_frames.max(1);

        // Sample values along timeline duration for drawing curve
        let mut samples = Vec::with_capacity(total_f as usize + 1);
        for f in 0..=total_f {
            let raw_val = match graph_prop.as_str() {
                "Position X" => layer.transform.position.evaluate(f)[0],
                "Position Y" => layer.transform.position.evaluate(f)[1],
                "Scale X" => layer.transform.scale.evaluate(f)[0],
                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                "Rotation" => layer.transform.rotation.evaluate(f),
                "Opacity" => layer.transform.opacity.evaluate(f),
                _ => layer.transform.position.evaluate(f)[0],
            };
            let val = if raw_val.is_nan() { 0.0 } else { raw_val };
            samples.push((f, val));
        }

        // Allocate drawing region
        let (rect, _graph_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 70.0),
            egui::Sense::click_and_drag(),
        );
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(25));
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(50)));
        
        let min_val = samples.iter().map(|(_, v)| *v).fold(f32::INFINITY, f32::min);
        let max_val = samples.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
        let val_range = (max_val - min_val).max(0.001);

        // Convert keyframe time/value to screen space coordinates inside the allocated rect
        let points: Vec<egui::Pos2> = samples.iter().map(|&(f, v)| {
            let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
            let y = rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
            egui::pos2(x, y)
        }).collect();

        // Draw graph spline segments (Value Curve)
        for window in points.windows(2) {
            ui.painter().line_segment(
                [window[0], window[1]],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 50)),
            );
        }

        // Draw Speed Graph Velocity Line (First Derivative v(t) = dy/dt)
        let mut max_speed = 0.0f32;
        let mut speed_pts = Vec::with_capacity(points.len());
        for win in samples.windows(2) {
            let dt = (win[1].0 as f32 - win[0].0 as f32).max(1.0);
            let speed = ((win[1].1 - win[0].1) / dt).abs();
            max_speed = max_speed.max(speed);
            speed_pts.push(speed);
        }

        if max_speed > 0.001 {
            for i in 0..speed_pts.len() {
                let sx = rect.left() + (i as f32 / total_f as f32) * rect.width();
                let sy = rect.bottom() - 4.0 - (speed_pts[i] / max_speed) * (rect.height() - 16.0);
                let p1 = egui::pos2(sx, sy);

                let next_i = (i + 1).min(speed_pts.len() - 1);
                let nsx = rect.left() + (next_i as f32 / total_f as f32) * rect.width();
                let nsy = rect.bottom() - 4.0 - (speed_pts[next_i] / max_speed) * (rect.height() - 16.0);
                let p2 = egui::pos2(nsx, nsy);

                ui.painter().line_segment([p1, p2], egui::Stroke::new(1.2, egui::Color32::from_rgb(0, 220, 180)));
            }

            // Peak Speed Badge HUD
            let speed_badge_pos = egui::pos2(rect.right() - 110.0, rect.top() + 6.0);
            ui.painter().text(
                speed_badge_pos,
                egui::Align2::LEFT_TOP,
                format!("⚡ Peak: {:.0} px/s", max_speed * 30.0),
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(0, 220, 180),
            );
        }

        // Render interactive keyframe anchor points & tangent handles (real editing)
        // Anchors are drawn at actual keyframe positions and can be dragged in time;
        // tangent handles edit the keyframe's custom bezier control points.
        {
            use crate::core::keyframe::{InterpolationType, BezierControlPoint};

            // Mutable access to Vec2-typed keyframe tracks (Position / Scale)
            fn keyframes_of_vec2<'a>(layer: &'a mut Layer, prop: &str) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 2]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 2]>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Position X" | "Position Y" => animated(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => animated(&mut layer.transform.scale),
                    _ => None,
                }
            }

            // Mutable access to scalar keyframe tracks (Rotation / Opacity)
            fn keyframes_of_f32<'a>(layer: &'a mut Layer, prop: &str) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<f32>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Rotation" => animated(&mut layer.transform.rotation),
                    "Opacity" => animated(&mut layer.transform.opacity),
                    _ => None,
                }
            }

            macro_rules! with_keyframes {
                ($layer:expr, $prop:expr, $kfs:ident => $body:expr) => {{
                    let prop: String = $prop.clone();
                    if matches!(prop.as_str(), "Position X" | "Position Y" | "Scale X" | "Scale Y") {
                        if let Some($kfs) = keyframes_of_vec2($layer, &prop) { Some({ $body }) } else { None }
                    } else {
                        if let Some($kfs) = keyframes_of_f32($layer, &prop) { Some({ $body }) } else { None }
                    }
                }};
            }


            let frame_to_x = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let val_to_y = |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            // Adds a value delta to a keyframe of either supported value type
            trait AddVal { fn add_val(&mut self, delta: f32, is_y: bool); }
            impl AddVal for f32 { fn add_val(&mut self, d: f32, _y: bool) { *self += d; } }
            impl AddVal for [f32; 2] { fn add_val(&mut self, d: f32, y: bool) { let i = if y { 1 } else { 0 }; self[i] += d; } }
            fn set_keyframe_value<T: AddVal>(kf: &mut crate::core::keyframe::Keyframe<T>, delta: f32, is_y: bool) {
                kf.value.add_val(delta, is_y);
            }

            // Snapshot keyframe positions first (immutable), then edit mutably on drag
            let kf_positions: Vec<(usize, u32, f32)> = if matches!(graph_prop.as_str(), "Position X" | "Position Y" | "Scale X" | "Scale Y") {
                let comp_idx = if graph_prop.ends_with('Y') { 1usize } else { 0usize };
                keyframes_of_vec2(layer, &graph_prop).map(|kfs| {
                    kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value[comp_idx])).collect::<Vec<_>>()
                }).unwrap_or_default()
            } else {
                keyframes_of_f32(layer, &graph_prop).map(|kfs| {
                    kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value)).collect::<Vec<_>>()
                }).unwrap_or_default()
            };

            for (kf_idx, kf_frame, kf_val) in &kf_positions {
                let pt = egui::pos2(frame_to_x(*kf_frame), val_to_y(*kf_val));

                // --- Anchor point: drag horizontally to retime, vertically to change value ---
                let anchor_rect = egui::Rect::from_center_size(pt, egui::vec2(14.0, 14.0));
                let anchor_resp = ui.interact(anchor_rect, egui::Id::new(("graph_anchor", kf_idx)), egui::Sense::drag());
                if anchor_resp.dragged() {
                    let delta_frames = (anchor_resp.drag_delta().x / rect.width() * total_f as f32).round() as i32;
                    let new_frame = (*kf_frame as i32 + delta_frames).clamp(0, total_f as i32) as u32;
                    // Vertical drag → value change (screen up = value up)
                    let delta_val = -anchor_resp.drag_delta().y / (rect.height() - 8.0) * val_range;
                    with_keyframes!(layer, graph_prop, kfs => {
                        if kfs[*kf_idx].frame != new_frame {
                            kfs[*kf_idx].frame = new_frame;
                            kfs.sort_by_key(|k| k.frame);
                        }
                        if let Some(kf) = kfs.get_mut(*kf_idx) {
                            set_keyframe_value(kf, delta_val, graph_prop.ends_with('Y'));
                        }
                        *project_changed = true;
                    });
                }
                let anchor_color = if anchor_resp.dragged() {
                    egui::Color32::WHITE
                } else if anchor_resp.hovered() {
                    egui::Color32::from_rgb(255, 245, 150)
                } else {
                    egui::Color32::from_rgb(255, 230, 100)
                };
                ui.painter().circle_filled(pt, 4.0, anchor_color);
                if anchor_resp.hovered() {
                    ui.painter().circle_stroke(pt, 7.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 230, 100)));
                }

                // --- Tangent handles: drag to edit custom bezier control points ---
                // Extract current bezier points (default Easy Ease if linear/hold)
                fn bezier_pts<T>(kf: &crate::core::keyframe::Keyframe<T>) -> (f32, f32, f32, f32) {
                    match &kf.interpolation {
                        InterpolationType::Bezier { custom_bezier: Some(pts), .. } => (pts[0], pts[1], pts[2], pts[3]),
                        _ => (0.33, 0.0, 0.67, 1.0),
                    }
                }
                let bezier_from_kfs = |get: Option<(f32, f32, f32, f32)>| get.unwrap_or((0.33, 0.0, 0.67, 1.0));
                let _ = bezier_from_kfs;
                let (bx1, by1, bx2, by2): (f32, f32, f32, f32) = with_keyframes!(layer, graph_prop, kfs => {
                    kfs.get(*kf_idx).map(bezier_pts).unwrap_or((0.33, 0.0, 0.67, 1.0))
                }).unwrap_or((0.33, 0.0, 0.67, 1.0));

                let h_out = egui::pos2(pt.x + bx2 * 44.0, pt.y - by2 * 24.0);
                let h_in = egui::pos2(pt.x - bx1 * 44.0, pt.y + by1 * 24.0);

                let h_out_rect = egui::Rect::from_center_size(h_out, egui::vec2(14.0, 14.0));
                let h_in_rect = egui::Rect::from_center_size(h_in, egui::vec2(14.0, 14.0));
                let h_out_resp = ui.interact(h_out_rect, egui::Id::new(("graph_h_out", kf_idx)), egui::Sense::drag());
                let h_in_resp = ui.interact(h_in_rect, egui::Id::new(("graph_h_in", kf_idx)), egui::Sense::drag());

                let mut new_pts: Option<[f32; 4]> = None;
                if h_out_resp.dragged() {
                    let d = h_out_resp.drag_delta();
                    let nx2 = (bx2 + d.x / 44.0).clamp(bx1 + 0.01, 1.0);
                    let ny2 = (by2 - d.y / 24.0).clamp(-1.5, 2.5);
                    new_pts = Some([bx1, by1, nx2, ny2]);
                }
                if h_in_resp.dragged() {
                    let d = h_in_resp.drag_delta();
                    let nx1 = (bx1 - d.x / 44.0).clamp(0.0, bx2 - 0.01);
                    let ny1 = (by1 + d.y / 24.0).clamp(-1.5, 2.5);
                    new_pts = Some([nx1, ny1, bx2, by2]);
                }
                if let Some(pts) = new_pts {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if let Some(kf) = kfs.get_mut(*kf_idx) {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some(pts),
                            };
                            *project_changed = true;
                        }
                    });
                }

                let any_hover = h_out_resp.hovered() || h_in_resp.dragged() || h_in_resp.hovered() || h_out_resp.dragged();
                let stroke_color = if any_hover {
                    egui::Color32::from_rgb(255, 120, 150)
                } else {
                    egui::Color32::from_rgb(100, 200, 255)
                };
                ui.painter().line_segment([pt, h_out], egui::Stroke::new(1.2, stroke_color));
                ui.painter().line_segment([pt, h_in], egui::Stroke::new(1.2, stroke_color));

                let h_out_color = if h_out_resp.hovered() || h_out_resp.dragged() { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 220, 255) };
                let h_in_color = if h_in_resp.hovered() || h_in_resp.dragged() { egui::Color32::WHITE } else { egui::Color32::from_rgb(100, 220, 255) };
                ui.painter().circle_filled(h_out, 4.0, h_out_color);
                ui.painter().circle_filled(h_in, 4.0, h_in_color);
            }
        }
    });
}
