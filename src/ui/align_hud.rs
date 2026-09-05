use crate::core::property::Animatable;
use crate::KagariApp;
use eframe::egui;

pub fn draw_alignment_hud(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 2.0;

        let comp = app.history.current().active_composition();
        let comp_w = comp.width as f32;
        let comp_h = comp.height as f32;

        let mut project_changed = false;

        ui.small("Align: ");

        // 1. Align Left
        if ui.button("⇤").on_hover_text("Align Left").clicked() {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if let Some(idx) = app.selection.selected_layer_idx {
                if idx < comp_mut.layers.len() {
                    let pos = comp_mut.layers[idx]
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    comp_mut.layers[idx].transform.position =
                        Animatable::new_constant([0.0, pos[1]]);
                    project_changed = true;
                    app.history.commit(temp_proj);
                }
            }
        }

        // 2. Align Horizontal Center
        if ui
            .button("⇥🔒⇤")
            .on_hover_text("Align Horizontal Center")
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if let Some(idx) = app.selection.selected_layer_idx {
                if idx < comp_mut.layers.len() {
                    let pos = comp_mut.layers[idx]
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    comp_mut.layers[idx].transform.position =
                        Animatable::new_constant([comp_w * 0.5, pos[1]]);
                    project_changed = true;
                    app.history.commit(temp_proj);
                }
            }
        }

        // 3. Align Right
        if ui.button("⇥").on_hover_text("Align Right").clicked() {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if let Some(idx) = app.selection.selected_layer_idx {
                if idx < comp_mut.layers.len() {
                    let pos = comp_mut.layers[idx]
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    comp_mut.layers[idx].transform.position =
                        Animatable::new_constant([comp_w, pos[1]]);
                    project_changed = true;
                    app.history.commit(temp_proj);
                }
            }
        }

        ui.add_space(4.0);

        // 4. Align Top
        if ui.button("⤒").on_hover_text("Align Top").clicked() {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if let Some(idx) = app.selection.selected_layer_idx {
                if idx < comp_mut.layers.len() {
                    let pos = comp_mut.layers[idx]
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    comp_mut.layers[idx].transform.position =
                        Animatable::new_constant([pos[0], 0.0]);
                    project_changed = true;
                    app.history.commit(temp_proj);
                }
            }
        }

        // 5. Align Vertical Center
        if ui
            .button("⇡🔒⇣")
            .on_hover_text("Align Vertical Center")
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if let Some(idx) = app.selection.selected_layer_idx {
                if idx < comp_mut.layers.len() {
                    let pos = comp_mut.layers[idx]
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    comp_mut.layers[idx].transform.position =
                        Animatable::new_constant([pos[0], comp_h * 0.5]);
                    project_changed = true;
                    app.history.commit(temp_proj);
                }
            }
        }

        // 6. Align Bottom
        if ui.button("⤓").on_hover_text("Align Bottom").clicked() {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if let Some(idx) = app.selection.selected_layer_idx {
                if idx < comp_mut.layers.len() {
                    let pos = comp_mut.layers[idx]
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    comp_mut.layers[idx].transform.position =
                        Animatable::new_constant([pos[0], comp_h]);
                    project_changed = true;
                    app.history.commit(temp_proj);
                }
            }
        }

        ui.separator();
        ui.small("Distribute: ");

        // 7. Distribute Left / Horizontally
        if ui
            .button("⇤⇥")
            .on_hover_text("Distribute Horizontally (Even Spacing)")
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            let n = comp_mut.layers.len();
            if n > 1 {
                let step = comp_w / (n as f32 + 1.0);
                for (i, layer) in comp_mut.layers.iter_mut().enumerate() {
                    let pos = layer
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    layer.transform.position =
                        Animatable::new_constant([step * (i as f32 + 1.0), pos[1]]);
                }
                project_changed = true;
                app.history.commit(temp_proj);
            }
        }

        // 8. Distribute Vertically
        if ui
            .button("⤒⤓")
            .on_hover_text("Distribute Vertically (Even Spacing)")
            .clicked()
        {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            let n = comp_mut.layers.len();
            if n > 1 {
                let step = comp_h / (n as f32 + 1.0);
                for (i, layer) in comp_mut.layers.iter_mut().enumerate() {
                    let pos = layer
                        .transform
                        .position
                        .evaluate(app.playback.current_frame);
                    layer.transform.position =
                        Animatable::new_constant([pos[0], step * (i as f32 + 1.0)]);
                }
                project_changed = true;
                app.history.commit(temp_proj);
            }
        }

        if project_changed {
            crate::core::frame_cache::bump_version();
        }
    });
}
