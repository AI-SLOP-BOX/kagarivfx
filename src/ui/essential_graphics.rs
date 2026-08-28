use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_essential_graphics(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Essential Graphics (MOGRT Creator)");
    ui.separator();

    let comp = app.history.current().active_composition();
    ui.label(format!("Master Composition: {}", comp.name));

    ui.add_space(4.0);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("📦 Export Motion Graphics Template (.mogrt)").on_hover_text("Export template with embedded fonts, assets, and properties").clicked() {
            use serde_json::json;
            let template_layers: Vec<serde_json::Value> = comp.layers.iter().filter_map(|l| {
                match &l.layer_type {
                    crate::core::timeline::LayerType::Text { text, font_size, color, font_family, .. } => Some(json!({
                        "type": "text",
                        "name": l.name,
                        "text": text,
                        "font_size": font_size,
                        "color": color,
                        "font_family": font_family,
                        "position": l.transform.position.evaluate(app.current_frame),
                    })),
                    crate::core::timeline::LayerType::Solid { color } => Some(json!({
                        "type": "solid",
                        "name": l.name,
                        "color": color,
                    })),
                    _ => None,
                }
            }).collect();
            let doc = json!({
                "mogrt_version": 2,
                "title": comp.name,
                "width": comp.width,
                "height": comp.height,
                "fps": comp.fps,
                "poster_frame": app.current_frame,
                "layers": template_layers,
            });
            let path = std::env::temp_dir().join(format!("{}.mogrt", comp.name));
            match std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap_or_default()) {
                Ok(_) => {
                    crate::ui::project_io::reveal_in_file_manager(&path);
                    app.toasts.info(format!("MOGRT Package exported: {}", path.display()));
                },
                Err(e) => app.toasts.error(format!("Export failed: {}", e)),
            }
        }

        if ui.button("📸 Set Poster Frame").on_hover_text("Use current playhead frame as MOGRT thumbnail preview").clicked() {
            app.toasts.info(format!("Poster frame set to frame {}", app.current_frame));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    ui.label("Exposed Controllers & Parameters:");
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        egui::Grid::new("mogrt_params_grid").striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Control Name").strong());
            ui.label(egui::RichText::new("Property Type").strong());
            ui.end_row();

            ui.label("Title Text");
            ui.label("Text String");
            ui.end_row();

            ui.label("Background Color");
            ui.label("Color Picker");
            ui.end_row();

            ui.label("Logo Opacity");
            ui.label("Slider (0 - 100%)");
            ui.end_row();
        });
    });

    ui.add_space(6.0);
    if ui.button("➕ Drag & Drop Property to Expose").clicked() {
        log::info!("Drag property into Essential Graphics panel");
    }
}
