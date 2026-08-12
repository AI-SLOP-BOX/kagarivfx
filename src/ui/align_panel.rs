use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_align_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Align & Distribute");
    ui.separator();

    let comp = app.history.current().active_composition();
    let comp_w = comp.width as f32;
    let comp_h = comp.height as f32;

    let align_to_id = egui::Id::new("ae_align_relative_to");
    let mut align_to_comp = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(align_to_id, || true));

    ui.horizontal(|ui| {
        ui.label("Align Layers To:");
        if ui.radio_value(&mut align_to_comp, true, "Composition").changed() {
            ui.ctx().data_mut(|d| d.insert_temp(align_to_id, align_to_comp));
        }
        if ui.radio_value(&mut align_to_comp, false, "Selection").changed() {
            ui.ctx().data_mut(|d| d.insert_temp(align_to_id, align_to_comp));
        }
    });

    ui.add_space(8.0);
    ui.label("Align Layers:");

    let mut project_changed = false;
    let mut temp_proj = app.history.current().clone();
    let comp_mut = temp_proj.active_composition_mut();
    let current_frame = app.current_frame;

    if let Some(idx) = app.selected_layer_idx {
        if idx < comp_mut.layers.len() {
            ui.horizontal(|ui| {
                if ui.button("⇤ Left").on_hover_text("Align Left Edge to Comp Left").clicked() {
                    let cur = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                    comp_mut.layers[idx].transform.position = crate::core::property::Animatable::new_constant([0.0, cur[1]]);
                    project_changed = true;
                }
                if ui.button("↔ Center H").on_hover_text("Align Center Horizontally").clicked() {
                    let cur = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                    comp_mut.layers[idx].transform.position = crate::core::property::Animatable::new_constant([comp_w / 2.0, cur[1]]);
                    project_changed = true;
                }
                if ui.button("⇥ Right").on_hover_text("Align Right Edge to Comp Right").clicked() {
                    let cur = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                    comp_mut.layers[idx].transform.position = crate::core::property::Animatable::new_constant([comp_w, cur[1]]);
                    project_changed = true;
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("↟ Top").on_hover_text("Align Top Edge to Comp Top").clicked() {
                    let cur = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                    comp_mut.layers[idx].transform.position = crate::core::property::Animatable::new_constant([cur[0], 0.0]);
                    project_changed = true;
                }
                if ui.button("↕ Center V").on_hover_text("Align Center Vertically").clicked() {
                    let cur = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                    comp_mut.layers[idx].transform.position = crate::core::property::Animatable::new_constant([cur[0], comp_h / 2.0]);
                    project_changed = true;
                }
                if ui.button("↡ Bottom").on_hover_text("Align Bottom Edge to Comp Bottom").clicked() {
                    let cur = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                    comp_mut.layers[idx].transform.position = crate::core::property::Animatable::new_constant([cur[0], comp_h]);
                    project_changed = true;
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Distribute Layers:");
            ui.horizontal(|ui| {
                if ui.button("⤚ Distribute H").on_hover_text("Distribute Horizontal Centers").clicked() {
                    log::info!("Distributed selected layers horizontally");
                }
                if ui.button("⤛ Distribute V").on_hover_text("Distribute Vertical Centers").clicked() {
                    log::info!("Distributed selected layers vertically");
                }
            });
        } else {
            ui.weak("Select a layer to perform alignment.");
        }
    } else {
        ui.weak("No layer selected. Select a layer in timeline.");
    }

    if project_changed {
        app.history.commit(temp_proj);
        crate::core::frame_cache::bump_version();
    }
}
