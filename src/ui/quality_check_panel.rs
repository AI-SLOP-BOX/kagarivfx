use crate::core::editor_assist::{analyze_exposure_clipping, sharpness_score};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

#[derive(Debug, Clone)]
pub struct QualityCheckResult {
    pub frame: u32,
    pub shadow_clipping: f32,
    pub highlight_clipping: f32,
    pub sharpness: f32,
    pub width: u32,
    pub height: u32,
}

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_quality_check_panel {
        return;
    }
    let mut open = app.show_quality_check_panel;
    egui::Window::new("📊 Analyze / Quality Check")
        .open(&mut open)
        .default_width(380.0)
        .default_height(440.0)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 48.0))
        .show(ctx, |ui| {
            draw_header(ui);
            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new("▶ Analyze Current Frame"),
                )
                .on_hover_text("Analyze the currently cached preview frame")
                .clicked()
            {
                analyze_current_frame(app);
            }
            ui.add_space(8.0);
            if let Some(result) = app.quality_check_result.clone() {
                draw_result(app, ui, &result);
            } else {
                ui.label(
                    egui::RichText::new("Preview a frame, then run Analyze.")
                        .color(colors::TEXT_SECONDARY),
                );
                ui.label(
                    egui::RichText::new(
                        "The check uses the rendered preview cache and does not re-render while idle.",
                    )
                    .small()
                    .color(colors::TEXT_MUTED),
                );
            }
        });
    app.show_quality_check_panel = open;
}

fn draw_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("Frame QC");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("NON-DESTRUCTIVE")
                    .small()
                    .strong()
                    .color(colors::ACCENT_BLUE),
            );
        });
    });
    ui.label(
        egui::RichText::new("Exposure clipping and focus diagnostics")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
}

fn analyze_current_frame(app: &mut KagariApp) {
    let frame = app.playback.current_frame;
    let layer_indices = {
        let comp = app.history.current().active_composition();
        comp.layers
            .iter()
            .enumerate()
            .filter(|(_, layer)| layer.is_active(frame))
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };
    let Some(entry) = app.frame_cache.get_with_layers(frame, &layer_indices) else {
        app.toasts
            .warning("Current frame is not cached yet — preview it first");
        return;
    };
    let Some(exposure) =
        analyze_exposure_clipping(&entry.pixels, entry.width, entry.height, 4, 250)
    else {
        app.toasts.error("Could not analyze the cached frame");
        return;
    };
    let sharpness = sharpness_score(&entry.pixels, entry.width, entry.height).unwrap_or(0.0);
    app.quality_check_result = Some(QualityCheckResult {
        frame,
        shadow_clipping: exposure.shadow_clipped_fraction,
        highlight_clipping: exposure.highlight_clipped_fraction,
        sharpness,
        width: entry.width,
        height: entry.height,
    });
    app.toasts
        .info(format!("Frame {frame} quality check complete"));
}

fn draw_result(app: &mut KagariApp, ui: &mut egui::Ui, result: &QualityCheckResult) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("FRAME {}", result.frame))
                .monospace()
                .strong()
                .color(colors::ACCENT_YELLOW),
        );
        ui.label(
            egui::RichText::new(format!("{} × {}", result.width, result.height))
                .small()
                .color(colors::TEXT_MUTED),
        );
        if ui.small_button("Go to frame").clicked() {
            app.playback.current_frame = result.frame;
        }
    });
    ui.separator();
    metric_row(ui, "Shadow clipping", result.shadow_clipping, 0.02, true);
    metric_row(
        ui,
        "Highlight clipping",
        result.highlight_clipping,
        0.02,
        true,
    );
    ui.add_space(8.0);
    let focus_color = if result.sharpness < 20.0 {
        colors::ACCENT_ORANGE
    } else {
        colors::ACCENT_BLUE
    };
    ui.horizontal(|ui| {
        ui.label("Sharpness");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{:.1}", result.sharpness))
                    .monospace()
                    .strong()
                    .color(focus_color),
            );
        });
    });
    if result.sharpness < 20.0 {
        ui.label(
            egui::RichText::new("⚠ Possible soft-focus frame")
                .small()
                .color(colors::ACCENT_ORANGE),
        );
    }
}

fn metric_row(ui: &mut egui::Ui, label: &str, fraction: f32, warning: f32, lower_is_better: bool) {
    let warned = if lower_is_better {
        fraction > warning
    } else {
        fraction < warning
    };
    let color = if warned {
        colors::ACCENT_ORANGE
    } else {
        colors::ACCENT_BLUE
    };
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{:.2}%", fraction * 100.0))
                    .monospace()
                    .strong()
                    .color(color),
            );
        });
    });
    ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).desired_width(ui.available_width()));
    if warned {
        ui.label(
            egui::RichText::new("⚠ Above recommended 2% threshold")
                .small()
                .color(colors::ACCENT_ORANGE),
        );
    }
    ui.add_space(5.0);
}
