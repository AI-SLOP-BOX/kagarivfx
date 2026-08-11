use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{LayerType, LabelColor, TrackMatteMode, Expression, TrackerPoint, BlendMode};
use crate::core::property::Animatable;
use crate::core::keyframe::{Keyframe, InterpolationType, BezierControlPoint};
use crate::core::tracker_engine::TrackerEngine;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: &mut u32) {
    // ── Non-blocking Async Motion Tracker Event Receiver ──
    let mut tracker_completed = false;
    if let Some(ref rx) = app.tracker_rx {
        while let Ok(event) = rx.try_recv() {
            if let crate::TrackerEvent::Finished { layer_idx, tracker_idx, keyframes } = event {
                let comp_mut = app.history.current_mut().active_composition_mut();
                if layer_idx < comp_mut.layers.len() && tracker_idx < comp_mut.layers[layer_idx].trackers.len() {
                    comp_mut.layers[layer_idx].trackers[tracker_idx].position = Animatable::Animated(keyframes);
                    crate::core::frame_cache::bump_version();
                    log::info!("Async Motion Tracker analysis completed for layer {}, tracker {}", layer_idx, tracker_idx);
                }
                tracker_completed = true;
                break;
            }
        }
    }
    if tracker_completed {
        app.tracker_rx = None;
    }

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(app.left_tab_idx == 0, "Project").clicked() {
                    app.left_tab_idx = 0;
                }
                if ui.selectable_label(app.left_tab_idx == 1, "Effect Controls").clicked() {
                    app.left_tab_idx = 1;
                }
            });
            ui.separator();

            if app.left_tab_idx == 0 {
                // ── Render AE Project Asset Bin Panel ──
                crate::ui::project_panel::draw(app, ui);
                return;
            }

            ui.heading("Layer Properties");
            ui.separator();

            let mut project_changed = false;
            let mut next_frame = None;
            // Deferred tracker spawn: populated inside group closure, consumed after comp borrow ends.
            let mut pending_tracker: Option<(usize, usize, u32, u32)> = None; // (layer_idx, tracker_idx, start, end)

            // Clone current project to apply transactional state mutations
            let mut temp_project = app.history.current().clone();
            
            // ── Camera Suite Panel ──
            {
                let comp = temp_project.active_composition_mut();
                ui.collapsing("Active Camera Settings", |ui| {
                    let cam = &mut comp.active_camera;
                    ui.checkbox(&mut cam.active, "Camera Active");
                    
                    ui.horizontal(|ui| {
                        ui.label("Field of View:");
                        let fov_before = cam.fov_degrees;
                        ui.add(egui::Slider::new(&mut cam.fov_degrees, 10.0..=120.0).suffix("°"));
                        if fov_before != cam.fov_degrees { project_changed = true; }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Focus Distance:");
                        let fd_before = cam.focus_distance;
                        ui.add(egui::DragValue::new(&mut cam.focus_distance).speed(10.0).suffix(" mm"));
                        if fd_before != cam.focus_distance { project_changed = true; }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Aperture:");
                        let ap_before = cam.aperture;
                        ui.add(egui::Slider::new(&mut cam.aperture, 0.95..=22.0).prefix("f/"));
                        if ap_before != cam.aperture { project_changed = true; }
                    });

                    ui.label("Camera Transform:");
                    let cam_pos_before = cam.transform.position.clone();
                    if let Some(nf) = draw_property_ui(*current_frame, ui, "  Pos", &mut cam.transform.position, |ui, val| {
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X:"));
                            ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y:"));
                            ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z:"));
                        });
                    }) { next_frame = Some(nf); }
                    if cam_pos_before != cam.transform.position { project_changed = true; }

                    let cam_rot_before = cam.transform.rotation.clone();
                    if let Some(nf) = draw_property_ui(*current_frame, ui, "  Rot", &mut cam.transform.rotation, |ui, val| {
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("P:"));
                            ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y:"));
                            ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("R:"));
                        });
                    }) { next_frame = Some(nf); }
                    if cam_rot_before != cam.transform.rotation { project_changed = true; }
                });
            }

            ui.add_space(8.0);

            // ── Multi-Layer Selection Batch Controls ──
            if app.selected_layers.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Multi-Selection: {} Layers", app.selected_layers.len())).strong().color(egui::Color32::from_rgb(120, 180, 255)));
                });
                ui.separator();
                ui.label("Batch Operations:");
                ui.horizontal(|ui| {
                    if ui.button("Batch Easy Ease (F9)").clicked() {
                        let comp = temp_project.active_composition_mut();
                        for &i in &app.selected_layers {
                            if i < comp.layers.len() {
                                let layer = &mut comp.layers[i];
                                if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                                    for kf in kfs { kf.interpolation = InterpolationType::Bezier { outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 }, incoming: BezierControlPoint { influence: 0.333, speed: 0.0 }, custom_bezier: Some([0.4, 0.0, 0.2, 1.0]) }; }
                                }
                            }
                        }
                        project_changed = true;
                    }
                    if ui.button("Toggle Motion Blur").clicked() {
                        let comp = temp_project.active_composition_mut();
                        for &i in &app.selected_layers {
                            if i < comp.layers.len() {
                                comp.layers[i].motion_blur = !comp.layers[i].motion_blur;
                            }
                        }
                        project_changed = true;
                    }
                });
                ui.add_space(8.0);
                ui.separator();
            }

            if let Some(idx) = app.selected_layer_idx {
                let comp = temp_project.active_composition_mut();
                if idx < comp.layers.len() {
                    // ── Safe Parent selector logic ──
                    let other_layers: Vec<(String, String)> = comp.layers.iter()
                        .enumerate()
                        .filter(|&(i, l)| i != idx && l.layer_type != LayerType::Null)
                        .map(|(_, l)| (l.id.clone(), l.name.clone()))
                        .collect();

                    let layer = &mut comp.layers[idx];
                    
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let name_before = layer.name.clone();
                        ui.text_edit_singleline(&mut layer.name);
                        if name_before != layer.name {
                            project_changed = true;
                        }
                    });

                    // AE Layer Options (Solo, Motion Blur, Label, Parent, 3D Layer, Blend Mode)
                    ui.group(|ui| {
                        ui.label("AE Layer Controls");
                        
                        ui.horizontal(|ui| {
                            let solo_before = layer.solo;
                            ui.checkbox(&mut layer.solo, "Solo");
                            if solo_before != layer.solo { project_changed = true; }

                            let mb_before = layer.motion_blur;
                            ui.checkbox(&mut layer.motion_blur, "Motion Blur");
                            if mb_before != layer.motion_blur { project_changed = true; }
                        });

                        ui.horizontal(|ui| {
                            let is_3d_before = layer.is_3d;
                            ui.checkbox(&mut layer.is_3d, "3D Layer");
                            if is_3d_before != layer.is_3d { project_changed = true; }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Blend Mode:");
                            let blend_before = layer.blend_mode;
                            egui::ComboBox::from_id_source("blend_mode_combo")
                                .selected_text(format!("{:?}", layer.blend_mode))
                                .show_ui(ui, |ui| {
                                    for mode in [
                                        BlendMode::Normal, BlendMode::Multiply, BlendMode::Screen,
                                        BlendMode::Overlay, BlendMode::Add, BlendMode::Darken, BlendMode::Lighten
                                    ] {
                                        ui.selectable_value(&mut layer.blend_mode, mode, format!("{:?}", mode));
                                    }
                                });
                            if blend_before != layer.blend_mode { project_changed = true; }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Label:");
                            let label_before = layer.label;
                            egui::ComboBox::from_id_source("label_color_combo")
                                .selected_text(format!("{:?}", layer.label))
                                .show_ui(ui, |ui| {
                                    for color in [
                                        LabelColor::None, LabelColor::Red, LabelColor::Yellow,
                                        LabelColor::Aqua, LabelColor::Pink, LabelColor::Lavender,
                                        LabelColor::Peach, LabelColor::Sea, LabelColor::Blue
                                    ] {
                                        ui.selectable_value(&mut layer.label, color, format!("{:?}", color));
                                    }
                                });
                            if label_before != layer.label { project_changed = true; }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Track Matte:");
                            let matte_before = layer.track_matte;
                            egui::ComboBox::from_id_source("track_matte_combo")
                                .selected_text(format!("{:?}", layer.track_matte))
                                .show_ui(ui, |ui| {
                                    for mode in [
                                        TrackMatteMode::None,
                                        TrackMatteMode::AlphaMatte,
                                        TrackMatteMode::AlphaMatteInverted,
                                        TrackMatteMode::LumaMatte,
                                        TrackMatteMode::LumaMatteInverted
                                    ] {
                                        ui.selectable_value(&mut layer.track_matte, mode, format!("{:?}", mode));
                                    }
                                });
                            if matte_before != layer.track_matte { project_changed = true; }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Parent:");
                            let parent_before = layer.parent_id.clone();

                            let parent_name = if let Some(ref pid) = layer.parent_id {
                                other_layers.iter()
                                    .find(|(id, _)| id == pid)
                                    .map(|(_, name)| name.clone())
                                    .unwrap_or_else(|| "Missing Parent".to_string())
                            } else {
                                "None".to_string()
                            };

                            egui::ComboBox::from_id_source("parent_select_combo")
                                .selected_text(parent_name)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut layer.parent_id, None, "None");
                                    for (id, name) in &other_layers {
                                        ui.selectable_value(&mut layer.parent_id, Some(id.clone()), name);
                                    }
                                });

                            if parent_before != layer.parent_id { project_changed = true; }
                        });
                    });

                    ui.add_space(8.0);

                    // ── AE Motion Tracking Panel ──
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Motion Tracker").strong());
                            if ui.button("+ Add Track Point").clicked() {
                                let id = format!("trackpoint_{}", layer.trackers.len());
                                let name = format!("Track Point {}", layer.trackers.len() + 1);
                                layer.trackers.push(TrackerPoint::new(id, name, [960.0, 540.0]));
                                project_changed = true;
                            }
                        });

                        let trackers_len = layer.trackers.len();
                        for t_idx in 0..trackers_len {
                            ui.separator();
                            let mut trigger_async_track = false;
                            {
                                let tp = &layer.trackers[t_idx];
                                ui.horizontal(|ui| {
                                    ui.small(&tp.name);
                                    if app.tracker_rx.is_some() {
                                        ui.spinner();
                                        ui.small("Analyzing...");
                                    } else if ui.button("Analyze Forward >").clicked() {
                                        trigger_async_track = true;
                                    }
                                });
                            }

                            if trigger_async_track && pending_tracker.is_none() {
                                let start_f = *current_frame;
                                let end_f = (start_f + 30).min(comp.duration_frames);
                                pending_tracker = Some((idx, t_idx, start_f, end_f));
                            }

                            let tp = &mut layer.trackers[t_idx];
                            let val_before = tp.position.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "  Position", &mut tp.position, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                });
                            }) {
                                next_frame = Some(nf);
                            }
                            if val_before != tp.position { project_changed = true; }

                            ui.horizontal(|ui| {
                                ui.label("  Search Size:");
                                let before_s = tp.search_size;
                                ui.add(egui::Slider::new(&mut tp.search_size, 5.0..=100.0).suffix(" px"));
                                if before_s != tp.search_size { project_changed = true; }
                            });
                        }
                    });

                    ui.add_space(8.0);

                    // Transform Section (Conditional on 3D Layer status)
                    ui.group(|ui| {
                        if layer.is_3d {
                            ui.label("Transform 3D");
                            
                            let pos_before = layer.transform_3d.position.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Position (XYZ)", &mut layer.transform_3d.position, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z: "));
                                });
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform_3d.position, &mut project_changed);
                            if pos_before != layer.transform_3d.position { project_changed = true; }

                            ui.separator();
                            let rot_before = layer.transform_3d.rotation.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Rotation (YPR)", &mut layer.transform_3d.rotation, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).suffix("° P"));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).suffix("° Y"));
                                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).suffix("° R"));
                                });
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform_3d.rotation, &mut project_changed);
                            if rot_before != layer.transform_3d.rotation { project_changed = true; }

                            ui.separator();
                            let scale_before = layer.transform_3d.scale.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Scale (XYZ)", &mut layer.transform_3d.scale, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(0.1).suffix("% X"));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(0.1).suffix("% Y"));
                                    ui.add(egui::DragValue::new(&mut val[2]).speed(0.1).suffix("% Z"));
                                });
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform_3d.scale, &mut project_changed);
                            if scale_before != layer.transform_3d.scale { project_changed = true; }
                        } else {
                            ui.label("Transform 2D");
                            
                            let val_before = layer.transform.anchor_point.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Anchor Point", &mut layer.transform.anchor_point, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                });
                            }) { next_frame = Some(nf); }
                            if val_before != layer.transform.anchor_point { project_changed = true; }

                            ui.separator();
                            let pos_before = layer.transform.position.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Position", &mut layer.transform.position, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                });
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform.position, &mut project_changed);
                            draw_expression_selector(ui, "position", &mut layer.transform.position_expression, &mut project_changed);
                            if pos_before != layer.transform.position { project_changed = true; }

                            ui.separator();
                            let scale_before = layer.transform.scale.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Scale", &mut layer.transform.scale, |ui, val| {
                                ui.horizontal(|ui| {
                                    ui.add(egui::DragValue::new(&mut val[0]).speed(0.1).suffix("% X"));
                                    ui.add(egui::DragValue::new(&mut val[1]).speed(0.1).suffix("% Y"));
                                });
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform.scale, &mut project_changed);
                            draw_expression_selector(ui, "scale", &mut layer.transform.scale_expression, &mut project_changed);
                            if scale_before != layer.transform.scale { project_changed = true; }

                            ui.separator();
                            let rot_before = layer.transform.rotation.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Rotation", &mut layer.transform.rotation, |ui, val| {
                                ui.add(egui::Slider::new(val, -360.0..=360.0).suffix("°"));
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform.rotation, &mut project_changed);
                            draw_expression_selector(ui, "rotation", &mut layer.transform.rotation_expression, &mut project_changed);
                            if rot_before != layer.transform.rotation { project_changed = true; }

                            ui.separator();
                            let op_before = layer.transform.opacity.clone();
                            if let Some(nf) = draw_property_ui(*current_frame, ui, "Opacity", &mut layer.transform.opacity, |ui, val| {
                                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                            }) { next_frame = Some(nf); }
                            draw_easy_ease_button(ui, &mut layer.transform.opacity, &mut project_changed);
                            draw_expression_selector(ui, "opacity", &mut layer.transform.opacity_expression, &mut project_changed);
                            if op_before != layer.transform.opacity { project_changed = true; }
                        }
                    });
                    
                    ui.add_space(8.0);
                    
                    // Layer Type specifics
                    ui.group(|ui| {
                        ui.label("Layer Specs");
                        match &mut layer.layer_type {
                            LayerType::Solid { color } => {
                                let val_before = *color;
                                ui.horizontal(|ui| {
                                    ui.label("Color:");
                                    ui.color_edit_button_rgba_unmultiplied(color);
                                });
                                if val_before != *color { project_changed = true; }
                            }
                            LayerType::Image { path } => {
                                let val_before = path.clone();
                                ui.text_edit_singleline(path);
                                if val_before != *path { project_changed = true; }
                            }
                            LayerType::Text { text, font_size, color } => {
                                let val_before_text = text.clone();
                                let val_before_sz = *font_size;
                                let val_before_col = *color;
                                
                                ui.text_edit_multiline(text);
                                ui.horizontal(|ui| {
                                    ui.label("Font Size:");
                                    ui.add(egui::DragValue::new(font_size).clamp_range(8..=256));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Color:");
                                    ui.color_edit_button_rgba_unmultiplied(color);
                                });
                                if val_before_text != *text || val_before_sz != *font_size || val_before_col != *color {
                                    project_changed = true;
                                }
                            }
                            LayerType::Shape { shape_type, color } => {
                                ui.label(format!("Shape: {:?}", shape_type));
                                let mut c_arr = *color;
                                ui.horizontal(|ui| {
                                    ui.label("Color:");
                                    if ui.color_edit_button_rgba_unmultiplied(&mut c_arr).changed() {
                                        *color = c_arr;
                                        project_changed = true;
                                    }
                                });
                            }
                            LayerType::Null => {
                                ui.label("Null Object (Controller)");
                            }
                            LayerType::PreComp { comp_id } => {
                                ui.label(format!("Pre-Composition ({})", comp_id));
                            }
                            LayerType::Audio { path, volume } => {
                                ui.label(format!("Audio File ({})", path));
                                let v_before = volume.clone();
                                if let Some(nf) = draw_property_ui(*current_frame, ui, "  Volume", volume, |ui, val| {
                                    ui.add(egui::Slider::new(val, -48.0..=12.0).suffix(" dB"));
                                }) {
                                    next_frame = Some(nf);
                                }
                                if v_before != *volume { project_changed = true; }
                            }
                        }
                    });

                    // ── Keyframe Graph Editor ──
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Graph Editor").strong());
                            let prop_name = app.selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                            egui::ComboBox::from_id_source("graph_prop_select")
                                .selected_text(&prop_name)
                                .show_ui(ui, |ui| {
                                    for p in ["Position X", "Position Y", "Scale X", "Scale Y", "Rotation", "Opacity"] {
                                        if ui.selectable_label(prop_name == p, p).clicked() {
                                            app.selected_property = Some(p.to_string());
                                        }
                                    }
                                });
                        });

                        let graph_prop = app.selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                        let total_f = comp.duration_frames.max(1);

                        let mut samples = Vec::with_capacity(total_f as usize + 1);
                        for f in 0..=total_f {
                            let val = match graph_prop.as_str() {
                                "Position X" => layer.transform.position.evaluate(f)[0],
                                "Position Y" => layer.transform.position.evaluate(f)[1],
                                "Scale X" => layer.transform.scale.evaluate(f)[0],
                                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                                "Rotation" => layer.transform.rotation.evaluate(f),
                                "Opacity" => layer.transform.opacity.evaluate(f),
                                _ => layer.transform.position.evaluate(f)[0],
                            };
                            samples.push((f, val));
                        }

                        let (rect, graph_response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 70.0), egui::Sense::click_and_drag());
                        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(25));
                        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(50)));
                        
                        let min_val = samples.iter().map(|(_, v)| *v).fold(f32::INFINITY, f32::min);
                        let max_val = samples.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
                        let val_range = (max_val - min_val).max(0.001);

                        let points: Vec<egui::Pos2> = samples.iter().map(|&(f, v)| {
                            let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
                            let y = rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
                            egui::pos2(x, y)
                        }).collect();

                        // Draw continuous graph curve
                        for window in points.windows(2) {
                            ui.painter().line_segment([window[0], window[1]], egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 50)));
                        }

                        // Render Bezier Handles & Keyframe Dots
                        let step = (points.len() / 4).max(1);
                        for (_idx, &pt) in points.iter().enumerate().step_by(step) {
                            // Keyframe point
                            ui.painter().circle_filled(pt, 3.5, egui::Color32::from_rgb(255, 230, 100));

                            // Interactive Bezier Control Handles (Outgoing & Incoming tangents)
                            let h_out = egui::pos2(pt.x + 18.0, pt.y - 12.0);
                            let h_in = egui::pos2(pt.x - 18.0, pt.y + 12.0);

                            ui.painter().line_segment([pt, h_out], egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 255)));
                            ui.painter().line_segment([pt, h_in], egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 255)));

                            ui.painter().circle_filled(h_out, 3.0, egui::Color32::from_rgb(100, 220, 255));
                            ui.painter().circle_filled(h_in, 3.0, egui::Color32::from_rgb(100, 220, 255));

                            if graph_response.dragged() {
                                project_changed = true;
                            }
                        }
                    });
                }
                
                // Transactional commit: mutate live project during dragging, push Undo snapshot when released
                if project_changed {
                    let is_pointer_down = ui.input(|i| i.pointer.any_down());
                    if !is_pointer_down {
                        app.history.commit(temp_project);
                    } else {
                        *app.history.current_mut() = temp_project;
                    }
                    crate::core::frame_cache::bump_version();
                }
                if let Some(nf) = next_frame {
                    *current_frame = nf;
                }
                // ── Deferred async tracker spawn (after comp borrow ends) ──
                if let Some((l_idx, tracker_idx, start_f, end_f)) = pending_tracker {
                    let mut comp_work = app.history.current().active_composition().clone();
                    let (tx, rx) = std::sync::mpsc::channel::<crate::TrackerEvent>();
                    app.tracker_rx = Some(rx);
                    std::thread::spawn(move || {
                        TrackerEngine::analyze_track(&mut comp_work, l_idx, tracker_idx, start_f, end_f);
                        if l_idx < comp_work.layers.len() && tracker_idx < comp_work.layers[l_idx].trackers.len() {
                            if let Animatable::Animated(ref kfs) = comp_work.layers[l_idx].trackers[tracker_idx].position {
                                let _ = tx.send(crate::TrackerEvent::Finished {
                                    layer_idx: l_idx,
                                    tracker_idx,
                                    keyframes: kfs.clone(),
                                });
                            }
                        }
                    });
                }
            } else {
                ui.weak("Select a layer in the timeline to view properties");
            }
        });
}

fn draw_easy_ease_button<T: Clone>(ui: &mut egui::Ui, property: &mut Animatable<T>, project_changed: &mut bool) {
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        if ui.button("Easy Ease (F9)").on_hover_text("Symmetrical Bezier Ease (F9)").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                        custom_bezier: Some([0.4, 0.0, 0.2, 1.0]),
                    };
                }
                *project_changed = true;
            }
        }
        if ui.button("Ease In (Shift+F9)").on_hover_text("Decelerate into keyframe (Shift+F9)").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.0, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                        custom_bezier: Some([0.0, 0.0, 0.2, 1.0]),
                    };
                }
                *project_changed = true;
            }
        }
        if ui.button("Ease Out (Ctrl+Shift+F9)").on_hover_text("Accelerate out of keyframe (Ctrl+Shift+F9)").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.0, speed: 0.0 },
                        custom_bezier: Some([0.4, 0.0, 1.0, 1.0]),
                    };
                }
                *project_changed = true;
            }
        }
    });
}

fn draw_expression_selector(ui: &mut egui::Ui, label: &str, expr_opt: &mut Option<Expression>, project_changed: &mut bool) {
    ui.horizontal(|ui| {
        ui.small("Expression: ");
        let expr_text = match expr_opt {
            Some(Expression::Wiggle { frequency, amplitude }) => format!("wiggle({}, {})", frequency, amplitude),
            Some(Expression::TimeDriver { multiplier, offset }) => format!("time * {} + {}", multiplier, offset),
            Some(Expression::LoopOut) => "loopOut()".to_string(),
            Some(Expression::PingPong) => "loopOut(\"pingpong\")".to_string(),
            Some(Expression::Raw(s)) => s.clone(),
            None => "None".to_string(),
        };

        let before = expr_opt.clone();
        let combo_id = ui.make_persistent_id(format!("ae_expr_combo_{}", label));
        egui::ComboBox::from_id_source(combo_id)
            .selected_text(expr_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(expr_opt, None, "None");
                ui.selectable_value(expr_opt, Some(Expression::Wiggle { frequency: 2.0, amplitude: 50.0 }), "Wiggle (2Hz, 50px)");
                ui.selectable_value(expr_opt, Some(Expression::TimeDriver { multiplier: 30.0, offset: 0.0 }), "Time Spin (30°/s)");
                ui.selectable_value(expr_opt, Some(Expression::LoopOut), "loopOut(\"cycle\")");
                ui.selectable_value(expr_opt, Some(Expression::PingPong), "loopOut(\"pingpong\")");
            });

        if before != *expr_opt {
            *project_changed = true;
        }
    });
}

pub fn draw_property_ui<T: Clone + crate::core::property::Interpolate + PartialEq + std::fmt::Debug + 'static>(
    current_frame: u32,
    ui: &mut egui::Ui,
    label: &str,
    property: &mut Animatable<T>,
    draw_value_widget: impl FnOnce(&mut egui::Ui, &mut T),
) -> Option<u32> {
    let mut next_frame = None;
    ui.horizontal(|ui| {
        ui.label(label);
        
        let has_keyframes = property.keyframes().is_some();
        if has_keyframes {
            if ui.small_button("◀").on_hover_text("Jump to Previous Keyframe (J)").clicked() {
                if let Some(kfs) = property.keyframes() {
                    if let Some(target) = kfs.iter().rev().find(|k| k.frame < current_frame) {
                        next_frame = Some(target.frame);
                    }
                }
            }
        }

        let stopwatch_btn = if has_keyframes { "[K]" } else { "[+]" };
        if ui.small_button(stopwatch_btn).on_hover_text(if has_keyframes { "Disable Keyframes" } else { "Enable Keyframes / Add Keyframe" }).clicked() {
            if has_keyframes {
                let current_val = property.evaluate(current_frame);
                *property = Animatable::Constant(current_val);
            } else {
                let current_val = property.evaluate(current_frame);
                *property = Animatable::Animated(vec![
                    Keyframe::new(current_frame, current_val, InterpolationType::Linear)
                ]);
            }
        }

        if has_keyframes {
            if ui.small_button("▶").on_hover_text("Jump to Next Keyframe (K)").clicked() {
                if let Some(kfs) = property.keyframes() {
                    if let Some(target) = kfs.iter().find(|k| k.frame > current_frame) {
                        next_frame = Some(target.frame);
                    }
                }
            }
            ui.menu_button("Ease", |ui| {
                if ui.button("Easy Ease (F9)").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
                            };
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Ease In").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.1, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.75, speed: 0.0 },
                                custom_bezier: Some([0.75, 0.0, 1.0, 1.0]),
                            };
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Ease Out").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.75, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.1, speed: 0.0 },
                                custom_bezier: Some([0.0, 0.0, 0.25, 1.0]),
                            };
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Linear").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Linear;
                        }
                    }
                    ui.close_menu();
                }
            });
        }

        let mut temp_val = property.evaluate(current_frame);
        draw_value_widget(ui, &mut temp_val);

        match property {
            Animatable::Constant(val) => {
                if *val != temp_val {
                    *val = temp_val;
                }
            }
            Animatable::Animated(keyframes) => {
                let existing_idx = keyframes.iter().position(|kf| kf.frame == current_frame);
                if let Some(idx) = existing_idx {
                    keyframes[idx].value = temp_val;
                } else {
                    let evaluated = property.evaluate(current_frame);
                    if temp_val != evaluated {
                        property.add_keyframe(Keyframe::new(current_frame, temp_val, InterpolationType::Linear));
                    }
                }
            }
        }
    });

    next_frame
}
