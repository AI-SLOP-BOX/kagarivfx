use crate::core::timeline::EffectType;
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::ChromaKey {
            screen_color,
            screen_gain,
            clip_black,
            clip_white,
        } => {
            let c_before = screen_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Key Color", screen_color, |ui, val| {
                    ui.color_edit_button_rgb(val);
                })
            {
                *next_frame = Some(nf);
            }
            if c_before != *screen_color {
                *project_changed = true;
            }
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Screen Gain",
                screen_gain,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.5..=2.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Clip Black",
                clip_black,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Clip White",
                clip_white,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::LumaKeyRange {
            low_threshold,
            high_threshold,
            invert,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Low Threshold",
                low_threshold,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=255.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "High Threshold",
                high_threshold,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=255.0));
                },
            );
            if ui.checkbox(invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        EffectType::SimpleChoker { choke_amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Choke Amount",
                choke_amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
        }
        EffectType::MatteChokeSpread { radius, expand } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=64.0).suffix(" px"));
                },
            );
            if ui
                .checkbox(expand, "Expand (spread instead of choke)")
                .changed()
            {
                *project_changed = true;
            }
        }
        EffectType::AlphaFeather { radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=64.0).suffix(" px"));
                },
            );
        }
        EffectType::AlphaFromLuminance { invert } => {
            if ui.checkbox(invert, "Invert (dark = opaque)").changed() {
                *project_changed = true;
            }
        }
        EffectType::MatteChoker {
            choke_amount,
            gray_level,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Choke Amount",
                choke_amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=50.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gray Level",
                gray_level,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::SetMatte {
            source_layer_idx,
            source_channel,
            invert_matte,
            composite_mode,
        } => {
            ui.label("🎭 Set Matte");
            ui.horizontal(|ui| {
                ui.label("Source Layer Index:");
                let mut idx_i32 = *source_layer_idx as i32;
                if ui
                    .add(egui::DragValue::new(&mut idx_i32).range(0..=99))
                    .changed()
                {
                    *source_layer_idx = idx_i32.max(0) as usize;
                    *project_changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Use For Matte:");
                egui::ComboBox::from_id_salt("set_matte_channel")
                    .selected_text(format!("{:?}", source_channel))
                    .show_ui(ui, |ui| {
                        for ch in [
                            crate::core::set_matte::MatteSourceChannel::Alpha,
                            crate::core::set_matte::MatteSourceChannel::Luminance,
                            crate::core::set_matte::MatteSourceChannel::Red,
                            crate::core::set_matte::MatteSourceChannel::Green,
                            crate::core::set_matte::MatteSourceChannel::Blue,
                            crate::core::set_matte::MatteSourceChannel::Lightness,
                        ] {
                            if ui
                                .selectable_value(source_channel, ch, format!("{:?}", ch))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                    });
            });
            if ui.checkbox(invert_matte, "Invert Matte").changed() {
                *project_changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("Composite Mode:");
                egui::ComboBox::from_id_salt("set_matte_mode")
                    .selected_text(format!("{:?}", composite_mode))
                    .show_ui(ui, |ui| {
                        for m in [
                            crate::core::set_matte::MatteCompositeMode::Replace,
                            crate::core::set_matte::MatteCompositeMode::Intersect,
                            crate::core::set_matte::MatteCompositeMode::Add,
                            crate::core::set_matte::MatteCompositeMode::Subtract,
                        ] {
                            if ui
                                .selectable_value(composite_mode, m, format!("{:?}", m))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                    });
            });
        }
        EffectType::LinearColorKey {
            key_color,
            match_mode,
            tolerance,
            softness,
        } => {
            ui.label("🗝️ Linear Color Key");
            let kc_before = key_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Key Color", key_color, |ui, val| {
                    let mut col = egui::Color32::from_rgb(
                        (val[0] * 255.0) as u8,
                        (val[1] * 255.0) as u8,
                        (val[2] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        val[0] = col.r() as f32 / 255.0;
                        val[1] = col.g() as f32 / 255.0;
                        val[2] = col.b() as f32 / 255.0;
                    }
                })
            {
                *next_frame = Some(nf);
            }
            if kc_before != *key_color {
                *project_changed = true;
            }

            ui.horizontal(|ui| {
                ui.label("Match Colors:");
                egui::ComboBox::from_id_salt("key_match_mode")
                    .selected_text(format!("{:?}", match_mode))
                    .show_ui(ui, |ui| {
                        for m in [
                            crate::core::linear_color_key::ColorMatchMode::UsingRGB,
                            crate::core::linear_color_key::ColorMatchMode::UsingHue,
                            crate::core::linear_color_key::ColorMatchMode::UsingChroma,
                        ] {
                            if ui
                                .selectable_value(match_mode, m, format!("{:?}", m))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                    });
            });
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tolerance",
                tolerance,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Softness",
                softness,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
        }
        _ => {}
    }
}
