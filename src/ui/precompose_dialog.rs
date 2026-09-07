use crate::core::timeline::PrecompAttributesMode;
use crate::KagariApp;
use eframe::egui;

pub fn draw_precompose_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_precompose_dialog {
        return;
    }

    let mut open = app.show_precompose_dialog;
    egui::Window::new("📦 Pre-compose (Cmd+Shift+C)")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.heading("Pre-compose Selected Layers");
            ui.separator();

            let id_str = format!("Pre-comp_{}", app.history.current().compositions.len() + 1);
            ui.horizontal(|ui| {
                ui.label("New Comp Name:");
                ui.text_edit_singleline(&mut app.precompose_name);
            });

            if app.precompose_name.is_empty() {
                app.precompose_name = id_str.clone();
            }

            ui.add_space(8.0);
            ui.radio_value(
                &mut app.precompose_move_attributes,
                true,
                "Move all attributes into the new composition",
            );
            ui.radio_value(
                &mut app.precompose_move_attributes,
                false,
                "Leave all attributes in current composition",
            );

            ui.add_space(4.0);
            let mut open_new_tab = ui.ctx().data(|d| {
                d.get_temp::<bool>(egui::Id::new("precomp_open_new_tab"))
                    .unwrap_or(true)
            });
            if ui
                .checkbox(&mut open_new_tab, "Open in New Composition Viewer")
                .changed()
            {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("precomp_open_new_tab"), open_new_tab)
                });
            }

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    let mut temp_proj = app.history.current().clone();
                    let current_comp_idx = temp_proj.active_composition_idx;
                    let next_comp_num = temp_proj.compositions.len() + 1;
                    let selected_indices: Vec<usize> =
                        app.selection.selected_layers.iter().copied().collect();

                    if !selected_indices.is_empty() {
                        let current_comp = &mut temp_proj.compositions[current_comp_idx];

                        let layer_ids: Vec<String> = selected_indices
                            .iter()
                            .filter_map(|&idx| current_comp.layers.get(idx).map(|l| l.id.clone()))
                            .collect();

                        let mode = if app.precompose_move_attributes {
                            PrecompAttributesMode::MoveToNewComp
                        } else {
                            PrecompAttributesMode::LeaveInParent
                        };

                        let new_comp_id = format!("comp_{}", next_comp_num);
                        if let Some(new_sub_comp) = current_comp.precompose_layers(
                            &layer_ids,
                            new_comp_id.clone(),
                            app.precompose_name.clone(),
                            mode,
                        ) {
                            temp_proj.compositions.push(new_sub_comp);
                            if open_new_tab {
                                temp_proj.active_composition_idx = temp_proj.compositions.len() - 1;
                            }
                            app.history.commit(temp_proj);
                            app.toasts
                                .info(format!("Pre-composed into '{}'", app.precompose_name));
                        }
                    }

                    app.show_precompose_dialog = false;
                }

                if ui.button("Cancel").clicked() {
                    app.show_precompose_dialog = false;
                }
            });
        });

    app.show_precompose_dialog = open;
}
