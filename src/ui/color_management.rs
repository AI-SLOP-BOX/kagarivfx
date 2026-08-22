use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_color_management(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Color Management (OCIO & ACES)");
    ui.separator();

    ui.label("Working Color Space:");
    let space_id = egui::Id::new("ae_color_space_combo");
    let mut space_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(space_id, || 0));
    egui::ComboBox::from_id_salt("color_space_combo")
        .selected_text(match space_idx {
            0 => "Rec.709 Gamma 2.4 (sRGB)",
            1 => "ACEScg (AP1 Linear)",
            2 => "ACES2065-1 (AP0 Linear)",
            3 => "Display P3",
            _ => "Apple RGB",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut space_idx, 0, "Rec.709 Gamma 2.4 (sRGB)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(space_id, space_idx)); }
            if ui.selectable_value(&mut space_idx, 1, "ACEScg (AP1 Linear)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(space_id, space_idx)); }
            if ui.selectable_value(&mut space_idx, 2, "ACES2065-1 (AP0 Linear)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(space_id, space_idx)); }
            if ui.selectable_value(&mut space_idx, 3, "Display P3").clicked() { ui.ctx().data_mut(|d| d.insert_temp(space_id, space_idx)); }
        });

    ui.add_space(6.0);
    ui.label("Project Bit Depth:");
    let depth_id = egui::Id::new("ae_color_bit_depth");
    let mut depth_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(depth_id, || 2));
    ui.horizontal(|ui| {
        if ui.selectable_value(&mut depth_idx, 0, "8-bpc").clicked() { ui.ctx().data_mut(|d| d.insert_temp(depth_id, depth_idx)); }
        if ui.selectable_value(&mut depth_idx, 1, "16-bpc").clicked() { ui.ctx().data_mut(|d| d.insert_temp(depth_id, depth_idx)); }
        if ui.selectable_value(&mut depth_idx, 2, "32-bpc (Float)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(depth_id, depth_idx)); }
    });

    ui.add_space(6.0);
    let linear_id = egui::Id::new("ae_linearize_working_space");
    let mut linearize = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(linear_id, || true));
    if ui.checkbox(&mut linearize, "Linearize Working Space").changed() {
        ui.ctx().data_mut(|d| d.insert_temp(linear_id, linearize));
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label("Display Simulation:");
    let disp_id = egui::Id::new("ae_display_sim");
    let mut disp_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(disp_id, || 0));
    egui::ComboBox::from_id_salt("display_sim_combo")
        .selected_text(match disp_idx {
            0 => "Macintosh sRGB",
            1 => "Rec.709 HDTV (Video Studio)",
            _ => "DCI-P3 Cinema",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut disp_idx, 0, "Macintosh sRGB").clicked() { ui.ctx().data_mut(|d| d.insert_temp(disp_id, disp_idx)); }
            if ui.selectable_value(&mut disp_idx, 1, "Rec.709 HDTV (Video Studio)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(disp_id, disp_idx)); }
            if ui.selectable_value(&mut disp_idx, 2, "DCI-P3 Cinema").clicked() { ui.ctx().data_mut(|d| d.insert_temp(disp_id, disp_idx)); }
        });
}
