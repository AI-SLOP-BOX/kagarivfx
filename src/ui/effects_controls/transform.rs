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
        EffectType::Offset { shift_x, shift_y } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Offset X",
                shift_x,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -300.0..=300.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Offset Y",
                shift_y,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -300.0..=300.0));
                },
            );
        }
        EffectType::MotionTile {
            tile_center: _,
            tile_width,
            tile_height,
            output_width,
            output_height,
            mirror_edges,
            phase,
        } => {
            ui.label("🔲 Motion Tile");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tile Width",
                tile_width,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=500.0).suffix(" %"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Tile Height",
                tile_height,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=500.0).suffix(" %"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Output Width",
                output_width,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 100.0..=1000.0).suffix(" %"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Output Height",
                output_height,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 100.0..=1000.0).suffix(" %"));
                },
            );
            ui.checkbox(mirror_edges, "Mirror Edges");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Phase",
                phase,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -360.0..=360.0).suffix(" °"));
                },
            );
        }
        EffectType::Transform {
            anchor_point,
            position,
            scale_width,
            scale_height,
            uniform_scale,
            skew_deg,
            skew_axis_deg,
            rotation_deg,
            opacity,
        } => {
            ui.label("📐 Transform");
            let ap_before = anchor_point.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Anchor Point",
                anchor_point,
                |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            if ap_before != *anchor_point {
                *project_changed = true;
            }

            let pos_before = position.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Position", position, |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                })
            {
                *next_frame = Some(nf);
            }
            if pos_before != *position {
                *project_changed = true;
            }

            ui.horizontal(|ui| {
                if ui.checkbox(uniform_scale, "Uniform Scale").changed() {
                    *project_changed = true;
                }
            });
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                if *uniform_scale {
                    "Scale"
                } else {
                    "Scale Width"
                },
                scale_width,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1000.0).suffix("%"));
                },
            );
            if !*uniform_scale {
                draw_prop(
                    ui,
                    current_frame,
                    project_changed,
                    next_frame,
                    "Scale Height",
                    scale_height,
                    |ui, v| {
                        ui.add(egui::Slider::new(v, 0.0..=1000.0).suffix("%"));
                    },
                );
            }
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Skew",
                skew_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -85.0..=85.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Skew Axis",
                skew_axis_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Rotation",
                rotation_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -3600.0..=3600.0).suffix("°"));
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
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
        }
        EffectType::DisplacementMap {
            source_layer,
            max_horizontal,
            max_vertical,
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
                "Max Horizontal",
                max_horizontal,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -200.0..=200.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Vertical",
                max_vertical,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -200.0..=200.0));
                },
            );
        }
        _ => {}
    }
}
