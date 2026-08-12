use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_font_picker(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Typography & Faux Font Switches");
    ui.separator();

    ui.label("Font Family & Weight Variant:");
    let font_id = egui::Id::new("ae_font_family_select");
    let mut font_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(font_id, || 0));

    egui::ComboBox::from_id_source("font_family_combo")
        .selected_text(match font_idx {
            0 => "Inter - Regular",
            1 => "Inter - Bold",
            2 => "Roboto - Black",
            3 => "Helvetica Neue - Light",
            _ => "Courier New - Monospace",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut font_idx, 0, "Inter - Regular").clicked() { ui.ctx().data_mut(|d| d.insert_temp(font_id, font_idx)); }
            if ui.selectable_value(&mut font_idx, 1, "Inter - Bold").clicked() { ui.ctx().data_mut(|d| d.insert_temp(font_id, font_idx)); }
            if ui.selectable_value(&mut font_idx, 2, "Roboto - Black").clicked() { ui.ctx().data_mut(|d| d.insert_temp(font_id, font_idx)); }
            if ui.selectable_value(&mut font_idx, 3, "Helvetica Neue - Light").clicked() { ui.ctx().data_mut(|d| d.insert_temp(font_id, font_idx)); }
        });

    ui.add_space(8.0);
    ui.separator();
    ui.label("AE Faux Font Switches:");

    let faux_bold_id = egui::Id::new("ae_faux_bold");
    let mut faux_bold = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(faux_bold_id, || false));
    let faux_italic_id = egui::Id::new("ae_faux_italic");
    let mut faux_italic = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(faux_italic_id, || false));
    let all_caps_id = egui::Id::new("ae_all_caps");
    let mut all_caps = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(all_caps_id, || false));
    let small_caps_id = egui::Id::new("ae_small_caps");
    let mut small_caps = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(small_caps_id, || false));

    ui.horizontal(|ui| {
        if ui.checkbox(&mut faux_bold, "B (Faux Bold)").changed() { ui.ctx().data_mut(|d| d.insert_temp(faux_bold_id, faux_bold)); }
        if ui.checkbox(&mut faux_italic, "I (Faux Italic)").changed() { ui.ctx().data_mut(|d| d.insert_temp(faux_italic_id, faux_italic)); }
        if ui.checkbox(&mut all_caps, "TT (All Caps)").changed() { ui.ctx().data_mut(|d| d.insert_temp(all_caps_id, all_caps)); }
        if ui.checkbox(&mut small_caps, "Tt (Small Caps)").changed() { ui.ctx().data_mut(|d| d.insert_temp(small_caps_id, small_caps)); }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("Text Canvas Preview:");
    let sample = if all_caps { "AFTER EFFECTS STUDIO" } else { "After Effects Studio" };
    ui.add(egui::Label::new(egui::RichText::new(sample).size(22.0)));
}
