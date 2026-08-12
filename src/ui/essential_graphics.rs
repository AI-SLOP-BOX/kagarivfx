use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_essential_graphics(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Essential Graphics (MOGRT Creator)");
    ui.separator();

    let comp = app.history.current().active_composition();
    ui.label(format!("Master Composition: {}", comp.name));

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("📦 Export Motion Graphics Template (.mogrt)").on_hover_text("Export template for Premiere Pro").clicked() {
            log::info!("Exported MOGRT template for composition {}", comp.name);
        }
    });

    ui.add_space(8.0);
    ui.separator();

    ui.label("Exposed Controllers & Parameters:");
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        egui::Grid::new("mogrt_params_grid").striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Control Name").strong());
            ui.label(egui::RichText::new("Property Type").strong());
            ui.end_row();

            ui.label("Title Text");
            ui.label("Text String");
            ui.end_row();

            ui.label("Background Color");
            ui.label("Color Picker");
            ui.end_row();

            ui.label("Logo Opacity");
            ui.label("Slider (0 - 100%)");
            ui.end_row();
        });
    });

    ui.add_space(6.0);
    if ui.button("➕ Drag & Drop Property to Expose").clicked() {
        log::info!("Drag property into Essential Graphics panel");
    }
}
