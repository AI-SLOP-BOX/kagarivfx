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
            if let crate::TrackerEvent::Finished { ref layer_id, layer_idx, tracker_idx, keyframes } = event {
                let comp_mut = app.history.current_mut().active_composition_mut();
                let layer_opt = comp_mut.layers.iter().position(|l| l.id == *layer_id)
                    .or_else(|| if layer_idx < comp_mut.layers.len() { Some(layer_idx) } else { None });

                if let Some(idx) = layer_opt {
                    let layer = &mut comp_mut.layers[idx];
                    if tracker_idx < layer.trackers.len() {
                        layer.trackers[tracker_idx].position = Animatable::Animated(keyframes);
                        crate::core::frame_cache::bump_version();
                        log::info!("Async Motion Tracker analysis completed for layer {} ({}), tracker {}", layer.name, layer_id, tracker_idx);
                    }
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
                if ui.selectable_label(app.left_tab_idx == 0, "Project").clicked() { app.left_tab_idx = 0; }
                if ui.selectable_label(app.left_tab_idx == 1, "Effect Controls").clicked() { app.left_tab_idx = 1; }
                if ui.selectable_label(app.left_tab_idx == 2, "Flowchart").clicked() { app.left_tab_idx = 2; }
            });
            ui.separator();

            if app.left_tab_idx == 0 {
                crate::ui::project_panel::draw(app, ui);
                return;
            }

            if app.left_tab_idx == 2 {
                crate::ui::flowchart_inspector::draw_flowchart_inspector(app, ui);
                return;
            }

            ui.heading("Layer Properties");
            ui.separator();

            let mut project_changed = false;
            let mut next_frame = None;
            // Deferred tracker spawn: populated inside group closure, consumed after comp borrow ends.
            let mut pending_tracker: Option<(usize, usize, u32, u32)> = None; // (layer_idx, tracker_idx, start, end)

            let mut selected_prop = app.selected_property.clone();
            // Access live project mutably without per-frame cloning
            let temp_project = app.history.current_mut();
            
            // ── Camera Suite Panel ──
            {
                let comp = temp_project.active_composition_mut();
                crate::ui::inspector_camera::draw_camera_settings(ui, comp, *current_frame, &mut project_changed, &mut next_frame);
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
                    if ui.button("📋 Copy Expression Ref").clicked() {
                        ui.output_mut(|o| o.copied_text = "thisComp.layer(thisLayer).transform.position".to_string());
                        app.toasts.info("Copied expression reference to clipboard!");
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
                            egui::ComboBox::from_id_source(format!("blend_mode_combo_{}", layer.id))
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
                            egui::ComboBox::from_id_source(format!("label_color_combo_{}", layer.id))
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
                            egui::ComboBox::from_id_source(format!("track_matte_combo_{}", layer.id))
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

                        // ── Motion Blur Advanced Parameters ──
                        if layer.motion_blur {
                            ui.horizontal(|ui| {
                                ui.label("Shutter Angle:");
                                let sa_id = ui.make_persistent_id(format!("ae_shutter_angle_{}", layer.id));
                                let mut shutter_angle = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(sa_id, || 180.0f32));
                                if ui.add(egui::DragValue::new(&mut shutter_angle).speed(1.0).clamp_range(0.0..=720.0).suffix("°")).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(sa_id, shutter_angle));
                                }

                                ui.add_space(8.0);
                                ui.label("Phase:");
                                let sp_id = ui.make_persistent_id(format!("ae_shutter_phase_{}", layer.id));
                                let mut shutter_phase = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(sp_id, || -90.0f32));
                                if ui.add(egui::DragValue::new(&mut shutter_phase).speed(1.0).clamp_range(-180.0..=180.0).suffix("°")).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(sp_id, shutter_phase));
                                }
                            });
                        }

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

                            egui::ComboBox::from_id_source(format!("parent_select_combo_{}", layer.id))
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

                    // ── Masks Control Section ──
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.collapsing("🎭 Masks", |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("+ Add Rect Mask").clicked() {
                                    let mask_idx = layer.masks.len() + 1;
                                    let new_mask = crate::core::mask::Mask::new_rect(
                                        format!("mask_rect_{}", mask_idx),
                                        format!("Mask Rect {}", mask_idx),
                                        200.0, 200.0, 300.0, 200.0,
                                    );
                                    layer.masks.push(new_mask);
                                    project_changed = true;
                                }
                                if ui.button("+ Add Oval Mask").clicked() {
                                    let mask_idx = layer.masks.len() + 1;
                                    let new_mask = crate::core::mask::Mask::new_ellipse(
                                        format!("mask_oval_{}", mask_idx),
                                        format!("Mask Oval {}", mask_idx),
                                        400.0, 400.0, 150.0, 100.0,
                                    );
                                    layer.masks.push(new_mask);
                                    project_changed = true;
                                }
                            });

                            if layer.masks.is_empty() {
                                ui.weak("No masks applied to this layer");
                            } else {
                                let mut mask_to_remove = None;
                                for (m_idx, mask) in layer.masks.iter_mut().enumerate() {
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut mask.enabled, "");
                                        ui.text_edit_singleline(&mut mask.name);
                                        
                                        // Invert toggle
                                        let inv_before = mask.inverted;
                                        ui.checkbox(&mut mask.inverted, "Invert 🔄");
                                        if inv_before != mask.inverted { project_changed = true; }

                                        if ui.small_button("🗑").clicked() {
                                            mask_to_remove = Some(m_idx);
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Mode:");
                                        let mode_before = mask.mode;
                                        egui::ComboBox::from_id_source(format!("mask_mode_{}", mask.id))
                                            .selected_text(format!("{:?}", mask.mode))
                                            .show_ui(ui, |ui| {
                                                use crate::core::mask::MaskMode;
                                                for mode in [MaskMode::Add, MaskMode::Subtract, MaskMode::Intersect, MaskMode::None] {
                                                    ui.selectable_value(&mut mask.mode, mode, format!("{:?}", mode));
                                                }
                                            });
                                        if mode_before != mask.mode { project_changed = true; }
                                    });

                                    // Feather slider
                                    let feather_before = mask.feather.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "  Feather", &mut mask.feather, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=250.0).suffix(" px"));
                                    }) { next_frame = Some(nf); }
                                    if feather_before != mask.feather { project_changed = true; }

                                    // Opacity slider
                                    let op_before = mask.opacity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "  Opacity", &mut mask.opacity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                                    }) { next_frame = Some(nf); }
                                    if op_before != mask.opacity { project_changed = true; }

                                    // Expansion slider
                                    let exp_before = mask.expansion.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "  Expansion", &mut mask.expansion, |ui, val| {
                                        ui.add(egui::Slider::new(val, -100.0..=100.0).suffix(" px"));
                                    }) { next_frame = Some(nf); }
                                    if exp_before != mask.expansion { project_changed = true; }
                                }

                                if let Some(m_idx) = mask_to_remove {
                                    layer.masks.remove(m_idx);
                                    project_changed = true;
                                }
                            }
                        });
                    });

                    // ── Keyframe Graph Editor ──
                    ui.add_space(8.0);
                    crate::ui::graph_editor::draw_graph_editor(&mut selected_prop, ui, comp.duration_frames, layer, &mut project_changed);
                }
                
                // Transactional commit: lazy snapshot push on mouse release (zero clones while idle or dragging)
                if project_changed {
                    let is_pointer_down = ui.input(|i| i.pointer.any_down());
                    if !is_pointer_down {
                        let snapshot = app.history.current().clone();
                        app.history.commit(snapshot);
                    }
                    crate::core::frame_cache::bump_version();
                }
                if let Some(nf) = next_frame {
                    *current_frame = nf;
                }
                // ── Deferred async tracker spawn (after comp borrow ends) ──
                if let Some((l_idx, tracker_idx, start_f, end_f)) = pending_tracker {
                    if let Some(old_flag) = app.tracker_cancel_flag.take() {
                        old_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                    app.tracker_cancel_flag = Some(cancel_flag.clone());

                    let mut comp_work = app.history.current().active_composition().clone();
                    let (tx, rx) = std::sync::mpsc::channel::<crate::TrackerEvent>();
                    app.tracker_rx = Some(rx);
                    std::thread::spawn(move || {
                        TrackerEngine::analyze_track_cancellable(&mut comp_work, l_idx, tracker_idx, start_f, end_f, Some(cancel_flag));
                        if l_idx < comp_work.layers.len() && tracker_idx < comp_work.layers[l_idx].trackers.len() {
                            let l_id = comp_work.layers[l_idx].id.clone();
                            if let Animatable::Animated(ref kfs) = comp_work.layers[l_idx].trackers[tracker_idx].position {
                                let _ = tx.send(crate::TrackerEvent::Finished {
                                    layer_idx: l_idx,
                                    layer_id: l_id,
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
