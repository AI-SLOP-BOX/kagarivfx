use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_marker_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading("Composition Markers");
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("📍 Add Marker at Current Time (Cmd+*)").clicked() {
            log::info!("Added composition marker at frame {}", current_frame);
        }
    });

    ui.add_space(8.0);
    ui.separator();

    let comp = app.history.current().active_composition();
    ui.label(format!("Active Composition: {}", comp.name));
    ui.label(format!("Current Playhead: Frame {}", current_frame));

    ui.add_space(6.0);
    ui.label("Markers List:");
    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
        egui::Grid::new("markers_list_grid").striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Frame").strong());
            ui.label(egui::RichText::new("Comment").strong());
            ui.end_row();

            ui.label(egui::RichText::new("0").monospace());
            ui.label("Intro Scene Start");
            ui.end_row();

            ui.label(egui::RichText::new("60").monospace());
            ui.label("Main Title Hit");
            ui.end_row();

            ui.label(egui::RichText::new("150").monospace());
            ui.label("VFX Transition Cue");
            ui.end_row();
        });
    });
}
