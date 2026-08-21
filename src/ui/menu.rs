use eframe::egui;

pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    app.history = crate::core::history::ProjectHistory::new(
                        crate::core::timeline::Project::default(),
                    );
                    app.selected_layer_idx = None;
                    app.selected_layers.clear();
                    crate::core::frame_cache::bump_version();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save Project As...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("After Effects OSS Project", &["json", "aevfx"])
                        .set_file_name("project.aevfx.json")
                        .save_file()
                    {
                        let project = app.history.current();
                        match serde_json::to_string_pretty(project) {
                            Ok(json) => match std::fs::write(&path, json) {
                                Ok(_) => {
                                    app.project_path = path.to_string_lossy().to_string();
                                    app.toasts.info(format!("💾 Project saved: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                                }
                                Err(err) => {
                                    app.toasts.error(format!("❌ Failed to save project file: {}", err));
                                }
                            },
                            Err(err) => {
                                app.toasts.error(format!("❌ Failed to serialize project: {}", err));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Open Project...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("After Effects OSS Project", &["json", "aevfx"])
                        .pick_file()
                    {
                        match std::fs::read_to_string(&path) {
                            Ok(json) => match serde_json::from_str::<crate::core::timeline::Project>(&json) {
                                Ok(project) => {
                                    app.history = crate::core::history::ProjectHistory::new(project);
                                    app.selected_layer_idx = None;
                                    app.selected_layers.clear();
                                    app.project_path = path.to_string_lossy().to_string();
                                    crate::core::frame_cache::bump_version();
                                    app.toasts.info(format!("📂 Project opened: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                                }
                                Err(err) => {
                                    app.toasts.error(format!("❌ Failed to parse project file: {}", err));
                                }
                            },
                            Err(err) => {
                                app.toasts.error(format!("❌ Could not read file: {}", err));
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export OpenTimelineIO (.otio.json)").clicked() {
                    let comp = app.history.current().active_composition();
                    let otio = crate::core::integration::OtioTimeline::from_composition(comp);
                    match serde_json::to_string_pretty(&otio) {
                        Ok(json) => match std::fs::write(&app.otio_path, json) {
                            Ok(_) => {
                                app.toasts.info(format!("🎬 Exported OTIO: {}", app.otio_path));
                            }
                            Err(err) => {
                                app.toasts.error(format!("❌ Failed to save OTIO file: {}", err));
                            }
                        },
                        Err(err) => {
                            app.toasts.error(format!("❌ Failed to serialize OTIO: {}", err));
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import OpenTimelineIO (.otio.json)").clicked() {
                    match std::fs::read_to_string(&app.otio_path) {
                        Ok(json) => match serde_json::from_str::<crate::core::integration::OtioTimeline>(&json) {
                            Ok(otio) => {
                                let comp = otio.to_composition();
                                app.modify_project(|p| p.compositions[0] = comp);
                                app.toasts.info(format!("🎬 Imported OTIO: {}", app.otio_path));
                            }
                            Err(err) => {
                                app.toasts.error(format!("❌ Invalid OTIO format: {}", err));
                            }
                        },
                        Err(err) => {
                            app.toasts.error(format!("❌ Could not read OTIO file: {}", err));
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export Video (MP4)...").clicked() {
                    app.show_export_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo_sc = crate::ui::shortcuts::format_shortcut("Z", true, false, false);
                let undo_btn = egui::Button::new("Undo (元に戻す)").shortcut_text(undo_sc);
                if ui.add_enabled(app.history.can_undo(), undo_btn).clicked() {
                    app.history.undo();
                    ui.close_menu();
                }
                let redo_sc = crate::ui::shortcuts::format_shortcut("Z", true, true, false);
                let redo_btn = egui::Button::new("Redo (やり直す)").shortcut_text(redo_sc);
                if ui.add_enabled(app.history.can_redo(), redo_btn).clicked() {
                    app.history.redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Duplicate").shortcut_text("Cmd+D")).clicked() {
                    // Handled via keyboard shortcut in timeline
                    app.toasts.info("Select a layer in the Timeline, then press Cmd+D to duplicate");
                    ui.close_menu();
                }
            });
            ui.menu_button("Composition", |ui| {
                let comp_sc = crate::ui::shortcuts::format_shortcut("K", true, false, false);
                let btn = egui::Button::new("Composition Settings...").shortcut_text(comp_sc);
                if ui.add(btn).clicked() {
                    app.show_comp_settings = true;
                    ui.close_menu();
                }
                let rq_sc = crate::ui::shortcuts::format_shortcut("M", true, false, false);
                if ui.add(egui::Button::new("Add to Render Queue").shortcut_text(rq_sc)).clicked() {
                    app.show_export_dialog = true;
                    ui.close_menu();
                }
            });
            ui.menu_button("Layer", |ui| {
                ui.menu_button("New", |ui| {
                    let solid_sc = crate::ui::shortcuts::format_shortcut("Y", true, false, false);
                    if ui.add(egui::Button::new("Solid...").shortcut_text(solid_sc)).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Solid {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new(id, name, crate::core::timeline::LayerType::Solid { color: [0.2, 0.5, 0.9, 1.0] }, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Text").shortcut_text("Cmd+Alt+Shift+T")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Text {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new(id, name, crate::core::timeline::LayerType::new_text("Title Text", 72, [1.0, 1.0, 1.0, 1.0]), total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Null Object").shortcut_text("Cmd+Alt+Shift+Y")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Null {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new_null(id, name, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Adjustment Layer").shortcut_text("Cmd+Alt+Y")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Adjustment Layer {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new_adjustment(id, name, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("Adjustment Layer").shortcut_text("Cmd+Alt+Y")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Adjustment Layer {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new_adjustment(id, name, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Time", |ui| {
                    if ui.add(egui::Button::new("Enable Time Remapping").shortcut_text("Cmd+Alt+T")).clicked() {
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Time Stretch...").shortcut_text("Cmd+Shift+K")).clicked() {
                        app.toasts.info("Layer Time Stretch: Scale layer in/out duration by factor");
                        ui.close_menu();
                    }
                });
                ui.separator();
                if ui.add(egui::Button::new("Pre-Compose...").shortcut_text("Cmd+Shift+C")).clicked() {
                    ui.close_menu();
                }
            });
            ui.menu_button("Effect", |ui| {
                ui.menu_button("Blur & Sharpen", |ui| {
                    if ui.button("Gaussian Blur").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Color Correction", |ui| {
                    if ui.button("Color Tint").clicked() { ui.close_menu(); }
                    if ui.button("Levels").clicked() { ui.close_menu(); }
                    if ui.button("Hue/Saturation").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Stylize", |ui| {
                    if ui.button("Glow").clicked() { ui.close_menu(); }
                    if ui.button("Vignette").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Distort", |ui| {
                    if ui.button("Mesh Warp").clicked() { ui.close_menu(); }
                    if ui.button("Chromatic Aberration").clicked() { ui.close_menu(); }
                });
            });
            ui.menu_button("Animation", |ui| {
                if ui.add(egui::Button::new("Easy Ease").shortcut_text("F9")).clicked() { ui.close_menu(); }
                if ui.button("Keyframe Assistant").clicked() { ui.close_menu(); }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut app.show_grid, "Show Grid");
                ui.checkbox(&mut app.show_guides, "Show Safe Zones");
                ui.checkbox(&mut app.show_handles, "Show Handles");
                ui.separator();
                if ui.button("Reset Timeline Zoom (100%)").clicked() {
                    app.timeline_zoom = 1.0;
                    ui.close_menu();
                }
                if ui.button("Purge All RAM Cache").clicked() {
                    app.frame_cache.invalidate_all();
                    crate::core::frame_cache::bump_version();
                    ui.close_menu();
                }
                ui.menu_button("Workspaces", |ui| {
                    if ui.button("🖥 Standard").clicked() {
                        app.right_tab_idx = 0;
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                    if ui.button("📈 Motion Graphics").clicked() {
                        app.right_tab_idx = 0;
                        app.show_graph_editor = true;
                        ui.close_menu();
                    }
                    if ui.button("🎛 VFX & Color").clicked() {
                        app.right_tab_idx = 30; // Effect Controls
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                    if ui.button("🎵 Audio Editing").clicked() {
                        app.right_tab_idx = 7; // Audio Panel
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard Shortcuts Reference...").clicked() {
                    let help_id = egui::Id::new("show_shortcuts_modal");
                    ctx.data_mut(|d| d.insert_temp(help_id, true));
                    ui.close_menu();
                }
            });
        });
    });

    let help_id = egui::Id::new("show_shortcuts_modal");
    let mut show_help = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(help_id, || false));
    if show_help {
        egui::Window::new("⌨️ Keyboard Shortcuts Reference")
            .open(&mut show_help)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("After Effects OSS - Shortcuts");
                ui.separator();
                egui::Grid::new("shortcuts_grid").striped(true).show(ui, |ui| {
                    ui.label("Spacebar"); ui.label("Play / Pause RAM Preview"); ui.end_row();
                    ui.label("V"); ui.label("Selection Tool"); ui.end_row();
                    ui.label("H"); ui.label("Hand Tool (Pan)"); ui.end_row();
                    ui.label("Z"); ui.label("Zoom Tool"); ui.end_row();
                    ui.label("W"); ui.label("Rotation Tool"); ui.end_row();
                    ui.label("Y"); ui.label("Anchor Point Tool"); ui.end_row();
                    ui.label("Cmd + Z"); ui.label("Undo"); ui.end_row();
                    ui.label("Cmd + Shift + Z"); ui.label("Redo"); ui.end_row();
                    ui.label("J / K"); ui.label("Jump to Prev / Next Keyframe"); ui.end_row();
                    ui.label("F9"); ui.label("Apply Easy Ease to Keyframes"); ui.end_row();
                });
            });
        ctx.data_mut(|d| d.insert_temp(help_id, show_help));
    }

    // 📦 Pre-Compose Dialog (Cmd+Shift+C)
    crate::ui::precompose_dialog::draw_precompose_dialog(app, ctx);
    crate::ui::recovery_dialog::draw_recovery_dialog(app, ctx);
}
