use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{ProjectItem, ProjectItemType, Layer, LayerType};
use crate::core::property::Animatable;

pub fn draw(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Project");
    ui.separator();

    // ── Asset Search Filter ──
    let search_id = egui::Id::new("project_panel_search");
    let mut search_query = ui.ctx().data_mut(|d| d.get_temp_mut_or_insert_with(search_id, String::new).clone());
    ui.horizontal(|ui| {
        ui.label("Search:");
        if ui.add(egui::TextEdit::singleline(&mut search_query).hint_text("Search bin...")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(search_id, search_query.clone()));
        }
    });
    let query = search_query.to_lowercase();
    ui.add_space(4.0);

    // ── Action Buttons: New Comp / Import Asset ──
    let mut project_changed = false;
    let mut temp_project = app.history.current().clone();

    ui.horizontal(|ui| {
        if ui.button("+ New Comp").clicked() {
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
            project_changed = true;
        }

        if ui.button("+ Import File...").clicked() {
            let item_count = temp_project.assets.len() + 1;
            temp_project.assets.push(ProjectItem::new(
                format!("imported_{}", item_count),
                format!("Asset_Footage_{}.png", item_count),
                ProjectItemType::Image {
                    path: format!("assets/footage_{}.png", item_count),
                    width: 1920,
                    height: 1080,
                },
            ));
            project_changed = true;
        }
    });

    ui.add_space(6.0);
    ui.separator();

    // ── Project Items List (Assets & Compositions Bin) ──
    let mut selected_asset_idx: Option<usize> = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("selected_project_asset")));
    let mut add_to_timeline_item: Option<ProjectItem> = None;

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        let assets_len = temp_project.assets.len();
        for i in 0..assets_len {
            let item = &temp_project.assets[i];
            if !query.is_empty() && !item.name.to_lowercase().contains(&query) {
                continue;
            }

            let is_selected = selected_asset_idx == Some(i);
            let (icon_str, item_tag) = match &item.item_type {
                ProjectItemType::Composition { .. } => ("[COMP]", "Composition"),
                ProjectItemType::Image { .. } => ("[IMG]", "Footage Image"),
                ProjectItemType::Audio { .. } => ("[AUD]", "Audio File"),
                ProjectItemType::Solid { .. } => ("[SOL]", "Solid Color"),
                ProjectItemType::Folder { .. } => ("[DIR]", "Folder"),
            };

            ui.horizontal(|ui| {
                let response = ui.selectable_label(is_selected, format!("{} {}", icon_str, item.name));
                if response.clicked() {
                    selected_asset_idx = Some(i);
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("selected_project_asset"), Some(i)));
                }

                if response.double_clicked() {
                    add_to_timeline_item = Some(item.clone());
                }

                ui.weak(format!("({})", item_tag));
            });
        }
    });

    ui.add_space(8.0);
    ui.separator();

    // ── Selected Asset Metadata Preview Header ──
    if let Some(idx) = selected_asset_idx {
        if idx < temp_project.assets.len() {
            let item = &temp_project.assets[idx];
            ui.group(|ui| {
                ui.label(egui::RichText::new(&item.name).strong());
                match &item.item_type {
                    ProjectItemType::Composition { comp_idx } => {
                        if *comp_idx < temp_project.compositions.len() {
                            let c = &temp_project.compositions[*comp_idx];
                            ui.small(format!("Resolution: {} x {} px", c.width, c.height));
                            ui.small(format!("Frame Rate: {} fps", c.fps));
                            ui.small(format!("Duration: {} frames", c.duration_frames));
                        }
                    }
                    ProjectItemType::Image { path, width, height } => {
                        ui.small(format!("File: {}", path));
                        ui.small(format!("Dimensions: {} x {} px", width, height));
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

    // Double clicked or clicked "Add to Active Comp"
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
            _ => Layer::new(
                format!("layer_gen_{}", len),
                item.name,
                LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] },
                comp.duration_frames,
            ),
        };
        comp.add_layer(new_layer);
        project_changed = true;
    }

    if project_changed {
        app.history.commit(temp_project);
        crate::core::frame_cache::bump_version();
    }
}
