use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::property::Animatable;

pub fn draw_camera_dof_hud(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 4.0;
        ui.small("📷 3D DoF:");

        let selected_idx = app.selected_layer_idx;

        // 1. Auto-Focus to Selected Layer Button
        if ui.button("🎯 Focus to Layer").on_hover_text("Auto-calculate Focus Distance to selected 3D Layer").clicked() {
            let target_info = if let Some(idx) = selected_idx {
                let comp = app.history.current().active_composition();
                if idx < comp.layers.len() {
                    let target_pos = comp.layers[idx].transform.position.evaluate(app.current_frame);
                    let distance = (target_pos[0].powi(2) + target_pos[1].powi(2)).sqrt().max(10.0);
                    Some((idx, comp.layers[idx].name.clone(), target_pos, distance))
                } else { None }
            } else { None };

            if let Some((_idx, layer_name, target_pos, distance)) = target_info {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if let Some(camera_layer) = comp_mut.layers.iter_mut().find(|l| matches!(l.layer_type, crate::core::timeline::LayerType::Null)) {
                    camera_layer.transform.position = Animatable::new_constant([target_pos[0], target_pos[1]]);
                }
                app.history.commit(temp_proj);
                crate::core::frame_cache::bump_version();
                app.toasts.info(format!("Focused 3D Camera to {} ({:.0}px)", layer_name, distance));
            }
        }

        // 2. Quick Bokeh Aperture Adjuster
        ui.label(egui::RichText::new("Aperture (Bokeh):").small());
        let mut mock_aperture = 25.0f32;
        if ui.add(egui::Slider::new(&mut mock_aperture, 0.0..=150.0).suffix(" px").show_value(true)).changed() {
            crate::core::frame_cache::bump_version();
        }
    });
}
