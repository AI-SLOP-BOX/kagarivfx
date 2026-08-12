use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_layer_styles(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Layer Styles (Photoshop Effects)");
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // --- Drop Shadow ---
        ui.collapsing("👤 Drop Shadow", |ui| {
            let mut blend_mode = 0;
            ui.horizontal(|ui| {
                ui.label("Blend Mode:");
                egui::ComboBox::from_id_source("ds_blend")
                    .selected_text("Multiply")
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut blend_mode, 0, "Multiply");
                        ui.selectable_value(&mut blend_mode, 1, "Normal");
                    });
            });

            let mut opacity: f32 = 75.0;
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).suffix("%"));
            });

            let mut angle: f32 = 120.0;
            ui.horizontal(|ui| {
                ui.label("Angle:");
                ui.add(egui::Slider::new(&mut angle, -180.0..=180.0).suffix("°"));
            });

            let mut distance: f32 = 5.0;
            ui.horizontal(|ui| {
                ui.label("Distance:");
                ui.add(egui::Slider::new(&mut distance, 0.0..=200.0).suffix(" px"));
            });

            let mut size: f32 = 5.0;
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut size, 0.0..=200.0).suffix(" px"));
            });
        });

        // --- Outer Glow ---
        ui.collapsing("🌟 Outer Glow", |ui| {
            let mut opacity: f32 = 75.0;
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).suffix("%"));
            });

            let mut spread: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Spread:");
                ui.add(egui::Slider::new(&mut spread, 0.0..=100.0).suffix("%"));
            });

            let mut size: f32 = 10.0;
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut size, 0.0..=200.0).suffix(" px"));
            });
        });

        // --- Bevel and Emboss ---
        ui.collapsing("⛰ Bevel and Emboss", |ui| {
            let mut depth: f32 = 100.0;
            ui.horizontal(|ui| {
                ui.label("Depth:");
                ui.add(egui::Slider::new(&mut depth, 1.0..=1000.0).suffix("%"));
            });

            let mut size: f32 = 5.0;
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut size, 0.0..=200.0).suffix(" px"));
            });
        });

        // --- Stroke ---
        ui.collapsing("✏ Stroke", |ui| {
            let mut size: f32 = 3.0;
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut size, 1.0..=250.0).suffix(" px"));
            });

            let mut position = 0;
            ui.horizontal(|ui| {
                ui.label("Position:");
                egui::ComboBox::from_id_source("stroke_pos")
                    .selected_text("Outside")
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut position, 0, "Outside");
                        ui.selectable_value(&mut position, 1, "Inside");
                        ui.selectable_value(&mut position, 2, "Center");
                    });
            });
        });
    });
}
