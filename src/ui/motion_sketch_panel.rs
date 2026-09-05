//! Motion Sketch Panel (AE Parity).
//!
//! Interactive live mouse path recording for real-time motion path capture.
//! When armed and dragging in viewport, records cursor coordinates into position keyframes
//! as time steps forward.

use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_motion_sketch_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("✏️ Motion Sketch");
    ui.label(
        egui::RichText::new("Draw motion paths in real-time by dragging in the viewport")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
    ui.separator();

    let smoothing_id = egui::Id::new("ae_motion_sketch_smoothing");
    let mut smoothing: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(smoothing_id, || 2.0));

    let capture_speed_id = egui::Id::new("ae_motion_sketch_speed");
    let mut speed_pct: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(capture_speed_id, || 100.0));

    ui.horizontal(|ui| {
        ui.label("Smoothing:");
        if ui
            .add(egui::Slider::new(&mut smoothing, 0.0..=20.0).suffix(" px"))
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(smoothing_id, smoothing));
        }
    });

    ui.horizontal(|ui| {
        ui.label("Capture Speed:");
        if ui
            .add(egui::Slider::new(&mut speed_pct, 25.0..=200.0).suffix(" %"))
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(capture_speed_id, speed_pct));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    let is_sketching_id = egui::Id::new("ae_motion_sketch_armed");
    let is_armed: bool = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(is_sketching_id, || false));

    if is_armed {
        ui.colored_label(
            colors::ACCENT_RED,
            "🔴 ARMED: Click and drag in Viewport to record motion path...",
        );
        if custom_widgets::ae_button(ui, "⏹ Stop / Disarm").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(is_sketching_id, false));
            app.toasts.info("Motion Sketch disarmed");
        }
    } else {
        if custom_widgets::ae_button(ui, "🔴 Start Capture")
            .on_hover_text("Arm motion sketch. Drag in viewport to record.")
            .clicked()
        {
            if app.selection.selected_layer_idx.is_some() {
                ui.ctx().data_mut(|d| d.insert_temp(is_sketching_id, true));
                app.toasts
                    .info("Motion Sketch armed: Drag in Viewport to record!");
            } else {
                app.toasts.error("Select a target layer first");
            }
        }
    }
}
