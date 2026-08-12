use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_content_aware_fill(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Content-Aware Fill");
    ui.separator();

    ui.label("Fill Method:");
    let method_id = egui::Id::new("ae_caf_fill_method");
    let mut method_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(method_id, || 0));
    egui::ComboBox::from_id_source("caf_method_combo")
        .selected_text(match method_idx {
            0 => "Object (Motion Objects Removal)",
            1 => "Surface (Flat Texture Fill)",
            _ => "Edge Blend (Smooth Gradient)",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut method_idx, 0, "Object (Motion Objects Removal)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx)); }
            if ui.selectable_value(&mut method_idx, 1, "Surface (Flat Texture Fill)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx)); }
            if ui.selectable_value(&mut method_idx, 2, "Edge Blend (Smooth Gradient)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx)); }
        });

    ui.add_space(6.0);
    let alpha_exp_id = egui::Id::new("ae_caf_alpha_expansion");
    let mut alpha_exp: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(alpha_exp_id, || 5.0));
    ui.horizontal(|ui| {
        ui.label("Alpha Expansion:");
        if ui.add(egui::Slider::new(&mut alpha_exp, 0.0..=50.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(alpha_exp_id, alpha_exp));
        }
    });

    ui.add_space(6.0);
    ui.label("Range:");
    let range_id = egui::Id::new("ae_caf_range");
    let mut range_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(range_id, || 0));
    ui.horizontal(|ui| {
        if ui.selectable_value(&mut range_idx, 0, "Work Area").clicked() { ui.ctx().data_mut(|d| d.insert_temp(range_id, range_idx)); }
        if ui.selectable_value(&mut range_idx, 1, "Entire Duration").clicked() { ui.ctx().data_mut(|d| d.insert_temp(range_id, range_idx)); }
    });

    ui.add_space(10.0);
    ui.separator();

    if ui.button("⚡ Generate Fill Layer").on_hover_text("Synthesize Content-Aware Fill PNG sequence").clicked() {
        log::info!("Started Content-Aware Fill synthesis...");
    }
}
