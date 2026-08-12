use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_metadata_panel(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("File & Project Metadata (XMP)");
    ui.separator();

    egui::Grid::new("metadata_grid").striped(true).show(ui, |ui| {
        ui.label(egui::RichText::new("Property").strong());
        ui.label(egui::RichText::new("Value").strong());
        ui.end_row();

        ui.label("Color Space");
        ui.label("Rec.709 (sRGB)");
        ui.end_row();

        ui.label("Color Depth");
        ui.label("32-bpc Float (High Dynamic Range)");
        ui.end_row();

        ui.label("Working Gamma");
        ui.label("Linearized Working Space (2.2)");
        ui.end_row();

        ui.label("Timecode Format");
        ui.label("29.97 Drop-Frame (00:00:00:00)");
        ui.end_row();

        ui.label("Audio Sample Rate");
        ui.label("48.000 kHz / 24-bit Stereo");
        ui.end_row();
    });

    ui.add_space(8.0);
    ui.separator();

    ui.label("XMP Schema Tags:");
    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
        ui.small("dc:creator = After Effects OSS Studio");
        ui.small("dc:format = video/mp4");
        ui.small("xmp:CreateDate = 2026-08-12T16:02:30Z");
    });
}
