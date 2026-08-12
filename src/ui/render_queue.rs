use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_render_queue_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Render Queue");
    ui.separator();

    let comp = app.history.current().active_composition();

    ui.horizontal(|ui| {
        if ui.button("⚡ Render All Queue (Cmd+M)").on_hover_text("Start rendering active queue items").clicked() {
            app.show_export_dialog = true;
        }
        if ui.button("+ Add Active Comp").clicked() {
            log::info!("Added active composition {} to Render Queue", comp.name);
        }
        if ui.button("Clear Queue").clicked() {
            log::info!("Cleared Render Queue");
        }
    });

    ui.add_space(8.0);

    let frame = egui::Frame::none()
        .fill(egui::Color32::from_rgb(20, 24, 32))
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Item 1").strong().color(egui::Color32::from_rgb(0, 180, 255)));
            ui.label(format!("Comp: {}", comp.name));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Status: Queued").strong().color(egui::Color32::YELLOW));
            });
        });

        ui.separator();

        egui::Grid::new("render_queue_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Render Settings:");
            ui.label(egui::RichText::new("Best Quality / Full Resolution").color(egui::Color32::WHITE));
            ui.end_row();

            ui.label("Output Module:");
            ui.label(egui::RichText::new("H.264 High Bitrate (MP4)").color(egui::Color32::WHITE));
            ui.end_row();

            ui.label("Output To:");
            ui.label(egui::RichText::new(format!("./exports/{}.mp4", comp.name.to_lowercase().replace(' ', "_"))).color(egui::Color32::from_rgb(140, 200, 255)));
            ui.end_row();

            ui.label("Frame Range:");
            ui.label(format!("0 to {} ({} frames @ {} fps)", comp.duration_frames, comp.duration_frames, comp.fps));
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.add(egui::ProgressBar::new(0.0).text("Ready to Render"));
    });
}
