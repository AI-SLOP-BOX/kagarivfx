use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::LightType;
use crate::ui::theme::colors;

pub fn draw_camera_light_options(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    let comp = app.history.current_mut().active_composition_mut();
    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 3D Camera Options ──
        crate::ui::custom_widgets::ae_section_header(ui, "3D Camera", "📷");
        let cam = &mut comp.active_camera;

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("FOV").small().color(colors::TEXT_SECONDARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("°").small().color(colors::TEXT_MUTED));
                if ui.add(egui::DragValue::new(&mut cam.fov_degrees).speed(1.0).range(1.0..=179.0)).changed() {
                    changed = true;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Focus Distance").small().color(colors::TEXT_SECONDARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
                if ui.add(egui::DragValue::new(&mut cam.focus_distance).speed(1.0).range(0.0..=100000.0)).changed() {
                    changed = true;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Aperture").small().color(colors::TEXT_SECONDARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
                if ui.add(egui::DragValue::new(&mut cam.aperture).speed(0.1).range(0.0..=500.0)).changed() {
                    changed = true;
                }
            });
        });

        ui.add_space(4.0);
        crate::ui::custom_widgets::ae_section_header(ui, "Depth of Field", "🎯");

        if ui.checkbox(&mut cam.dof_enabled, "Enable DOF").clicked() {
            changed = true;
        }

        if cam.dof_enabled {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Max Blur").small().color(colors::TEXT_SECONDARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
                    if ui.add(egui::DragValue::new(&mut cam.dof_max_blur).speed(0.5).range(1.0..=64.0)).changed() {
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Iris Sides").small().color(colors::TEXT_SECONDARY));
                egui::ComboBox::from_id_salt("iris_sides")
                    .selected_text(match cam.dof_iris_sides {
                        0 => "Circle",
                        3 => "Triangle",
                        5 => "Pentagon",
                        6 => "Hexagon",
                        8 => "Octagon",
                        _ => "Circle",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut cam.dof_iris_sides, 0, "Circle").clicked() { changed = true; }
                        if ui.selectable_value(&mut cam.dof_iris_sides, 3, "Triangle").clicked() { changed = true; }
                        if ui.selectable_value(&mut cam.dof_iris_sides, 5, "Pentagon").clicked() { changed = true; }
                        if ui.selectable_value(&mut cam.dof_iris_sides, 6, "Hexagon").clicked() { changed = true; }
                        if ui.selectable_value(&mut cam.dof_iris_sides, 8, "Octagon").clicked() { changed = true; }
                    });
            });
        }

        // ── 3D Light Options ──
        ui.add_space(8.0);
        crate::ui::custom_widgets::ae_section_header(ui, "3D Lights", "💡");

        let light_count = comp.lights.len();
        if light_count == 0 {
            ui.label(egui::RichText::new("No lights in composition.").small().color(colors::TEXT_MUTED));
            if crate::ui::custom_widgets::ae_button_accent(ui, "+ Add Light").clicked() {
                comp.lights.push(crate::core::timeline::Light3D::default());
                changed = true;
            }
        } else {
            for (li, light) in comp.lights.iter_mut().enumerate() {
                ui.collapsing(format!("{} {}", light.name, li + 1), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Name").small().color(colors::TEXT_SECONDARY));
                        if ui.add(egui::TextEdit::singleline(&mut light.name).desired_width(120.0)).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Type").small().color(colors::TEXT_SECONDARY));
                        let mut type_idx = match &light.light_type {
                            LightType::Ambient => 0,
                            LightType::Point => 1,
                            LightType::Spot { .. } => 2,
                            LightType::Parallel => 3,
                        };
                        egui::ComboBox::from_id_salt(format!("light_type_{}", li))
                            .selected_text(match &light.light_type {
                                LightType::Ambient => "Ambient",
                                LightType::Point => "Point",
                                LightType::Spot { .. } => "Spot",
                                LightType::Parallel => "Parallel",
                            })
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut type_idx, 0, "Ambient").clicked() {
                                    light.light_type = LightType::Ambient;
                                    changed = true;
                                }
                                if ui.selectable_value(&mut type_idx, 1, "Point").clicked() {
                                    light.light_type = LightType::Point;
                                    changed = true;
                                }
                                if ui.selectable_value(&mut type_idx, 2, "Spot").clicked() {
                                    light.light_type = LightType::Spot { cone_angle_deg: 90.0, cone_feather_pct: 50.0 };
                                    changed = true;
                                }
                                if ui.selectable_value(&mut type_idx, 3, "Parallel").clicked() {
                                    light.light_type = LightType::Parallel;
                                    changed = true;
                                }
                            });
                    });

                    if let LightType::Spot { ref mut cone_angle_deg, ref mut cone_feather_pct } = light.light_type {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Cone Angle").small().color(colors::TEXT_SECONDARY));
                            if ui.add(egui::Slider::new(cone_angle_deg, 0.0..=180.0).suffix("°")).changed() {
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Cone Feather").small().color(colors::TEXT_SECONDARY));
                            if ui.add(egui::Slider::new(cone_feather_pct, 0.0..=100.0).suffix("%")).changed() {
                                changed = true;
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Intensity").small().color(colors::TEXT_SECONDARY));
                        if ui.add(egui::Slider::new(&mut light.intensity, 0.0..=500.0).suffix("%")).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                        let c = &mut light.color;
                        let mut col = egui::Color32::from_rgba_premultiplied(
                            (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                            (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                        );
                        if ui.color_edit_button_srgba(&mut col).changed() {
                            let [r, g, b, a] = col.to_array();
                            *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                            changed = true;
                        }
                    });

                    if ui.checkbox(&mut light.casts_shadows, "Casts Shadows").clicked() {
                        changed = true;
                    }
                });
            }

            ui.add_space(4.0);
            if crate::ui::custom_widgets::ae_button(ui, "+ Add Light").clicked() {
                comp.lights.push(crate::core::timeline::Light3D::default());
                changed = true;
            }
        }
    });

    if changed {
        crate::core::frame_cache::bump_version();
    }
}
