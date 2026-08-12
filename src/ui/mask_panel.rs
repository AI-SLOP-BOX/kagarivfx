use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_mask_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Vector Masks & Shape Paths");
    ui.separator();

    let comp = app.history.current().active_composition();
    if let Some(idx) = app.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer_name = &comp.layers[idx].name;
            ui.label(format!("Selected Layer: {}", layer_name));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("➕ Add New Mask (Cmd+Shift+N)").on_hover_text("Creates a rectangular vector mask").clicked() {
                    log::info!("Added vector mask to layer {}", layer_name);
                }
                if ui.button("Invert Masks").clicked() {
                    log::info!("Inverted mask modes on layer {}", layer_name);
                }
            });

            ui.add_space(8.0);
            ui.separator();

            ui.label("Mask Controls & Properties:");
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Mask 1 Mode:");
                    let mode_id = egui::Id::new("ae_mask1_mode");
                    let mut mode_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(mode_id, || 0));
                    egui::ComboBox::from_id_source("mask1_mode_combo")
                        .selected_text(match mode_idx {
                            0 => "Add",
                            1 => "Subtract",
                            2 => "Intersect",
                            3 => "Difference",
                            _ => "None",
                        })
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut mode_idx, 0, "Add").clicked() { ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx)); }
                            if ui.selectable_value(&mut mode_idx, 1, "Subtract").clicked() { ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx)); }
                            if ui.selectable_value(&mut mode_idx, 2, "Intersect").clicked() { ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx)); }
                            if ui.selectable_value(&mut mode_idx, 3, "Difference").clicked() { ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx)); }
                        });
                });

                let feather_id = egui::Id::new("ae_mask1_feather");
                let mut feather: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(feather_id, || 0.0));
                ui.horizontal(|ui| {
                    ui.label("Mask Feather:");
                    if ui.add(egui::Slider::new(&mut feather, 0.0..=250.0).suffix(" px")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(feather_id, feather));
                    }
                });

                let opacity_id = egui::Id::new("ae_mask1_opacity");
                let mut opacity: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(opacity_id, || 100.0));
                ui.horizontal(|ui| {
                    ui.label("Mask Opacity:");
                    if ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).suffix(" %")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(opacity_id, opacity));
                    }
                });

                let expansion_id = egui::Id::new("ae_mask1_expansion");
                let mut expansion: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(expansion_id, || 0.0));
                ui.horizontal(|ui| {
                    ui.label("Mask Expansion:");
                    if ui.add(egui::Slider::new(&mut expansion, -500.0..=500.0).suffix(" px")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(expansion_id, expansion));
                    }
                });
            });
        } else {
            ui.weak("Select a layer to view and edit masks.");
        }
    } else {
        ui.weak("No layer selected.");
    }
}
