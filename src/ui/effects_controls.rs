use eframe::egui;
use crate::core::timeline::{Effect, EffectType, ColorConversionMode};
use crate::core::property::Animatable;
use crate::ui::inspector::draw_property_ui;

pub struct EffectPreset {
    pub name: &'static str,
    pub button_label: &'static str,
    pub search_key: &'static str,
    pub id_prefix: &'static str,
    pub create_fn: fn(idx: usize) -> Effect,
}

pub fn get_all_effect_presets() -> &'static [EffectPreset] {
    &[
        EffectPreset {
            name: "Gaussian Blur",
            button_label: "+ Gaussian Blur",
            search_key: "gaussian blur",
            id_prefix: "blur",
            create_fn: |idx| Effect {
                id: format!("blur_{}", idx),
                name: "Gaussian Blur".to_string(),
                effect_type: EffectType::GaussianBlur { blur_radius: Animatable::new_constant(10.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Color Tint",
            button_label: "+ Color Tint",
            search_key: "color tint",
            id_prefix: "tint",
            create_fn: |idx| Effect {
                id: format!("tint_{}", idx),
                name: "Color Tint".to_string(),
                effect_type: EffectType::ColorTint {
                    color: Animatable::new_constant([1.0, 0.2, 0.2, 1.0]),
                    intensity: Animatable::new_constant(50.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Drop Shadow",
            button_label: "+ Drop Shadow",
            search_key: "drop shadow",
            id_prefix: "shadow",
            create_fn: |idx| Effect {
                id: format!("shadow_{}", idx),
                name: "Drop Shadow".to_string(),
                effect_type: EffectType::DropShadow {
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                    opacity: Animatable::new_constant(50.0),
                    direction: Animatable::new_constant(135.0),
                    distance: Animatable::new_constant(5.0),
                    softness: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chromatic Aberration",
            button_label: "+ Chromatic Aberration",
            search_key: "chromatic aberration",
            id_prefix: "ca",
            create_fn: |idx| Effect {
                id: format!("ca_{}", idx),
                name: "Chromatic Aberration".to_string(),
                effect_type: EffectType::ChromaticAberration {
                    shift_r: Animatable::new_constant(3.0),
                    shift_b: Animatable::new_constant(3.0),
                    edge_falloff: Animatable::new_constant(0.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Vignette",
            button_label: "+ Vignette",
            search_key: "vignette",
            id_prefix: "vignette",
            create_fn: |idx| Effect {
                id: format!("vignette_{}", idx),
                name: "Vignette".to_string(),
                effect_type: EffectType::Vignette {
                    intensity: Animatable::new_constant(60.0),
                    roundness: Animatable::new_constant(1.0),
                    feather: Animatable::new_constant(50.0),
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Levels",
            button_label: "+ Levels (Gamma/Crush)",
            search_key: "levels",
            id_prefix: "levels",
            create_fn: |idx| Effect {
                id: format!("levels_{}", idx),
                name: "Levels".to_string(),
                effect_type: EffectType::Levels {
                    input_black: Animatable::new_constant(0.0),
                    input_white: Animatable::new_constant(1.0),
                    gamma: Animatable::new_constant(1.0),
                    output_black: Animatable::new_constant(0.0),
                    output_white: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Hue / Saturation",
            button_label: "+ Hue / Saturation",
            search_key: "hue / saturation",
            id_prefix: "huesat",
            create_fn: |idx| Effect {
                id: format!("huesat_{}", idx),
                name: "Hue / Saturation".to_string(),
                effect_type: EffectType::HueSaturation {
                    hue_shift: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glow",
            button_label: "+ Glow",
            search_key: "glow",
            id_prefix: "glow",
            create_fn: |idx| Effect {
                id: format!("glow_{}", idx),
                name: "Glow".to_string(),
                effect_type: EffectType::Glow {
                    threshold: Animatable::new_constant(50.0),
                    radius: Animatable::new_constant(20.0),
                    intensity: Animatable::new_constant(50.0),
                    color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Mesh Warp",
            button_label: "+ Mesh Warp (Grid)",
            search_key: "mesh warp",
            id_prefix: "meshwarp",
            create_fn: |idx| Effect {
                id: format!("meshwarp_{}", idx),
                name: "Mesh Warp".to_string(),
                effect_type: EffectType::MeshWarp {
                    top_left: Animatable::new_constant([0.0, 0.0]),
                    top_right: Animatable::new_constant([1920.0, 0.0]),
                    bottom_left: Animatable::new_constant([0.0, 1080.0]),
                    bottom_right: Animatable::new_constant([1920.0, 1080.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Cinematic 3D LUT",
            button_label: "+ Cinematic 3D LUT",
            search_key: "lut",
            id_prefix: "lut",
            create_fn: |idx| Effect {
                id: format!("lut_{}", idx),
                name: "Cinematic 3D LUT".to_string(),
                effect_type: EffectType::ColorGradeLUT {
                    lut_path: "alexa_logc_to_rec709.cube".to_string(),
                    intensity: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Color Space Converter",
            button_label: "+ Log Space Converter",
            search_key: "log space converter",
            id_prefix: "convert",
            create_fn: |idx| Effect {
                id: format!("convert_{}", idx),
                name: "Color Space Converter".to_string(),
                effect_type: EffectType::ColorSpaceConvert {
                    mode: ColorConversionMode::LogCToLinear,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Physical Film Grain",
            button_label: "+ Physical Film Grain",
            search_key: "film grain",
            id_prefix: "grain",
            create_fn: |idx| Effect {
                id: format!("grain_{}", idx),
                name: "Physical Film Grain".to_string(),
                effect_type: EffectType::FilmGrain {
                    intensity: Animatable::new_constant(15.0),
                    grain_size: 1.5,
                    color_film: true,
                },
                enabled: true,
            },
        },
    ]
}

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
