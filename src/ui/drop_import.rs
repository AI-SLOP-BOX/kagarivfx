//! Drag-and-drop media import: images become Image layers, videos spawn the
//! ffmpeg frame-extraction pipeline on a worker thread, WAVs become Audio
//! layers. Results stream back through a channel and are inserted above the
//! current selection.
use eframe::egui;
use crate::AfterEffectsApp;

pub enum ImportResult {
    Video(crate::core::video_import::VideoAsset, String),
    Err(String),
}

fn insert_layer(app: &mut AfterEffectsApp, layer: crate::core::timeline::Layer, label: &str) {
    let comp_dur = app.history.current().active_composition().duration_frames;
    let insert_at = app.selected_layer_idx.map(|i| i + 1).unwrap_or(0);
    let mut l = layer;
    l.in_frame = l.in_frame.min(comp_dur.saturating_sub(1));
    l.out_frame = comp_dur.max(l.in_frame + 1);
    let proj = app.history.current_mut();
    let comp = proj.active_composition_mut();
    let at = insert_at.min(comp.layers.len());
    comp.layers.insert(at, l);
    crate::core::frame_cache::bump_version();
    app.toasts.info(format!("Imported {}", label));
}

pub fn handle_dropped_files(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    // 1) Drain finished video imports first.
    if let Some(rx) = app.import_rx.take() {
        while let Ok(res) = rx.try_recv() {
            match res {
                ImportResult::Video(asset, name) => {
                    let dur = asset.frame_count.max(1);
                    let layer = crate::core::timeline::Layer::new(
                        format!("vid_{}", asset.frames_dir.replace('/', "_")),
                        name.clone(),
                        crate::core::timeline::LayerType::Video {
                            source: asset.source_path.clone(),
                            frames_dir: asset.frames_dir.clone(),
                            frame_count: asset.frame_count,
                            audio_wav: asset.audio_wav.clone(),
                            speed: 1.0,
                        },
                        dur,
                    );
                    insert_layer(app, layer, &format!("video '{}'", name));
                }
                ImportResult::Err(e) => {
                    app.toasts.error(format!("Import failed: {}", e));
                }
            }
        }
        app.import_rx = Some(rx); // keep listening for more files
        let _ = ctx; // silence unused in some feature combos
    }

    // 2) Pick up newly dropped files.
    let dropped = ctx.input(|i| i.raw.dropped_files.clone());
    if dropped.is_empty() { return; }
    for f in dropped {
        let Some(path) = f.path else { continue };
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "media".into());
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let path_str = path.to_string_lossy().to_string();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => {
                match image::image_dimensions(&path) {
                    Ok((iw, ih)) => {
                        let (cw, ch) = {
                            let c = app.history.current().active_composition();
                            (c.width as f32, c.height as f32)
                        };
                        let fit = ((cw / iw as f32).min(ch / ih as f32)).min(1.0) * 100.0;
                        let mut layer = crate::core::timeline::Layer::new(
                            format!("img_{}", name),
                            name.clone(),
                            crate::core::timeline::LayerType::Image { path: path_str.clone() },
                            1,
                        );
                        layer.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                        layer.transform.scale = crate::core::property::Animatable::new_constant([fit, fit]);
                        layer.out_frame = app.history.current().active_composition().duration_frames;
                        insert_layer(app, layer, &format!("image '{}' ({}×{})", name, iw, ih));
                    }
                    Err(e) => app.toasts.error(format!("Cannot read {}: {}", name, e)),
                }
            }
            "wav" => {
                let (cw, ch) = {
                    let c = app.history.current().active_composition();
                    (c.width as f32, c.height as f32)
                };
                let mut layer = crate::core::timeline::Layer::new(
                    format!("aud_{}", name),
                    format!("🔊 {}", name),
                    crate::core::timeline::LayerType::Audio { path: path_str.clone(), volume: crate::core::property::Animatable::new_constant(1.0) },
                    1,
                );
                layer.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                layer.out_frame = app.history.current().active_composition().duration_frames;
                insert_layer(app, layer, &format!("audio '{}'", name));
            }
            "mp4" | "mov" | "mkv" | "avi" | "webm" => {
                if !crate::core::video_import::ffmpeg_available() {
                    app.toasts.error("Video import needs ffmpeg on PATH");
                    continue;
                }
                app.toasts.info(format!("Extracting frames from '{}'…", name));
                let (tx, rx) = std::sync::mpsc::channel();
                app.import_rx = Some(rx);
                let fps = app.history.current().active_composition().fps as f32;
                let dest = std::path::PathBuf::from("media").join(&name);
                std::thread::spawn(move || {
                    match crate::core::video_import::import_video(&path_str, &dest, fps) {
                        Ok(a) => { let _ = tx.send(ImportResult::Video(a, name)); }
                        Err(e) => { let _ = tx.send(ImportResult::Err(e)); }
                    }
                });
            }
            other => {
                app.toasts.error(format!("Unsupported file type: .{}", other));
            }
        }
    }
}
