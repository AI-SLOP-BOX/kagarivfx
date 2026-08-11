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
                        if let Ok(json) = serde_json::to_string_pretty(project) {
                            if std::fs::write(&path, json).is_ok() {
                                app.project_path = path.to_string_lossy().to_string();
                                log::info!("Native project saved to {}", app.project_path);
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
                        if let Ok(json) = std::fs::read_to_string(&path) {
                            if let Ok(project) = serde_json::from_str::<crate::core::timeline::Project>(&json) {
                                app.history = crate::core::history::ProjectHistory::new(project);
                                app.selected_layer_idx = None;
                                app.selected_layers.clear();
                                app.project_path = path.to_string_lossy().to_string();
                                crate::core::frame_cache::bump_version();
                                log::info!("Native project loaded from {}", app.project_path);
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export OpenTimelineIO (.otio.json)").clicked() {
                    let comp = app.history.current().active_composition();
                    let otio = crate::core::integration::OtioTimeline::from_composition(comp);
                    if let Ok(json) = serde_json::to_string_pretty(&otio) {
                        let _ = std::fs::write(&app.otio_path, json);
                        log::info!("Exported OTIO timeline to {}", app.otio_path);
                    }
                    ui.close_menu();
                }
                if ui.button("Import OpenTimelineIO (.otio.json)").clicked() {
                    if let Ok(json) = std::fs::read_to_string(&app.otio_path) {
                        if let Ok(otio) = serde_json::from_str::<crate::core::integration::OtioTimeline>(&json) {
                            let comp = otio.to_composition();
                            app.modify_project(|p| p.compositions[0] = comp);
                            log::info!("Imported OTIO timeline from {}", app.otio_path);
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
                let undo_btn = egui::Button::new("Undo (元に戻す)").shortcut_text("Cmd+Z");
                if ui.add_enabled(app.history.can_undo(), undo_btn).clicked() {
                    app.history.undo();
                    ui.close_menu();
                }
                let redo_btn = egui::Button::new("Redo (やり直す)").shortcut_text("Cmd+Shift+Z");
                if ui.add_enabled(app.history.can_redo(), redo_btn).clicked() {
                    app.history.redo();
                    ui.close_menu();
                }
            });
            ui.menu_button("Composition", |ui| {
                let btn = egui::Button::new("Composition Settings...").shortcut_text("Cmd+K");
                if ui.add(btn).clicked() {
                    app.show_comp_settings = true;
                    ui.close_menu();
                }
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
}
