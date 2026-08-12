use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_audio_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Audio Levels & Panning");
    ui.separator();

    let master_vol = app.master_volume;
    ui.horizontal(|ui| {
        ui.label("Master Audio Level:");
        ui.add(egui::Slider::new(&mut app.master_volume, 0.0..=2.0).suffix(" x"));
    });
    let db_val = if master_vol > 0.001 { 20.0 * master_vol.log10() } else { -60.0 };
    ui.small(format!("Master Level: {:.1} dB", db_val));

    ui.add_space(8.0);
    ui.separator();

    let comp = app.history.current().active_composition();
    if let Some(idx) = app.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer_name = &comp.layers[idx].name;
            ui.label(format!("Selected Layer: {}", layer_name));

            let pan_id = egui::Id::new(format!("ae_audio_pan_{}", idx));
            let mut pan: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(pan_id, || 0.0));
            ui.horizontal(|ui| {
                ui.label("L/R Pan:");
                if ui.add(egui::Slider::new(&mut pan, -100.0..=100.0).suffix(" %")).changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(pan_id, pan));
                }
            });

            ui.add_space(6.0);
            let mut waveform_on = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_show_waveform"), || true));
            if ui.checkbox(&mut waveform_on, "Show Layer Audio Waveform (L)").changed() {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_show_waveform"), waveform_on));
            }
        } else {
            ui.weak("Select a layer to adjust audio pan and waveform settings.");
        }
    } else {
        ui.weak("No layer selected.");
    }
}
