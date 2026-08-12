use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Composition, LayerType};

pub fn draw_character_panel(
    app: &mut AfterEffectsApp,
    ui: &mut egui::Ui,
    comp: &mut Composition,
    _current_frame: u32,
) -> bool {
    let mut project_changed = false;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Character & Paragraph")
                    .strong()
                    .color(egui::Color32::from_rgb(255, 200, 100)),
            );
            ui.weak("— AE Text Formatting");
        });
        ui.separator();

        let sel_idx = app.selected_layer_idx;
        if let Some(idx) = sel_idx {
            if idx < comp.layers.len() {
                let layer = &mut comp.layers[idx];
                if let LayerType::Text { text, font_size, color } = &mut layer.layer_type {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Character Properties").strong());
                        ui.add_space(4.0);

                        // Text content edit
                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            let txt_before = text.clone();
                            ui.text_edit_singleline(text);
                            if txt_before != *text {
                                project_changed = true;
                            }
                        });

                        // Font Family Selector
                        let font_family_id = ui.make_persistent_id(format!("ae_text_font_family_{}", layer.id));
                        let mut font_family = ui.ctx().data_mut(|d| d.get_temp::<String>(font_family_id).unwrap_or_else(|| "Inter".to_string()));
                        ui.horizontal(|ui| {
                            ui.label("Font Family:");
                            egui::ComboBox::from_id_source(font_family_id)
                                .selected_text(&font_family)
                                .show_ui(ui, |ui| {
                                    for font_name in ["Inter", "Roboto", "Helvetica", "Arial", "Courier New", "Impact", "Times New Roman"] {
                                        if ui.selectable_label(font_family == font_name, font_name).clicked() {
                                            font_family = font_name.to_string();
                                            ui.ctx().data_mut(|d| d.insert_temp(font_family_id, font_family.clone()));
                                            project_changed = true;
                                        }
                                    }
                                });
                        });

                        // Font Size (px)
                        ui.horizontal(|ui| {
                            ui.label("Font Size:");
                            let fs_before = *font_size;
                            ui.add(egui::DragValue::new(font_size).clamp_range(6..=300).suffix(" px"));
                            if fs_before != *font_size {
                                project_changed = true;
                            }
                        });

                        // Tracking (Letter Spacing)
                        let tracking_id = ui.make_persistent_id(format!("ae_text_tracking_{}", layer.id));
                        let mut tracking = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(tracking_id, || 0.0f32));
                        ui.horizontal(|ui| {
                            ui.label("Tracking:");
                            if ui.add(egui::DragValue::new(&mut tracking).speed(1.0).clamp_range(-100.0..=500.0).suffix(" VA")).changed() {
                                ui.ctx().data_mut(|d| d.insert_temp(tracking_id, tracking));
                                project_changed = true;
                            }
                        });

                        // Leading (Line Height)
                        let leading_id = ui.make_persistent_id(format!("ae_text_leading_{}", layer.id));
                        let mut leading = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(leading_id, || 1.2f32));
                        ui.horizontal(|ui| {
                            ui.label("Leading:");
                            if ui.add(egui::DragValue::new(&mut leading).speed(0.05).clamp_range(0.5..=3.0).suffix(" em")).changed() {
                                ui.ctx().data_mut(|d| d.insert_temp(leading_id, leading));
                                project_changed = true;
                            }
                        });

                        ui.separator();
                        ui.label(egui::RichText::new("Color & Stroke").strong());

                        // Fill Color Picker
                        ui.horizontal(|ui| {
                            ui.label("Fill Color:");
                            let col_before = *color;
                            ui.color_edit_button_rgba_unmultiplied(color);
                            if col_before != *color {
                                project_changed = true;
                            }
                        });

                        // Stroke Color & Width
                        let stroke_color_id = ui.make_persistent_id(format!("ae_text_stroke_col_{}", layer.id));
                        let mut stroke_color = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(stroke_color_id, || [0.0f32, 0.0, 0.0, 1.0]));
                        let stroke_width_id = ui.make_persistent_id(format!("ae_text_stroke_w_{}", layer.id));
                        let mut stroke_width = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(stroke_width_id, || 0.0f32));

                        ui.horizontal(|ui| {
                            ui.label("Stroke Width:");
                            if ui.add(egui::DragValue::new(&mut stroke_width).speed(0.5).clamp_range(0.0..=50.0).suffix(" px")).changed() {
                                ui.ctx().data_mut(|d| d.insert_temp(stroke_width_id, stroke_width));
                                project_changed = true;
                            }
                            if stroke_width > 0.0 {
                                if ui.color_edit_button_rgba_unmultiplied(&mut stroke_color).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(stroke_color_id, stroke_color));
                                    project_changed = true;
                                }
                            }
                        });
                    });

                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Paragraph Alignment").strong());
                        let align_id = ui.make_persistent_id(format!("ae_text_align_{}", layer.id));
                        let mut align_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(align_id, || 0usize));

                        ui.horizontal(|ui| {
                            if ui.selectable_label(align_idx == 0, "[Left]").clicked() {
                                align_idx = 0;
                                ui.ctx().data_mut(|d| d.insert_temp(align_id, 0usize));
                                project_changed = true;
                            }
                            if ui.selectable_label(align_idx == 1, "[Center]").clicked() {
                                align_idx = 1;
                                ui.ctx().data_mut(|d| d.insert_temp(align_id, 1usize));
                                project_changed = true;
                            }
                            if ui.selectable_label(align_idx == 2, "[Right]").clicked() {
                                align_idx = 2;
                                ui.ctx().data_mut(|d| d.insert_temp(align_id, 2usize));
                                project_changed = true;
                            }
                        });
                    });
                } else {
                    ui.weak("Select a Text Layer to edit character formatting.");
                }
            } else {
                ui.weak("Select a Text Layer to edit character formatting.");
            }
        } else {
            ui.weak("Select a Text Layer to edit character formatting.");
        }
    });

    project_changed
}
