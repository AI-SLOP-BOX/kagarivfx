use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Effect, EffectType, ColorConversionMode};
use crate::core::property::Animatable;
use crate::ui::inspector::draw_property_ui;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: &mut u32) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(240.0)
        .show(ctx, |ui| {
            ui.heading("Effects & Presets");
            ui.separator();
            
            let mut project_changed = false;
            let mut next_frame = None;
            let mut current_frame_reset = None;

            // Clone current project to apply transactional state mutations
            let mut temp_project = app.history.current().clone();
            
            if let Some(idx) = app.selected_layer_idx {
                ui.label("Add Effect to Selected Layer:");
                ui.vertical(|ui| {
                    if ui.button("+ Gaussian Blur").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("blur_{}", len),
                                name: "Gaussian Blur".to_string(),
                                effect_type: EffectType::GaussianBlur {
                                    blur_radius: Animatable::new_constant(10.0),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Color Tint").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("tint_{}", len),
                                name: "Color Tint".to_string(),
                                effect_type: EffectType::ColorTint {
                                    color: Animatable::new_constant([1.0, 0.0, 0.0, 1.0]),
                                    intensity: Animatable::new_constant(100.0),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Drop Shadow").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("shadow_{}", len),
                                name: "Drop Shadow".to_string(),
                                effect_type: EffectType::DropShadow {
                                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                                    opacity: Animatable::new_constant(50.0),
                                    direction: Animatable::new_constant(135.0),
                                    distance: Animatable::new_constant(5.0),
                                    softness: Animatable::new_constant(5.0),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Chromatic Aberration").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("ca_{}", len),
                                name: "Chromatic Aberration".to_string(),
                                effect_type: EffectType::ChromaticAberration {
                                    shift_r: Animatable::new_constant(3.0),
                                    shift_b: Animatable::new_constant(3.0),
                                    edge_falloff: Animatable::new_constant(0.5),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Vignette").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("vignette_{}", len),
                                name: "Vignette".to_string(),
                                effect_type: EffectType::Vignette {
                                    intensity: Animatable::new_constant(60.0),
                                    roundness: Animatable::new_constant(1.0),
                                    feather: Animatable::new_constant(50.0),
                                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Levels (Gamma/Crush)").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("levels_{}", len),
                                name: "Levels".to_string(),
                                effect_type: EffectType::Levels {
                                    input_black: Animatable::new_constant(0.0),
                                    input_white: Animatable::new_constant(1.0),
                                    gamma: Animatable::new_constant(1.0),
                                    output_black: Animatable::new_constant(0.0),
                                    output_white: Animatable::new_constant(1.0),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Hue / Saturation").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("huesat_{}", len),
                                name: "Hue / Saturation".to_string(),
                                effect_type: EffectType::HueSaturation {
                                    hue_shift: Animatable::new_constant(0.0),
                                    saturation: Animatable::new_constant(0.0),
                                    lightness: Animatable::new_constant(0.0),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Glow").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("glow_{}", len),
                                name: "Glow".to_string(),
                                effect_type: EffectType::Glow {
                                    threshold: Animatable::new_constant(50.0),
                                    radius: Animatable::new_constant(20.0),
                                    intensity: Animatable::new_constant(50.0),
                                    color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Mesh Warp (Grid)").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("meshwarp_{}", len),
                                name: "Mesh Warp".to_string(),
                                effect_type: EffectType::MeshWarp {
                                    top_left: Animatable::new_constant([0.0, 0.0]),
                                    top_right: Animatable::new_constant([1920.0, 0.0]),
                                    bottom_left: Animatable::new_constant([0.0, 1080.0]),
                                    bottom_right: Animatable::new_constant([1920.0, 1080.0]),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Cinematic 3D LUT").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("lut_{}", len),
                                name: "Cinematic 3D LUT".to_string(),
                                effect_type: EffectType::ColorGradeLUT {
                                    lut_path: "alexa_logc_to_rec709.cube".to_string(),
                                    intensity: Animatable::new_constant(100.0),
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Log Space Converter").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("convert_{}", len),
                                name: "Color Space Converter".to_string(),
                                effect_type: EffectType::ColorSpaceConvert {
                                    mode: ColorConversionMode::LogCToLinear,
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                    if ui.button("+ Physical Film Grain").clicked() {
                        let comp = app.history.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            let len = comp.layers[idx].effects.len();
                            comp.layers[idx].effects.push(Effect {
                                id: format!("grain_{}", len),
                                name: "Physical Film Grain".to_string(),
                                effect_type: EffectType::FilmGrain {
                                    intensity: Animatable::new_constant(15.0),
                                    grain_size: 1.5,
                                    color_film: true,
                                },
                                enabled: true,
                            });
                            project_changed = true;
                        }
                    }
                });
            } else {
                ui.weak("Select a layer to apply effects");
            }
            
            ui.separator();
            
            // Show list of applied effects
            if let Some(idx) = app.selected_layer_idx {
                let comp = temp_project.active_composition_mut();
                if idx < comp.layers.len() {
                    ui.label("Applied Effects:");
                    let layer = &mut comp.layers[idx];
                    for effect in &mut layer.effects {
                        ui.collapsing(&effect.name, |ui| {
                            let val_before_enabled = effect.enabled;
                            ui.checkbox(&mut effect.enabled, "Enabled");
                            if val_before_enabled != effect.enabled {
                                project_changed = true;
                            }
                            
                            match &mut effect.effect_type {
                                EffectType::GaussianBlur { blur_radius } => {
                                    let val_before = blur_radius.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Blur Radius", blur_radius, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if val_before != *blur_radius {
                                        project_changed = true;
                                    }
                                }
                                EffectType::ColorTint { color, intensity } => {
                                    let color_before = color.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Tint Color", color, |ui, val| {
                                        ui.color_edit_button_rgba_unmultiplied(val);
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if color_before != *color {
                                        project_changed = true;
                                    }

                                    let intensity_before = intensity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Intensity", intensity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if intensity_before != *intensity {
                                        project_changed = true;
                                    }
                                }
                                EffectType::DropShadow { color, opacity, direction, distance, softness } => {
                                    let color_before = color.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Shadow Color", color, |ui, val| {
                                        ui.color_edit_button_rgba_unmultiplied(val);
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if color_before != *color {
                                        project_changed = true;
                                    }

                                    let opacity_before = opacity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Opacity", opacity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if opacity_before != *opacity {
                                        project_changed = true;
                                    }

                                    let direction_before = direction.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Direction", direction, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if direction_before != *direction {
                                        project_changed = true;
                                    }

                                    let distance_before = distance.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Distance", distance, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0).suffix(" px"));
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if distance_before != *distance {
                                        project_changed = true;
                                    }

                                    let softness_before = softness.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Softness", softness, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) {
                                        next_frame = Some(nf);
                                    }
                                    if softness_before != *softness {
                                        project_changed = true;
                                    }
                                }
                                EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
                                    let shift_r_before = shift_r.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Red Shift", shift_r, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
                                    }) { next_frame = Some(nf); }
                                    if shift_r_before != *shift_r { project_changed = true; }

                                    let shift_b_before = shift_b.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Blue Shift", shift_b, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
                                    }) { next_frame = Some(nf); }
                                    if shift_b_before != *shift_b { project_changed = true; }

                                    let ef_before = edge_falloff.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Edge Falloff", edge_falloff, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }) { next_frame = Some(nf); }
                                    if ef_before != *edge_falloff { project_changed = true; }
                                }
                                EffectType::Vignette { intensity, roundness, feather, color } => {
                                    let i_before = intensity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Intensity", intensity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) { next_frame = Some(nf); }
                                    if i_before != *intensity { project_changed = true; }

                                    let r_before = roundness.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Roundness", roundness, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }) { next_frame = Some(nf); }
                                    if r_before != *roundness { project_changed = true; }

                                    let f_before = feather.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Feather", feather, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) { next_frame = Some(nf); }
                                    if f_before != *feather { project_changed = true; }

                                    let c_before = color.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Color", color, |ui, val| {
                                        ui.color_edit_button_rgba_unmultiplied(val);
                                    }) { next_frame = Some(nf); }
                                    if c_before != *color { project_changed = true; }
                                }
                                EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
                                    let ib_before = input_black.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Input Black", input_black, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }) { next_frame = Some(nf); }
                                    if ib_before != *input_black { project_changed = true; }

                                    let iw_before = input_white.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Input White", input_white, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }) { next_frame = Some(nf); }
                                    if iw_before != *input_white { project_changed = true; }

                                    let g_before = gamma.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Gamma", gamma, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.1..=10.0));
                                    }) { next_frame = Some(nf); }
                                    if g_before != *gamma { project_changed = true; }

                                    let ob_before = output_black.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Output Black", output_black, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }) { next_frame = Some(nf); }
                                    if ob_before != *output_black { project_changed = true; }

                                    let ow_before = output_white.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Output White", output_white, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=1.0));
                                    }) { next_frame = Some(nf); }
                                    if ow_before != *output_white { project_changed = true; }
                                }
                                EffectType::HueSaturation { hue_shift, saturation, lightness } => {
                                    let h_before = hue_shift.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Hue Shift", hue_shift, |ui, val| {
                                        ui.add(egui::Slider::new(val, -180.0..=180.0).suffix("°"));
                                    }) { next_frame = Some(nf); }
                                    if h_before != *hue_shift { project_changed = true; }

                                    let s_before = saturation.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Saturation", saturation, |ui, val| {
                                        ui.add(egui::Slider::new(val, -100.0..=100.0));
                                    }) { next_frame = Some(nf); }
                                    if s_before != *saturation { project_changed = true; }

                                    let l_before = lightness.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Lightness", lightness, |ui, val| {
                                        ui.add(egui::Slider::new(val, -100.0..=100.0));
                                    }) { next_frame = Some(nf); }
                                    if l_before != *lightness { project_changed = true; }
                                }
                                EffectType::Glow { threshold, radius, intensity, color } => {
                                    let t_before = threshold.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Threshold", threshold, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) { next_frame = Some(nf); }
                                    if t_before != *threshold { project_changed = true; }

                                    let r_before = radius.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Radius", radius, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=200.0).suffix(" px"));
                                    }) { next_frame = Some(nf); }
                                    if r_before != *radius { project_changed = true; }

                                    let i_before = intensity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Intensity", intensity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0));
                                    }) { next_frame = Some(nf); }
                                    if i_before != *intensity { project_changed = true; }

                                    let c_before = color.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Glow Color", color, |ui, val| {
                                        ui.color_edit_button_rgba_unmultiplied(val);
                                    }) { next_frame = Some(nf); }
                                    if c_before != *color { project_changed = true; }
                                }
                                EffectType::MotionBlur { shutter_angle, samples } => {
                                    let sa_before = shutter_angle.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Shutter Angle", shutter_angle, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
                                    }) { next_frame = Some(nf); }
                                    if sa_before != *shutter_angle { project_changed = true; }

                                    ui.horizontal(|ui| {
                                        ui.label("Samples:");
                                        let before_s = *samples;
                                        ui.add(egui::DragValue::new(samples).clamp_range(2..=16));
                                        if before_s != *samples { project_changed = true; }
                                    });
                                }
                                EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
                                    let tl_before = top_left.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Top Left Corner", top_left, |ui, val| {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                            ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                        });
                                    }) { next_frame = Some(nf); }
                                    if tl_before != *top_left { project_changed = true; }

                                    let tr_before = top_right.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Top Right Corner", top_right, |ui, val| {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                            ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                        });
                                    }) { next_frame = Some(nf); }
                                    if tr_before != *top_right { project_changed = true; }

                                    let bl_before = bottom_left.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Bottom Left Corner", bottom_left, |ui, val| {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                            ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                        });
                                    }) { next_frame = Some(nf); }
                                    if bl_before != *bottom_left { project_changed = true; }

                                    let br_before = bottom_right.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Bottom Right Corner", bottom_right, |ui, val| {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                                            ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                                        });
                                    }) { next_frame = Some(nf); }
                                    if br_before != *bottom_right { project_changed = true; }
                                }
                                EffectType::ColorGradeLUT { lut_path, intensity } => {
                                    ui.horizontal(|ui| {
                                        ui.label("LUT Path:");
                                        let path_before = lut_path.clone();
                                        ui.text_edit_singleline(lut_path);
                                        if path_before != *lut_path { project_changed = true; }
                                    });

                                    let i_before = intensity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Intensity", intensity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                                    }) { next_frame = Some(nf); }
                                    if i_before != *intensity { project_changed = true; }
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
                                    if mode_before != *mode { project_changed = true; }
                                }
                                EffectType::FilmGrain { intensity, grain_size, color_film } => {
                                    let i_before = intensity.clone();
                                    if let Some(nf) = draw_property_ui(*current_frame, ui, "Grain Intensity", intensity, |ui, val| {
                                        ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                                    }) { next_frame = Some(nf); }
                                    if i_before != *intensity { project_changed = true; }

                                    ui.horizontal(|ui| {
                                        ui.label("Grain Size:");
                                        let size_before = *grain_size;
                                        ui.add(egui::Slider::new(grain_size, 1.0..=5.0));
                                        if size_before != *grain_size { project_changed = true; }
                                    });

                                    ui.horizontal(|ui| {
                                        let c_before = *color_film;
                                        ui.checkbox(color_film, "Color Film Grain");
                                        if c_before != *color_film { project_changed = true; }
                                    });
                                }
                            }
                        });
                    }
                }
            }
            
            ui.separator();
            ui.heading("External NLE Link");
            ui.add_space(4.0);
            
            // Connection Status Indicators
            if let Some(app_name) = &app.connected_app {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(egui::Color32::from_rgb(50, 220, 50), format!("🟢 Connected to {}", app_name));
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(egui::Color32::from_rgb(220, 100, 100), "🔴 Disconnected");
                });
                ui.weak("Listening on 127.0.0.1:9000...");
            }
            ui.add_space(8.0);
            
            // OTIO File Path Input
            ui.label("OTIO File Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.otio_path);
            });
            
            ui.horizontal(|ui| {
                if ui.button("📥 Import OTIO").clicked() {
                    if let Ok(json_str) = std::fs::read_to_string(&app.otio_path) {
                        if let Ok(otio_timeline) = crate::core::integration::OtioTimeline::from_json(&json_str) {
                            let new_comp = otio_timeline.to_composition();
                            let comp = temp_project.active_composition_mut();
                            comp.name = new_comp.name;
                            comp.width = new_comp.width;
                            comp.height = new_comp.height;
                            comp.fps = new_comp.fps;
                            comp.duration_frames = new_comp.duration_frames;
                            comp.layers = new_comp.layers;
                            current_frame_reset = Some(0);
                            project_changed = true;
                            log::info!("Successfully imported OTIO composition");
                        } else {
                            log::error!("Failed to parse OTIO JSON");
                        }
                    } else {
                        log::error!("Failed to read OTIO file from path: {}", app.otio_path);
                    }
                }
                if ui.button("📤 Export OTIO").clicked() {
                    let active_comp = temp_project.active_composition();
                    let otio_timeline = crate::core::integration::OtioTimeline::from_composition(active_comp);
                    if let Ok(json_str) = otio_timeline.to_json() {
                        if std::fs::write(&app.otio_path, json_str).is_ok() {
                            log::info!("Successfully exported OTIO composition to: {}", app.otio_path);
                        } else {
                            log::error!("Failed to write OTIO file to path: {}", app.otio_path);
                        }
                    }
                }
            });

            // Commit changes and set output state
            if project_changed {
                app.history.commit(temp_project);
            }
            if let Some(nf) = next_frame {
                *current_frame = nf;
            }
            if let Some(cf) = current_frame_reset {
                app.current_frame = cf;
                *current_frame = cf;
            }
        });
}
