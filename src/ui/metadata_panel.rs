use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_metadata_panel(app: &KagariApp, ui: &mut egui::Ui) {
    let comp = app.history.current().active_composition();

    crate::ui::custom_widgets::ae_section_header(ui, "Composition Info", "📋");

    egui::Grid::new("metadata_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Property")
                    .small()
                    .strong()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new("Value")
                    .small()
                    .strong()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Name")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(&comp.name)
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Resolution")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{} × {} px", comp.width, comp.height))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Frame Rate")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{} fps", comp.fps))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            let duration_sec = comp.duration_frames as f64 / comp.fps as f64;
            let dur_min = (duration_sec / 60.0) as u32;
            let dur_sec = duration_sec % 60.0;
            ui.label(
                egui::RichText::new("Duration")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} frames ({:02}:{:05.2})",
                    comp.duration_frames, dur_min, dur_sec
                ))
                .small()
                .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Pixel Aspect")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new("1.0 (Square)")
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();
        });

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Layer Stats", "📊");

    let total_layers = comp.layers.len();
    let visible_layers = comp.layers.iter().filter(|l| l.visible).count();
    let locked_layers = comp.layers.iter().filter(|l| l.locked).count();
    let total_effects: usize = comp.layers.iter().map(|l| l.effects.len()).sum();

    egui::Grid::new("layer_stats_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Total Layers")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", total_layers))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Visible")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", visible_layers))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Locked")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", locked_layers))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Effects")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", total_effects))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Lights")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", comp.lights.len()))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Sub-Comps")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}", comp.sub_compositions.len()))
                    .small()
                    .color(colors::TEXT_ACCENT),
            );
            ui.end_row();
        });

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Render Settings", "⚙");

    egui::Grid::new("render_settings_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Color Space")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let space_name = match app.color_space_idx {
                0 => "Rec.709 sRGB",
                1 => "ACEScg",
                2 => "ACES2065-1",
                3 => "Display P3",
                _ => "Rec.709 sRGB",
            };
            ui.label(
                egui::RichText::new(space_name)
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Bit Depth")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let depth_name = match app.bit_depth_idx {
                0 => "8-bpc",
                1 => "16-bpc",
                2 => "32-bpc Float",
                _ => "8-bpc",
            };
            ui.label(
                egui::RichText::new(depth_name)
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();

            ui.label(
                egui::RichText::new("Shutter Angle")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.label(
                egui::RichText::new(format!("{}°", comp.motion_blur_shutter_angle))
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.end_row();
        });

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Export Info", "📤");

    let format_name = match app.export_format_preset {
        0 => "H.264 MP4",
        1 => "ProRes 422",
        2 => "ProRes 4444",
        3 => "GIF",
        _ => "PNG Sequence",
    };
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Format")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.label(
            egui::RichText::new(format_name)
                .small()
                .color(colors::TEXT_PRIMARY),
        );
    });
}
