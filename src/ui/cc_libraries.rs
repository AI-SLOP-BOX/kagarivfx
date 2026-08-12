use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_cc_libraries(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Creative Cloud Libraries");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Library:");
        egui::ComboBox::from_id_source("cc_lib_select")
            .selected_text("My Studio Assets (Cloud)")
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut 0, 0, "My Studio Assets (Cloud)");
                ui.selectable_value(&mut 0, 1, "Brand Motion Graphics");
                ui.selectable_value(&mut 0, 2, "Color Swatches & LUTs");
            });
    });

    ui.add_space(6.0);
    ui.text_edit_singleline(&mut "".to_string());
    ui.add_space(6.0);
    ui.separator();

    ui.label("Cloud Assets & Swatches:");
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        ui.collapsing("🎨 Color Swatches", |ui| {
            ui.horizontal(|ui| {
                let (r1, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                ui.painter().rect_filled(r1, 2.0, egui::Color32::from_rgb(20, 115, 230));
                let (r2, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                ui.painter().rect_filled(r2, 2.0, egui::Color32::from_rgb(230, 184, 0));
                let (r3, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                ui.painter().rect_filled(r3, 2.0, egui::Color32::from_rgb(230, 50, 50));
            });
        });

        ui.collapsing("🔤 Character Styles", |ui| {
            if ui.button("Header Bold (Roboto, 48pt)").clicked() {}
            if ui.button("Subhead Regular (Inter, 24pt)").clicked() {}
        });

        ui.collapsing("🖼 Graphics & Motion Templates", |ui| {
            if ui.button("📦 Lower Third Animated.mogrt").clicked() {}
            if ui.button("📦 Logo Reveal Glitch.mogrt").clicked() {}
        });
    });

    ui.add_space(8.0);
    ui.separator();
    if ui.button("➕ Add Selected Asset to CC Library").clicked() {
        log::info!("Added asset to Creative Cloud Library");
    }
}
