use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::Composition;
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::theme::colors;
use eframe::egui;

fn remove_key_at<T>(track: &mut Option<crate::core::property::Animatable<T>>, frame: u32) {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        keys.retain(|key| key.frame != frame);
        if keys.is_empty() {
            *track = None;
        }
    }
}

pub fn draw_camera_settings(
    ui: &mut egui::Ui,
    comp: &mut Composition,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    ui.collapsing("Active Camera Settings", |ui| {
        let duration_frames = comp.duration_frames;
        let cam = comp.resolve_camera_mut();
        ui.checkbox(&mut cam.active, "Camera Active");

        ui.horizontal(|ui| {
            ui.label("Field of View:");
            let mut fov_value = cam.fov_at(current_frame);
            let changed = ui
                .add(egui::Slider::new(&mut fov_value, 10.0..=120.0).suffix("°"))
                .changed();
            let key = ui.button("◆").clicked();
            let remove = ui
                .button("×")
                .on_hover_text("Remove FOV keyframe at current frame")
                .clicked();
            if remove {
                remove_key_at(&mut cam.fov_animation, current_frame);
                *project_changed = true;
            } else if changed || key {
                cam.set_fov_at(current_frame, fov_value);
                *project_changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Focus Distance:");
            let mut focus_value = cam.focus_distance_at(current_frame);
            let changed = ui
                .add(
                    egui::DragValue::new(&mut focus_value)
                        .speed(10.0)
                        .suffix(" mm"),
                )
                .changed();
            let key = ui.button("◆").clicked();
            let remove = ui
                .button("×")
                .on_hover_text("Remove focus-distance keyframe at current frame")
                .clicked();
            if remove {
                remove_key_at(&mut cam.focus_distance_animation, current_frame);
                *project_changed = true;
            } else if changed || key {
                cam.set_focus_distance_at(current_frame, focus_value);
                *project_changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Aperture:");
            let mut aperture_value = cam.aperture_at(current_frame);
            let changed = ui
                .add(egui::Slider::new(&mut aperture_value, 0.95..=22.0).prefix("f/"))
                .changed();
            let key = ui.button("◆").clicked();
            let remove = ui
                .button("×")
                .on_hover_text("Remove aperture keyframe at current frame")
                .clicked();
            if remove {
                remove_key_at(&mut cam.aperture_animation, current_frame);
                *project_changed = true;
            } else if changed || key {
                cam.set_aperture_at(current_frame, aperture_value);
                *project_changed = true;
            }
        });

        let lens_tracks = [
            ("FOV", cam.fov_animation.as_ref()),
            ("Focus", cam.focus_distance_animation.as_ref()),
            ("Aperture", cam.aperture_animation.as_ref()),
            ("DOF Blur", cam.dof_max_blur_animation.as_ref()),
            ("DOF Enabled", cam.dof_enabled_animation.as_ref()),
        ];
        if lens_tracks.iter().any(|(_, track)| track.is_some()) {
            ui.label(egui::RichText::new("Lens Animation").small().strong());
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 68.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 3.0, egui::Color32::from_gray(28));
            for (row, (_, track)) in lens_tracks.iter().enumerate() {
                ui.painter().text(
                    egui::pos2(rect.left() + 4.0, rect.top() + row as f32 * 13.0 + 9.0),
                    egui::Align2::LEFT_CENTER,
                    lens_tracks[row].0,
                    egui::FontId::proportional(8.0),
                    colors::TEXT_MUTED,
                );
                if let Some(track) = track {
                    if let Some(keys) = track.keyframes() {
                        let min_frame = keys.first().map(|k| k.frame).unwrap_or(0);
                        let max_frame = keys
                            .last()
                            .map(|k| k.frame)
                            .unwrap_or(min_frame)
                            .max(min_frame + 1);
                        let min_value = keys.iter().map(|k| k.value).fold(f32::INFINITY, f32::min);
                        let max_value = keys
                            .iter()
                            .map(|k| k.value)
                            .fold(f32::NEG_INFINITY, f32::max);
                        let range = (max_value - min_value).max(0.001);
                        let points: Vec<_> = keys
                            .iter()
                            .map(|key| {
                                let x = rect.left()
                                    + ((key.frame - min_frame) as f32
                                        / (max_frame - min_frame) as f32)
                                        * rect.width();
                                let y = rect.top() + row as f32 * 13.0 + 11.0
                                    - ((key.value - min_value) / range) * 8.0;
                                egui::pos2(x, y)
                            })
                            .collect();
                        for pair in points.windows(2) {
                            ui.painter().line_segment(
                                [pair[0], pair[1]],
                                egui::Stroke::new(1.0, colors::ACCENT_CYAN),
                            );
                        }
                    }
                }
            }
            let end_frame = lens_tracks
                .iter()
                .filter_map(|(_, track)| {
                    track
                        .and_then(|t| t.keyframes())
                        .and_then(|keys| keys.last().map(|key| key.frame))
                })
                .max()
                .unwrap_or(1)
                .max(current_frame)
                .max(1);
            let x = rect.left() + (current_frame as f32 / end_frame as f32) * rect.width();
            ui.painter().line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, colors::HANDLE_HOVER_FILL),
            );
        }

        crate::ui::graph_editor::draw_camera_lens_graph(
            ui,
            cam,
            duration_frames,
            current_frame,
            project_changed,
        );

        // ── Depth of Field ──
        ui.separator();
        ui.label(
            egui::RichText::new("🎯 Depth of Field")
                .small()
                .strong()
                .color(colors::ACCENT_CYAN),
        );

        ui.horizontal(|ui| {
            ui.label("DOF Enabled:");
            let mut enabled = cam.dof_enabled_at(current_frame);
            if ui.checkbox(&mut enabled, "").changed() {
                cam.set_dof_enabled_at(current_frame, enabled);
                *project_changed = true;
            }
        });

        if cam.dof_enabled_at(current_frame) {
            ui.horizontal(|ui| {
                ui.label("Max Blur Radius:");
                let mut blur = cam.dof_max_blur_at(current_frame);
                let changed = ui
                    .add(egui::Slider::new(&mut blur, 1.0..=64.0).suffix(" px"))
                    .changed();
                let key = ui.button("◆").clicked();
                let remove = ui.button("×").clicked();
                if remove {
                    remove_key_at(&mut cam.dof_max_blur_animation, current_frame);
                    *project_changed = true;
                } else if changed || key {
                    cam.set_dof_max_blur_at(current_frame, blur);
                    *project_changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Iris Shape:");
                let iris_before = cam.dof_iris_sides;
                egui::ComboBox::from_id_salt("iris_shape")
                    .selected_text(match cam.dof_iris_sides {
                        0 => "Circle",
                        3 => "Triangle",
                        5 => "Pentagon",
                        6 => "Hexagon",
                        8 => "Octagon",
                        _ => "Circle",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut cam.dof_iris_sides, 0, "Circle");
                        ui.selectable_value(&mut cam.dof_iris_sides, 3, "Triangle");
                        ui.selectable_value(&mut cam.dof_iris_sides, 5, "Pentagon");
                        ui.selectable_value(&mut cam.dof_iris_sides, 6, "Hexagon");
                        ui.selectable_value(&mut cam.dof_iris_sides, 8, "Octagon");
                    });
                if iris_before != cam.dof_iris_sides {
                    *project_changed = true;
                }
            });
        }

        ui.label("Camera Transform:");
        let cam_pos_before = cam.transform.position.clone();
        if let Some(nf) = draw_property_ui(
            current_frame,
            ui,
            "  Pos",
            &mut cam.transform.position,
            |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X:"));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y:"));
                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z:"));
                });
            },
        ) {
            *next_frame = Some(nf);
        }
        if cam_pos_before != cam.transform.position {
            *project_changed = true;
        }

        let cam_rot_before = cam.transform.rotation.clone();
        if let Some(nf) = draw_property_ui(
            current_frame,
            ui,
            "  Rot",
            &mut cam.transform.rotation,
            |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("P:"));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y:"));
                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("R:"));
                });
            },
        ) {
            *next_frame = Some(nf);
        }
        if cam_rot_before != cam.transform.rotation {
            *project_changed = true;
        }
    });
}
