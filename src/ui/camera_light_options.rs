use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_camera_light_options(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("3D Camera & 3D Light Properties");
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // --- 1. 3D Camera Options ---
        ui.collapsing("📷 3D Camera Options", |ui| {
            let mut zoom: f32 = 1777.7;
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut zoom, 100.0..=10000.0).suffix(" px"));
            });

            let mut dof: bool = true;
            ui.checkbox(&mut dof, "Depth of Field (DOF)");

            let mut focus_dist: f32 = 1777.7;
            ui.horizontal(|ui| {
                ui.label("Focus Distance:");
                ui.add(egui::Slider::new(&mut focus_dist, 0.0..=10000.0).suffix(" px"));
            });

            let mut aperture: f32 = 15.0;
            ui.horizontal(|ui| {
                ui.label("Aperture:");
                ui.add(egui::Slider::new(&mut aperture, 0.0..=500.0).suffix(" px"));
            });

            let mut blur_level: f32 = 100.0;
            ui.horizontal(|ui| {
                ui.label("Blur Level:");
                ui.add(egui::Slider::new(&mut blur_level, 0.0..=500.0).suffix("%"));
            });
        });

        // --- 2. 3D Light Options ---
        ui.collapsing("💡 3D Light Options", |ui| {
            ui.label("Light Type:");
            let mut light_type = 0;
            egui::ComboBox::from_id_source("light_type_combo")
                .selected_text(match light_type {
                    0 => "Point Light",
                    1 => "Spot Light",
                    2 => "Parallel Light",
                    _ => "Ambient Light",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut light_type, 0, "Point Light");
                    ui.selectable_value(&mut light_type, 1, "Spot Light");
                    ui.selectable_value(&mut light_type, 2, "Parallel Light");
                    ui.selectable_value(&mut light_type, 3, "Ambient Light");
                });

            let mut intensity: f32 = 100.0;
            ui.horizontal(|ui| {
                ui.label("Intensity:");
                ui.add(egui::Slider::new(&mut intensity, 0.0..=500.0).suffix("%"));
            });

            let mut cone_angle: f32 = 90.0;
            ui.horizontal(|ui| {
                ui.label("Cone Angle:");
                ui.add(egui::Slider::new(&mut cone_angle, 0.0..=180.0).suffix("°"));
            });

            let mut casts_shadows: bool = true;
            ui.checkbox(&mut casts_shadows, "Casts Shadows");

            let mut shadow_darkness: f32 = 100.0;
            ui.horizontal(|ui| {
                ui.label("Shadow Darkness:");
                ui.add(egui::Slider::new(&mut shadow_darkness, 0.0..=100.0).suffix("%"));
            });
        });
    });
}
