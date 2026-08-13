use eframe::egui;
use crate::core::timeline::{EffectType, ColorConversionMode};
use crate::ui::inspector::draw_property_ui;

pub fn draw_effect_type_ui(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::GaussianBlur { blur_radius } => {
            let val_before = blur_radius.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Blur Radius", blur_radius, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if val_before != *blur_radius {
                *project_changed = true;
            }
        }
        EffectType::ColorTint { color, intensity } => {
            let color_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Tint Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) {
                *next_frame = Some(nf);
            }
            if color_before != *color {
                *project_changed = true;
            }

            let intensity_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if intensity_before != *intensity {
                *project_changed = true;
            }
        }
        EffectType::DropShadow { color, opacity, direction, distance, softness } => {
            let color_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Shadow Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) {
                *next_frame = Some(nf);
            }
            if color_before != *color {
                *project_changed = true;
            }

            let opacity_before = opacity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Opacity", opacity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if opacity_before != *opacity {
                *project_changed = true;
            }

            let direction_before = direction.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Direction", direction, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
            }) {
                *next_frame = Some(nf);
            }
            if direction_before != *direction {
                *project_changed = true;
            }

            let distance_before = distance.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Distance", distance, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix(" px"));
            }) {
                *next_frame = Some(nf);
            }
            if distance_before != *distance {
                *project_changed = true;
            }

            let softness_before = softness.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Softness", softness, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if softness_before != *softness {
                *project_changed = true;
            }
        }
        EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
            let shift_r_before = shift_r.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Red Shift", shift_r, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
            }) { *next_frame = Some(nf); }
            if shift_r_before != *shift_r { *project_changed = true; }

            let shift_b_before = shift_b.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Blue Shift", shift_b, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
            }) { *next_frame = Some(nf); }
            if shift_b_before != *shift_b { *project_changed = true; }

            let ef_before = edge_falloff.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Edge Falloff", edge_falloff, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ef_before != *edge_falloff { *project_changed = true; }
        }
        EffectType::Vignette { intensity, roundness, feather, color } => {
            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }

            let r_before = roundness.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Roundness", roundness, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if r_before != *roundness { *project_changed = true; }

            let f_before = feather.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Feather", feather, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if f_before != *feather { *project_changed = true; }

            let c_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) { *next_frame = Some(nf); }
            if c_before != *color { *project_changed = true; }
        }
        EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
            let ib_before = input_black.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Input Black", input_black, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ib_before != *input_black { *project_changed = true; }

            let iw_before = input_white.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Input White", input_white, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if iw_before != *input_white { *project_changed = true; }

            let g_before = gamma.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Gamma", gamma, |ui, val| {
                ui.add(egui::Slider::new(val, 0.1..=10.0));
            }) { *next_frame = Some(nf); }
            if g_before != *gamma { *project_changed = true; }

            let ob_before = output_black.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Output Black", output_black, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ob_before != *output_black { *project_changed = true; }

            let ow_before = output_white.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Output White", output_white, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ow_before != *output_white { *project_changed = true; }
        }
        EffectType::HueSaturation { hue_shift, saturation, lightness } => {
            let h_before = hue_shift.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Hue Shift", hue_shift, |ui, val| {
                ui.add(egui::Slider::new(val, -180.0..=180.0).suffix("°"));
            }) { *next_frame = Some(nf); }
            if h_before != *hue_shift { *project_changed = true; }

            let s_before = saturation.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Saturation", saturation, |ui, val| {
                ui.add(egui::Slider::new(val, -100.0..=100.0));
            }) { *next_frame = Some(nf); }
            if s_before != *saturation { *project_changed = true; }

            let l_before = lightness.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Lightness", lightness, |ui, val| {
                ui.add(egui::Slider::new(val, -100.0..=100.0));
            }) { *next_frame = Some(nf); }
            if l_before != *lightness { *project_changed = true; }
        }
        EffectType::Glow { threshold, radius, intensity, color } => {
            let t_before = threshold.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Threshold", threshold, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if t_before != *threshold { *project_changed = true; }

            let r_before = radius.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Radius", radius, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=200.0).suffix(" px"));
            }) { *next_frame = Some(nf); }
            if r_before != *radius { *project_changed = true; }

            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }

            let c_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Glow Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) { *next_frame = Some(nf); }
            if c_before != *color { *project_changed = true; }
        }
        EffectType::MotionBlur { shutter_angle, samples } => {
            let sa_before = shutter_angle.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Shutter Angle", shutter_angle, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
            }) { *next_frame = Some(nf); }
            if sa_before != *shutter_angle { *project_changed = true; }

            ui.horizontal(|ui| {
                ui.label("Samples:");
                let before_s = *samples;
                ui.add(egui::DragValue::new(samples).clamp_range(2..=16));
                if before_s != *samples { *project_changed = true; }
            });
        }
        EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
            let tl_before = top_left.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Top Left Corner", top_left, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if tl_before != *top_left { *project_changed = true; }

            let tr_before = top_right.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Top Right Corner", top_right, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if tr_before != *top_right { *project_changed = true; }

            let bl_before = bottom_left.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Bottom Left Corner", bottom_left, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if bl_before != *bottom_left { *project_changed = true; }

            let br_before = bottom_right.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Bottom Right Corner", bottom_right, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if br_before != *bottom_right { *project_changed = true; }
        }
        EffectType::ColorGradeLUT { lut_path, intensity } => {
            ui.horizontal(|ui| {
                ui.label("LUT Path:");
                let path_before = lut_path.clone();
                ui.text_edit_singleline(lut_path);
                if path_before != *lut_path { *project_changed = true; }
            });

            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }
        }
        EffectType::ColorSpaceConvert { mode } => {
            let mode_before = *mode;
            egui::ComboBox::from_id_source(format!("convert_combo_{:?}", ui.next_auto_id()))
                .selected_text(format!("{:?}", mode))
                .show_ui(ui, |ui| {
                    for m in [
                        ColorConversionMode::LogCToLinear,
                        ColorConversionMode::LinearToLogC,
                        ColorConversionMode::SLog3ToLinear,
                        ColorConversionMode::LinearToSLog3,
                    ] {
                        ui.selectable_value(mode, m, format!("{:?}", m));
                    }
                });
            if mode_before != *mode { *project_changed = true; }
        }
        EffectType::FilmGrain { intensity, grain_size, color_film } => {
            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Grain Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }

            ui.horizontal(|ui| {
                ui.label("Grain Size:");
                let size_before = *grain_size;
                ui.add(egui::Slider::new(grain_size, 1.0..=5.0));
                if size_before != *grain_size { *project_changed = true; }
            });

            ui.horizontal(|ui| {
                let c_before = *color_film;
                ui.checkbox(color_film, "Color Film Grain");
                if c_before != *color_film { *project_changed = true; }
            });
        }
    }
}
