use eframe::egui;
use crate::core::timeline::Composition;
use crate::ui::inspector::draw_property_ui;

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
