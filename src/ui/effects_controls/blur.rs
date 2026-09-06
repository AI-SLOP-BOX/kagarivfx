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
        EffectType::DirectionalBlur { angle, length } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Direction",
                angle,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Length",
                length,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0));
                },
            );
        }
        EffectType::RadialBlur { amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radial Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
        }
        EffectType::CompoundBlur {
            source_layer,
            max_blur,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Source Layer ID",
                source_layer,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=10.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Blur",
                max_blur,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0));
                },
            );
        }
        EffectType::RadialBlurZoom { amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0));
                },
            );
        }
        EffectType::TiltShift {
            focus_y,
            focus_height,
            max_blur,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Focus Y",
                focus_y,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Band Height",
                focus_height,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.02..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Blur",
                max_blur,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=32.0).suffix(" px"));
                },
            );
        }
        EffectType::CameraLensBlur {
            blur_radius,
            iris_blades,
            iris_rotation_deg,
            iris_roundness,
            highlight_gain,
            highlight_threshold,
        } => {
            ui.label("📷 Camera Lens Blur");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Blur Radius",
                blur_radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix(" px"));
                },
            );
            ui.horizontal(|ui| {
                ui.label("Iris Blades:");
                let mut b_i32 = *iris_blades as i32;
                if ui.add(egui::Slider::new(&mut b_i32, 3..=16)).changed() {
                    *iris_blades = b_i32.clamp(3, 16) as u32;
                    *project_changed = true;
                }
            });
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Iris Rotation",
                iris_rotation_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Iris Roundness",
                iris_roundness,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Highlight Gain",
                highlight_gain,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Highlight Threshold",
                highlight_threshold,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::MotionBlur {
            shutter_angle,
            samples,
        } => {
            let sa_before = shutter_angle.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Shutter Angle",
                shutter_angle,
                |ui, val| {
                    ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
                },
            ) {
                *next_frame = Some(nf);
            }
            if sa_before != *shutter_angle {
                *project_changed = true;
            }

            ui.horizontal(|ui| {
                ui.label("Samples:");
                let before_s = *samples;
                ui.add(egui::DragValue::new(samples).range(2..=16));
                if before_s != *samples {
                    *project_changed = true;
                }
            });
        }
        _ => {}
    }
}
