#![allow(clippy::field_reassign_with_default)]

use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Composition, LayerType};
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

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
                    .color(colors::ACCENT_YELLOW),
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
                    // ── Full AE Range Selector & Animator Controls ──
                    ui.group(|ui| {
                        let has_anim = layer.text_animator.is_some();
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🎛 Text Animator").strong().color(colors::ACCENT_CYAN));
                            if !has_anim {
                                if ui.button("+ Add Animator").clicked() {
                                    layer.text_animator = Some(crate::core::text_animator::TextAnimatorSettings::default());
                                    project_changed = true;
                                }
                            } else if ui.small_button("🗑 Remove").clicked() {
                                layer.text_animator = None;
                                project_changed = true;
                            }
                        });

                        if let Some(ref mut anim) = layer.text_animator {
                            ui.separator();
                            if ui.checkbox(&mut anim.enabled, "Enable Animator").clicked() {
                                project_changed = true;
                            }

                            // Range Selector
                            ui.collapsing("🎯 Range Selector", |ui| {
                                let sel = &mut anim.selector;
                                ui.horizontal(|ui| {
                                    ui.label("Start / End:");
                                    if ui.add(egui::Slider::new(&mut sel.start, 0.0..=100.0).suffix("%")).changed() { project_changed = true; }
                                    if ui.add(egui::Slider::new(&mut sel.end, 0.0..=100.0).suffix("%")).changed() { project_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Offset:");
                                    if ui.add(egui::Slider::new(&mut sel.offset, -100.0..=100.0).suffix("%")).changed() { project_changed = true; }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Shape:");
                                    let mut shape_idx = match sel.shape {
                                        crate::core::text_animator::SelectorShape::Square => 0,
                                        crate::core::text_animator::SelectorShape::RampUp => 1,
                                        crate::core::text_animator::SelectorShape::RampDown => 2,
                                        crate::core::text_animator::SelectorShape::Triangle => 3,
                                        crate::core::text_animator::SelectorShape::Round => 4,
                                        crate::core::text_animator::SelectorShape::Smooth => 5,
                                        crate::core::text_animator::SelectorShape::Wobble => 6,
                                        crate::core::text_animator::SelectorShape::Random => 7,
                                        crate::core::text_animator::SelectorShape::Expression => 8,
                                    };
                                    egui::ComboBox::from_id_salt("range_shape")
                                        .selected_text(match sel.shape {
                                            crate::core::text_animator::SelectorShape::Square => "Square",
                                            crate::core::text_animator::SelectorShape::RampUp => "Ramp Up",
                                            crate::core::text_animator::SelectorShape::RampDown => "Ramp Down",
                                            crate::core::text_animator::SelectorShape::Triangle => "Triangle",
                                            crate::core::text_animator::SelectorShape::Round => "Round",
                                            crate::core::text_animator::SelectorShape::Smooth => "Smooth",
                                            crate::core::text_animator::SelectorShape::Wobble => "Wobble",
                                            crate::core::text_animator::SelectorShape::Random => "Random",
                                            crate::core::text_animator::SelectorShape::Expression => "Expression",
                                        })
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_value(&mut shape_idx, 0, "Square").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Square; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 1, "Ramp Up").clicked() { sel.shape = crate::core::text_animator::SelectorShape::RampUp; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 2, "Ramp Down").clicked() { sel.shape = crate::core::text_animator::SelectorShape::RampDown; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 3, "Triangle").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Triangle; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 4, "Round").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Round; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 5, "Smooth").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Smooth; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 6, "Wobble").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Wobble; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 7, "Random").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Random; project_changed = true; }
                                            if ui.selectable_value(&mut shape_idx, 8, "Expression").clicked() { sel.shape = crate::core::text_animator::SelectorShape::Expression; project_changed = true; }
                                        });
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Ease High / Low:");
                                    if ui.add(egui::DragValue::new(&mut sel.ease_high).prefix("Hi: ").suffix("%")).changed() { project_changed = true; }
                                    if ui.add(egui::DragValue::new(&mut sel.ease_low).prefix("Lo: ").suffix("%")).changed() { project_changed = true; }
                                });

                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut sel.random_order, "Randomize Order").clicked() { project_changed = true; }
                                });
                            });

                            // Target Property Offsets
                            ui.collapsing("📐 Transform Properties", |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Position Offset:");
                                    if ui.add(egui::DragValue::new(&mut anim.position_offset[0]).prefix("X: ")).changed() { project_changed = true; }
                                    if ui.add(egui::DragValue::new(&mut anim.position_offset[1]).prefix("Y: ")).changed() { project_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Scale Target:");
                                    if ui.add(egui::DragValue::new(&mut anim.scale[0]).speed(0.05).prefix("X: ")).changed() { project_changed = true; }
                                    if ui.add(egui::DragValue::new(&mut anim.scale[1]).speed(0.05).prefix("Y: ")).changed() { project_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Rotation Offset:");
                                    if ui.add(egui::DragValue::new(&mut anim.rotation).suffix("°")).changed() { project_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Opacity Target:");
                                    if ui.add(egui::Slider::new(&mut anim.opacity, 0.0..=1.0)).changed() { project_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Tracking Delta:");
                                    if ui.add(egui::DragValue::new(&mut anim.tracking).suffix(" px")).changed() { project_changed = true; }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Blur Amount:");
                                    if ui.add(egui::DragValue::new(&mut anim.blur_amount).speed(0.5).suffix(" px")).changed() { project_changed = true; }
                                });
                            });
                        }
                    });

                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("✨ One-Tap Motion Presets").strong().color(colors::ACCENT_CYAN));
                        ui.horizontal(|ui| {
                            if custom_widgets::ae_button(ui, "⌨ Typewriter").on_hover_text("Reveal characters one by one").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.opacity = 0.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Square;
                                anim.selector.start = 0.0;
                                anim.selector.end = 100.0;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                            if custom_widgets::ae_button(ui, "💥 Word Pop").on_hover_text("Bounce text up character by character").clicked() {
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
                            if custom_widgets::ae_button(ui, "🌊 Wave Tracking").on_hover_text("Expand text letter-spacing smoothly").clicked() {
                                let mut anim = crate::core::text_animator::TextAnimatorSettings::default();
                                anim.enabled = true;
                                anim.tracking = 25.0;
                                anim.selector.shape = crate::core::text_animator::SelectorShape::Triangle;
                                layer.text_animator = Some(anim);
                                project_changed = true;
                            }
                            if custom_widgets::ae_button(ui, "🌟 Neon Pulse").on_hover_text("Pulsating opacity and tracking wave").clicked() {
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
            if custom_widgets::ae_button(ui, preset).clicked() {
                if let Some(idx) = app.selected_layer_idx {
                    crate::ui::character_panel::apply_text_preset(app, preset, idx, current_frame, duration_frames);
                }
            }
        }
    });
}