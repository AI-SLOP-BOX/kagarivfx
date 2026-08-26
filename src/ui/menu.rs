use eframe::egui;

pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    app.history = crate::core::history::ProjectHistory::new(
                        crate::core::timeline::Project::default(),
                    );
                    app.selected_layer_idx = None;
                    app.selected_layers.clear();
                    crate::core::frame_cache::bump_version();
                    ui.close_menu();
                }
                ui.separator();
                let save_sc = crate::ui::shortcuts::format_shortcut("S", true, false, false);
                if ui.add(egui::Button::new("Save Project").shortcut_text(save_sc)).clicked() {
                    let path = app.project_path.clone();
                    let project = app.history.current();
                    match crate::core::project_migration::save_project_atomic(project, &path) {
                        Ok(_) => {
                            let _ = app.autosave.save_now(project);
                            app.toasts.info(format!("Saved: {}", path));
                        }
                        Err(e) => {
                            app.toasts.error(format!("Save failed: {}", e));
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Save Project As...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("After Effects OSS Project", &["json", "aevfx"])
                        .set_file_name("project.aevfx.json")
                        .save_file()
                    {
                        if let Err(e) = crate::ui::project_io::save_project_to_path(app, &path) {
                            app.toasts.error(e);
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import Subtitles (.srt / .vtt)...").on_hover_text("Create timed, bottom-center caption text layers — pair with Kdenlive/Shotcut Whisper output").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Subtitles", &["srt", "vtt"])
                        .pick_file()
                    {
                        match std::fs::read_to_string(&path)
                            .map_err(|e| e.to_string())
                            .map(|s| crate::core::subtitles::parse_srt(&s, app.history.current().active_composition().fps))
                        {
                            Ok(cues) => {
                                if cues.is_empty() {
                                    app.toasts.error("No cues found in subtitle file");
                                } else {
                                    let (cw, ch) = { let cc = app.history.current().active_composition(); (cc.width as f32, cc.height as f32) };
                                    let layers = crate::core::subtitles::cues_to_layers(&cues, cw, ch, 48);
                                    let n = layers.len();
                                    let proj = app.history.current_mut();
                                    let comp = proj.active_composition_mut();
                                    for l in layers {
                                        comp.add_layer(l);
                                    }
                                    crate::core::frame_cache::bump_version();
                                    app.toasts.info(format!("{} caption layers created", n));
                                }
                            }
                            Err(e) => app.toasts.error(e),
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Import Blender Camera Track (.json)...").on_hover_text("Bake a tracked camera solve onto this comp's active 3D camera — run tools/blender_camera_export.py inside Blender first").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Camera Track JSON", &["json"])
                        .pick_file()
                    {
                        match std::fs::read_to_string(&path)
                            .map_err(|e| e.to_string())
                            .and_then(|s| crate::core::camera_track::BlenderCamTrack::parse(&s))
                        {
                            Ok(track) => {
                                let baked = std::cell::Cell::new(0usize);
                                app.modify_project(|p| {
                                    let n = track.apply_to_comp(p.active_composition_mut(), true);
                                    baked.set(n);
                                });
                                crate::core::frame_cache::bump_version();
                                app.toasts.info(format!(
                                    "Camera track baked: {} keyframes from '{}'",
                                    baked.get(),
                                    path.file_name().unwrap_or_default().to_string_lossy()
                                ));
                            }
                            Err(e) => app.toasts.error(e),
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Open Project...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("After Effects OSS Project", &["json", "aevfx"])
                        .pick_file()
                    {
                        if let Err(e) = crate::ui::project_io::open_project_from_path(app, &path) {
                            app.toasts.error(e);
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export OpenTimelineIO (.otio.json)").clicked() {
                    let comp = app.history.current().active_composition();
                    let otio = crate::core::integration::OtioTimeline::from_composition(comp);
                    match serde_json::to_string_pretty(&otio) {
                        Ok(json) => match std::fs::write(&app.otio_path, json) {
                            Ok(_) => {
                                app.toasts.info(format!("Exported OTIO: {}", app.otio_path));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Failed to save OTIO file: {}", err));
                            }
                        },
                        Err(err) => {
                            app.toasts.error(format!("Failed to serialize OTIO: {}", err));
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import Video (FFmpeg)...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video Files", &["mp4", "mov", "avi", "mkv", "webm"])
                        .pick_file()
                    {
                        // Extract at the active composition's fps so 1 seq frame == 1 comp frame
                        let comp = app.history.current().active_composition();
                        let fps = comp.fps as f32;
                        let name = path.file_stem().map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "video".to_string());
                        let dest = std::env::temp_dir().join("aevfx_media").join(&name);
                        ui.ctx().request_repaint();
                        match crate::core::video_import::import_video(
                            &path.to_string_lossy(), &dest, fps,
                        ) {
                            Ok(asset) => {
                                let src = path.to_string_lossy().to_string();
                                app.modify_project(|p| {
                                    let layer_count = p.compositions.len();
                                    let comp = p.active_composition_mut();
                                    let layer = crate::core::timeline::Layer::new(
                                        format!("video_{}", layer_count),
                                        name.clone(),
                                        crate::core::timeline::LayerType::Video {
                                            source: src.clone(),
                                            frames_dir: asset.frames_dir.clone(),
                                            frame_count: asset.frame_count,
                                            audio_wav: asset.audio_wav.clone(),
                                            speed: 1.0,
                                        },
                                        comp.fps,
                                    );
                                    comp.layers.push(layer);
                                });
                                app.toasts.info(format!(
                                    "Imported video: {} ({} frames)",
                                    name, asset.frame_count
                                ));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Video import failed: {}", err));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import Image...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "tga", "webp"])
                        .pick_file()
                    {
                        let src = path.to_string_lossy().to_string();
                        let name = path.file_stem().map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "image".to_string());
                        app.modify_project(|p| {
                            let layer_count = p.compositions.len();
                            let comp = p.active_composition_mut();
                            let layer = crate::core::timeline::Layer::new(
                                format!("img_{}", layer_count),
                                name.clone(),
                                crate::core::timeline::LayerType::Image { path: src.clone() },
                                comp.duration_frames,
                            );
                            comp.layers.push(layer);
                        });
                        app.toasts.info(format!("Imported image: {}", name));
                    }
                    ui.close_menu();
                }
                if ui.button("Export MLT XML (Shotcut / Kdenlive)...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("MLT XML", &["xml", "mlt"])
                        .set_file_name(format!("{}.mlt.xml",
                            app.history.current().active_composition().name))
                        .save_file()
                    {
                        let comp = app.history.current().active_composition();
                        let xml = crate::core::mlt_export::MltExporter::export_to_xml(comp);
                        match std::fs::write(&path, xml) {
                            Ok(_) => {
                                app.toasts.info(format!(
                                    "Exported MLT XML: {}",
                                    path.to_string_lossy()
                                ));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Failed to save MLT file: {}", err));
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Import OpenTimelineIO (.otio.json)").clicked() {
                    match std::fs::read_to_string(&app.otio_path) {
                        Ok(json) => match serde_json::from_str::<crate::core::integration::OtioTimeline>(&json) {
                            Ok(otio) => {
                                let comp = otio.to_composition();
                                app.modify_project(|p| p.compositions[0] = comp);
                                app.toasts.info(format!("Imported OTIO: {}", app.otio_path));
                            }
                            Err(err) => {
                                app.toasts.error(format!("Invalid OTIO format: {}", err));
                            }
                        },
                        Err(err) => {
                            app.toasts.error(format!("Could not read OTIO file: {}", err));
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export Video (MP4)...").clicked() {
                    app.show_export_dialog = true;
                    ui.close_menu();
                }
                if ui.button("⚡ Quick Export Active Comp").on_hover_text("Export the active composition to MP4 with current settings (no dialog)").clicked() {
                    let comp_name = app.history.current().active_composition().name.clone();
                    crate::ui::export_dialog::start_comp_export(app, ctx, &comp_name);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo_sc = crate::ui::shortcuts::format_shortcut("Z", true, false, false);
                let undo_btn = egui::Button::new("Undo (元に戻す)").shortcut_text(undo_sc);
                if ui.add_enabled(app.history.can_undo(), undo_btn).clicked() {
                    app.history.undo();
                    ui.close_menu();
                }
                let redo_sc = crate::ui::shortcuts::format_shortcut("Z", true, true, false);
                let redo_btn = egui::Button::new("Redo (やり直す)").shortcut_text(redo_sc);
                if ui.add_enabled(app.history.can_redo(), redo_btn).clicked() {
                    app.history.redo();
                    ui.close_menu();
                }
                if ui.button("🕘 Undo History…").on_hover_text("Open the named-step history panel and jump to any step").clicked() {
                    app.show_history_panel = !app.show_history_panel;
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Duplicate").shortcut_text("Cmd+D")).clicked() {
                    if let Some(sel_idx) = app.selected_layer_idx {
                        let comp = app.history.current_mut().active_composition_mut();
                        if sel_idx < comp.layers.len() {
                            let mut cloned = comp.layers[sel_idx].clone();
                            let n = comp.layers.len();
                            cloned.id = format!("{}_copy_{}", cloned.id, n);
                            cloned.name = format!("{} copy", cloned.name);
                            comp.layers.insert(sel_idx + 1, cloned);
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("Layer duplicated");
                        }
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("Composition", |ui| {
                if ui.add(egui::Button::new("New Composition...").shortcut_text("Cmd+N")).clicked() {
                    app.show_new_comp_dialog = true;
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Duplicate Composition")).clicked() {
                    app.modify_project(|p| {
                        let idx = p.active_composition_idx;
                        if let Some(src) = p.compositions.get(idx).cloned() {
                            let mut copy = src.clone();
                            copy.name = format!("{} copy", src.name);
                            copy.id = format!("{}_copy_{}", src.id, p.compositions.len());
                            p.compositions.push(copy);
                            p.active_composition_idx = p.compositions.len() - 1;
                        }
                    });
                    crate::core::frame_cache::bump_version();
                    app.toasts.info("Composition duplicated");
                    ui.close_menu();
                }
                let comp_sc = crate::ui::shortcuts::format_shortcut("K", true, false, false);
                let btn = egui::Button::new("Composition Settings...").shortcut_text(comp_sc);
                if ui.add(btn).clicked() {
                    app.show_comp_settings = true;
                    ui.close_menu();
                }
                let rq_sc = crate::ui::shortcuts::format_shortcut("M", true, false, false);
                if ui.add(egui::Button::new("Add to Render Queue").shortcut_text(rq_sc)).clicked() {
                    app.show_export_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export Captions (.srt)").on_hover_text("Write all 'Caption …' text layers as an SRT file").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Subtitles", &["srt"])
                        .set_file_name(format!("{}_captions.srt",
                            app.history.current().active_composition().name.replace(' ', "_")))
                        .save_file()
                    {
                        let proj = app.history.current();
                        let comp = proj.active_composition();
                        let srt = crate::core::subtitles::layers_to_srt(&comp.layers, comp.fps);
                        if srt.is_empty() {
                            app.toasts.error("No caption layers found");
                        } else {
                            match std::fs::write(&path, &srt) {
                                Ok(_) => app.toasts.info(format!("Captions exported: {}", path.display())),
                                Err(e) => app.toasts.error(format!("Write failed: {}", e)),
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Save Frame As… (PNG)").on_hover_text("Render the current frame at full resolution and save as PNG").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG Image", &["png"])
                        .set_file_name(format!("{}_frame_{}.png",
                            app.history.current().active_composition().name,
                            app.current_frame))
                        .save_file()
                    {
                        let comp = app.history.current().active_composition().clone();
                        let frame = app.current_frame;
                        // 2× supersample render then alpha-weighted downsample
                        // for clean anti-aliased edges (clamped by raster max).
                        let max_dim = crate::core::software_renderer::MAX_RENDER_DIMENSION;
                        let sw = (comp.width.saturating_mul(2)).min(max_dim);
                        let sh = (comp.height.saturating_mul(2)).min(max_dim);
                        let px = crate::core::software_renderer::render_frame_to_pixels(
                            &comp, frame, sw, sh, 0.0, 0,
                        );
                        let (pixels, w, h) = if sw > comp.width || sh > comp.height {
                            (crate::core::supersample::downsample2x(&px, comp.width, comp.height), comp.width, comp.height)
                        } else {
                            (px, sw, sh)
                        };
                        match image::save_buffer(&path, &pixels, w, h, image::ColorType::Rgba8) {
                            Ok(_) => app.toasts.info(format!("Frame {} saved to {}", frame, path.display())),
                            Err(e) => app.toasts.error(format!("Save failed: {}", e)),
                        }
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("Layer", |ui| {
                ui.menu_button("New", |ui| {
                    let solid_sc = crate::ui::shortcuts::format_shortcut("Y", true, false, false);
                    if ui.add(egui::Button::new("Solid...").shortcut_text(solid_sc)).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Solid {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new(id, name, crate::core::timeline::LayerType::Solid { color: [0.2, 0.5, 0.9, 1.0] }, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Text").shortcut_text("Cmd+Alt+Shift+T")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Text {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new(id, name, crate::core::timeline::LayerType::new_text("Title Text", 72, [1.0, 1.0, 1.0, 1.0]), total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Null Object").shortcut_text("Cmd+Alt+Shift+Y")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Null {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new_null(id, name, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Adjustment Layer").shortcut_text("Cmd+Alt+Y")).clicked() {
                        let total_frames = app.history.current().active_composition().duration_frames;
                        let comp_mut = app.history.current_mut().active_composition_mut();
                        let id = format!("layer_{}", comp_mut.layers.len());
                        let name = format!("Adjustment Layer {}", comp_mut.layers.len());
                        let layer = crate::core::timeline::Layer::new_adjustment(id, name, total_frames);
                        comp_mut.add_layer(layer);
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.button("Light").on_hover_text("Adds a 3D light to the composition").clicked() {
                        let n = app.history.current().active_composition().lights.len();
                        let mut light = crate::core::timeline::Light3D::default();
                        let nanos = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.subsec_nanos())
                            .unwrap_or(0);
                        light.id = format!("light_{}_{nanos}", n);
                        light.name = format!("Light {}", n + 1);
                        let name = light.name.clone();
                        app.history.current_mut().active_composition_mut().lights.push(light);
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Added {}", name));
                        ui.close_menu();
                    }
                });
                ui.menu_button("Transform", |ui| {
                    // Center / Fit / Flip / Reset commands (AE Layer > Transform parity)
                    if ui.button("Center in Comp").on_hover_text("Move position to the composition center").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let (cw, ch) = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                                }
                            });
                            app.toasts.info("Centered in Comp");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Fit to Comp").on_hover_text("Scale so the layer covers the full comp frame").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let (cw, ch) = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let bs = l.bounding_size();
                                    if bs[0] > 1.0 && bs[1] > 1.0 {
                                        let sx = cw / bs[0] * 100.0;
                                        let sy = ch / bs[1] * 100.0;
                                        let s = sx.max(sy);
                                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                        l.transform.position = crate::core::property::Animatable::new_constant([cw / 2.0, ch / 2.0]);
                                    }
                                }
                            });
                            app.toasts.info("Fit to Comp");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Fit to Comp Width").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let cw = app.history.current().active_composition().width as f32;
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let bs = l.bounding_size();
                                    if bs[0] > 1.0 {
                                        let s = cw / bs[0] * 100.0;
                                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                    }
                                }
                            });
                            app.toasts.info("Fit to Comp Width");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Fit to Comp Height").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let ch = app.history.current().active_composition().height as f32;
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let bs = l.bounding_size();
                                    if bs[1] > 1.0 {
                                        let s = ch / bs[1] * 100.0;
                                        l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                    }
                                }
                            });
                            app.toasts.info("Fit to Comp Height");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Flip Horizontal").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let cf = app.current_frame;
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let s = l.transform.scale.evaluate(cf);
                                    l.transform.scale = crate::core::property::Animatable::new_constant([-s[0], s[1]]);
                                }
                            });
                            app.toasts.info("Flipped Horizontal");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Flip Vertical").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let cf = app.current_frame;
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    let s = l.transform.scale.evaluate(cf);
                                    l.transform.scale = crate::core::property::Animatable::new_constant([s[0], -s[1]]);
                                }
                            });
                            app.toasts.info("Flipped Vertical");
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Reset Position").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let dims = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.position = crate::core::property::Animatable::new_constant([dims.0 / 2.0, dims.1 / 2.0]);
                                }
                            });
                            app.toasts.info("Position reset");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Reset Scale").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.scale = crate::core::property::Animatable::new_constant([100.0, 100.0]);
                                }
                            });
                            app.toasts.info("Scale reset");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Reset Rotation").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.rotation = crate::core::property::Animatable::new_constant(0.0);
                                }
                            });
                            app.toasts.info("Rotation reset");
                            ui.close_menu();
                        }
                    }
                    if ui.button("Reset All Transforms").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let dims = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.transform.position = crate::core::property::Animatable::new_constant([dims.0 / 2.0, dims.1 / 2.0]);
                                    l.transform.scale = crate::core::property::Animatable::new_constant([100.0, 100.0]);
                                    l.transform.rotation = crate::core::property::Animatable::new_constant(0.0);
                                    l.transform.opacity = crate::core::property::Animatable::new_constant(100.0);
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("All transforms reset");
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Time", |ui| {
                    if ui.add(egui::Button::new("Enable Time Remapping").shortcut_text("Cmd+Alt+T")).clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.enable_time_remapping();
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("Time remapping enabled — edit keyframes in the Graph Editor");
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Time-Reverse Layer")).clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.time_reverse();
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("Layer time-reversed");
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("🎯 Stabilize Motion (from Track)")).on_hover_text("Bake counter-movement position keyframes from the layer's first tracker").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let mut baked_count = 0usize;
                            app.modify_project(|p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    baked_count = crate::core::stabilizer::stabilize_layer_smoothed(l, 2);
                                }
                            });
                            if baked_count > 0 {
                                app.toasts.info(format!("Stabilized: {} position keyframes baked", baked_count));
                            } else {
                                app.toasts.error("Layer has no tracked data — run the Tracker first");
                            }
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Freeze Frame at Playhead")).clicked() {
                        let frame = app.current_frame;
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.freeze_at(frame);
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info(format!("Frozen at source frame {}", frame));
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Remove Time Remapping").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.clear_time_remap();
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("Time remapping removed");
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Posterize Time 12fps (Stop Motion)").on_hover_text("Quantizes layer time to 12fps — toggles off if already enabled").clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            let already = matches!(
                                app.history.current().active_composition().layers.get(idx).and_then(|l| l.posterize_time.as_ref()),
                                Some(pt) if pt.enabled
                            );
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.posterize_time = if already {
                                        None
                                    } else {
                                        Some(crate::core::posterize_time::PosterizeTimeSettings::default())
                                    };
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info(if already { "Posterize Time removed" } else { "Posterize Time: 12fps stop-motion" });
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("Time Stretch ×2 (Slow)")).clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.time_stretch(2.0);
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("Layer stretched to ×2 duration");
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Time Stretch ×0.5 (Fast)")).clicked() {
                        if let Some(idx) = app.selected_layer_idx {
                            app.modify_project(move |p| {
                                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                    l.time_stretch(0.5);
                                }
                            });
                            crate::core::frame_cache::bump_version();
                            app.toasts.info("Layer compressed to ×0.5 duration");
                        } else {
                            app.toasts.info("Select a layer first");
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Arrange", |ui| {
                    let len = app.history.current().active_composition().layers.len();
                    let Some(i) = app.selected_layer_idx else {
                        ui.label("Select a layer first");
                        return;
                    };
                    if ui.button("Bring to Front").clicked() {
                        app.modify_project(move |p| {
                            let comp = p.active_composition_mut();
                            if i < comp.layers.len() && comp.layers.len() > 1 {
                                let l = comp.layers.remove(i);
                                comp.layers.push(l);
                            }
                        });
                        app.selected_layer_idx = if i < len { Some(len - 1) } else { app.selected_layer_idx };
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Bring Forward").shortcut_text("Cmd+]")).clicked() {
                        if i + 1 < len {
                            app.modify_project(move |p| {
                                let comp = p.active_composition_mut();
                                if i + 1 < comp.layers.len() {
                                    comp.layers.swap(i, i + 1);
                                }
                            });
                            app.selected_layer_idx = Some(i + 1);
                            crate::core::frame_cache::bump_version();
                        }
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Send Backward").shortcut_text("Cmd+[")).clicked() {
                        if i > 0 {
                            app.modify_project(move |p| {
                                let comp = p.active_composition_mut();
                                if i < comp.layers.len() {
                                    comp.layers.swap(i - 1, i);
                                }
                            });
                            app.selected_layer_idx = Some(i - 1);
                            crate::core::frame_cache::bump_version();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Send to Back").clicked() {
                        app.modify_project(move |p| {
                            let comp = p.active_composition_mut();
                            if i < comp.layers.len() && comp.layers.len() > 1 {
                                let l = comp.layers.remove(i);
                                comp.layers.insert(0, l);
                            }
                        });
                        app.selected_layer_idx = if i < len { Some(0) } else { app.selected_layer_idx };
                        crate::core::frame_cache::bump_version();
                        ui.close_menu();
                    }
                });
                ui.separator();
                if ui.button("Un-Solo All Layers").clicked() {
                    app.modify_project(|p| {
                        for l in p.active_composition_mut().layers.iter_mut() {
                            l.solo = false;
                        }
                    });
                    crate::core::frame_cache::bump_version();
                    app.toasts.info("All layers un-soloed");
                    ui.close_menu();
                }
                if ui.button("Unlock All Layers").clicked() {
                    app.modify_project(|p| {
                        for l in p.active_composition_mut().layers.iter_mut() {
                            l.locked = false;
                        }
                    });
                    crate::core::frame_cache::bump_version();
                    app.toasts.info("All layers unlocked");
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Pre-Compose...").shortcut_text("Cmd+Shift+C")).clicked() {
                    ui.close_menu();
                }
            });
            ui.menu_button("Effect", |ui| {
                ui.menu_button("Expression Controls", |ui| {
                    if ui.button("Slider Control").on_hover_text("Keyframeable scalar — drive other layers via effect_param()").clicked() {
                        apply_effect_by_name(app, "Slider Control");
                        ui.close_menu();
                    }
                    if ui.button("Angle Control").clicked() {
                        apply_effect_by_name(app, "Angle Control");
                        ui.close_menu();
                    }
                    if ui.button("Point Control").clicked() {
                        apply_effect_by_name(app, "Point Control");
                        ui.close_menu();
                    }
                    if ui.button("Color Control").clicked() {
                        apply_effect_by_name(app, "Color Control");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Blur & Sharpen", |ui| {
                    if ui.button("Gaussian Blur").clicked() {
                        apply_effect_by_name(app, "Gaussian Blur");
                        ui.close_menu();
                    }
                    if ui.button("Directional Blur").clicked() {
                        apply_effect_by_name(app, "Directional Blur");
                        ui.close_menu();
                    }
                    if ui.button("Radial Blur").clicked() {
                        apply_effect_by_name(app, "Radial Blur");
                        ui.close_menu();
                    }
                    if ui.button("Sharpen").clicked() {
                        apply_effect_by_name(app, "Sharpen");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Color Correction", |ui| {
                    if ui.button("Color Tint").clicked() {
                        apply_effect_by_name(app, "Color Tint");
                        ui.close_menu();
                    }
                    if ui.button("Levels").clicked() {
                        apply_effect_by_name(app, "Levels");
                        ui.close_menu();
                    }
                    if ui.button("Hue/Saturation").clicked() {
                        apply_effect_by_name(app, "Hue/Saturation");
                        ui.close_menu();
                    }
                    if ui.button("Vibrance").clicked() {
                        apply_effect_by_name(app, "Vibrance");
                        ui.close_menu();
                    }
                });
                ui.menu_button("OpenFX Plugins", |ui| {
                    if ui.button("Scan Standard Paths...").on_hover_text("Search /Library/OFX/Plugins and $OFX_PLUGIN_PATH for plugin bundles, then probe their ABI exports").clicked() {
                        let found = crate::core::openfx_bridge::discover_all_ofx_plugins();
                        if found.is_empty() {
                            app.toasts.info("No OpenFX plugins found in standard paths");
                        } else {
                            let mut loadable = 0usize;
                            let mut names: Vec<String> = Vec::new();
                            for p in &found {
                                if let crate::core::openfx_bridge::OfxProbeResult::Loaded { plugin_version, .. } =
                                    crate::core::openfx_bridge::probe_ofx_plugin(&p.binary_path)
                                {
                                    loadable += 1;
                                    if names.len() < 4 {
                                        names.push(format!("{} v{}.{}", p.name, plugin_version.0, plugin_version.1));
                                    }
                                }
                            }
                            if loadable > 0 {
                                app.toasts.info(format!(
                                    "{loadable}/{} OFX effect(s) loadable: {}{}",
                                    found.len(),
                                    names.join(", "),
                                    if loadable > 4 { "…" } else { "" }
                                ));
                            } else {
                                app.toasts.info(format!("Found {} bundle(s), none expose OfxImageEffectAPI", found.len()));
                            }
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Stylize", |ui| {
                    if ui.button("Glow").clicked() {
                        apply_effect_by_name(app, "Glow");
                        ui.close_menu();
                    }
                    if ui.button("Lens Flare (GPU)").clicked() {
                        apply_effect_by_name(app, "Lens Flare");
                        ui.close_menu();
                    }
                    if ui.button("Vignette").clicked() {
                        apply_effect_by_name(app, "Vignette");
                        ui.close_menu();
                    }
                    if ui.button("Film Grain").clicked() {
                        apply_effect_by_name(app, "Film Grain");
                        ui.close_menu();
                    }
                    if ui.button("Drop Shadow").clicked() {
                        apply_effect_by_name(app, "Drop Shadow");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Distort", |ui| {
                    if ui.button("Twirl").clicked() {
                        apply_effect_by_name(app, "Twirl");
                        ui.close_menu();
                    }
                    if ui.button("Bulge").clicked() {
                        apply_effect_by_name(app, "Bulge");
                        ui.close_menu();
                    }
                    if ui.button("Mesh Warp").clicked() {
                        apply_effect_by_name(app, "Mesh Warp");
                        ui.close_menu();
                    }
                    if ui.button("Chromatic Aberration").clicked() {
                        apply_effect_by_name(app, "Chromatic Aberration");
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button("Animation", |ui| {
                if ui.add(egui::Button::new("Easy Ease").shortcut_text("F9")).clicked() {
                    if let Some(idx) = app.selected_layer_idx {
                        app.modify_project(move |p| {
                            if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                                l.easy_ease_transform();
                            }
                        });
                        crate::core::frame_cache::bump_version();
                        app.toasts.info("Easy Ease applied to transform keyframes");
                    } else {
                        app.toasts.info("Select a layer first");
                    }
                    ui.close_menu();
                }
                if ui.button("Sequence Layers...").on_hover_text("Arrange selected layers to play one after another").clicked() {
                    app.show_sequence_layers = true;
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut app.show_grid, "Show Grid");
                ui.checkbox(&mut app.show_guides, "Show Safe Zones");
                ui.checkbox(&mut app.show_handles, "Show Handles");
                ui.separator();
                // ── GPU Compute (experimental) ──
                let mut gpu_fx = crate::core::compute_pipeline::gpu_effects_enabled();
                if ui.checkbox(&mut gpu_fx, "GPU Compute Effects (beta)").changed() {
                    crate::core::compute_pipeline::set_gpu_effects_enabled(gpu_fx);
                    app.history.current_mut().use_gpu_compute = gpu_fx;
                    if gpu_fx {
                        match crate::core::compute_pipeline::global() {
                            Some(ctx) => app.toasts.info(format!("GPU compute: {}", ctx.backend_label())),
                            None => {
                                app.toasts.error("No GPU adapter — staying on CPU");
                                crate::core::compute_pipeline::set_gpu_effects_enabled(false);
                                app.history.current_mut().use_gpu_compute = false;
                            }
                        }
                    } else {
                        app.toasts.info("GPU compute off — deterministic CPU rendering");
                    }
                }
                if crate::core::compute_pipeline::gpu_effects_enabled() {
                    ui.label(
                        egui::RichText::new(crate::core::compute_pipeline::timing_hud_line())
                            .small()
                            .color(egui::Color32::from_rgb(110, 110, 110)),
                    );
                }
                if ui.button("Reset Timeline Zoom (100%)").clicked() {
                    app.timeline_zoom = 1.0;
                    ui.close_menu();
                }
                if ui.button("Purge All RAM Cache").clicked() {
                    app.frame_cache.invalidate_all();
                    crate::core::frame_cache::bump_version();
                    ui.close_menu();
                }
                ui.menu_button("Workspaces", |ui| {
                    if ui.button("Standard").clicked() {
                        app.right_tab_idx = 0;
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                    if ui.button("Motion Graphics").clicked() {
                        app.right_tab_idx = 0;
                        app.show_graph_editor = true;
                        ui.close_menu();
                    }
                    if ui.button("VFX & Color").clicked() {
                        app.right_tab_idx = 30; // Effect Controls
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                    if ui.button("Audio Editing").clicked() {
                        app.right_tab_idx = 7; // Audio Panel
                        app.show_graph_editor = false;
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button("Help", |ui| {
                if ui.add(egui::Button::new("⚙ Preferences…").shortcut_text("Cmd+,")).clicked() {
                    app.show_preferences = true;
                    ui.close_menu();
                }
                if ui.button("✨ Show Welcome Screen").clicked() {
                    app.show_welcome = true;
                    ui.close_menu();
                }
                if ui.button("Keyboard Shortcuts Reference...").clicked() {
                    let help_id = egui::Id::new("show_shortcuts_modal");
                    ctx.data_mut(|d| d.insert_temp(help_id, true));
                    ui.close_menu();
                }
            });
        });
    });

    let help_id = egui::Id::new("show_shortcuts_modal");
    let mut show_help = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(help_id, || false));
    if show_help {
        egui::Window::new("Keyboard Shortcuts Reference")
            .open(&mut show_help)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("After Effects OSS - Shortcuts");
                ui.separator();
                egui::Grid::new("shortcuts_grid").striped(true).show(ui, |ui| {
                    ui.label("Spacebar"); ui.label("Play / Pause RAM Preview"); ui.end_row();
                    ui.label("V"); ui.label("Selection Tool"); ui.end_row();
                    ui.label("H"); ui.label("Hand Tool (Pan)"); ui.end_row();
                    ui.label("Z"); ui.label("Zoom Tool"); ui.end_row();
                    ui.label("W"); ui.label("Rotation Tool"); ui.end_row();
                    ui.label("Y"); ui.label("Anchor Point Tool"); ui.end_row();
                    ui.label("Cmd + Z"); ui.label("Undo"); ui.end_row();
                    ui.label("Cmd + Shift + Z"); ui.label("Redo"); ui.end_row();
                    ui.label("J / K / L"); ui.label("Prev Keyframe / Stop / Next Keyframe"); ui.end_row();
                    ui.label("Arrow Keys"); ui.label("Nudge Selected Layer 1px (Shift = 10px)"); ui.end_row();
                    ui.label("PageUp / PageDown"); ui.label("Step Frame Backward / Forward"); ui.end_row();
                    ui.label("Home / End"); ui.label("First / Last Frame"); ui.end_row();
                    ui.label("B / N"); ui.label("Set Work Area Start / End"); ui.end_row();
                    ui.label("= / -"); ui.label("Timeline Zoom In / Out (Cmd+Scroll on ruler)"); ui.end_row();
                    ui.label("F9"); ui.label("Apply Easy Ease to Keyframes"); ui.end_row();
                    ui.label("Click / Shift+Click"); ui.label("Select / Add to Selection (keyframes)"); ui.end_row();
                    ui.label(", / ."); ui.label("Nudge Selected Keyframes (Shift = x10)"); ui.end_row();
                    ui.label("Delete"); ui.label("Delete Selected Keyframes"); ui.end_row();
                    ui.label("Cmd + C / V"); ui.label("Copy / Paste Selected Keyframes"); ui.end_row();
                    ui.label("Cmd + A"); ui.label("Select All Keyframes of Layer"); ui.end_row();
                    ui.label("Esc"); ui.label("Deselect (Keyframes, then Layers)"); ui.end_row();
                    ui.label("M"); ui.label("Add / Remove Timeline Marker"); ui.end_row();
                    ui.label("[ / ]"); ui.label("Jump to Prev / Next Marker"); ui.end_row();
                });
            });
        ctx.data_mut(|d| d.insert_temp(help_id, show_help));
    }

    // 📦 Pre-Compose Dialog (Cmd+Shift+C)
    crate::ui::precompose_dialog::draw_precompose_dialog(app, ctx);
    crate::ui::recovery_dialog::draw_recovery_dialog(app, ctx);
    crate::ui::sequence_layers_dialog::draw_sequence_layers_dialog(app, ctx);
}

fn apply_effect_by_name(app: &mut crate::AfterEffectsApp, effect_name: &str) {
    if let Some(idx) = app.selected_layer_idx {
        let comp = app.history.current_mut().active_composition_mut();
        if idx < comp.layers.len() {
            let layer = &mut comp.layers[idx];
            let len = layer.effects.len();
            let effect = match effect_name {
                "Slider Control" => crate::core::timeline::Effect {
                    id: format!("slider_{}", len), name: "Slider Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::SliderControl {
                        value: crate::core::property::Animatable::new_constant(50.0),
                    }, enabled: true,
                },
                "Angle Control" => crate::core::timeline::Effect {
                    id: format!("angle_{}", len), name: "Angle Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::AngleControl {
                        angle_degrees: crate::core::property::Animatable::new_constant(0.0),
                    }, enabled: true,
                },
                "Point Control" => crate::core::timeline::Effect {
                    id: format!("point_{}", len), name: "Point Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::PointControl {
                        point: crate::core::property::Animatable::new_constant([960.0, 540.0]),
                    }, enabled: true,
                },
                "Color Control" => crate::core::timeline::Effect {
                    id: format!("color_{}", len), name: "Color Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::ColorControl {
                        color: crate::core::property::Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                    }, enabled: true,
                },
                "Checkbox Control" => crate::core::timeline::Effect {
                    id: format!("checkbox_{}", len), name: "Checkbox Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::CheckboxControl {
                        checked: false,
                    }, enabled: true,
                },
                "Dropdown Control" => crate::core::timeline::Effect {
                    id: format!("dropdown_{}", len), name: "Dropdown Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::DropdownControl {
                        value: 0,
                        options: vec!["Option 1".to_string(), "Option 2".to_string(), "Option 3".to_string()],
                    }, enabled: true,
                },
                "3D Point Control" => crate::core::timeline::Effect {
                    id: format!("point3d_{}", len), name: "3D Point Control".to_string(),
                    effect_type: crate::core::timeline::EffectType::Point3DControl {
                        point: crate::core::property::Animatable::new_constant([0.0, 0.0, 0.0]),
                    }, enabled: true,
                },
                "Lens Flare" => crate::core::timeline::Effect {
                    id: format!("flare_{}", len), name: "Lens Flare".to_string(),
                    effect_type: crate::core::timeline::EffectType::LensFlare {
                        enabled: crate::core::property::Animatable::new_constant(1.0),
                        position_x: crate::core::property::Animatable::new_constant(0.5),
                        position_y: crate::core::property::Animatable::new_constant(0.35),
                        intensity: crate::core::property::Animatable::new_constant(1.0),
                        threshold: crate::core::property::Animatable::new_constant(0.8),
                        color: crate::core::property::Animatable::new_constant([1.0, 0.95, 0.9, 1.0]),
                        link_to_light: None,
                    }, enabled: true,
                },
                "Gaussian Blur" => crate::core::timeline::Effect {
                    id: format!("blur_{}", len), name: "Gaussian Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::GaussianBlur {
                        blur_radius: crate::core::property::Animatable::new_constant(5.0),
                    }, enabled: true,
                },
                "Directional Blur" => crate::core::timeline::Effect {
                    id: format!("dirblur_{}", len), name: "Directional Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::DirectionalBlur {
                        angle: crate::core::property::Animatable::new_constant(0.0),
                        length: crate::core::property::Animatable::new_constant(10.0),
                    }, enabled: true,
                },
                "Radial Blur" => crate::core::timeline::Effect {
                    id: format!("radblur_{}", len), name: "Radial Blur".to_string(),
                    effect_type: crate::core::timeline::EffectType::RadialBlur {
                        amount: crate::core::property::Animatable::new_constant(10.0),
                    }, enabled: true,
                },
                "Sharpen" => crate::core::timeline::Effect {
                    id: format!("sharp_{}", len), name: "Sharpen".to_string(),
                    effect_type: crate::core::timeline::EffectType::Sharpen {
                        amount: crate::core::property::Animatable::new_constant(50.0),
                    }, enabled: true,
                },
                "Color Tint" => crate::core::timeline::Effect {
                    id: format!("tint_{}", len), name: "Color Tint".to_string(),
                    effect_type: crate::core::timeline::EffectType::ColorTint {
                        color: crate::core::property::Animatable::new_constant([1.0, 0.2, 0.4, 1.0]),
                        intensity: crate::core::property::Animatable::new_constant(1.0),
                    }, enabled: true,
                },
                "Levels" => crate::core::timeline::Effect {
                    id: format!("levels_{}", len), name: "Levels".to_string(),
                    effect_type: crate::core::timeline::EffectType::Levels {
                        input_black: crate::core::property::Animatable::new_constant(0.0),
                        input_white: crate::core::property::Animatable::new_constant(255.0),
                        gamma: crate::core::property::Animatable::new_constant(1.0),
                        output_black: crate::core::property::Animatable::new_constant(0.0),
                        output_white: crate::core::property::Animatable::new_constant(255.0),
                    }, enabled: true,
                },
                "Hue/Saturation" => crate::core::timeline::Effect {
                    id: format!("hs_{}", len), name: "Hue/Saturation".to_string(),
                    effect_type: crate::core::timeline::EffectType::HueSaturation {
                        hue_shift: crate::core::property::Animatable::new_constant(0.0),
                        saturation: crate::core::property::Animatable::new_constant(0.0),
                        lightness: crate::core::property::Animatable::new_constant(0.0),
                    }, enabled: true,
                },
                "Vibrance" => crate::core::timeline::Effect {
                    id: format!("vib_{}", len), name: "Vibrance".to_string(),
                    effect_type: crate::core::timeline::EffectType::Vibrance {
                        amount: crate::core::property::Animatable::new_constant(50.0),
                    }, enabled: true,
                },
                "Glow" => crate::core::timeline::Effect {
                    id: format!("glow_{}", len), name: "Glow".to_string(),
                    effect_type: crate::core::timeline::EffectType::Glow {
                        threshold: crate::core::property::Animatable::new_constant(60.0),
                        radius: crate::core::property::Animatable::new_constant(10.0),
                        intensity: crate::core::property::Animatable::new_constant(1.0),
                        color: crate::core::property::Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                    }, enabled: true,
                },
                "Vignette" => crate::core::timeline::Effect {
                    id: format!("vig_{}", len), name: "Vignette".to_string(),
                    effect_type: crate::core::timeline::EffectType::Vignette {
                        intensity: crate::core::property::Animatable::new_constant(0.5),
                        roundness: crate::core::property::Animatable::new_constant(0.5),
                        feather: crate::core::property::Animatable::new_constant(0.5),
                        color: crate::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                    }, enabled: true,
                },
                "Film Grain" => crate::core::timeline::Effect {
                    id: format!("grain_{}", len), name: "Film Grain".to_string(),
                    effect_type: crate::core::timeline::EffectType::FilmGrain {
                        intensity: crate::core::property::Animatable::new_constant(0.1),
                        grain_size: 2.0,
                        color_film: false,
                    }, enabled: true,
                },
                "Drop Shadow" => crate::core::timeline::Effect {
                    id: format!("ds_{}", len), name: "Drop Shadow".to_string(),
                    effect_type: crate::core::timeline::EffectType::DropShadow {
                        color: crate::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                        opacity: crate::core::property::Animatable::new_constant(75.0),
                        direction: crate::core::property::Animatable::new_constant(120.0),
                        distance: crate::core::property::Animatable::new_constant(5.0),
                        softness: crate::core::property::Animatable::new_constant(5.0),
                    }, enabled: true,
                },
                "Twirl" => crate::core::timeline::Effect {
                    id: format!("twirl_{}", len), name: "Twirl".to_string(),
                    effect_type: crate::core::timeline::EffectType::Twirl {
                        angle: crate::core::property::Animatable::new_constant(50.0),
                        radius: crate::core::property::Animatable::new_constant(100.0),
                    }, enabled: true,
                },
                "Bulge" => crate::core::timeline::Effect {
                    id: format!("bulge_{}", len), name: "Bulge".to_string(),
                    effect_type: crate::core::timeline::EffectType::Bulge {
                        amount: crate::core::property::Animatable::new_constant(50.0),
                        radius: crate::core::property::Animatable::new_constant(100.0),
                    }, enabled: true,
                },
                "Mesh Warp" => crate::core::timeline::Effect {
                    id: format!("meshwarp_{}", len), name: "Mesh Warp".to_string(),
                    effect_type: crate::core::timeline::EffectType::MeshWarp {
                        top_left: crate::core::property::Animatable::new_constant([0.0, 0.0]),
                        top_right: crate::core::property::Animatable::new_constant([1.0, 0.0]),
                        bottom_left: crate::core::property::Animatable::new_constant([0.0, 1.0]),
                        bottom_right: crate::core::property::Animatable::new_constant([1.0, 1.0]),
                    }, enabled: true,
                },
                "Corner Pin" => {
                    let (cw, ch) = (comp.width as f32, comp.height as f32);
                    crate::core::timeline::Effect {
                        id: format!("cornerpin_{}", len), name: "Corner Pin".to_string(),
                        effect_type: crate::core::timeline::EffectType::CornerPin {
                            top_left: crate::core::property::Animatable::new_constant([0.0, 0.0]),
                            top_right: crate::core::property::Animatable::new_constant([cw, 0.0]),
                            bottom_right: crate::core::property::Animatable::new_constant([cw, ch]),
                            bottom_left: crate::core::property::Animatable::new_constant([0.0, ch]),
                        }, enabled: true,
                    }
                }
                "Chromatic Aberration" => crate::core::timeline::Effect {
                    id: format!("ca_{}", len), name: "Chromatic Aberration".to_string(),
                    effect_type: crate::core::timeline::EffectType::ChromaticAberration {
                        shift_r: crate::core::property::Animatable::new_constant(5.0),
                        shift_b: crate::core::property::Animatable::new_constant(-5.0),
                        edge_falloff: crate::core::property::Animatable::new_constant(0.5),
                    }, enabled: true,
                },
                _ => return,
            };
            layer.effects.push(effect);
            crate::core::frame_cache::bump_version();
            app.toasts.info(format!("Added '{}' effect", effect_name));
        }
    }
}
