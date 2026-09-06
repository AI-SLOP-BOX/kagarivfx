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
        EffectType::MeshWarp {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } => {
            let tl_before = top_left.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Top Left Corner", top_left, |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                })
            {
                *next_frame = Some(nf);
            }
            if tl_before != *top_left {
                *project_changed = true;
            }

            let tr_before = top_right.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Top Right Corner",
                top_right,
                |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            if tr_before != *top_right {
                *project_changed = true;
            }

            let bl_before = bottom_left.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Left Corner",
                bottom_left,
                |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            if bl_before != *bottom_left {
                *project_changed = true;
            }

            let br_before = bottom_right.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Right Corner",
                bottom_right,
                |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            if br_before != *bottom_right {
                *project_changed = true;
            }
        }
        EffectType::CornerPin {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } => {
            let tl_before = top_left.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Top Left Pin", top_left, |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                })
            {
                *next_frame = Some(nf);
            }
            if tl_before != *top_left {
                *project_changed = true;
            }

            let tr_before = top_right.clone();
            if let Some(nf) =
                draw_property_ui(current_frame, ui, "Top Right Pin", top_right, |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                })
            {
                *next_frame = Some(nf);
            }
            if tr_before != *top_right {
                *project_changed = true;
            }

            let br_before = bottom_right.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Right Pin",
                bottom_right,
                |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            if br_before != *bottom_right {
                *project_changed = true;
            }

            let bl_before = bottom_left.clone();
            if let Some(nf) = draw_property_ui(
                current_frame,
                ui,
                "Bottom Left Pin",
                bottom_left,
                |ui, val| {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    });
                },
            ) {
                *next_frame = Some(nf);
            }
            if bl_before != *bottom_left {
                *project_changed = true;
            }

            ui.label(
                egui::RichText::new("Pin corners in layer pixel space (animatable)")
                    .small()
                    .color(crate::ui::theme::colors::TEXT_SECONDARY),
            );
        }
        EffectType::Twirl { angle, radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Twirl Angle",
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
                "Twirl Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=300.0));
                },
            );
        }
        EffectType::Bulge { amount, radius } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bulge Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -100.0..=100.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bulge Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=300.0));
                },
            );
        }
        EffectType::Spherize {
            radius,
            refractive_index,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=500.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Refractive Index",
                refractive_index,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.5..=2.0));
                },
            );
        }
        EffectType::TurbulentDisplace {
            amount,
            size,
            evolution,
            complexity,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -200.0..=200.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Size",
                size,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=500.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Evolution",
                evolution,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Complexity",
                complexity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=10.0));
                },
            );
        }
        EffectType::WaveWarp {
            wave_height,
            wave_width,
            speed,
            direction_deg,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wave Height",
                wave_height,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=200.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wave Width",
                wave_width,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=500.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Speed",
                speed,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=20.0));
                },
            );
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
        }
        EffectType::CcLens { convergence, zoom } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Convergence",
                convergence,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=200.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Zoom",
                zoom,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
        }
        EffectType::PolarCoordinates { interpolation, .. } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Interpolation",
                interpolation,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%"));
                },
            );
        }
        EffectType::OpticsCompensation {
            field_of_view_deg,
            zoom,
            ..
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "FOV",
                field_of_view_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=180.0).suffix("°"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Zoom",
                zoom,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0));
                },
            );
        }
        EffectType::RadialFastBlur { amount, .. } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::BendIt {
            top_offset,
            bottom_offset,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Top Offset",
                top_offset,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -50.0..=50.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bottom Offset",
                bottom_offset,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -50.0..=50.0).suffix(" px"));
                },
            );
        }
        EffectType::Tiler { scale_percent, .. } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Scale",
                scale_percent,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 10.0..=500.0).suffix("%"));
                },
            );
        }
        EffectType::Vortex { radius, angle_deg } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=2000.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Angle",
                angle_deg,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -720.0..=720.0).suffix("°"));
                },
            );
        }
        EffectType::HeatDistortion { strength, speed } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Strength",
                strength,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=30.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Speed",
                speed,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=5.0).suffix("×"));
                },
            );
        }
        EffectType::RainRipples {
            drop_count,
            wave_strength,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Drop Count",
                drop_count,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=100.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wave Strength",
                wave_strength,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=20.0));
                },
            );
        }
        EffectType::Fisheye { strength } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Strength (− = pincushion)",
                strength,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -1.0..=1.0));
                },
            );
        }
        EffectType::LensCorrection { k1, k2 } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "K1 (Barrel + / Pincushion −)",
                k1,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -0.5..=0.5));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "K2",
                k2,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -0.5..=0.5));
                },
            );
        }
        EffectType::GlitchDisplacement { seed, amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Seed",
                seed,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=9999.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=10.0));
                },
            );
        }
        EffectType::PinchPunch { radius, amount } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=2000.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Amount (+pinch / −punch)",
                amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -2.0..=2.0));
                },
            );
        }
        EffectType::ScanlineGlitch {
            jitter_amount,
            seed,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Jitter",
                jitter_amount,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=50.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Seed",
                seed,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=9999.0));
                },
            );
        }
        EffectType::GlassEdgeBevel {
            bevel_size,
            refraction,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bevel Size",
                bevel_size,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=64.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Refraction",
                refraction,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=3.0));
                },
            );
        }
        EffectType::RefractionLens { radius, ior } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Radius",
                radius,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=2000.0).suffix(" px"));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "IOR",
                ior,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=3.0));
                },
            );
        }
        _ => {}
    }
}
