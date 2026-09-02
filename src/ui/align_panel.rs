use crate::AfterEffectsApp;
use eframe::egui;

pub fn draw_align_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Align & Distribute");
    ui.separator();

    let comp = app.history.current().active_composition();
    let comp_w = comp.width as f32;
    let comp_h = comp.height as f32;

    let align_to_id = egui::Id::new("ae_align_relative_to");
    let mut align_to_comp = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(align_to_id, || true));

    ui.horizontal(|ui| {
        ui.label("Align Layers To:");
        if ui
            .radio_value(&mut align_to_comp, true, "Composition")
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(align_to_id, align_to_comp));
        }
        if ui
            .radio_value(&mut align_to_comp, false, "Selection")
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(align_to_id, align_to_comp));
        }
    });

    ui.add_space(8.0);
    ui.label("Align Layers:");

    let mut project_changed = false;
    let mut temp_proj = app.history.current().clone();
    let comp_mut = temp_proj.active_composition_mut();
    let current_frame = app.playback.current_frame;

    let mut sel_vec: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
    if let Some(i) = app.selection.selected_layer_idx {
        if !sel_vec.contains(&i) {
            sel_vec.push(i);
        }
    }

    if !sel_vec.is_empty() {
        let (bounds_min_x, bounds_max_x, bounds_min_y, bounds_max_y) = if align_to_comp {
            (0.0, comp_w, 0.0, comp_h)
        } else {
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for &idx in &sel_vec {
                if let Some(l) = comp_mut.layers.get(idx) {
                    let pos = l.transform.position.evaluate(current_frame);
                    min_x = min_x.min(pos[0]);
                    max_x = max_x.max(pos[0]);
                    min_y = min_y.min(pos[1]);
                    max_y = max_y.max(pos[1]);
                }
            }
            (min_x, max_x, min_y, max_y)
        };

        ui.horizontal(|ui| {
            if ui
                .button("⇤ Left")
                .on_hover_text("Align Left Edge")
                .clicked()
            {
                for &idx in &sel_vec {
                    if let Some(l) = comp_mut.layers.get_mut(idx) {
                        let cur = l.transform.position.evaluate(current_frame);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([bounds_min_x, cur[1]]);
                    }
                }
                project_changed = true;
            }
            if ui
                .button("↔ Center H")
                .on_hover_text("Align Center Horizontally")
                .clicked()
            {
                let target_x = (bounds_min_x + bounds_max_x) * 0.5;
                for &idx in &sel_vec {
                    if let Some(l) = comp_mut.layers.get_mut(idx) {
                        let cur = l.transform.position.evaluate(current_frame);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([target_x, cur[1]]);
                    }
                }
                project_changed = true;
            }
            if ui
                .button("⇥ Right")
                .on_hover_text("Align Right Edge")
                .clicked()
            {
                for &idx in &sel_vec {
                    if let Some(l) = comp_mut.layers.get_mut(idx) {
                        let cur = l.transform.position.evaluate(current_frame);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([bounds_max_x, cur[1]]);
                    }
                }
                project_changed = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("↟ Top").on_hover_text("Align Top Edge").clicked() {
                for &idx in &sel_vec {
                    if let Some(l) = comp_mut.layers.get_mut(idx) {
                        let cur = l.transform.position.evaluate(current_frame);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([cur[0], bounds_min_y]);
                    }
                }
                project_changed = true;
            }
            if ui
                .button("↕ Center V")
                .on_hover_text("Align Center Vertically")
                .clicked()
            {
                let target_y = (bounds_min_y + bounds_max_y) * 0.5;
                for &idx in &sel_vec {
                    if let Some(l) = comp_mut.layers.get_mut(idx) {
                        let cur = l.transform.position.evaluate(current_frame);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([cur[0], target_y]);
                    }
                }
                project_changed = true;
            }
            if ui
                .button("↡ Bottom")
                .on_hover_text("Align Bottom Edge")
                .clicked()
            {
                for &idx in &sel_vec {
                    if let Some(l) = comp_mut.layers.get_mut(idx) {
                        let cur = l.transform.position.evaluate(current_frame);
                        l.transform.position =
                            crate::core::property::Animatable::new_constant([cur[0], bounds_max_y]);
                    }
                }
                project_changed = true;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label("Distribute Layers:");
        ui.horizontal(|ui| {
            if ui
                .button("⤚ Distribute H")
                .on_hover_text("Distribute Horizontal Centers")
                .clicked()
            {
                comp_mut.distribute_selected_layers(&sel_vec, true, current_frame);
                project_changed = true;
            }
            if ui
                .button("⤛ Distribute V")
                .on_hover_text("Distribute Vertical Centers")
                .clicked()
            {
                comp_mut.distribute_selected_layers(&sel_vec, false, current_frame);
                project_changed = true;
            }
        });
    } else {
        ui.weak("No layers selected. Select one or more layers in timeline.");
    }

    if project_changed {
        app.history.commit(temp_proj);
        crate::core::frame_cache::bump_version();
    }
}
