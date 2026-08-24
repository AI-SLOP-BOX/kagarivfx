use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_font_picker(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Typography & Faux Font Switches");
    ui.separator();

    // Real families available to the rasterizer
    let families = crate::core::font_rasterizer::with_font_rasterizer(|r| r.available_families());
    if families.is_empty() {
        ui.weak("No system fonts detected.");
        return;
    }

    // Current family = selected text layer's formatting, else first available
    let current_family = app
        .selected_layer_idx
        .and_then(|idx| {
            let comp = app.history.current().active_composition();
            comp.layers.get(idx).and_then(|l| {
                l.text_formatting
                    .as_ref()
                    .map(|tf| tf.font_family.clone())
            })
        })
        .unwrap_or_else(|| families[0].clone());

    let mut selected = families.iter().position(|f| *f == current_family).unwrap_or(0);
    ui.label("Font Family:");
    egui::ComboBox::from_id_salt("font_family_combo")
        .selected_text(families.get(selected).map(|s| s.as_str()).unwrap_or("?"))
        .width(ui.available_width() - 12.0)
        .show_ui(ui, |ui| {
            for (i, fam) in families.iter().enumerate() {
                ui.selectable_value(&mut selected, i, fam);
            }
        });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Apply to Selected Text").on_hover_text("Writes the family into the selected text layer's formatting").clicked() {
            if let Some(idx) = app.selected_layer_idx {
                let fam = families[selected].clone();
                {
                    let comp = app.history.current_mut().active_composition_mut();
                    if let Some(layer) = comp.layers.get_mut(idx) {
                        layer
                            .text_formatting
                            .get_or_insert_with(crate::core::timeline::TextFormatting::default)
                            .font_family = fam.clone();
                    }
                }
                crate::core::frame_cache::bump_version();
                app.toasts.info(format!("Font set to {}", fam));
            } else {
                app.toasts.error("Select a text layer first");
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("AE Faux Font Switches:");

    let (ref mut faux_bold, ref mut faux_italic, ref mut all_caps, ref mut small_caps) =
        app.faux_font_switches;

    ui.horizontal(|ui| {
        ui.checkbox(faux_bold, "B (Faux Bold)");
        ui.checkbox(faux_italic, "I (Faux Italic)");
        ui.checkbox(all_caps, "TT (All Caps)");
        ui.checkbox(small_caps, "Tt (Small Caps)");
    });
    ui.weak(
        egui::RichText::new("Preview only — faux styles are not baked into renders yet.")
            .small()
            .color(colors::TEXT_MUTED),
    );

    ui.add_space(8.0);
    ui.separator();
    ui.label("Preview:");
    let base = families.get(selected).cloned().unwrap_or_default();
    let sample = if app.faux_font_switches.2 { "AFTER EFFECTS STUDIO" } else { "After Effects Studio" };
    ui.add(
        egui::Label::new(
            egui::RichText::new(sample)
                .size(22.0)
                .color(colors::TEXT_PRIMARY),
        ),
    );
    ui.monospace(format!("family: {} / weight: {}", base,
        if app.faux_font_switches.0 { "Bold" } else { "Regular" }));
}
