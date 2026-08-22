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
                            egui::ComboBox::from_id_salt("ae_text_font_family")
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
                            ui.add(egui::DragValue::new(font_size).range(6..=300).suffix(" px"));
                            if fs_before != *font_size { project_changed = true; }
                        });

                        // Tracking
                        ui.horizontal(|ui| {
                            ui.label("Tracking:");
                            if ui.add(egui::DragValue::new(tracking)
                                .speed(1.0).range(-100.0..=500.0).suffix(" VA"))
                                .changed() { project_changed = true; }
                        });

                        // Leading
                        ui.horizontal(|ui| {
                            ui.label("Leading:");
                            if ui.add(egui::DragValue::new(leading)
                                .speed(0.05).range(0.5..=3.0).suffix(" em"))
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
                                .speed(0.5).range(0.0..=50.0).suffix(" px"))
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

/// Applies a text animation preset to the selected layer's position/opacity.
pub fn apply_text_preset(
    app: &mut AfterEffectsApp,
    preset: &str,
    layer_idx: usize,
    current_frame: u32,
    duration_frames: u32,
) {
    use crate::core::keyframe::{Keyframe, InterpolationType};
    use crate::core::property::Animatable;

    let dur = if duration_frames > 0 { duration_frames } else { 30 }; // default: 1s at 30fps
    let ease = InterpolationType::Bezier {
        outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.33, speed: 0.0 },
        incoming: crate::core::keyframe::BezierControlPoint { influence: 0.33, speed: 0.0 },
        custom_bezier: Some([0.25, 0.1, 0.25, 1.0]), // Easy Ease
    };

    app.modify_project(|p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(layer_idx) else { return };

        let base_pos = layer.transform.position.evaluate(current_frame);
        let end_frame = current_frame + dur;

        match preset {
            "Fade In" => {
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Slide In Left" => {
                layer.transform.position = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, [base_pos[0] - 200.0, base_pos[1]], ease),
                    Keyframe::new(end_frame, base_pos, ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Slide In Right" => {
                layer.transform.position = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, [base_pos[0] + 200.0, base_pos[1]], ease),
                    Keyframe::new(end_frame, base_pos, ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Slide In Up" => {
                layer.transform.position = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, [base_pos[0], base_pos[1] + 150.0], ease),
                    Keyframe::new(end_frame, base_pos, ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            "Scale Up" => {
                layer.transform.scale = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, [0.0, 0.0], ease),
                    Keyframe::new(end_frame, [100.0, 100.0], ease),
                ]);
                layer.transform.opacity = Animatable::new_animated(vec![
                    Keyframe::new(current_frame, 0.0, ease),
                    Keyframe::new(end_frame, 100.0, ease),
                ]);
            }
            _ => {}
        }
    });
}

/// Text animation presets section — adds to the bottom of the panel.
pub fn draw_animation_presets(
    app: &mut AfterEffectsApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    duration_frames: u32,
) {
    ui.separator();
    ui.label("Text Animation Presets:");
    let presets = ["Fade In", "Slide In Left", "Slide In Right", "Slide In Up", "Scale Up"];
    ui.horizontal_wrapped(|ui| {
        for preset in presets {
            if ui.small_button(preset).clicked() {
                if let Some(idx) = app.selected_layer_idx {
                    crate::ui::character_panel::apply_text_preset(app, preset, idx, current_frame, duration_frames);
                }
            }
        }
    });
}