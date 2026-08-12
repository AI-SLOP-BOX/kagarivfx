use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_tracker_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading("Tracker");
    ui.separator();

    let comp = app.history.current().active_composition();
    let sel_idx = app.selected_layer_idx;

    if let Some(idx) = sel_idx {
        if idx < comp.layers.len() {
            let layer_name = comp.layers[idx].name.clone();
            ui.label(format!("Motion Source: {}", layer_name));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Track Motion").on_hover_text("Track 2D Feature Points").clicked() {
                    log::info!("Started 2D Motion Tracking on layer {}", layer_name);
                }
                if ui.button("Stabilize Motion").on_hover_text("Stabilize position/rotation").clicked() {
                    log::info!("Started Motion Stabilization on layer {}", layer_name);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Track Camera").on_hover_text("3D Camera Tracker analysis").clicked() {
                    log::info!("Started 3D Camera Tracker analysis on layer {}", layer_name);
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Analysis Controls:");

            ui.horizontal(|ui| {
                if ui.button("◀◀").on_hover_text("Analyze 1 Frame Backward").clicked() {
                    log::info!("Analyzed frame backward from {}", current_frame);
                }
                if ui.button("◀ Analyze").on_hover_text("Analyze Backward").clicked() {
                    log::info!("Analyzing backward...");
                }
                if ui.button("Analyze ▶").on_hover_text("Analyze Forward (Alt+L)").clicked() {
                    log::info!("Analyzing forward...");
                }
                if ui.button("▶▶").on_hover_text("Analyze 1 Frame Forward").clicked() {
                    log::info!("Analyzed frame forward from {}", current_frame);
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Reset Track").clicked() {
                    log::info!("Reset tracker points on layer {}", layer_name);
                }
                if ui.button("Apply Motion").on_hover_text("Apply tracking data to target layer").clicked() {
                    log::info!("Applied motion tracking data");
                }
            });
        } else {
            ui.weak("Select a layer to perform motion tracking.");
        }
    } else {
        ui.weak("No layer selected. Select a layer in timeline.");
    }
}
