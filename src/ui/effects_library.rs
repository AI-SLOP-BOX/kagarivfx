use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Effect, EffectType, ColorConversionMode};
use crate::core::property::Animatable;
use crate::ui::inspector::draw_property_ui;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: &mut u32) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(240.0)
        .show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.right_tab_idx, 0, "Effects & Presets");
                    ui.selectable_value(&mut app.right_tab_idx, 4, "Preview");
                    ui.selectable_value(&mut app.right_tab_idx, 2, "Info");
                    ui.selectable_value(&mut app.right_tab_idx, 7, "Audio");
                    ui.selectable_value(&mut app.right_tab_idx, 23, "Mixer");
                    ui.selectable_value(&mut app.right_tab_idx, 1, "Align");
                    ui.selectable_value(&mut app.right_tab_idx, 3, "Tracker");
                    ui.selectable_value(&mut app.right_tab_idx, 5, "Paint");
                    ui.selectable_value(&mut app.right_tab_idx, 21, "Fonts");
                    ui.selectable_value(&mut app.right_tab_idx, 27, "Character");
                    ui.selectable_value(&mut app.right_tab_idx, 28, "Paragraph");
                    ui.selectable_value(&mut app.right_tab_idx, 26, "Layer Styles");
                    ui.selectable_value(&mut app.right_tab_idx, 19, "Lumetri Color");
                    ui.selectable_value(&mut app.right_tab_idx, 20, "Libraries");
                    ui.selectable_value(&mut app.right_tab_idx, 18, "3D Views");
                    ui.selectable_value(&mut app.right_tab_idx, 25, "3D Options");
                    ui.selectable_value(&mut app.right_tab_idx, 11, "Essential Graphics");
                    ui.selectable_value(&mut app.right_tab_idx, 12, "Content-Aware Fill");
                    ui.selectable_value(&mut app.right_tab_idx, 9, "Masks");
                    ui.selectable_value(&mut app.right_tab_idx, 10, "Expressions");
                    ui.selectable_value(&mut app.right_tab_idx, 16, "Color (OCIO)");
                    ui.selectable_value(&mut app.right_tab_idx, 13, "Metadata");
                    ui.selectable_value(&mut app.right_tab_idx, 14, "Scripting Console");
                    ui.selectable_value(&mut app.right_tab_idx, 15, "Workspaces");
                    ui.selectable_value(&mut app.right_tab_idx, 22, "Render Presets");
                    ui.selectable_value(&mut app.right_tab_idx, 24, "Velocity");
                    ui.selectable_value(&mut app.right_tab_idx, 8, "Time");
                    ui.selectable_value(&mut app.right_tab_idx, 6, "Markers");
                });
            });
            ui.separator();

            let mut project_changed = false;
            let mut next_frame = None;
            let mut current_frame_reset = None;

            // Access live project mutably without per-frame cloning
            let temp_project = app.history.current_mut();

            if app.right_tab_idx == 3 {
                crate::ui::tracker_panel::draw_tracker_panel(app, ui, *current_frame);
                return;
            }

            if app.right_tab_idx == 4 {
                let total_frames = temp_project.active_composition().duration_frames;
                crate::ui::transport_panel::draw_transport_panel(app, ui, current_frame, total_frames);
                return;
            }

            if app.right_tab_idx == 5 {
                crate::ui::paint_panel::draw_paint_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 6 {
                crate::ui::marker_panel::draw_marker_panel(app, ui, *current_frame);
                return;
            }

            if app.right_tab_idx == 7 {
                crate::ui::audio_meter::draw_content(app, ui);
                ui.add_space(8.0);
                crate::ui::audio_panel::draw_audio_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 8 {
                crate::ui::time_remap_panel::draw_time_remap_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 9 {
                crate::ui::mask_panel::draw_mask_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 10 {
                crate::ui::expression_panel::draw_expression_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 11 {
                crate::ui::essential_graphics::draw_essential_graphics(app, ui);
                return;
            }

            if app.right_tab_idx == 12 {
                crate::ui::content_aware_fill::draw_content_aware_fill(app, ui);
                return;
            }

            if app.right_tab_idx == 13 {
                crate::ui::metadata_panel::draw_metadata_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 14 {
                crate::ui::scripting_console::draw_scripting_console(app, ui);
                return;
            }

            if app.right_tab_idx == 15 {
                crate::ui::workspace_manager::draw_workspace_manager(app, ui);
                return;
            }

            if app.right_tab_idx == 16 {
                crate::ui::color_management::draw_color_management(app, ui);
                return;
            }

            if app.right_tab_idx == 17 {
                crate::ui::flowchart_inspector::draw_flowchart_inspector(app, ui);
                return;
            }

            if app.right_tab_idx == 18 {
                crate::ui::camera_views::draw_camera_views(app, ui);
                return;
            }

            if app.right_tab_idx == 19 {
                crate::ui::lumetri_color::draw_lumetri_color(app, ui);
                return;
            }

            if app.right_tab_idx == 20 {
                crate::ui::cc_libraries::draw_cc_libraries(app, ui);
                return;
            }

            if app.right_tab_idx == 21 {
                crate::ui::font_picker::draw_font_picker(app, ui);
                return;
            }

            if app.right_tab_idx == 22 {
                crate::ui::render_presets::draw_render_presets(app, ui);
                return;
            }

            if app.right_tab_idx == 23 {
                crate::ui::audio_mixer::draw_audio_mixer(app, ui);
                return;
            }

            if app.right_tab_idx == 24 {
                crate::ui::speed_graph_options::draw_speed_graph_options(app, ui);
                return;
            }

            if app.right_tab_idx == 25 {
                crate::ui::camera_light_options::draw_camera_light_options(app, ui);
                return;
            }

            if app.right_tab_idx == 26 {
                crate::ui::layer_styles::draw_layer_styles(app, ui);
                return;
            }

            if app.right_tab_idx == 27 {
                let mut temp_proj = app.history.current().clone();
                let changed = crate::ui::character_panel::draw_character_panel(app, ui, temp_proj.active_composition_mut(), *current_frame);
                if changed {
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                }
                return;
            }

            if app.right_tab_idx == 28 {
                crate::ui::paragraph_panel::draw_paragraph_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 1 {
                crate::ui::align_panel::draw_align_panel(app, ui);
                return;
            }

            if app.right_tab_idx == 2 {
                ui.heading("Info");
                ui.separator();
                if let Some(idx) = app.selected_layer_idx {
                    let comp = temp_project.active_composition();
                    if idx < comp.layers.len() {
                        let layer = &comp.layers[idx];
                        ui.label(format!("Layer: {}", layer.name));
                        ui.label(format!("ID: {}", layer.id));
                        let pos = layer.transform.position.evaluate(*current_frame);
                        let scale = layer.transform.scale.evaluate(*current_frame);
                        let rot = layer.transform.rotation.evaluate(*current_frame);
                        let op = layer.transform.opacity.evaluate(*current_frame);
                        ui.weak(format!("Position: ({:.1}, {:.1})", pos[0], pos[1]));
                        ui.weak(format!("Scale: ({:.1}%, {:.1}%)", scale[0], scale[1]));
                        ui.weak(format!("Rotation: {:.1}°", rot));
                        ui.weak(format!("Opacity: {:.1}%", op));
                    }
                } else {
                    ui.weak("No layer selected.");
                }
            } else {
                ui.heading("Effects & Presets");
                ui.separator();
            
            if let Some(idx) = app.selected_layer_idx {
                ui.label("Add Effect to Selected Layer:");
                ui.horizontal(|ui| {
                    ui.small("Search:");
                    ui.add(egui::TextEdit::singleline(&mut app.effects_search_query).hint_text("Search effects..."));
                });
                let q = app.effects_search_query.to_lowercase();

                ui.vertical(|ui| {
                    for preset in crate::ui::effects_controls::get_all_effect_presets() {
                        if (q.is_empty() || preset.search_key.contains(&q)) && ui.button(preset.button_label).clicked() {
                            let comp = temp_project.active_composition_mut();
                            if idx < comp.layers.len() {
                                let len = comp.layers[idx].effects.len();
                                comp.layers[idx].effects.push((preset.create_fn)(len));
                                project_changed = true;
                            }
                        }
                    }
                });
            } else {
                ui.weak("Select a layer to apply effects");
            }
            
            ui.separator();
            
            // Show list of applied effects
            if let Some(idx) = app.selected_layer_idx {
                let comp = temp_project.active_composition_mut();
                if idx < comp.layers.len() {
                    ui.label("Applied Effects:");
                    let layer = &mut comp.layers[idx];
                    let mut effect_to_remove = None;
                    let mut effect_to_swap = None;
                    let effects_count = layer.effects.len();

                    for (e_idx, effect) in layer.effects.iter_mut().enumerate() {
                        let fx_persistent_id = ui.make_persistent_id(format!("ae_fx_item_{}_{}", effect.id, e_idx));
                        ui.push_id(fx_persistent_id, |ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("[X]").on_hover_text("Delete Effect").clicked() {
                                    effect_to_remove = Some(e_idx);
                                }
                                if e_idx > 0 && ui.small_button("▲").on_hover_text("Move Up").clicked() {
                                    effect_to_swap = Some((e_idx, e_idx - 1));
                                }
                                if e_idx + 1 < effects_count && ui.small_button("▼").on_hover_text("Move Down").clicked() {
                                    effect_to_swap = Some((e_idx, e_idx + 1));
                                }
                                let fx_label = if effect.enabled { "[fx]" } else { "[fx off]" };
                                if ui.selectable_label(effect.enabled, fx_label).on_hover_text("Toggle Effect Bypass (ON/OFF)").clicked() {
                                    effect.enabled = !effect.enabled;
                                    project_changed = true;
                                }
                            });
                            ui.collapsing(&effect.name, |ui| {
                                crate::ui::effects_controls::draw_effect_type_ui(
                                    &mut effect.effect_type,
                                    ui,
                                    *current_frame,
                                    &mut project_changed,
                                    &mut next_frame,
                                );
                            });
                    });
                }
                    if let Some(r_idx) = effect_to_remove {
                        layer.effects.remove(r_idx);
                        project_changed = true;
                    }
                    if let Some((a, b)) = effect_to_swap {
                        layer.effects.swap(a, b);
                        project_changed = true;
                    }
                }
            }
            }
            
            ui.separator();
            ui.heading("External NLE Link");
            ui.add_space(4.0);
            
            // Connection Status Indicators
            if let Some(app_name) = &app.connected_app {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(egui::Color32::from_rgb(50, 220, 50), format!("[ONLINE] Connected to {}", app_name));
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(egui::Color32::from_rgb(220, 100, 100), "[OFFLINE] Listening on 127.0.0.1:9000");
                });
            }
            ui.add_space(8.0);
            
            // OTIO File Path Input
            ui.label("OTIO File Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.otio_path);
            });
            
            ui.horizontal(|ui| {
                if ui.button("Import OTIO").clicked() {
                    if let Ok(json_str) = std::fs::read_to_string(&app.otio_path) {
                        if let Ok(otio_timeline) = crate::core::integration::OtioTimeline::from_json(&json_str) {
                            let new_comp = otio_timeline.to_composition();
                            let comp = temp_project.active_composition_mut();
                            comp.name = new_comp.name;
                            comp.width = new_comp.width;
                            comp.height = new_comp.height;
                            comp.fps = new_comp.fps;
                            comp.duration_frames = new_comp.duration_frames;
                            comp.layers = new_comp.layers;
                            current_frame_reset = Some(0);
                            project_changed = true;
                            log::info!("Successfully imported OTIO composition");
                        } else {
                            log::error!("Failed to parse OTIO JSON");
                        }
                    } else {
                        log::error!("Failed to read OTIO file from path: {}", app.otio_path);
                    }
                }
                if ui.button("Export OTIO").clicked() {
                    let active_comp = temp_project.active_composition();
                    let otio_timeline = crate::core::integration::OtioTimeline::from_composition(active_comp);
                    if let Ok(json_str) = otio_timeline.to_json() {
                        if std::fs::write(&app.otio_path, json_str).is_ok() {
                            log::info!("Successfully exported OTIO composition to: {}", app.otio_path);
                        } else {
                            log::error!("Failed to write OTIO file to path: {}", app.otio_path);
                        }
                    }
                }
            });

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
            if let Some(cf) = current_frame_reset {
                app.current_frame = cf;
                *current_frame = cf;
            }
        });
}
