use crate::core::timeline::{Composition, Layer, LayerType};
use crate::AfterEffectsApp;
use eframe::egui;

pub fn draw_precompose_dialog(app: &mut AfterEffectsApp, ctx: &egui::Context) {
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
                    let current_comp = &mut temp_proj.compositions[current_comp_idx];

                    let (w, h, fps, duration) = (
                        current_comp.width,
                        current_comp.height,
                        current_comp.fps,
                        current_comp.duration_frames,
                    );
                    let selected_indices: Vec<usize> =
                        app.selection.selected_layers.iter().copied().collect();

                    if !selected_indices.is_empty() {
                        let mut new_comp = Composition::new(
                            format!("comp_{}", next_comp_num),
                            app.precompose_name.clone(),
                            w,
                            h,
                            fps,
                            duration,
                        );

                        // Move selected layers to new comp
                        let mut layers_to_keep = Vec::new();
                        let mut extracted_effects = Vec::new();
                        let mut extracted_masks = Vec::new();

                        for (idx, mut layer) in current_comp.layers.drain(..).enumerate() {
                            if selected_indices.contains(&idx) {
                                if !app.precompose_move_attributes {
                                    // Leave attributes in current comp: extract effects and masks
                                    extracted_effects.append(&mut layer.effects);
                                    extracted_masks.append(&mut layer.masks);
                                }
                                new_comp.add_layer(layer);
                            } else {
                                layers_to_keep.push(layer);
                            }
                        }
                        current_comp.layers = layers_to_keep;

                        // Create PreComp placeholder layer in parent comp
                        let mut precomp_layer = Layer::new(
                            format!("precomp_layer_{}", current_comp.layers.len() + 1),
                            app.precompose_name.clone(),
                            LayerType::PreComp {
                                comp_id: new_comp.id.clone(),
                            },
                            duration,
                        );
                        if !app.precompose_move_attributes {
                            precomp_layer.effects = extracted_effects;
                            precomp_layer.masks = extracted_masks;
                        }
                        current_comp.add_layer(precomp_layer);

                        temp_proj.compositions.push(new_comp);
                        if open_new_tab {
                            temp_proj.active_composition_idx = temp_proj.compositions.len() - 1;
                        }
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                        app.toasts
                            .info(format!("Pre-composed into '{}'", app.precompose_name));
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
