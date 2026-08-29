use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_content_aware_fill(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Content-Aware Fill");
    ui.separator();

    ui.label("Fill Method:");
    let method_id = egui::Id::new("ae_caf_fill_method");
    let mut method_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(method_id, || 0));
    egui::ComboBox::from_id_salt("caf_method_combo")
        .selected_text(match method_idx {
            0 => "Object (Motion Objects Removal)",
            1 => "Surface (Flat Texture Fill)",
            _ => "Edge Blend (Smooth Gradient)",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut method_idx, 0, "Object (Motion Objects Removal)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx)); }
            if ui.selectable_value(&mut method_idx, 1, "Surface (Flat Texture Fill)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx)); }
            if ui.selectable_value(&mut method_idx, 2, "Edge Blend (Smooth Gradient)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(method_id, method_idx)); }
        });

    ui.add_space(6.0);
    let alpha_exp_id = egui::Id::new("ae_caf_alpha_expansion");
    let mut alpha_exp: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(alpha_exp_id, || 5.0));
    ui.horizontal(|ui| {
        ui.label("Alpha Expansion:");
        if ui.add(egui::Slider::new(&mut alpha_exp, 0.0..=50.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(alpha_exp_id, alpha_exp));
        }
    });

    ui.add_space(6.0);
    ui.label("Range:");
    let range_id = egui::Id::new("ae_caf_range");
    let mut range_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(range_id, || 0));
    ui.horizontal(|ui| {
        if ui.selectable_value(&mut range_idx, 0, "Work Area").clicked() { ui.ctx().data_mut(|d| d.insert_temp(range_id, range_idx)); }
        if ui.selectable_value(&mut range_idx, 1, "Entire Duration").clicked() { ui.ctx().data_mut(|d| d.insert_temp(range_id, range_idx)); }
    });

    ui.add_space(10.0);
    ui.separator();

    if ui.button("Generate Fill Layer").on_hover_text("Render the current frame and synthesize a fill for the selected layer's first mask").clicked() {
        let Some(layer_idx) = app.selected_layer_idx else {
            app.toasts.error("Select a layer with a mask first");
            return;
        };
        let project = app.history.current();
        let comp = project.active_composition();
        let Some(layer) = comp.layers.get(layer_idx) else { return };
        let Some(mask) = layer.masks.first() else {
            app.toasts.error("Selected layer has no mask — draw a mask around the object to remove");
            return;
        };

        // Render the frame, then synthesize a fill over the mask polygon
        let (w, h) = (comp.width, comp.height);
        let frame_idx = app.current_frame;
        let mut pixels = crate::core::software_renderer::render_frame_to_pixels(
            comp, frame_idx, w, h, 0.0, 0,
        );
        let polygon = mask.path.to_polygon(frame_idx, 12);
        let method = match method_idx {
            1 => crate::core::content_aware_engine::FillMethod::Surface,
            2 => crate::core::content_aware_engine::FillMethod::EdgeBlend,
            _ => crate::core::content_aware_engine::FillMethod::Object,
        };
        let filled = crate::core::content_aware_engine::generate_content_aware_fill_frame(
            &pixels, w, h, &polygon, alpha_exp, method,
        );
        pixels = filled;

        // Write the synthesized frame as a PNG via the image crate
        let out_path = std::env::temp_dir().join(format!("caf_frame_{}.png", frame_idx));
        match image::save_buffer(&out_path, &pixels, w, h, image::ColorType::Rgba8) {
            Ok(_) => {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                let new_layer = crate::core::timeline::Layer::new(
                    format!("fill_layer_{}", frame_idx),
                    format!("Fill Layer [CAF] (Frame {})", frame_idx),
                    crate::core::timeline::LayerType::Image { path: out_path.to_string_lossy().to_string() },
                    comp_mut.duration_frames,
                );
                comp_mut.layers.insert(layer_idx, new_layer);
                app.history.commit(temp_proj);
                crate::core::frame_cache::bump_version();
                app.toasts.info(format!("Synthesized & inserted Fill Layer for frame {}", frame_idx));
            }
            Err(e) => app.toasts.error(format!("Failed to write fill: {}", e)),
        }
    }

    if ui.button("🖼 Create Reference Frame").on_hover_text("Export current frame as a reference PNG for manual painting/cleanup").clicked() {
        let comp = app.history.current().active_composition();
        let (w, h) = (comp.width, comp.height);
        let pixels = crate::core::software_renderer::render_frame_to_pixels(
            comp, app.current_frame, w, h, 0.0, 0,
        );
        let out_path = std::env::temp_dir().join(format!("ref_frame_{}.png", app.current_frame));
        if let Ok(_) = image::save_buffer(&out_path, &pixels, w, h, image::ColorType::Rgba8) {
            crate::ui::project_io::reveal_in_file_manager(&out_path);
            app.toasts.info(format!("Exported Reference Frame: {}", out_path.display()));
        }
    }
}
