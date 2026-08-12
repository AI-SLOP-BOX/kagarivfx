use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_time_remap_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Time Stretch & Time Remapping");
    ui.separator();

    let comp = app.history.current().active_composition();
    if let Some(idx) = app.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer_name = &comp.layers[idx].name;
            ui.label(format!("Selected Layer: {}", layer_name));

            ui.add_space(4.0);
            if ui.button("⏱ Enable Time Remapping (Cmd+Alt+T)").on_hover_text("Adds Time Remap keyframe track").clicked() {
                log::info!("Enabled Time Remapping on layer {}", layer_name);
            }

            ui.add_space(8.0);
            ui.separator();

            let stretch_id = egui::Id::new(format!("ae_time_stretch_{}", idx));
            let mut stretch_factor: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(stretch_id, || 100.0));
            ui.horizontal(|ui| {
                ui.label("Stretch Factor:");
                if ui.add(egui::DragValue::new(&mut stretch_factor).clamp_range(1.0..=1000.0).suffix(" %")).changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(stretch_id, stretch_factor));
                }
            });

            ui.add_space(6.0);
            ui.label("Frame Blending Mode:");
            let blend_id = egui::Id::new("ae_frame_blending_mode");
            let mut blend_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(blend_id, || 0));

            ui.horizontal(|ui| {
                if ui.selectable_value(&mut blend_idx, 0, "Off").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
                }
                if ui.selectable_value(&mut blend_idx, 1, "Frame Mix").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
                }
                if ui.selectable_value(&mut blend_idx, 2, "Pixel Motion").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
                }
            });
        } else {
            ui.weak("Select a layer to adjust time stretch & remapping.");
        }
    } else {
        ui.weak("No layer selected.");
    }
}
