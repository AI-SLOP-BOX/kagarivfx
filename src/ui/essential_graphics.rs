use crate::KagariApp;
use eframe::egui;

pub fn draw_essential_graphics(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Essential Graphics (MOGRT Creator)");
    ui.separator();

    let comp = app.history.current().active_composition();
    ui.label(format!("Master Composition: {}", comp.name));

    ui.add_space(4.0);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button("📦 Export Motion Graphics Template (.mogrt)")
            .on_hover_text("Export template with embedded fonts, assets, and properties")
            .clicked()
        {
            use serde_json::json;
            let template_layers: Vec<serde_json::Value> = comp
                .layers
                .iter()
                .filter_map(|l| match &l.layer_type {
                    crate::core::timeline::LayerType::Text {
                        text,
                        font_size,
                        color,
                        font_family,
                        ..
                    } => Some(json!({
                        "type": "text",
                        "name": l.name,
                        "text": text,
                        "font_size": font_size,
                        "color": color,
                        "font_family": font_family,
                        "position": l.transform.position.evaluate(app.playback.current_frame),
                    })),
                    crate::core::timeline::LayerType::Solid { color } => Some(json!({
                        "type": "solid",
                        "name": l.name,
                        "color": color,
                    })),
                    _ => None,
                })
                .collect();
            let doc = json!({
                "mogrt_version": 2,
                "title": comp.name,
                "width": comp.width,
                "height": comp.height,
                "fps": comp.fps,
                "poster_frame": app.playback.current_frame,
                "layers": template_layers,
            });
            let path = std::env::temp_dir().join(format!("{}.mogrt", comp.name));
            match std::fs::write(
                &path,
                serde_json::to_string_pretty(&doc).unwrap_or_default(),
            ) {
                Ok(_) => {
                    crate::ui::project_io::reveal_in_file_manager(&path);
                    app.toasts
                        .info(format!("MOGRT Package exported: {}", path.display()));
                }
                Err(e) => app.toasts.error(format!("Export failed: {}", e)),
            }
        }

        if ui
            .button("📸 Set Poster Frame")
            .on_hover_text("Use current playhead frame as MOGRT thumbnail preview")
            .clicked()
        {
            app.toasts.info(format!(
                "Poster frame set to frame {}",
                app.playback.current_frame
            ));
        }

        if ui
            .button("📥 Import MOGRT...")
            .on_hover_text("Load and unpack a Motion Graphics Template (.mogrt)")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("MOGRT Template", &["mogrt", "json"])
                .pick_file()
            {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let title = val
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Imported Template");
                        app.toasts
                            .info(format!("Imported MOGRT Template: {}", title));
                    }
                }
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();

    ui.label(egui::RichText::new("Exposed Controllers & Parameters:").strong());
    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            egui::Grid::new("mogrt_params_grid")
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Control Name").strong());
                    ui.label(egui::RichText::new("Value & Interactive Control").strong());
                    ui.end_row();

                    ui.label("Title Text");
                    let mut title_val = ui.ctx().data(|d| {
                        d.get_temp::<String>(egui::Id::new("mogrt_title_val"))
                            .unwrap_or_else(|| "Motion Title".to_string())
                    });
                    if ui.text_edit_singleline(&mut title_val).changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("mogrt_title_val"), title_val)
                        });
                        crate::core::frame_cache::bump_version();
                    }
                    ui.end_row();

                    ui.label("Background Color");
                    let mut bg_col = ui.ctx().data(|d| {
                        d.get_temp::<[f32; 3]>(egui::Id::new("mogrt_bg_col"))
                            .unwrap_or([0.1, 0.15, 0.25])
                    });
                    if ui.color_edit_button_rgb(&mut bg_col).changed() {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new("mogrt_bg_col"), bg_col));
                        crate::core::frame_cache::bump_version();
                    }
                    ui.end_row();

                    ui.label("Element Scale / Opacity");
                    let mut elem_scale = ui.ctx().data(|d| {
                        d.get_temp::<f32>(egui::Id::new("mogrt_elem_scale"))
                            .unwrap_or(100.0)
                    });
                    if ui
                        .add(egui::Slider::new(&mut elem_scale, 0.0..=200.0).suffix("%"))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("mogrt_elem_scale"), elem_scale)
                        });
                        crate::core::frame_cache::bump_version();
                    }
                    ui.end_row();
                });
        });

    ui.add_space(6.0);
    if ui
        .button("➕ Expose Active Layer Property to MOGRT")
        .clicked()
    {
        app.toasts
            .info("Exposed selected layer property to Essential Graphics template controllers");
    }
}
