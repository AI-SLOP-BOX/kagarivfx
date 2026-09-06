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
        EffectType::DropShadow {
            color,
            opacity,
            direction,
            distance,
            softness,
        } => {
            let color_before = color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Shadow Color", color, |ui, val| {
                    ui.color_edit_button_rgba_unmultiplied(val);
                })
            {
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
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Direction", direction, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
                })
            {
                *next_frame = Some(nf);
            }
            if direction_before != *direction {
                *project_changed = true;
            }

            let distance_before = distance.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Distance", distance, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0).suffix(" px"));
                })
            {
                *next_frame = Some(nf);
            }
            if distance_before != *distance {
                *project_changed = true;
            }

            let softness_before = softness.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Softness", softness, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if softness_before != *softness {
                *project_changed = true;
            }
        }
        EffectType::ChromaticAberration {
            shift_r,
            shift_b,
            edge_falloff,
            iris_linked: _,
        } => {
            let shift_r_before = shift_r.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Red Shift", shift_r, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
                })
            {
                *next_frame = Some(nf);
            }
            if shift_r_before != *shift_r {
                *project_changed = true;
            }

            let shift_b_before = shift_b.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Blue Shift", shift_b, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
                })
            {
                *next_frame = Some(nf);
            }
            if shift_b_before != *shift_b {
                *project_changed = true;
            }

            let ef_before = edge_falloff.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Edge Falloff",
                edge_falloff,
                |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                },
            ) {
                *next_frame = Some(nf);
            }
            if ef_before != *edge_falloff {
                *project_changed = true;
            }
        }
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => {
            let i_before = intensity.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }

            let r_before = roundness.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Roundness", roundness, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                })
            {
                *next_frame = Some(nf);
            }
            if r_before != *roundness {
                *project_changed = true;
            }

            let f_before = feather.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Feather", feather, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if f_before != *feather {
                *project_changed = true;
            }

            let c_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) {
                *next_frame = Some(nf);
            }
            if c_before != *color {
                *project_changed = true;
            }
        }
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            color,
        } => {
            let t_before = threshold.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Threshold", threshold, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if t_before != *threshold {
                *project_changed = true;
            }

            let r_before = radius.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Radius", radius, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=200.0).suffix(" px"));
            }) {
                *next_frame = Some(nf);
            }
            if r_before != *radius {
                *project_changed = true;
            }

            let i_before = intensity.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=100.0));
                })
            {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }

            let c_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Glow Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) {
                *next_frame = Some(nf);
            }
            if c_before != *color {
                *project_changed = true;
            }
        }
        EffectType::LensFlare {
            enabled,
            position_x,
            position_y,
            intensity,
            threshold,
            color,
            ..
        } => {
            let en_before = enabled.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Enabled", enabled, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0).show_value(false));
            }) {
                *next_frame = Some(nf);
            }
            if en_before != *enabled {
                *project_changed = true;
            }

            let px_before = position_x.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Position X", position_x, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                })
            {
                *next_frame = Some(nf);
            }
            if px_before != *position_x {
                *project_changed = true;
            }

            let py_before = position_y.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Position Y", position_y, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=1.0));
                })
            {
                *next_frame = Some(nf);
            }
            if py_before != *position_y {
                *project_changed = true;
            }

            let i_before = intensity.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=5.0));
                })
            {
                *next_frame = Some(nf);
            }
            if i_before != *intensity {
                *project_changed = true;
            }

            let th_before = threshold.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Threshold", threshold, |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=2.0));
                })
            {
                *next_frame = Some(nf);
            }
            if th_before != *threshold {
                *project_changed = true;
            }

            let c_before = color.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Flare Color", color, |ui, val| {
                    ui.color_edit_button_rgba_unmultiplied(val);
                })
            {
                *next_frame = Some(nf);
            }
            if c_before != *color {
                *project_changed = true;
            }
        }
        EffectType::GlowPro {
            threshold,
            radius,
            intensity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Threshold",
                threshold,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=128.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=4.0));
                },
            );
        }
        EffectType::CrtScanlines {
            line_spacing,
            intensity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Line Spacing",
                line_spacing,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=50.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Intensity",
                intensity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::NightVision { amplification } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amplification",
                amplification,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=8.0).suffix("×"));
                },
            );
        }
        EffectType::Emboss { angle_deg, depth } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -180.0..=180.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth",
                depth,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=10.0));
                },
            );
        }
        EffectType::BevelAlpha {
            depth,
            light_angle_deg,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth",
                depth,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=32.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Light Angle",
                light_angle_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
        }
        EffectType::ReflectionMap {
            reflect_y,
            fade_dist,
            opacity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Horizon Y",
                reflect_y,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=2000.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fade Distance",
                fade_dist,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=1000.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Opacity",
                opacity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::OpticalFlares {
            position: _,
            brightness,
            scale,
        } => {
            ui.label("✨ Optical Flares");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Brightness",
                brightness,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Scale",
                scale,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.1..=5.0));
                },
            );
        }
        _ => {}
    }
}
