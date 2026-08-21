#![allow(clippy::field_reassign_with_default)]

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
                if let LayerType::Text {
                    text,
                    font_size,
                    color,
                    font_family,
                    tracking,
                    leading,
                    align,
                    stroke_color,
                    stroke_width,
                    text_on_path: _,
                } = &mut layer.layer_type
                {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Character Properties").strong());
                        ui.add_space(4.0);

                        // Text content
                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            let txt_before = text.clone();
                            ui.text_edit_singleline(text);
                            if txt_before != *text { project_changed = true; }
                        });

                        // Font Family
                        ui.horizontal(|ui| {
                            ui.label("Font Family:");
                            egui::ComboBox::from_id_source("ae_text_font_family")
                                .selected_text(font_family.as_str())
                                .show_ui(ui, |ui| {
                                    for name in ["Inter", "Roboto", "Helvetica", "Arial",
                                                 "Courier New", "Impact", "Times New Roman"] {
                                        if ui.selectable_label(*font_family == name, name).clicked() {
                                            *font_family = name.to_string();
                                            project_changed = true;
                                        }
                                    }
                                });
                        });

                        // Font Size
                        ui.horizontal(|ui| {
                            ui.label("Font Size:");
                            let fs_before = *font_size;
                            ui.add(egui::DragValue::new(font_size).clamp_range(6..=300).suffix(" px"));
                            if fs_before != *font_size { project_changed = true; }
                        });

                        // Tracking
                        ui.horizontal(|ui| {
                            ui.label("Tracking:");
                            if ui.add(egui::DragValue::new(tracking)
                                .speed(1.0).clamp_range(-100.0..=500.0).suffix(" VA"))
                                .changed() { project_changed = true; }
                        });

                        // Leading
                        ui.horizontal(|ui| {
                            ui.label("Leading:");
                            if ui.add(egui::DragValue::new(leading)
                                .speed(0.05).clamp_range(0.5..=3.0).suffix(" em"))
                                .changed() { project_changed = true; }
                        });

                        ui.separator();
                        ui.label(egui::RichText::new("Color & Stroke").strong());

                        // Fill Color
                        ui.horizontal(|ui| {
                            ui.label("Fill Color:");
                            let col_before = *color;
                            ui.color_edit_button_rgba_unmultiplied(color);
                            if col_before != *color { project_changed = true; }
                        });

                        // Stroke Width & Color
                        ui.horizontal(|ui| {
                            ui.label("Stroke Width:");
                            if ui.add(egui::DragValue::new(stroke_width)
                                .speed(0.5).clamp_range(0.0..=50.0).suffix(" px"))
                                .changed() { project_changed = true; }
                            if *stroke_width > 0.0
                                && ui.color_edit_button_rgba_unmultiplied(stroke_color).changed() {
                                    project_changed = true;
                                }
                        });
                    });

                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Paragraph Alignment").strong());
                        ui.horizontal(|ui| {
                            if ui.selectable_label(*align == 0, "[Left]").clicked()   { *align = 0; project_changed = true; }
                            if ui.selectable_label(*align == 1, "[Center]").clicked() { *align = 1; project_changed = true; }
                            if ui.selectable_label(*align == 2, "[Right]").clicked()  { *align = 2; project_changed = true; }
                        });
                    });
                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("✨ One-Tap Motion Presets").strong().color(egui::Color32::from_rgb(0, 200, 255)));
                        ui.horizontal(|ui| {
                            if ui.button("⌨ Typewriter").on_hover_text("Reveal characters one by one").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.opacity = 0.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Square;
                                anim.selector.start = 0.0;
                                anim.selector.end = 100.0;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                            if ui.button("💥 Word Pop").on_hover_text("Bounce text up character by character").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.position_offset = [0.0, 50.0];
                                anim.opacity = 0.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::RampUp;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("🌊 Wave Tracking").on_hover_text("Expand text letter-spacing smoothly").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.tracking = 25.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Triangle;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                            if ui.button("🌟 Neon Pulse").on_hover_text("Pulsating opacity and tracking wave").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.opacity = 0.2;
                                anim.tracking = 15.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Smooth;
                                layer.text_animator = Some(anim);
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
