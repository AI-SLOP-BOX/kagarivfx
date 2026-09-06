use crate::ui::inspector_property::draw_property_ui;
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
        EffectType::SliderControl { value } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Value",
                value,
                |ui, v| {
                    ui.add(egui::DragValue::new(v).speed(0.1));
                },
            );
        }
        EffectType::AngleControl { angle_degrees } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_degrees,
                |ui, v| {
                    ui.add(egui::DragValue::new(v).speed(1.0).suffix("°"));
                },
            );
        }
        EffectType::PointControl { point } => {
            let p_before = point.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Point", point, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) {
                *next_frame = Some(nf);
            }
            if p_before != *point {
                *project_changed = true;
            }
        }
        EffectType::Letterbox { frac } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bars (frame frac)",
                frac,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=0.45));
                },
            );
        }
        EffectType::ColorControl { color } => {
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
        EffectType::CheckboxControl { checked } => {
            if ui.checkbox(checked, "Enabled").changed() {
                *project_changed = true;
            }
        }
        EffectType::DropdownControl { value, options } => {
            let id = egui::Id::new(("dropdown_ctrl", value as *const i32 as usize));
            egui::ComboBox::from_id_salt(id)
                .selected_text(options.get(*value as usize).cloned().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for (i, opt) in options.iter().enumerate() {
                        if ui.selectable_value(value, i as i32, opt).clicked() {
                            *project_changed = true;
                        }
                    }
                });
        }
        EffectType::Point3DControl { point } => {
            let p_before = point.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Point 3D", point, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z: "));
                });
            }) {
                *next_frame = Some(nf);
            }
            if p_before != *point {
                *project_changed = true;
            }
        }
        _ => {}
    }
}
