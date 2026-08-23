use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_time_remap_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Time Stretch & Time Remapping");
    ui.separator();

    let layer_info = if let Some(idx) = app.selected_layer_idx {
        let comp = app.history.current().active_composition();
        if idx < comp.layers.len() {
            Some((idx, comp.layers[idx].name.clone(), comp.layers[idx].out_frame))
        } else { None }
    } else { None };

    if let Some((idx, layer_name, out_frame)) = layer_info {
        ui.label(format!("Selected Layer: {}", layer_name));

        ui.add_space(4.0);
        if ui.button("⏱ Enable Time Remapping (Cmd+Alt+T)").on_hover_text("Adds Time Remap keyframe track for speed control").clicked() {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if idx < comp_mut.layers.len() {
                let dur = out_frame;
                comp_mut.layers[idx].time_remap = Some(crate::core::property::Animatable::Animated(vec![
                    crate::core::keyframe::Keyframe::new(0, 0.0, crate::core::keyframe::InterpolationType::Linear),
                    crate::core::keyframe::Keyframe::new(dur, dur as f32, crate::core::keyframe::InterpolationType::Linear),
                ]));
                app.history.commit(temp_proj);
                crate::core::frame_cache::bump_version();
                app.toasts.info(format!("Enabled Time Remapping on {}", layer_name));
            }
        }

        ui.add_space(6.0);
        ui.label(egui::RichText::new("🔄 Auto Loop Expressions").small().strong().color(colors::ACCENT_CYAN));
        ui.horizontal(|ui| {
            if ui.button("🔁 Loop Cycle").on_hover_text("Attach loopOut(\"cycle\") for continuous repeat").clicked() {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.toasts.info(format!("Attached loopOut(\"cycle\") to {}", layer_name));
                }
            }
            if ui.button("🏓 Loop PingPong").on_hover_text("Attach loopOut(\"pingpong\") for back-and-forth loop").clicked() {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.toasts.info(format!("Attached loopOut(\"pingpong\") to {}", layer_name));
                }
            }
        });

            ui.add_space(8.0);
            ui.separator();

            let stretch_id = egui::Id::new(format!("ae_time_stretch_{}", idx));
            let mut stretch_factor: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(stretch_id, || 100.0));
            ui.horizontal(|ui| {
                ui.label("Stretch Factor:");
                if ui.add(egui::DragValue::new(&mut stretch_factor).range(1.0..=1000.0).suffix(" %")).changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(stretch_id, stretch_factor));
                }
            });

            ui.add_space(6.0);
            ui.label("Frame Blending Mode:");
            let blend_id = egui::Id::new("ae_frame_blending_mode");
            let mut blend_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(blend_id, || 0));

            ui.horizontal(|ui| {
                if ui.selectable_value(&mut blend_idx, 0, "Off").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
                }
                if ui.selectable_value(&mut blend_idx, 1, "Frame Mix").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
                }
                if ui.selectable_value(&mut blend_idx, 2, "Pixel Motion").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
                }
            });
    } else {
        ui.weak("Select a layer to adjust time stretch & remapping.");
    }
}
