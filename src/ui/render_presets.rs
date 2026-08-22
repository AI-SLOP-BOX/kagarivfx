use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_render_presets(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Render Queue Output Presets");
    ui.separator();

    ui.label("Select Output Module Preset:");
    let preset_id = egui::Id::new("ae_render_preset_select");
    let mut preset_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(preset_id, || 0));

    egui::ComboBox::from_id_salt("render_preset_combo")
        .selected_text(match preset_idx {
            0 => "Lossless (Apple ProRes 4444 + Alpha)",
            1 => "H.264 High Quality (MP4 50 Mbps)",
            2 => "ProRes 422 HQ (Broadcast Master)",
            3 => "PNG Sequence with Alpha (RGBA)",
            _ => "Audio Only (WAV 48kHz 24-bit)",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut preset_idx, 0, "Lossless (Apple ProRes 4444 + Alpha)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_idx)); }
            if ui.selectable_value(&mut preset_idx, 1, "H.264 High Quality (MP4 50 Mbps)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_idx)); }
            if ui.selectable_value(&mut preset_idx, 2, "ProRes 422 HQ (Broadcast Master)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_idx)); }
            if ui.selectable_value(&mut preset_idx, 3, "PNG Sequence with Alpha (RGBA)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_idx)); }
        });

    ui.add_space(8.0);
    ui.separator();
    ui.label("Output Channels & Color:");

    let mut channel_idx: usize = 0;
    ui.horizontal(|ui| {
        ui.label("Channels:");
        ui.selectable_value(&mut channel_idx, 0, "RGB");
        ui.selectable_value(&mut channel_idx, 1, "RGBA (RGB + Alpha)");
        ui.selectable_value(&mut channel_idx, 2, "Alpha Only");
    });

    ui.add_space(6.0);
    let mut depth_idx: usize = 2;
    ui.horizontal(|ui| {
        ui.label("Depth:");
        ui.selectable_value(&mut depth_idx, 0, "Millions of Colors (8-bit)");
        ui.selectable_value(&mut depth_idx, 1, "Trillions of Colors (16-bit)");
        ui.selectable_value(&mut depth_idx, 2, "Floating Point (32-bit)");
    });

    ui.add_space(8.0);
    ui.separator();
    if ui.button("💾 Save as Custom Output Module Preset...").clicked() {
        log::info!("Saved custom render output preset");
    }
}
