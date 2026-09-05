use crate::core::timeline::{Layer, LayerType, ProjectItem, ProjectItemType};
use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Project");
    ui.separator();

    // ── Asset Search Filter ──
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(&mut app.project_search_query).hint_text("Search bin..."),
        );
    });
    let query = app.project_search_query.to_lowercase();

    // Read current state without cloning upfront
    let current_project = app.history.current();

    // ── AE Top Asset Thumbnail & Meta Box ──
    let selected_asset_idx: Option<usize> = ui
        .ctx()
        .data_mut(|d| d.get_temp(egui::Id::new("selected_project_asset")));
    if let Some(idx) = selected_asset_idx {
        if let Some(item) = current_project.assets.get(idx) {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    // Render miniature preview square box
                    let (thumb_rect, _) =
                        ui.allocate_exact_size(egui::vec2(44.0, 32.0), egui::Sense::hover());
                    ui.painter().rect_filled(thumb_rect, 2.0, colors::BG_DARK);
                    ui.painter().rect_stroke(
                        thumb_rect,
                        2.0,
                        egui::Stroke::new(1.0_f32, colors::BORDER_STRONG),
                    );
                    let center = thumb_rect.center();
                    ui.painter().text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::FILM_STRIP,
                        egui::FontId::monospace(14.0),
                        colors::TEXT_ACCENT,
                    );

                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&item.name).strong().size(13.0));
                        match &item.item_type {
                            ProjectItemType::Composition { comp_idx } => {
                                if let Some(c) = current_project.compositions.get(*comp_idx) {
                                    ui.small(format!(
                                        "{} x {} (1.00) | {:.2} fps | {}",
                                        c.width, c.height, c.fps, c.name
                                    ));
                                }
                            }
                            ProjectItemType::Image { width, height, .. } => {
                                ui.small(format!("{} x {} px | RGB 8-bpc", width, height));
                            }
                            ProjectItemType::Video { duration_sec, .. } => {
                                ui.small(format!("Video | {:.1}s", duration_sec));
                            }
                            ProjectItemType::Audio { duration_sec, .. } => {
                                ui.small(format!(
                                    "44.1 kHz / 16-bit / Stereo | {:.1}s",
                                    duration_sec
                                ));
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
    let mut remove_unused_requested = false;
    let mut reduce_project_requested = false;

    ui.horizontal(|ui| {
        if custom_widgets::ae_button(ui, "+ New Comp")
            .on_hover_text("Create New Composition")
            .clicked()
        {
            add_comp_requested = true;
        }

        if custom_widgets::ae_button(ui, "+ Import File...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Media Footage",
                    &["png", "jpg", "jpeg", "webp", "wav", "mp3", "mp4"],
                )
                .pick_file()
            {
                import_file_requested = Some(path);
            }
        }

        if custom_widgets::ae_button(ui, "+ New Folder").clicked() {
            add_folder_requested = true;
        }

        if custom_widgets::ae_button(ui, "🧹 Remove Unused")
            .on_hover_text("Remove unused footage and assets from project")
            .clicked()
        {
            remove_unused_requested = true;
        }

        if custom_widgets::ae_button(ui, "🗜 Reduce Project")
            .on_hover_text("Keep only the active composition and its dependencies")
            .clicked()
        {
            reduce_project_requested = true;
        }
    });

    ui.add_space(6.0);
    ui.separator();

    // ── Project Items List (Assets & Compositions Bin Tree) ──
    let mut selected_idx_update: Option<Option<usize>> = None;
    let mut add_to_timeline_item: Option<ProjectItem> = None;

    // Asset mutation requests collected during the render pass
    let mut move_to_folder: Option<(usize, Option<String>)> = None;

    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            // Pre-compute folder ids for nesting
            let folders: Vec<(usize, String, String)> = current_project
                .assets
                .iter()
                .enumerate()
                .filter_map(|(i, it)| match &it.item_type {
                    ProjectItemType::Folder { .. } => Some((i, it.id.clone(), it.name.clone())),
                    _ => None,
                })
                .collect();
            let in_folder = |i: usize| -> Option<String> {
                current_project
                    .assets
                    .get(i)
                    .and_then(|a| a.parent_folder.clone())
            };

            // Root-level items first (folders rendered as headers below their root slot)
            for (i, item) in current_project.assets.iter().enumerate() {
                if !query.is_empty() && !item.name.to_lowercase().contains(&query) {
                    continue;
                }
                if matches!(item.item_type, ProjectItemType::Folder { .. }) {
                    continue;
                }
                if in_folder(i).is_some() && query.is_empty() {
                    continue; // nested under its folder header below
                }

                let is_selected = selected_asset_idx == Some(i);
                draw_asset_row(
                    ui,
                    i,
                    item,
                    is_selected,
                    &mut selected_idx_update,
                    &mut add_to_timeline_item,
                    &mut move_to_folder,
                    &folders,
                );
            }

            // Folder bins with nested children
            for (fi, fid, fname) in &folders {
                if !query.is_empty() && !fname.to_lowercase().contains(&query) {
                    continue;
                }
                let children: Vec<(usize, &ProjectItem)> = current_project
                    .assets
                    .iter()
                    .enumerate()
                    .filter(|(_i, it)| it.parent_folder.as_deref() == Some(fid.as_str()))
                    .collect();

                egui::CollapsingHeader::new(format!(
                    "{} {} ({})",
                    egui_phosphor::regular::FOLDER_NOTCH,
                    fname,
                    children.len()
                ))
                .default_open(!query.is_empty())
                .show(ui, |ui| {
                    if children.is_empty() {
                        ui.small("empty bin");
                    }
                    for (ci, child) in children {
                        let is_sel = selected_asset_idx == Some(ci);
                        draw_asset_row(
                            ui,
                            ci,
                            child,
                            is_sel,
                            &mut selected_idx_update,
                            &mut add_to_timeline_item,
                            &mut move_to_folder,
                            &folders,
                        );
                    }
                    // Un-bin shortcut on the folder itself
                    if ui.small_button("⤴ Move selection out").clicked() {
                        if let Some(sel) = selected_asset_idx {
                            if let Some(it) = current_project.assets.get(sel) {
                                if it.parent_folder.as_deref() == Some(fname.as_str()) {
                                    move_to_folder = Some((sel, None));
                                }
                            }
                        }
                    }
                    let _ = fi;
                });
            }
        });

    if let Some(update) = selected_idx_update {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("selected_project_asset"), update));
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
                    ProjectItemType::Image {
                        path,
                        width,
                        height,
                    } => {
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
                        ui.small(format!(
                            "Color: R:{:.0} G:{:.0} B:{:.0}",
                            color[0] * 255.0,
                            color[1] * 255.0,
                            color[2] * 255.0
                        ));
                    }
                    ProjectItemType::Folder { .. } => {
                        ui.small("Project Bin Directory");
                    }
                }

                ui.add_space(4.0);
                if custom_widgets::ae_button(ui, "Add to Active Comp").clicked() {
                    add_to_timeline_item = Some(item.clone());
                }
            });
        }
    }

    // Lazy mutation: Clone project ONLY on action trigger!
    if add_comp_requested
        || import_file_requested.is_some()
        || add_folder_requested
        || add_to_timeline_item.is_some()
    {
        let mut temp_project = app.history.current().clone();
        let mut changed = false;

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
                ProjectItemType::Composition {
                    comp_idx: temp_project.compositions.len() - 1,
                },
            ));
        }

        if let Some(path) = import_file_requested {
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let file_path = path.to_string_lossy().to_string();
            let item_count = temp_project.assets.len() + 1;

            let item_type = if file_name.ends_with(".wav") || file_name.ends_with(".mp3") {
                ProjectItemType::Audio {
                    path: file_path,
                    duration_sec: 10.0,
                }
            } else {
                ProjectItemType::Image {
                    path: file_path,
                    width: 1920,
                    height: 1080,
                }
            };

            temp_project.assets.push(ProjectItem::new(
                format!("imported_{}", item_count),
                file_name,
                item_type,
            ));
        }

        if let Some((idx, target)) = move_to_folder {
            if let Some(it) = temp_project.assets.get_mut(idx) {
                it.parent_folder = target;
                changed = true;
                crate::core::frame_cache::bump_version();
            }
        }

        if add_folder_requested {
            let folder_count = temp_project.assets.len() + 1;
            temp_project.assets.push(ProjectItem::new(
                format!("folder_{}", folder_count),
                format!("Assets Folder {}", folder_count),
                ProjectItemType::Folder {
                    name: format!("Folder {}", folder_count),
                },
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
                ProjectItemType::Video {
                    path,
                    duration_sec: _,
                } => {
                    // Import the video (frame extraction) and add a Video layer
                    let media_dir = std::env::temp_dir().join("kagari_media").join(
                        std::path::Path::new(&path)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "video".into()),
                    );
                    match crate::core::video_import::import_video(
                        &path,
                        &media_dir,
                        comp.fps as f32,
                    ) {
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
                            vl.out_frame = vl.out_frame.min(asset.frame_count.max(1));
                            vl
                        }
                        Err(err) => {
                            eprintln!("video import failed: {}", err);
                            Layer::new(
                                format!("layer_video_failed_{}", len),
                                format!("{} (import failed)", item.name),
                                LayerType::Solid {
                                    color: [0.6, 0.1, 0.1, 1.0],
                                },
                                comp.duration_frames,
                            )
                        }
                    }
                }
                _ => Layer::new(
                    format!("layer_gen_{}", len),
                    item.name,
                    LayerType::Solid {
                        color: [0.5, 0.5, 0.5, 1.0],
                    },
                    comp.duration_frames,
                ),
            };
            comp.add_layer(new_layer);
            changed = true;
        }

        if remove_unused_requested {
            let mut used_names = std::collections::HashSet::new();
            for c in &temp_project.compositions {
                for l in &c.layers {
                    used_names.insert(l.name.clone());
                    match &l.layer_type {
                        LayerType::Image { path } => {
                            used_names.insert(path.clone());
                        }
                        LayerType::Video { source, .. } => {
                            used_names.insert(source.clone());
                        }
                        LayerType::Audio { path, .. } => {
                            used_names.insert(path.clone());
                        }
                        _ => {}
                    }
                }
            }
            let before_count = temp_project.assets.len();
            temp_project.assets.retain(|a| {
                matches!(
                    a.item_type,
                    ProjectItemType::Composition { .. } | ProjectItemType::Folder { .. }
                ) || used_names.contains(&a.name)
            });
            let removed = before_count.saturating_sub(temp_project.assets.len());
            app.toasts.info(format!("Removed {} unused items", removed));
            changed = true;
        }

        if reduce_project_requested {
            let active_idx = temp_project.active_composition_idx;
            if active_idx < temp_project.compositions.len() {
                let keep_comp = temp_project.compositions[active_idx].clone();
                temp_project.compositions = vec![keep_comp];
                temp_project.active_composition_idx = 0;
                app.toasts
                    .info("Project reduced to active composition and its dependencies");
                changed = true;
            }
        }

        // Process Replace Footage request
        let replace_info: Option<(usize, String, String)> = ui.ctx().data_mut(|d| {
            let idx = d.remove_temp::<usize>(egui::Id::new("ae_replace_footage_asset_idx"));
            let path = d.remove_temp::<String>(egui::Id::new("ae_replace_footage_path"));
            let name = d.remove_temp::<String>(egui::Id::new("ae_replace_footage_name"));
            match (idx, path, name) {
                (Some(i), Some(p), Some(n)) => Some((i, p, n)),
                _ => None,
            }
        });

        if let Some((asset_idx, new_path, new_name)) = replace_info {
            if let Some(asset) = temp_project.assets.get_mut(asset_idx) {
                let old_name = asset.name.clone();
                asset.name = new_name.clone();
                match &mut asset.item_type {
                    ProjectItemType::Image { path, .. } => *path = new_path.clone(),
                    ProjectItemType::Video { path, .. } => *path = new_path.clone(),
                    ProjectItemType::Audio { path, .. } => *path = new_path.clone(),
                    _ => {}
                }
                // Update layers using this footage
                for comp in &mut temp_project.compositions {
                    for layer in &mut comp.layers {
                        if layer.name == old_name {
                            layer.name = new_name.clone();
                        }
                        match &mut layer.layer_type {
                            LayerType::Image { path } if path.contains(&old_name) => {
                                *path = new_path.clone();
                            }
                            LayerType::Audio { path, .. } if path.contains(&old_name) => {
                                *path = new_path.clone();
                            }
                            _ => {}
                        }
                    }
                }
                app.toasts
                    .info(format!("Replaced footage: '{}' → '{}'", old_name, new_name));
                changed = true;
            }
        }

        if changed {
            app.history.commit(temp_project);
            crate::core::frame_cache::bump_version();
        }
    }
}

/// One row of the asset list; shared by root listing and folder bins.
#[allow(clippy::too_many_arguments)]
fn draw_asset_row(
    ui: &mut egui::Ui,
    i: usize,
    item: &ProjectItem,
    is_selected: bool,
    selected_idx_update: &mut Option<Option<usize>>,
    add_to_timeline_item: &mut Option<ProjectItem>,
    move_to_folder: &mut Option<(usize, Option<String>)>,
    folders: &[(usize, String, String)],
) {
    let row_probe =
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 20.0));
    if !ui.is_rect_visible(row_probe) {
        ui.add_space(20.0);
        return;
    }

    use ProjectItemType as T;
    let (icon_str, item_tag) = match &item.item_type {
        T::Composition { .. } => (egui_phosphor::regular::PACKAGE, "Composition"),
        T::Image { .. } => (egui_phosphor::regular::IMAGE, "Footage Image"),
        T::Video { .. } => (egui_phosphor::regular::FILM_STRIP, "Footage Video"),
        T::Audio { .. } => (egui_phosphor::regular::WAVEFORM, "Audio File"),
        T::Solid { .. } => (egui_phosphor::regular::SQUARE, "Solid Color"),
        T::Folder { .. } => (egui_phosphor::regular::FOLDER_NOTCH, "Folder Bin"),
    };

    let mut replace_footage_req: Option<(usize, std::path::PathBuf)> = None;

    ui.horizontal(|ui| {
        let response = ui.selectable_label(is_selected, format!("{} {}", icon_str, item.name));
        if response.clicked() {
            *selected_idx_update = Some(Some(i));
        }
        if response.double_clicked() {
            *add_to_timeline_item = Some(item.clone());
        }
        response.context_menu(|ui| {
            if ui.button("➕ Add to Composition").clicked() {
                *add_to_timeline_item = Some(item.clone());
                ui.close_menu();
            }
            if ui.button("🔄 Replace Footage...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Media Footage",
                        &["png", "jpg", "jpeg", "webp", "wav", "mp3", "mp4"],
                    )
                    .pick_file()
                {
                    replace_footage_req = Some((i, path));
                }
                ui.close_menu();
            }
        });
        ui.weak(format!("({})", item_tag));

        // Move-to-bin dropdown
        if !folders.is_empty() {
            let mb = ui.menu_button("📁→", |ui| {
                if ui
                    .selectable_label(item.parent_folder.is_none(), "(project root)")
                    .clicked()
                {
                    *move_to_folder = Some((i, None));
                    ui.close_menu();
                }
                for (_, fid, fname) in folders {
                    let inside = item.parent_folder.as_deref() == Some(fname.as_str());
                    if ui.selectable_label(inside, fname).clicked() {
                        *move_to_folder = Some((i, Some(fid.clone())));
                        ui.close_menu();
                    }
                }
            });
            mb.response
                .on_hover_text("Move this asset into/out of a bin");
        }
    });

    if let Some((asset_idx, new_path)) = replace_footage_req {
        let new_file_name = new_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let new_path_str = new_path.to_string_lossy().to_string();
        // Request replace footage handling
        *selected_idx_update = Some(Some(asset_idx));
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new("ae_replace_footage_asset_idx"), asset_idx);
            d.insert_temp(egui::Id::new("ae_replace_footage_path"), new_path_str);
            d.insert_temp(egui::Id::new("ae_replace_footage_name"), new_file_name);
        });
    }
}
