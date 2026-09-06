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
        EffectType::Halftone { cell_size } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Cell Size",
                cell_size,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=64.0).suffix(" px"));
                },
            );
        }
        EffectType::Solarize { threshold } => {
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
        EffectType::PixelSort { threshold } => {
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
        EffectType::CrossHatch {
            line_gap,
            threshold,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Line Gap",
                line_gap,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=32.0).suffix(" px"));
                },
            );
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
        EffectType::CmykHalftone { dot_size } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Dot Size",
                dot_size,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=32.0).suffix(" px"));
                },
            );
        }
        _ => {}
    }
}
