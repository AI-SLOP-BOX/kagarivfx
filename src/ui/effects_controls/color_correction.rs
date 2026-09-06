use crate::core::timeline::{ColorConversionMode, EffectType};
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
        EffectType::GaussianBlur { blur_radius } => {
            let val_before = blur_radius.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Blur Radius", blur_radius, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0));
                })
            {
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
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if intensity_before != *intensity {
                *project_changed = true;
            }
        }
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => {
            let ib_before = input_black.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Input Black", input_black, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                })
            {
                *next_frame = Some(nf);
            }
            if ib_before != *input_black {
                *project_changed = true;
            }

            let iw_before = input_white.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Input White", input_white, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                })
            {
                *next_frame = Some(nf);
            }
            if iw_before != *input_white {
                *project_changed = true;
            }

            let g_before = gamma.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Gamma", gamma, |ui, val| {
                ui.add(egui::Slider::new(val, 0.1..=10.0));
            }) {
                *next_frame = Some(nf);
            }
            if g_before != *gamma {
                *project_changed = true;
            }

            let ob_before = output_black.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Output Black",
                output_black,
                |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                },
            ) {
                *next_frame = Some(nf);
            }
            if ob_before != *output_black {
                *project_changed = true;
            }

            let ow_before = output_white.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Output White",
                output_white,
                |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                },
            ) {
                *next_frame = Some(nf);
            }
            if ow_before != *output_white {
                *project_changed = true;
            }
        }
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => {
            let h_before = hue_shift.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Hue Shift", hue_shift, |ui, val| {
                    ui.add(egui::Slider::new(val, -180.0..=180.0).suffix("°"));
                })
            {
                *next_frame = Some(nf);
            }
            if h_before != *hue_shift {
                *project_changed = true;
            }

            let s_before = saturation.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Saturation", saturation, |ui, val| {
                    ui.add(egui::Slider::new(val, -100.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if s_before != *saturation {
                *project_changed = true;
            }

            let l_before = lightness.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Lightness", lightness, |ui, val| {
                    ui.add(egui::Slider::new(val, -100.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if l_before != *lightness {
                *project_changed = true;
            }
        }
        EffectType::ColorGradeLUT {
            lut_path,
            intensity,
        } => {
            ui.horizontal(|ui| {
                ui.label("LUT Path:");
                let path_before = lut_path.clone();
                ui.text_edit_singleline(lut_path);
                if path_before != *lut_path {
                    *project_changed = true;
                }
            });

            let i_before = intensity.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                })
            {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }
        }
        EffectType::ColorSpaceConvert { mode } => {
            let mode_before = *mode;
            egui::ComboBox::from_id_salt(format!("convert_combo_{:?}", ui.next_auto_id()))
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
            if mode_before != *mode {
                *project_changed = true;
            }
        }
        EffectType::FilmGrain {
            intensity,
            grain_size,
            color_film,
        } => {
            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Grain Intensity",
                intensity,
                |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                },
            ) {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }

            ui.horizontal(|ui| {
                ui.label("Grain Size:");
                let size_before = *grain_size;
                ui.add(egui::Slider::new(grain_size, 1.0..=5.0));
                if size_before != *grain_size {
                    *project_changed = true;
                }
            });

            ui.horizontal(|ui| {
                let c_before = *color_film;
                ui.checkbox(color_film, "Color Film Grain");
                if c_before != *color_film {
                    *project_changed = true;
                }
            });
        }
        EffectType::Posterize { levels } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Posterize Levels",
                levels,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=32.0));
                },
            );
        }
        EffectType::Invert { invert_alpha } => {
            ui.horizontal(|ui| {
                let b_before = *invert_alpha;
                ui.checkbox(invert_alpha, "Invert Alpha");
                if b_before != *invert_alpha {
                    *project_changed = true;
                }
            });
        }
        EffectType::Threshold { threshold } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Threshold",
                threshold,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=255.0));
                },
            );
        }
        EffectType::Colorama {
            preset_index,
            cycle_phase,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Preset (0=Rainbow,1=Heat,2=Sepia,3=Solar)",
                preset_index,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=3.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Cycle Phase",
                cycle_phase,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
        }
        EffectType::Curves { channel } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Channel (0=Master,1=R,2=G,3=B)",
                channel,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=3.0));
                },
            );
            ui.label("Use S-curve preset (5-point catmull-rom)");
        }
        EffectType::ShiftChannels {
            take_red,
            take_green,
            take_blue,
            take_alpha,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Red (0=R,1=G,2=B,3=A,4=Off,5=On)",
                take_red,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Green",
                take_green,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Blue",
                take_blue,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Take Alpha",
                take_alpha,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
        }
        EffectType::ColorBalance {
            shadows,
            midtones,
            highlights,
            preserve_luminosity,
        } => {
            // Plain (non-Animatable) fields: edit in place, flag project change.
            for (label, band) in [
                ("Shadows", shadows),
                ("Midtones", midtones),
                ("Highlights", highlights),
            ] {
                ui.small(label);
                ui.horizontal(|ui| {
                    for (i, cname) in ["R", "G", "B"].iter().enumerate() {
                        ui.label(*cname);
                        if ui
                            .add(
                                egui::DragValue::new(&mut band[i])
                                    .speed(1.0)
                                    .range(-100.0..=100.0),
                            )
                            .changed()
                        {
                            *project_changed = true;
                        }
                    }
                });
            }
            if ui
                .checkbox(preserve_luminosity, "Preserve Luminosity")
                .changed()
            {
                *project_changed = true;
            }
        }
        EffectType::ChannelMixer { matrix, monochrome } => {
            ui.label("Output ← Input (%)");
            egui::Grid::new("channel_mixer_grid")
                .num_columns(4)
                .show(ui, |ui| {
                    let names = ["R", "G", "B"];
                    for (r, row) in matrix.iter_mut().enumerate() {
                        ui.label(format!("←{}", names[r]));
                        for v in row.iter_mut() {
                            if ui
                                .add(egui::DragValue::new(v).speed(1.0).range(-200.0..=200.0))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                        ui.end_row();
                    }
                });
            if ui.checkbox(monochrome, "Monochrome").changed() {
                *project_changed = true;
            }
        }
        EffectType::Tritone {
            shadow_color,
            mid_color,
            highlight_color,
        } => {
            // Color pickers for 3-tone mapping
            let sc_before = shadow_color.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Shadow Color",
                shadow_color,
                |ui, val| {
                    ui.color_edit_button_rgb(val);
                },
            ) {
                *next_frame = Some(nf);
            }
            if sc_before != *shadow_color {
                *project_changed = true;
            }

            let mc_before = mid_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Mid Color", mid_color, |ui, val| {
                    ui.color_edit_button_rgb(val);
                })
            {
                *next_frame = Some(nf);
            }
            if mc_before != *mid_color {
                *project_changed = true;
            }

            let hc_before = highlight_color.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Highlight Color",
                highlight_color,
                |ui, val| {
                    ui.color_edit_button_rgb(val);
                },
            ) {
                *next_frame = Some(nf);
            }
            if hc_before != *highlight_color {
                *project_changed = true;
            }
        }
        EffectType::Vibrance { amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
        }
        EffectType::WhiteBalance { temperature, tint } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Temperature",
                temperature,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tint",
                tint,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
        }
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Hue",
                hue_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -180.0..=180.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Saturation",
                saturation,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Lightness",
                lightness,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
        }
        EffectType::FilmEmulation {
            lift,
            gamma,
            gain,
            hue_shift_deg,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Lift",
                lift,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -0.5..=0.5));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gamma",
                gamma,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.1..=3.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gain",
                gain,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=3.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Hue Shift",
                hue_shift_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -180.0..=180.0).suffix("°"));
                },
            );
        }
        EffectType::GradientMap {
            low_color,
            mid_color,
            high_color,
        } => {
            let lc_before = low_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Shadow Color", low_color, |ui, val| {
                    ui.color_edit_button_rgb(val);
                })
            {
                *next_frame = Some(nf);
            }
            if lc_before != *low_color {
                *project_changed = true;
            }

            let mc_before = mid_color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Mid Color", mid_color, |ui, val| {
                    ui.color_edit_button_rgb(val);
                })
            {
                *next_frame = Some(nf);
            }
            if mc_before != *mid_color {
                *project_changed = true;
            }

            let hc_before = high_color.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Highlight Color",
                high_color,
                |ui, val| {
                    ui.color_edit_button_rgb(val);
                },
            ) {
                *next_frame = Some(nf);
            }
            if hc_before != *high_color {
                *project_changed = true;
            }
        }
        _ => {}
    }
}
