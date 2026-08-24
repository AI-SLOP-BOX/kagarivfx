use eframe::egui;
use crate::ExportEvent;
use crate::ui::theme::colors;

/// Spawn the async FFmpeg (or fallback) render worker for one composition.
/// Shared by the export dialog button and the Render Queue batch runner.
pub fn start_comp_export(app: &mut crate::AfterEffectsApp, ctx: &egui::Context, comp_name: &str) {
    let Some(comp) = app.history.current().compositions.iter().find(|c| c.name == comp_name).cloned() else {
        app.toasts.error(format!("Queue comp not found: {}", comp_name));
        return;
    };
    let total_frames = comp.duration_frames;

    app.is_exporting = true;
    app.export_progress = 0.0;
    app.export_status = Some(format!("Rendering '{}'…", comp.name));

    // Mux the first video layer's extracted WAV when present AND enabled
    let include_audio = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_include_audio"), || true));
    let audio_wav = if include_audio {
        comp.layers.iter().find_map(|l| {
            match &l.layer_type {
                crate::core::timeline::LayerType::Video { audio_wav, .. } => audio_wav.clone(),
                _ => None,
            }
        })
    } else {
        None
    };
    let codec_idx = app.export_codec_idx;
    let res_scale = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_export_res_scale"), || 1.0f32));

    let output_path = app.export_output_path.clone();
    let render_w = ((comp.width as f32 * res_scale) as u32).max(2);
    let render_h = ((comp.height as f32 * res_scale) as u32).max(2);

    // ── PNG Sequence branch: bypass FFmpeg, write numbered PNGs directly ──
    if codec_idx == 3 {
        if let Some(old_flag) = app.export_cancel_flag.take() {
            old_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        app.export_cancel_flag = Some(cancel_flag.clone());

        let out = std::path::PathBuf::from(&output_path);
        let dir = out.parent().map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let stem = out.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| comp.name.clone());

        let (tx_ui, rx_ui) = std::sync::mpsc::channel();
        app.export_rx = Some(rx_ui);
        crate::core::ffmpeg_export::start_png_sequence_export(
            dir,
            stem,
            render_w,
            render_h,
            total_frames.max(1),
            tx_ui,
            cancel_flag,
            move |frame| {
                crate::core::software_renderer::render_frame_to_pixels(
                    &comp, frame, render_w, render_h, 0.0, 0,
                )
            },
        );
        log::info!("Spawned PNG sequence export for {}", app.export_output_path);
        return;
    }

    let codec = match codec_idx {
        1 => crate::core::ffmpeg_export::VideoCodec::ProRes422,
        2 => crate::core::ffmpeg_export::VideoCodec::ProRes4444,
        _ => crate::core::ffmpeg_export::VideoCodec::H264,
    };
    let config = crate::core::ffmpeg_export::ExportConfig {
        audio_wav,
        output_path: output_path.clone(),
        width: render_w,
        height: render_h,
        fps: comp.fps,
        total_frames: total_frames.max(1),
        codec,
    };

    if let Some(old_flag) = app.export_cancel_flag.take() {
        old_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.export_cancel_flag = Some(cancel_flag.clone());

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

        let _ = crate::core::ffmpeg_export::start_export_cancelable(config, tx_ff, cancel_flag, move |frame| {
            crate::core::software_renderer::render_frame_to_pixels(
                &comp,
                frame,
                comp.width,
                comp.height,
                0.0,
                0,
            )
        });
    } else {
        // Fallback async render thread with progress feedback & cancellation support
        let (tx, rx) = std::sync::mpsc::channel();
        app.export_rx = Some(rx);
        let thread_cancel = cancel_flag.clone();
        let duration = total_frames.max(1);
        std::thread::spawn(move || {
            for frame in 0..=duration {
                if thread_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    log::info!("Export worker thread canceled cleanly");
                    return;
                }
                for layer in &comp.layers {
                    let _world_tf = comp.resolve_world_transform(layer, frame);
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

pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    // ── Non-blocking Channel Event Receiver ──
    let mut finished_export = false;
    let mut next_batch_comp: Option<String> = None;
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
                    app.toasts.info(format!("Export Complete: {}", msg));
                    // ── Batch runner: advance to the next queued comp ──
                    app.batch_idx += 1;
                    if app.batch_idx < app.batch_queue.len() {
                        next_batch_comp = Some(app.batch_queue[app.batch_idx].clone());
                    } else if !app.batch_queue.is_empty() {
                        app.toasts.info("Batch render complete: all queue items exported");
                        app.batch_queue.clear();
                        app.batch_idx = 0;
                    }
                }
                ExportEvent::Error(msg) => {
                    app.export_status = Some(format!("Error: {}", msg));
                    app.is_exporting = false;
                    finished_export = true;
                    app.toasts.error(format!("Export Failed: {}", msg));
                    // A failed item aborts the batch to avoid cascading failures
                    if !app.batch_queue.is_empty() {
                        app.toasts.error("Batch render aborted");
                        app.batch_queue.clear();
                        app.batch_idx = 0;
                    }
                }
            }
        }
    }

    if finished_export {
        app.export_rx = None;
    }
    if let Some(name) = next_batch_comp {
        start_comp_export(app, ctx, &name);
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

            ui.horizontal(|ui| {
                ui.label("Format Preset:");
                egui::ComboBox::from_id_salt("export_fmt_combo")
                    .selected_text(match app.export_format_preset {
                        0 => "H.264 / MP4 (Standard)",
                        1 => "Apple ProRes 422 HQ (MOV)",
                        2 => "PNG Image Sequence",
                        _ => "Lottie / Bodymovin (.json)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.export_format_preset, 0, "H.264 / MP4 (Standard)");
                        ui.selectable_value(&mut app.export_format_preset, 1, "Apple ProRes 422 HQ (MOV)");
                        ui.selectable_value(&mut app.export_format_preset, 2, "PNG Image Sequence");
                        ui.selectable_value(&mut app.export_format_preset, 3, "Lottie / Bodymovin (.json)");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Render Resolution Scale:");
                egui::ComboBox::from_id_salt("export_scale_combo")
                    .selected_text(match app.export_resolution_scale {
                        0 => "100% Full Resolution",
                        1 => "50% Half Resolution",
                        _ => "25% Quarter Resolution",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.export_resolution_scale, 0, "100% Full Resolution");
                        ui.selectable_value(&mut app.export_resolution_scale, 1, "50% Half Resolution");
                        ui.selectable_value(&mut app.export_resolution_scale, 2, "25% Quarter Resolution");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Target FPS:");
                ui.add(egui::DragValue::new(&mut app.export_fps).range(1..=120));
            });

            ui.horizontal(|ui| {
                ui.label("Output Path:");
                ui.text_edit_singleline(&mut app.export_output_path);
            });

            ui.add_space(10.0);
            if app.is_exporting {
                ctx.request_repaint_after(std::time::Duration::from_millis(100)); // Throttled: progress bar needs ~10fps, not 60+
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
                // Include-audio toggle (only meaningful for MP4)
                let audio_toggle_id = egui::Id::new("ae_export_include_audio");
                let mut include_audio = ctx.data_mut(|d| {
                    *d.get_temp_mut_or_insert_with(audio_toggle_id, || true)
                });
                let has_wav = app.history.current().active_composition().layers.iter().any(|l| {
                    matches!(&l.layer_type, crate::core::timeline::LayerType::Video { audio_wav: Some(_), .. })
                });
                if !has_wav {
                    include_audio = false;
                }
                ui.horizontal(|ui| {
                    let resp = ui.add_enabled(
                        has_wav,
                        egui::Checkbox::new(&mut include_audio, "Include Audio"),
                    );
                    let resp = if has_wav {
                        resp
                    } else {
                        resp.on_disabled_hover_text("No audio source: import a video with sound first")
                    };
                    if resp.changed() {
                        ctx.data_mut(|d| d.insert_temp(audio_toggle_id, include_audio));
                    }
                });
                ui.separator();

                // Estimated file size
                {
                    let bitrate_mbps = match app.export_codec_idx {
                        1 => 147.0, // ProRes 422
                        2 => 330.0, // ProRes 4444
                        _ => 10.0,   // H.264
                    };
                    let duration_sec = total_frames as f32 / comp.fps.max(1) as f32;
                    let est_mb = bitrate_mbps * duration_sec / 8.0;
                    let size_text = if est_mb > 1024.0 {
                        format!("{:.1} GB", est_mb / 1024.0)
                    } else {
                        format!("{:.0} MB", est_mb)
                    };
                    ui.label(egui::RichText::new(format!("Estimated file size: ~{}", size_text))
                        .small()
                        .color(colors::TEXT_SECONDARY));
                }

                // Resolution scale
                let res_scale_id = egui::Id::new("ae_export_res_scale");
                let mut res_scale = ctx.data_mut(|d| {
                    *d.get_temp_mut_or_insert_with(res_scale_id, || 1.0f32)
                });
                ui.horizontal(|ui| {
                    ui.label("Resolution:");
                    let orig_w = comp.width;
                    let orig_h = comp.height;
                    for (label, scale) in [("Full", 1.0), ("Half", 0.5), ("Third", 1.0/3.0), ("Quarter", 0.25)] {
                        if ui.selectable_label((res_scale - scale).abs() < 0.01, label).clicked() {
                            res_scale = scale;
                            ctx.data_mut(|d| d.insert_temp(res_scale_id, res_scale));
                        }
                        ui.separator();
                    }
                    if res_scale != 1.0 {
                        ui.label(format!("{}x{}", (orig_w as f32 * res_scale) as u32, (orig_h as f32 * res_scale) as u32));
                    }
                });

                // Codec selection
                ui.horizontal(|ui| {
                    ui.label("Video Codec:");
                    ui.selectable_value(&mut app.export_codec_idx, 0, "H.264");
                    ui.selectable_value(&mut app.export_codec_idx, 1, "ProRes 422");
                    ui.selectable_value(&mut app.export_codec_idx, 2, "ProRes 4444");
                    ui.selectable_value(&mut app.export_codec_idx, 3, "PNG Sequence");
                });

                ui.horizontal(|ui| {
                    let active_comp_name = app.history.current().active_composition().name.clone();
                    if app.export_format_preset == 3 {
                        // Lottie: synchronous JSON write — fast, no ffmpeg required.
                        if ui.button("Export Lottie JSON").clicked() {
                            let project = app.history.current().clone();
                            let json = crate::core::lottie_exporter::export_project_to_json(&project);
                            // The Lottie format cannot carry effects; warn instead of losing them silently.
                            let effect_count: usize = project
                                .compositions
                                .iter()
                                .flat_map(|c| c.layers.iter())
                                .map(|l| l.effects.iter().filter(|e| e.enabled).count())
                                .sum();
                            let mut path = app.export_output_path.trim().to_string();
                            if path.is_empty() {
                                path = format!("{}_lottie", active_comp_name.replace(' ', "_"));
                            }
                            if !path.ends_with(".json") {
                                path.push_str(".json");
                            }
                            match std::fs::write(&path, &json) {
                                Ok(()) => {
                                    app.export_status =
                                        Some(format!("Lottie export complete → {}", path));
                                    app.toasts.info(format!(
                                        "Lottie exported → {} ({} bytes)",
                                        path,
                                        json.len()
                                    ));
                                    if effect_count > 0 {
                                        app.toasts.warning(format!(
                                            "{} enabled effect(s) are NOT part of Lottie output (format limitation)",
                                            effect_count
                                        ));
                                    }
                                }
                                Err(e) => {
                                    app.export_status =
                                        Some(format!("Error: Lottie export failed: {}", e));
                                    app.toasts.error(format!("Lottie export failed: {}", e));
                                }
                            }
                        }
                    } else if ui.button("Start Async Render").clicked() {
                        start_comp_export(app, ctx, &active_comp_name);
                    }
                    if ui.button("Close").clicked() {
                        app.show_export_dialog = false;
                    }
                });
            }
        });

    app.show_export_dialog = open;
}
