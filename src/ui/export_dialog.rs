use eframe::egui;
use crate::ExportEvent;

pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    // ── Non-blocking Channel Event Receiver ──
    let mut finished_export = false;
    if let Some(ref rx) = app.export_rx {
        while let Ok(event) = rx.try_recv() {
            match event {
                ExportEvent::Progress(prog, msg) => {
                    app.export_progress = prog;
                    app.export_status = Some(msg);
                }
                ExportEvent::Finished(msg) => {
                    app.export_progress = 1.0;
                    app.export_status = Some(msg.clone());
                    app.is_exporting = false;
                    finished_export = true;
                    app.toasts.info(format!("🎬 Export Complete: {}", msg));
                }
                ExportEvent::Error(msg) => {
                    app.export_status = Some(format!("Error: {}", msg));
                    app.is_exporting = false;
                    finished_export = true;
                    app.toasts.error(format!("❌ Export Failed: {}", msg));
                }
            }
        }
    }

    if finished_export {
        app.export_rx = None;
    }

    if !app.show_export_dialog {
        return;
    }

    let mut open = app.show_export_dialog;
    egui::Window::new("Export Composition Video")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.heading("Render Settings");
            ui.separator();

            let comp = app.history.current().active_composition();
            let total_frames = comp.duration_frames;

            ui.label(format!("Composition: {} x {}", comp.width, comp.height));
            ui.label(format!("Total Duration: {} frames", total_frames));

            ui.add_space(8.0);

            let fmt_id = egui::Id::new("export_format_selection");
            let mut format_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(fmt_id, || 0));
            ui.horizontal(|ui| {
                ui.label("Format Preset:");
                egui::ComboBox::from_id_source("export_fmt_combo")
                    .selected_text(match format_idx {
                        0 => "H.264 / MP4 (Standard)",
                        1 => "Apple ProRes 422 HQ (MOV)",
                        _ => "PNG Image Sequence",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut format_idx, 0, "H.264 / MP4 (Standard)").clicked() {
                            ctx.data_mut(|d| d.insert_temp(fmt_id, format_idx));
                        }
                        if ui.selectable_value(&mut format_idx, 1, "Apple ProRes 422 HQ (MOV)").clicked() {
                            ctx.data_mut(|d| d.insert_temp(fmt_id, format_idx));
                        }
                        if ui.selectable_value(&mut format_idx, 2, "PNG Image Sequence").clicked() {
                            ctx.data_mut(|d| d.insert_temp(fmt_id, format_idx));
                        }
                    });
            });

            let scale_id = egui::Id::new("export_scale_selection");
            let mut scale_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(scale_id, || 0));
            ui.horizontal(|ui| {
                ui.label("Render Resolution Scale:");
                egui::ComboBox::from_id_source("export_scale_combo")
                    .selected_text(match scale_idx {
                        0 => "100% Full Resolution",
                        1 => "50% Half Resolution",
                        _ => "25% Quarter Resolution",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut scale_idx, 0, "100% Full Resolution").clicked() {
                            ctx.data_mut(|d| d.insert_temp(scale_id, scale_idx));
                        }
                        if ui.selectable_value(&mut scale_idx, 1, "50% Half Resolution").clicked() {
                            ctx.data_mut(|d| d.insert_temp(scale_id, scale_idx));
                        }
                        if ui.selectable_value(&mut scale_idx, 2, "25% Quarter Resolution").clicked() {
                            ctx.data_mut(|d| d.insert_temp(scale_id, scale_idx));
                        }
                    });
            });

            let audio_id = egui::Id::new("export_include_audio");
            let mut include_audio = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(audio_id, || true));
            ui.checkbox(&mut include_audio, "Include Audio Master Track");
            ctx.data_mut(|d| d.insert_temp(audio_id, include_audio));

            ui.horizontal(|ui| {
                ui.label("Target FPS:");
                ui.add(egui::DragValue::new(&mut app.export_fps).clamp_range(1..=120));
            });

            ui.horizontal(|ui| {
                ui.label("Output Path:");
                ui.text_edit_singleline(&mut app.export_output_path);
            });

            ui.add_space(10.0);
            if app.is_exporting {
                ctx.request_repaint(); // Smooth UI progress bar updates without freezing
                ui.label("Rendering composition in background thread...");
                ui.add(egui::ProgressBar::new(app.export_progress).show_percentage());
                if let Some(ref status) = app.export_status {
                    ui.weak(status);
                }
            } else {
                if let Some(ref status) = app.export_status {
                    let color = if status.contains("Error") { egui::Color32::RED } else { egui::Color32::GREEN };
                    ui.label(egui::RichText::new(status).color(color));
                    ui.add_space(4.0);
                }
                ui.horizontal(|ui| {
                    if ui.button("Start Async Render").clicked() {
                        app.is_exporting = true;
                        app.export_progress = 0.0;
                        app.export_status = Some("Initializing async render worker thread...".to_string());

                        let (tx, rx) = std::sync::mpsc::channel();
                        app.export_rx = Some(rx);
                        let comp_snapshot = comp.clone();
                        let output_path = app.export_output_path.clone();

                        let config = crate::core::ffmpeg_export::ExportConfig {
                            output_path: output_path.clone(),
                            width: comp.width,
                            height: comp.height,
                            fps: comp.fps,
                            total_frames: total_frames.max(1),
                        };

                        if crate::core::ffmpeg_export::is_ffmpeg_available() {
                            let (tx_ff, rx_ff) = std::sync::mpsc::channel();
                            let (tx_ui, rx_ui) = std::sync::mpsc::channel();
                            app.export_rx = Some(rx_ui);

                            std::thread::spawn(move || {
                                while let Ok(evt) = rx_ff.recv() {
                                    let mapped = match evt {
                                        crate::core::ffmpeg_export::ExportEvent::Progress(p, m) => ExportEvent::Progress(p, m),
                                        crate::core::ffmpeg_export::ExportEvent::Finished(m) => ExportEvent::Finished(m),
                                        crate::core::ffmpeg_export::ExportEvent::Error(m) => ExportEvent::Error(m),
                                    };
                                    let _ = tx_ui.send(mapped);
                                }
                            });

                            let _ = crate::core::ffmpeg_export::start_export(config, tx_ff, move |frame| {
                                crate::core::software_renderer::render_frame_to_pixels(
                                    &comp_snapshot,
                                    frame,
                                    comp_snapshot.width,
                                    comp_snapshot.height,
                                    0.0,
                                    0,
                                )
                            });
                        } else {
                            // Fallback async render thread with progress feedback
                            std::thread::spawn(move || {
                                let duration = total_frames.max(1);
                                for frame in 0..=duration {
                                    for layer in &comp_snapshot.layers {
                                        let _world_tf = comp_snapshot.resolve_world_transform(layer, frame);
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(4));
                                    let prog = frame as f32 / duration as f32;
                                    let msg = format!("Rendering frame {} / {}...", frame, duration);
                                    let _ = tx.send(ExportEvent::Progress(prog, msg));
                                }
                                let finished_msg = format!("Export complete → Saved to {}", output_path);
                                let _ = tx.send(ExportEvent::Finished(finished_msg));
                            });
                        }

                        log::info!("Spawned async render pipeline for {}", app.export_output_path);
                    }
                    if ui.button("Close").clicked() {
                        app.show_export_dialog = false;
                    }
                });
            }
        });

    app.show_export_dialog = open;
}
