use eframe::egui;
use crate::core::timeline::Composition;
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::theme::colors;

pub fn draw_camera_settings(
    ui: &mut egui::Ui,
    comp: &mut Composition,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    ui.collapsing("Active Camera Settings", |ui| {
        let cam = &mut comp.active_camera;
        ui.checkbox(&mut cam.active, "Camera Active");
        
        ui.horizontal(|ui| {
            ui.label("Field of View:");
            let fov_before = cam.fov_degrees;
            ui.add(egui::Slider::new(&mut cam.fov_degrees, 10.0..=120.0).suffix("°"));
            if fov_before != cam.fov_degrees { *project_changed = true; }
        });

        ui.horizontal(|ui| {
            ui.label("Focus Distance:");
            let fd_before = cam.focus_distance;
            ui.add(egui::DragValue::new(&mut cam.focus_distance).speed(10.0).suffix(" mm"));
            if fd_before != cam.focus_distance { *project_changed = true; }
        });

        ui.horizontal(|ui| {
            ui.label("Aperture:");
            let ap_before = cam.aperture;
            ui.add(egui::Slider::new(&mut cam.aperture, 0.95..=22.0).prefix("f/"));
            if ap_before != cam.aperture { *project_changed = true; }
        });

        // ── Depth of Field ──
        ui.separator();
        ui.label(egui::RichText::new("🎯 Depth of Field").small().strong().color(colors::ACCENT_CYAN));

        ui.horizontal(|ui| {
            ui.label("DOF Enabled:");
            if ui.checkbox(&mut cam.dof_enabled, "").changed() {
                *project_changed = true;
            }
        });

        if cam.dof_enabled {
            ui.horizontal(|ui| {
                ui.label("Max Blur Radius:");
                if ui.add(egui::Slider::new(&mut cam.dof_max_blur, 1.0..=64.0).suffix(" px")).changed() {
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
                if iris_before != cam.dof_iris_sides { *project_changed = true; }
            });
        }

        ui.label("Camera Transform:");
        let cam_pos_before = cam.transform.position.clone();
        if let Some(nf) = draw_property_ui(current_frame, ui, "  Pos", &mut cam.transform.position, |ui, val| {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X:"));
                ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z:"));
            });
        }) { *next_frame = Some(nf); }
        if cam_pos_before != cam.transform.position { *project_changed = true; }

        let cam_rot_before = cam.transform.rotation.clone();
        if let Some(nf) = draw_property_ui(current_frame, ui, "  Rot", &mut cam.transform.rotation, |ui, val| {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("P:"));
                ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y:"));
                ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("R:"));
            });
        }) { *next_frame = Some(nf); }
        if cam_rot_before != cam.transform.rotation { *project_changed = true; }
    });
}
