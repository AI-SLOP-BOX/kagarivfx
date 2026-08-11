use eframe::egui;

pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    app.history = crate::core::history::ProjectHistory::new(
                        crate::core::timeline::Project::default(),
                    );
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save Project (.aevfx.json)").clicked() {
                    let project = app.history.current();
                    match serde_json::to_string_pretty(project) {
                        Ok(json) => {
                            match std::fs::write(&app.project_path, &json) {
                                Ok(_) => log::info!("Native project saved to {}", app.project_path),
                                Err(e) => log::error!("Failed to save project: {}", e),
                            }
                        }
                        Err(e) => log::error!("Failed to serialize project: {}", e),
                    }
                    ui.close_menu();
                }
                if ui.button("Load Project (.aevfx.json)").clicked() {
                    match std::fs::read_to_string(&app.project_path) {
                        Ok(json) => {
                            match serde_json::from_str::<crate::core::timeline::Project>(&json) {
                                Ok(project) => {
                                    app.history = crate::core::history::ProjectHistory::new(project);
                                    app.selected_layer_idx = None;
                                    log::info!("Native project loaded from {}", app.project_path);
                                }
                                Err(e) => log::error!("Failed to parse project: {}", e),
                            }
                        }
                        Err(e) => log::error!("Failed to read project file: {}", e),
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
                let undo_btn = egui::Button::new("Undo (元に戻す)").shortcut_text("Ctrl+Z");
                if ui.add_enabled(app.history.can_undo(), undo_btn).clicked() {
                    app.history.undo();
                    ui.close_menu();
                }
                let redo_btn = egui::Button::new("Redo (やり直す)").shortcut_text("Ctrl+Y");
                if ui.add_enabled(app.history.can_redo(), redo_btn).clicked() {
                    app.history.redo();
                    ui.close_menu();
                }
            });
            ui.menu_button("Composition", |ui| {
                if ui.button("New Composition...").clicked() {
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut app.show_grid, "Show Grid");
                ui.checkbox(&mut app.show_guides, "Show Safe Zones");
                ui.checkbox(&mut app.show_handles, "Show Handles");
            });
        });
    });
}
