use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

/// Output-module presets bound to the real export pipeline
/// (`app.export_format_preset` + the shared codec selector).
pub fn draw_render_presets(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Render Queue Output Presets");
    ui.separator();

    // (format_preset, codec_idx, label)
    const PRESETS: [(usize, usize, &str); 4] = [
        (0, 0, "H.264 High Quality (MP4)"),
        (1, 1, "ProRes 422 HQ (MOV Master)"),
        (1, 2, "Lossless ProRes 4444 (MOV + Alpha)"),
        (2, 0, "PNG Image Sequence (RGBA)"),
    ];

    let current = PRESETS
        .iter()
        .position(|(f, c, _)| *f == app.export_format_preset && *c == app.export_codec_idx)
        .unwrap_or(1);
    let mut selected = current;
    let combo = egui::ComboBox::from_id_salt("render_preset_combo")
        .selected_text(PRESETS[selected].2)
        .show_ui(ui, |ui| {
            for (i, (f, c, label)) in PRESETS.iter().enumerate() {
                if ui.selectable_value(&mut selected, i, *label).changed() {
                    app.export_format_preset = *f;
                    app.export_codec_idx = *c;
                }
            }
        });
    combo
        .response
        .on_hover_text("Configures the Render Queue / Export dialog output module");

    ui.add_space(6.0);
    let target = app.history.current().active_composition().name.clone();
    if !app.render_queue_items.contains(&target)
        && custom_add_button(ui, "＋ Add Active Comp to Queue")
    {
        app.render_queue_items.push(target);
    }
    ui.weak(
        egui::RichText::new("Presets apply to the next render started from the Render Queue.")
            .small()
            .color(colors::TEXT_MUTED),
    );
}

fn custom_add_button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.button(label).clicked()
}
