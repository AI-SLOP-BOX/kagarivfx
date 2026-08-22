use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{ProjectItem, ProjectItemType, Layer, LayerType};

pub fn draw(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Project");
    ui.separator();

    // ── Asset Search Filter ──
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(egui::TextEdit::singleline(&mut app.project_search_query).hint_text("Search bin..."));
    });
    let query = app.project_search_query.to_lowercase();


    // Read current state without cloning upfront
    let current_project = app.history.current();

    // ── AE Top Asset Thumbnail & Meta Box ──
    let selected_asset_idx: Option<usize> = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("selected_project_asset")));
    if let Some(idx) = selected_asset_idx {
        if let Some(item) = current_project.assets.get(idx) {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    // Render miniature preview square box
                    let (thumb_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 32.0), egui::Sense::hover());
                    ui.painter().rect_filled(thumb_rect, 2.0, egui::Color32::from_gray(30));
                    ui.painter().rect_stroke(thumb_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(70)));
                    let center = thumb_rect.center();
                    ui.painter().text(center, egui::Align2::CENTER_CENTER, "🎞", egui::FontId::monospace(14.0), egui::Color32::from_rgb(0, 180, 255));

                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&item.name).strong().size(13.0));
                        match &item.item_type {
                            ProjectItemType::Composition { comp_idx } => {
                                if let Some(c) = current_project.compositions.get(*comp_idx) {
                                    ui.small(format!("{} x {} (1.00) | {:.2} fps | {}", c.width, c.height, c.fps, c.name));
                                }
                            }
                            ProjectItemType::Image { width, height, .. } => {
                                ui.small(format!("{} x {} px | RGB 8-bpc", width, height));
                            }
                            ProjectItemType::Video { duration_sec, .. } => {
                                ui.small(format!("Video | {:.1}s", duration_sec));
                            }
                            ProjectItemType::Audio { duration_sec, .. } => {
                                ui.small(format!("44.1 kHz / 16-bit / Stereo | {:.1}s", duration_sec));
                            }
                            ProjectItemType::Solid { .. } => {
                                ui.small("Solid Color Layer Footage");
                            }
                            ProjectItemType::Folder { .. } => {
                                ui.small("Folder Directory Bin");
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    }

    let mut add_comp_requested = false;
    let mut import_file_requested: Option<std::path::PathBuf> = None;
    let mut add_folder_requested = false;

    ui.horizontal(|ui| {
        if ui.button("+ New Comp").on_hover_text("Create New Composition").clicked() {
            add_comp_requested = true;
        }

        if ui.button("+ Import File...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Media Footage", &["png", "jpg", "jpeg", "webp", "wav", "mp3", "mp4"])
                .pick_file()
            {
                import_file_requested = Some(path);
            }
        }

        if ui.button("+ New Folder").clicked() {
            add_folder_requested = true;
        }
    });

    ui.add_space(6.0);
    ui.separator();

    // ── Project Items List (Assets & Compositions Bin Tree) ──
    let mut selected_idx_update: Option<Option<usize>> = None;
    let mut add_to_timeline_item: Option<ProjectItem> = None;

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        for (i, item) in current_project.assets.iter().enumerate() {
            if !query.is_empty() && !item.name.to_lowercase().contains(&query) {
                continue;
            }

            let is_selected = selected_asset_idx == Some(i);
            let (icon_str, item_tag) = match &item.item_type {
                ProjectItemType::Composition { .. } => ("[COMP]", "Composition"),
                ProjectItemType::Image { .. } => ("[IMG]", "Footage Image"),
                ProjectItemType::Video { .. } => ("[VID]", "Footage Video"),
                ProjectItemType::Audio { .. } => ("[AUD]", "Audio File"),
                ProjectItemType::Solid { .. } => ("[SOL]", "Solid Color"),
                ProjectItemType::Folder { .. } => ("[DIR]", "Folder Bin"),
            };

            if let ProjectItemType::Folder { .. } = &item.item_type {
                egui::CollapsingHeader::new(format!("[DIR] {}", item.name))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.small("Drop assets into folder to categorize");
                    });
            } else {
                ui.horizontal(|ui| {
                    let response = ui.selectable_label(is_selected, format!("{} {}", icon_str, item.name));
                    if response.clicked() {
                        selected_idx_update = Some(Some(i));
                    }

                    if response.double_clicked() {
                        add_to_timeline_item = Some(item.clone());
                    }

                    ui.weak(format!("({})", item_tag));
                });
            }
        }
    });

    if let Some(update) = selected_idx_update {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("selected_project_asset"), update));
    }

    ui.add_space(8.0);
    ui.separator();

    // ── Selected Asset Metadata Preview Header ──
    if let Some(idx) = selected_asset_idx {
        if let Some(item) = current_project.assets.get(idx) {
            ui.group(|ui| {
                ui.label(egui::RichText::new(&item.name).strong());
                match &item.item_type {
                    ProjectItemType::Composition { comp_idx } => {
                        if let Some(c) = current_project.compositions.get(*comp_idx) {
                            ui.small(format!("Resolution: {} x {} px", c.width, c.height));
                            ui.small(format!("Frame Rate: {} fps", c.fps));
                            ui.small(format!("Duration: {} frames", c.duration_frames));
                        }
                    }
                    ProjectItemType::Image { path, width, height } => {
                        ui.small(format!("File: {}", path));
                        ui.small(format!("Dimensions: {} x {} px", width, height));
                    }
                    ProjectItemType::Video { path, duration_sec } => {
                        ui.small(format!("File: {}", path));
                        ui.small(format!("Duration: {:.1}s", duration_sec));
                    }
                    ProjectItemType::Audio { path, duration_sec } => {
                        ui.small(format!("File: {}", path));
                        ui.small(format!("Length: {:.2} seconds", duration_sec));
                    }
                    ProjectItemType::Solid { color } => {
                        ui.small(format!("Color: R:{:.0} G:{:.0} B:{:.0}", color[0] * 255.0, color[1] * 255.0, color[2] * 255.0));
                    }
                    ProjectItemType::Folder { .. } => {
                        ui.small("Project Bin Directory");
                    }
                }

                ui.add_space(4.0);
                if ui.button("Add to Active Comp").clicked() {
                    add_to_timeline_item = Some(item.clone());
                }
            });
        }
    }

    // Lazy mutation: Clone project ONLY on action trigger!
    if add_comp_requested || import_file_requested.is_some() || add_folder_requested || add_to_timeline_item.is_some() {
        let mut temp_project = app.history.current().clone();

        if add_comp_requested {
            let comp_len = temp_project.compositions.len() + 1;
            let new_comp = crate::core::timeline::Composition::new(
                format!("comp_{}", comp_len),
                format!("Comp {}", comp_len),
                1920,
                1080,
                30,
                300,
            );
            temp_project.compositions.push(new_comp);
            temp_project.assets.push(ProjectItem::new(
                format!("item_comp_{}", comp_len),
                format!("Comp {}", comp_len),
                ProjectItemType::Composition { comp_idx: temp_project.compositions.len() - 1 },
            ));
        }

        if let Some(path) = import_file_requested {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let file_path = path.to_string_lossy().to_string();
            let item_count = temp_project.assets.len() + 1;

            let item_type = if file_name.ends_with(".wav") || file_name.ends_with(".mp3") {
                ProjectItemType::Audio { path: file_path, duration_sec: 10.0 }
            } else {
                ProjectItemType::Image { path: file_path, width: 1920, height: 1080 }
            };

            temp_project.assets.push(ProjectItem::new(
                format!("imported_{}", item_count),
                file_name,
                item_type,
            ));
        }

        if add_folder_requested {
            let folder_count = temp_project.assets.len() + 1;
            temp_project.assets.push(ProjectItem::new(
                format!("folder_{}", folder_count),
                format!("Assets Folder {}", folder_count),
                ProjectItemType::Folder { name: format!("Folder {}", folder_count) },
            ));
        }

        if let Some(item) = add_to_timeline_item {
            let comp = temp_project.active_composition_mut();
            let len = comp.layers.len() + 1;
            let new_layer = match item.item_type {
                ProjectItemType::Image { path, .. } => Layer::new(
                    format!("layer_asset_{}", len),
                    item.name,
                    LayerType::Image { path },
                    comp.duration_frames,
                ),
                ProjectItemType::Solid { color } => Layer::new(
                    format!("layer_solid_{}", len),
                    item.name,
                    LayerType::Solid { color },
                    comp.duration_frames,
                ),
                ProjectItemType::Video { path, duration_sec: _ } => {
                    // Import the video (frame extraction) and add a Video layer
                    let media_dir = std::env::temp_dir()
                        .join("aevfx_media")
                        .join(std::path::Path::new(&path)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "video".into()));
                    match crate::core::video_import::import_video(&path, &media_dir, comp.fps as f32) {
                        Ok(asset) => {
                            let mut vl = Layer::new(
                                format!("layer_video_{}", len),
                                item.name,
                                LayerType::Video {
                                    source: asset.source_path,
                                    frames_dir: asset.frames_dir,
                                    frame_count: asset.frame_count,
                                    audio_wav: asset.audio_wav,
                                    speed: 1.0,
                                },
                                comp.duration_frames,
                            );
                            vl.out_frame = vl
                                .out_frame
                                .min(asset.frame_count.max(1));
                            vl
                        }
                        Err(err) => {
                            eprintln!("video import failed: {}", err);
                            Layer::new(
                                format!("layer_video_failed_{}", len),
                                format!("{} (import failed)", item.name),
                                LayerType::Solid { color: [0.6, 0.1, 0.1, 1.0] },
                                comp.duration_frames,
                            )
                        }
                    }
                }
                _ => Layer::new(
                    format!("layer_gen_{}", len),
                    item.name,
                    LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] },
                    comp.duration_frames,
                ),
            };
            comp.add_layer(new_layer);
        }

        app.history.commit(temp_project);
        crate::core::frame_cache::bump_version();
    }
}
