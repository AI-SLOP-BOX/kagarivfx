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
        EffectType::MergePaths { operation } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Operation",
                operation,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=3.0).text("0=Add 1=Sub 2=Int 3=Exc"));
                },
            );
        }
        EffectType::OffsetPath {
            amount,
            line_join,
            miter_limit,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0).text("Amount (px)"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Line Join",
                line_join,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=2.0).text("Line Join"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Miter Limit",
                miter_limit,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=100.0).text("Miter Limit"));
                },
            );
        }
        _ => {}
    }
}
