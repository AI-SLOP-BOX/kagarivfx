use eframe::egui;
use std::sync::Mutex;
use std::time::Instant;

use crate::AfterEffectsApp;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

const START_TIME_ID: &str = "render_queue_export_start";
const CANCEL_CONFIRM_ID: &str = "render_queue_cancel_confirm";

fn preset_name(preset: usize) -> &'static str {
    match preset {
        1 => "Apple ProRes 422 HQ (MOV)",
        2 => "PNG Image Sequence",
        _ => "H.264 High Bitrate (MP4)",
    }
}

fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs <= 0.0 {
        return "--:--".to_string();
    }
    let total = secs.round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

pub fn draw_render_queue_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Render Queue");
    ui.separator();

    let comp_name = app.history.current().active_composition().name.clone();

    ui.horizontal(|ui| {
        if custom_widgets::ae_button_accent(ui, "⚡ Render All Queue (Cmd+M)")
            .on_hover_text("Sequentially export every queued composition to video")
            .clicked()
        {
            if app.render_queue_items.is_empty() {
                app.render_queue_items.push(comp_name.clone());
            }
            // Sequential background batch: each item runs the real FFmpeg
            // export; the next starts automatically when the previous finishes.
            app.batch_queue = app.render_queue_items.clone();
            app.batch_idx = 0;
            app.show_export_dialog = true;
            let first = app.batch_queue[0].clone();
            crate::ui::export_dialog::start_comp_export(app, ui.ctx(), &first);
        }
        if custom_widgets::ae_button(ui, "+ Add Active Comp")
            .on_hover_text("Add the active composition to the render queue")
            .clicked()
            && !app.render_queue_items.contains(&comp_name)
        {
            app.render_queue_items.push(comp_name.clone());
        }
        if custom_widgets::ae_button(ui, "Clear Queue")
            .on_hover_text("Remove all items from the queue")
            .clicked()
            && !app.is_exporting
        {
            app.render_queue_items.clear();
        }
    });

    ui.add_space(8.0);

    // Frame range derived from the work area when one is set.
    let comp_duration = app.history.current().active_composition().duration_frames;
    let wa_in = app.work_area_in.unwrap_or(0);
    let wa_out = app
        .work_area_out
        .unwrap_or(comp_duration.saturating_sub(1))
        .min(comp_duration.saturating_sub(1));
    let range_frames = wa_out.saturating_sub(wa_in).saturating_add(1);

    // Editable output path persisted across frames via egui's temp store.
    let path_id = egui::Id::new("render_queue_output_path");

    let frame = egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0, colors::BORDER_SUBTLE));

    frame.show(ui, |ui| {
        let (status_text, status_color) = if app.is_exporting {
            ("Rendering", colors::ACCENT_GREEN)
        } else {
            ("Queued", egui::Color32::YELLOW)
        };

        ui.horizontal(|ui| {
            let queue_pos = app
                .render_queue_items
                .iter()
                .position(|n| n == &comp_name)
                .map(|p| p + 1)
                .unwrap_or(app.render_queue_items.len().max(1));
            ui.label(
                egui::RichText::new(format!("Item {}", queue_pos))
                    .strong()
                    .color(colors::TEXT_ACCENT),
            );
            ui.label(format!("Comp: {}", comp_name));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("Status: {}", status_text)).strong().color(status_color));
            });
        });

        ui.separator();

        egui::Grid::new("render_queue_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Render Settings:");
                ui.label(
                    egui::RichText::new("Best Quality / Full Resolution").color(egui::Color32::WHITE),
                );
                ui.end_row();

                ui.label("Output Module:");
                ui.label(
                    egui::RichText::new(preset_name(app.export_format_preset))
                        .color(egui::Color32::WHITE),
                );
                ui.end_row();

                ui.label("Output To:");
                {
                    let mut edit = ui.ctx().data_mut(|d| {
                        d.get_temp_mut_or_insert_with(path_id, || {
                            std::sync::Arc::new(Mutex::new(app.export_output_path.clone()))
                        })
                        .lock()
                        .map(|g| g.clone())
                        .unwrap_or_default()
                    });
                    let resp = ui.add_sized(
                        [ui.available_width().max(120.0), 18.0],
                        egui::TextEdit::singleline(&mut edit),
                    );
                    if resp.changed() || resp.lost_focus() {
                        ui.ctx().data_mut(|d| {
                            let arc = d.get_temp_mut_or_insert_with(path_id, || {
                                std::sync::Arc::new(Mutex::new(edit.clone()))
                            });
                            if let Ok(mut g) = arc.lock() {
                                *g = edit.clone();
                            }
                        });
                        app.export_output_path = edit;
                    }
                }
                ui.end_row();

                ui.label("Frame Range:");
                ui.horizontal(|ui| {
                    let mut in_v = wa_in as i32;
                    let mut out_v = wa_out as i32;
                    let comp_fps = app.history.current().active_composition().fps.max(1);
                    ui.add(
                        egui::DragValue::new(&mut in_v)
                            .prefix("In: ")
                            .speed(0.5)
                            .range(0..=(comp_duration.saturating_sub(1) as i32)),
                    );
                    ui.add(
                        egui::DragValue::new(&mut out_v)
                            .prefix("Out: ")
                            .speed(0.5)
                            .range(in_v..=(comp_duration.saturating_sub(1) as i32)),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "({} frames @ {} fps, {:.2}s)",
                            range_frames,
                            comp_fps,
                            range_frames as f64 / comp_fps as f64
                        ))
                        .color(egui::Color32::GRAY)
                        .small(),
                    );
                    app.work_area_in = Some(in_v.max(0) as u32);
                    app.work_area_out = Some(out_v.max(in_v.max(0)) as u32);
                });
                ui.end_row();
            });

        ui.add_space(6.0);

        if app.is_exporting {
            // Track elapsed time for ETA estimation.
            let start: Instant = ui
                .ctx()
                .data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new(START_TIME_ID), Instant::now));

            let progress = app.export_progress.clamp(0.0, 1.0);
            let elapsed = start.elapsed().as_secs_f64();
            let eta_secs = if progress > 0.01 {
                elapsed / progress as f64 * (1.0 - progress as f64)
            } else {
                f64::INFINITY
            };

            ui.add(
                egui::ProgressBar::new(progress)
                    .text(format!("Rendering… {:.0}%", progress * 100.0)),
            );
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Elapsed: {}", format_duration(elapsed)))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.label(
                    egui::RichText::new(format!("Remaining (est.): {}", format_duration(eta_secs)))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                if let Some(status) = &app.export_status {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(status).small().color(colors::HUD_STATUS_TEXT));
                    });
                }
            });

            // Cancel with a lightweight confirmation step.
            let confirming = ui
                .ctx()
                .data_mut(|d| *d.get_temp_mut_or_default::<bool>(egui::Id::new(CANCEL_CONFIRM_ID)));
            ui.horizontal(|ui| {
                if confirming {
                    ui.label(
                        egui::RichText::new("Cancel this render?")
                            .small()
                            .color(egui::Color32::YELLOW),
                    );
                    if custom_widgets::ae_button(ui, "Yes, Cancel").clicked() {
                        if let Some(flag) = &app.export_cancel_flag {
                            flag.store(true, std::sync::atomic::Ordering::Relaxed);
                            log::info!("Render cancelled by user");
                        }
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(CANCEL_CONFIRM_ID), false));
                    }
                    if custom_widgets::ae_button(ui, "Keep Rendering").clicked() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(CANCEL_CONFIRM_ID), false));
                    }
                } else if custom_widgets::ae_button(ui, "■ Cancel Render").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(CANCEL_CONFIRM_ID), true));
                }
            });
        } else {
            // Reset transient render state when idle.
            ui.ctx().data_mut(|d| {
                d.remove::<Instant>(egui::Id::new(START_TIME_ID));
                d.remove::<bool>(egui::Id::new(CANCEL_CONFIRM_ID));
            });
            ui.add(
                egui::ProgressBar::new(0.0)
                    .text(format!("Ready to Render ({} frames)", range_frames)),
            );
        }
    });
}
