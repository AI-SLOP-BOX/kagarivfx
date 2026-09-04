use crate::core::property::Animatable;
use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_the_smoother_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("🌊 The Smoother");
    ui.label(
        egui::RichText::new("Reduce keyframe density using RDP curve fitting")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
    ui.separator();

    let tol_id = egui::Id::new("ae_smoother_tolerance");
    let mut tolerance: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(tol_id, || 2.0));

    let mode_id = egui::Id::new("ae_smoother_mode");
    let mut mode_idx: usize = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(mode_id, || 0));

    ui.horizontal(|ui| {
        ui.label("Apply To:");
        if ui
            .selectable_value(&mut mode_idx, 0, "Spatial Path")
            .clicked()
        {
            ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx));
        }
        if ui
            .selectable_value(&mut mode_idx, 1, "Temporal Graph")
            .clicked()
        {
            ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode_idx));
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Tolerance:");
        if ui
            .add(
                egui::Slider::new(&mut tolerance, 0.1..=50.0).suffix(if mode_idx == 0 {
                    " px"
                } else {
                    " val"
                }),
            )
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(tol_id, tolerance));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    if custom_widgets::ae_button(ui, "⚡ Apply Smoother")
        .on_hover_text("Simplify keyframes on active selected layer")
        .clicked()
    {
        let Some(layer_idx) = app.selection.selected_layer_idx else {
            app.toasts
                .error("Select a layer with animated keyframes first");
            return;
        };

        let mut temp_proj = app.history.current().clone();
        let comp = temp_proj.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(layer_idx) else {
            return;
        };

        let mut reduced_count = 0;

        if mode_idx == 0 {
            // Simplify 2D position path
            if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_vec2(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
        } else {
            // Simplify temporal scalar and vector curves (rotation, opacity, scale)
            if let Animatable::Animated(ref mut kfs) = layer.transform.rotation {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_scalar(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
            if let Animatable::Animated(ref mut kfs) = layer.transform.opacity {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_scalar(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
            if let Animatable::Animated(ref mut kfs) = layer.transform.scale {
                let original_len = kfs.len();
                *kfs = crate::core::the_smoother::simplify_rdp_vec2(kfs, tolerance);
                reduced_count += original_len.saturating_sub(kfs.len());
            }
        }

        if reduced_count > 0 {
            app.history.commit(temp_proj);
            crate::core::frame_cache::bump_version();
            app.toasts.info(format!(
                "The Smoother: Removed {} redundant keyframes",
                reduced_count
            ));
        } else {
            app.toasts
                .info("Keyframes are already optimal for this tolerance");
        }
    }
}
