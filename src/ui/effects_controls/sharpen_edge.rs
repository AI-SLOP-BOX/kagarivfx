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
        EffectType::Sharpen { amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Sharpen Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0));
                },
            );
        }
        EffectType::SobelEdges { invert } => {
            if ui.checkbox(invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        EffectType::DirectionalSharpen {
            angle_deg,
            strength,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Strength",
                strength,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
        }
        EffectType::Mosaic { block_w, block_h } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Block Width",
                block_w,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=128.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Block Height",
                block_h,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=128.0).suffix(" px"));
                },
            );
        }
        EffectType::MedianFilter { radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=16.0).suffix(" px"));
                },
            );
        }
        EffectType::FindEdges { invert } => {
            ui.label("✏️ Find Edges");
            if ui.checkbox(invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        EffectType::Minimax { operation, radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Operation (0=Min,1=Max)",
                operation,
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
                    ui.add(egui::Slider::new(v, 1.0..=50.0));
                },
            );
        }
        _ => {}
    }
}
