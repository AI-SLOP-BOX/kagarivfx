use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{LayerType, LabelColor, TrackMatteMode, TrackerPoint, BlendMode};
use crate::core::property::Animatable;
use crate::core::keyframe::{InterpolationType, BezierControlPoint};
use crate::core::tracker_engine::TrackerEngine;
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::theme::colors;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: &mut u32) {
    // ── Non-blocking Async Motion Tracker Event Receiver ──
    let mut tracker_completed = false;
    if let Some(ref rx) = app.tracker_rx {
        while let Ok(event) = rx.try_recv() {
            if let crate::TrackerEvent::Finished { ref layer_id, layer_idx, tracker_idx, keyframes } = event {
                let comp_mut = app.history.current_mut().active_composition_mut();
                let layer_opt = comp_mut.layers.iter().position(|l| l.id == *layer_id)
                    .or(if layer_idx < comp_mut.layers.len() { Some(layer_idx) } else { None });

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

    // Update panel animation
    let dt = ctx.input(|i| i.stable_dt);
    app.inspector_animation.update(dt);

    let animated_width = crate::ui::panel_animation::animate_panel_width(
        ctx, &app.inspector_animation, 400.0
    ).max(200.0);

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(280.0)
        .min_width(animated_width)
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

            // ── Composition info banner ──
            {
                let comp = temp_project.active_composition();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("📐 {}×{}  ⏱ {:.0}fps", comp.width, comp.height, comp.fps))
                        .small().color(colors::TEXT_SECONDARY));
                });
            }
            
            // ── Camera Suite Panel ──
            {
                let comp = temp_project.active_composition_mut();
                crate::ui::inspector_camera::draw_camera_settings(ui, comp, *current_frame, &mut project_changed, &mut next_frame);
            }

            ui.add_space(8.0);

            // ── Multi-Layer Selection Batch Controls ──
            if app.selected_layers.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Multi-Selection: {} Layers", app.selected_layers.len())).strong().color(colors::TEXT_ACCENT));
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
                    // Cache a comp snapshot for the inline expression Test button
                    // (thisComp.* context), rebuilt each frame while inspector is open.
                    let snap = std::sync::Arc::new(
                        crate::core::expression_engine::build_comp_snapshot(comp, *current_frame),
                    );
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new("ae_expr_comp_snap"), snap));

                    // ── Safe Parent selector logic ──
                    let other_layers: Vec<(String, String)> = comp.layers.iter()
                        .enumerate()
                        .filter(|&(i, l)| i != idx && !matches!(l.layer_type, LayerType::Null))
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
                            egui::ComboBox::from_id_salt(format!("blend_mode_combo_{}", layer.id))
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
                            egui::ComboBox::from_id_salt(format!("label_color_combo_{}", layer.id))
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
                            egui::ComboBox::from_id_salt(format!("track_matte_combo_{}", layer.id))
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
                                if ui.add(egui::DragValue::new(&mut shutter_angle).speed(1.0).range(0.0..=720.0).suffix("°")).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(sa_id, shutter_angle));
                                }

                                ui.add_space(8.0);
                                ui.label("Phase:");
                                let sp_id = ui.make_persistent_id(format!("ae_shutter_phase_{}", layer.id));
                                let mut shutter_phase = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(sp_id, || -90.0f32));
                                if ui.add(egui::DragValue::new(&mut shutter_phase).speed(1.0).range(-180.0..=180.0).suffix("°")).changed() {
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

                            egui::ComboBox::from_id_salt(format!("parent_select_combo_{}", layer.id))
                                .selected_text(parent_name)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut layer.parent_id, None, "None");
                                    for (id, name) in &other_layers {
                                        ui.selectable_value(&mut layer.parent_id, Some(id.clone()), name);
                                    }
                                });

                            // Pick whip button: click to enter pick mode, then click a layer in timeline
                            let whip_text = if app.pick_whip_mode { "🔗 Picking..." } else { "🔗" };
                            if ui.button(whip_text).on_hover_text("Pick Whip: click this, then click a layer to set as parent").clicked() {
                                app.pick_whip_mode = !app.pick_whip_mode;
                                app.pick_whip_target = None;
                            }

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
                                    } else if crate::ui::custom_widgets::ae_button(ui, "Analyze Forward >").clicked() {
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
                    crate::ui::inspector_layer::draw_layer_transforms(ui, layer, *current_frame, comp.fps, &mut project_changed, &mut next_frame);
                    
                    ui.add_space(8.0);
                    
                    // Layer Type specifics
                    let comp_fps = comp.fps as f32;
                    crate::ui::inspector_layer::draw_layer_type_specs(ui, layer, *current_frame, &mut project_changed, &mut next_frame, comp_fps);

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
                                        egui::ComboBox::from_id_salt(format!("mask_mode_{}", mask.id))
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
                    crate::ui::graph_editor::draw_graph_editor(&mut selected_prop, ui, comp.duration_frames, layer, &mut project_changed, &mut app.linked_tangent);
                }
                
                // ── Transactional Commit (Issue #2 fix) ──────────────────────────────
                // begin_drag() captures a pre-edit snapshot the moment the pointer goes
                // down (no-op if already started). The live project is mutated in-place
                // via current_mut(). commit_drag() pushes one single Undo entry on release.
                if project_changed {
                    let is_pointer_down = ui.input(|i| i.pointer.any_down());
                    if is_pointer_down {
                        app.begin_drag("Inspector Edit");
                    } else {
                        app.commit_drag();
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


