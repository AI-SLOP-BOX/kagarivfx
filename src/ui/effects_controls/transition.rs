use crate::core::timeline::EffectType;
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
        EffectType::LinearWipe { completion, angle } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wipe Angle",
                angle,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
        }
        EffectType::IrisWipe { completion } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
        }
        EffectType::RadialWipe { completion } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
        }
        EffectType::VenetianBlinds { completion, width } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Completion",
                completion,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Width",
                width,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=100.0).suffix(" px"));
                },
            );
        }
        EffectType::PageTurn {
            fold_position: _,
            fold_radius,
            fold_direction_deg,
            light_direction_deg,
            back_opacity,
            back_color: _,
        } => {
            ui.label("📖 CC Page Turn");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fold Radius",
                fold_radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 10.0..=500.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Fold Angle",
                fold_direction_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -180.0..=180.0).suffix(" °"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Light Direction",
                light_direction_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -180.0..=180.0).suffix(" °"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Back Opacity",
                back_opacity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix(" %"));
                },
            );
        }
        EffectType::LightSweep {
            direction_deg,
            sweep_intensity,
            edge_intensity,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Direction",
                direction_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Sweep Intensity",
                sweep_intensity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Edge Intensity",
                edge_intensity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        _ => {}
    }
}
