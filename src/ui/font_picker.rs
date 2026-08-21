use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_font_picker(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Typography & Faux Font Switches");
    ui.separator();

    ui.label("Font Family & Weight Variant:");
    egui::ComboBox::from_id_source("font_family_combo")
        .selected_text(match app.font_family_idx {
            0 => "Inter - Regular",
            1 => "Inter - Bold",
            2 => "Roboto - Black",
            3 => "Helvetica Neue - Light",
            _ => "Courier New - Monospace",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.font_family_idx, 0, "Inter - Regular");
            ui.selectable_value(&mut app.font_family_idx, 1, "Inter - Bold");
            ui.selectable_value(&mut app.font_family_idx, 2, "Roboto - Black");
            ui.selectable_value(&mut app.font_family_idx, 3, "Helvetica Neue - Light");
            ui.selectable_value(&mut app.font_family_idx, 4, "Courier New - Monospace");
        });

    ui.add_space(8.0);
    ui.separator();
    ui.label("AE Faux Font Switches:");

    let (ref mut faux_bold, ref mut faux_italic, ref mut all_caps, ref mut small_caps)
        = app.faux_font_switches;

    ui.horizontal(|ui| {
        ui.checkbox(faux_bold, "B (Faux Bold)");
        ui.checkbox(faux_italic, "I (Faux Italic)");
        ui.checkbox(all_caps, "TT (All Caps)");
        ui.checkbox(small_caps, "Tt (Small Caps)");
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("Text Canvas Preview:");
    let sample = if app.faux_font_switches.2 { "AFTER EFFECTS STUDIO" } else { "After Effects Studio" };
    ui.add(egui::Label::new(egui::RichText::new(sample).size(22.0)));
}
