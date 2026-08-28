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

                        // Kerning (Metrics / Optical)
                        ui.horizontal(|ui| {
                            ui.label("Kerning:");
                            let mut kern_mode = ui.ctx().data(|d| d.get_temp::<i32>(egui::Id::new("ae_kerning_mode")).unwrap_or(0));
                            egui::ComboBox::from_id_salt("ae_kerning_mode_combo")
                                .selected_text(match kern_mode {
                                    0 => "Metrics (Auto)",
                                    1 => "Optical",
                                    _ => "Manual",
                                })
                                .show_ui(ui, |ui| {
                                    if ui.selectable_value(&mut kern_mode, 0, "Metrics (Font Built-in)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_kerning_mode"), 0)); project_changed = true; }
                                    if ui.selectable_value(&mut kern_mode, 1, "Optical (Visual Spacing)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_kerning_mode"), 1)); project_changed = true; }
                                });
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

                        // Variable Font Axes
                        ui.separator();
                        ui.collapsing("🧬 Variable Font Axes", |ui| {
                            let mut vf_weight = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("vf_weight")).unwrap_or(400.0));
                            let mut vf_width = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("vf_width")).unwrap_or(100.0));
                            let mut vf_slant = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("vf_slant")).unwrap_or(0.0));

                            ui.horizontal(|ui| {
                                ui.label("Weight (wght):");
                                if ui.add(egui::Slider::new(&mut vf_weight, 100.0..=900.0).step_by(10.0)).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("vf_weight"), vf_weight));
                                    project_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Width (wdth):");
                                if ui.add(egui::Slider::new(&mut vf_width, 50.0..=150.0).suffix(" %")).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("vf_width"), vf_width));
                                    project_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Slant (slnt):");
                                if ui.add(egui::Slider::new(&mut vf_slant, -15.0..=15.0).suffix("°")).changed() {
                                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("vf_slant"), vf_slant));
                                    project_changed = true;
                                }
                            });
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
                    // ── AE Multi-Animator Stack (Layered Animators) ──
                    ui.group(|ui| {
                        let has_stack = layer.text_animator_stack.is_some();
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📚 Animators Stack").strong().color(colors::ACCENT_CYAN));
                            if ui.button("+ Add Animator").on_hover_text("Add another layered text animator").clicked() {
                                if layer.text_animator_stack.is_none() {
                                    layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default());
                                }
                                if let Some(ref mut stack) = layer.text_animator_stack {
                                    stack.animators.push(crate::core::text_animator_advanced::TextAnimatorAdvanced::default());
                                }
                                project_changed = true;
                            }
                            if has_stack && ui.small_button("Clear Stack").clicked() {
                                layer.text_animator_stack = None;
                                project_changed = true;
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new("✨ Presets:").small().color(colors::TEXT_SECONDARY));
                            if ui.small_button("⌨ Typewriter").clicked() {
                                let mut anim = crate::core::text_animator_advanced::TextAnimatorAdvanced::default();
                                anim.opacity = 0.0;
                                anim.unit = crate::core::text_animator_advanced::SelectorUnit::Characters;
                                if layer.text_animator_stack.is_none() { layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default()); }
                                if let Some(ref mut st) = layer.text_animator_stack { st.animators.push(anim); }
                                project_changed = true;
                            }
                            if ui.small_button("🌊 Char Wave").clicked() {
                                let mut anim = crate::core::text_animator_advanced::TextAnimatorAdvanced::default();
                                anim.position = [0.0, -30.0];
                                anim.unit = crate::core::text_animator_advanced::SelectorUnit::Characters;
                                if layer.text_animator_stack.is_none() { layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default()); }
                                if let Some(ref mut st) = layer.text_animator_stack { st.animators.push(anim); }
                                project_changed = true;
                            }
                            if ui.small_button("🐇 Word Hop").clicked() {
                                let mut anim = crate::core::text_animator_advanced::TextAnimatorAdvanced::default();
                                anim.position = [0.0, -50.0];
                                anim.unit = crate::core::text_animator_advanced::SelectorUnit::Words;
                                if layer.text_animator_stack.is_none() { layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default()); }
                                if let Some(ref mut st) = layer.text_animator_stack { st.animators.push(anim); }
                                project_changed = true;
                            }
                        });

                        if let Some(ref mut stack) = layer.text_animator_stack {
                            let mut to_remove = None;
                            for (ai, anim) in stack.animators.iter_mut().enumerate() {
                                ui.push_id(ai, |ui| {
                                    ui.collapsing(format!("🎛 Animator {}", ai + 1), |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.checkbox(&mut anim.enabled, "Enabled").clicked() { project_changed = true; }
                                            ui.label("Based On:");
                                            let mut unit_idx = match anim.unit {
                                                crate::core::text_animator_advanced::SelectorUnit::Characters => 0,
                                                crate::core::text_animator_advanced::SelectorUnit::Words => 1,
                                                crate::core::text_animator_advanced::SelectorUnit::Lines => 2,
                                            };
                                            egui::ComboBox::from_id_salt("anim_unit")
                                                .selected_text(match anim.unit {
                                                    crate::core::text_animator_advanced::SelectorUnit::Characters => "Characters",
                                                    crate::core::text_animator_advanced::SelectorUnit::Words => "Words",
                                                    crate::core::text_animator_advanced::SelectorUnit::Lines => "Lines",
                                                })
                                                .show_ui(ui, |ui| {
                                                    if ui.selectable_value(&mut unit_idx, 0, "Characters").clicked() { anim.unit = crate::core::text_animator_advanced::SelectorUnit::Characters; project_changed = true; }
                                                    if ui.selectable_value(&mut unit_idx, 1, "Words").clicked() { anim.unit = crate::core::text_animator_advanced::SelectorUnit::Words; project_changed = true; }
                                                    if ui.selectable_value(&mut unit_idx, 2, "Lines").clicked() { anim.unit = crate::core::text_animator_advanced::SelectorUnit::Lines; project_changed = true; }
                                                });
                                            if ui.small_button("🗑").clicked() { to_remove = Some(ai); }
                                        });

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
                                        });

                                        // Property targets
                                        ui.collapsing("📐 Transform & Advanced Properties", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Position:");
                                                if ui.add(egui::DragValue::new(&mut anim.position[0]).prefix("X: ")).changed() { project_changed = true; }
                                                if ui.add(egui::DragValue::new(&mut anim.position[1]).prefix("Y: ")).changed() { project_changed = true; }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Rotation / Opacity:");
                                                if ui.add(egui::DragValue::new(&mut anim.rotation).suffix("°")).changed() { project_changed = true; }
                                                if ui.add(egui::Slider::new(&mut anim.opacity, 0.0..=1.0)).changed() { project_changed = true; }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Tracking:");
                                                if ui.add(egui::DragValue::new(&mut anim.tracking).suffix(" px")).changed() { project_changed = true; }
                                                ui.label("Skew:");
                                                if ui.add(egui::DragValue::new(&mut anim.advanced.skew).suffix("°")).changed() { project_changed = true; }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Char Offset (Glyph Shift):");
                                                if ui.add(egui::DragValue::new(&mut anim.advanced.character_offset)).changed() { project_changed = true; }
                                            });
                                        });
                                    });
                                });
                            }
                            if let Some(r) = to_remove {
                                stack.animators.remove(r);
                                project_changed = true;
                            }
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