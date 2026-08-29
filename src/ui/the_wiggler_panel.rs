use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::property::Animatable;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;
use crate::core::the_wiggler::{WiggleNoiseType, WiggleDimension};

pub fn draw_the_wiggler_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("🎲 The Wiggler");
    ui.label(egui::RichText::new("Inject procedural noise keyframes into layer transforms").small().color(colors::TEXT_SECONDARY));
    ui.separator();

    let target_id = egui::Id::new("ae_wiggler_target_prop");
    let mut target_prop: usize = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(target_id, || 0));

    let noise_type_id = egui::Id::new("ae_wiggler_noise_type");
    let mut noise_type_idx: usize = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(noise_type_id, || 0));

    let freq_id = egui::Id::new("ae_wiggler_freq");
    let mut freq: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(freq_id, || 4.0));

    let mag_id = egui::Id::new("ae_wiggler_mag");
    let mut mag: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(mag_id, || 30.0));

    ui.horizontal(|ui| {
        ui.label("Property:");
        egui::ComboBox::from_id_salt("wiggler_prop_combo")
            .selected_text(match target_prop {
                0 => "Position",
                1 => "Rotation",
                2 => "Scale",
                _ => "Opacity",
            })
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut target_prop, 0, "Position").clicked() { ui.ctx().data_mut(|d| d.insert_temp(target_id, target_prop)); }
                if ui.selectable_value(&mut target_prop, 1, "Rotation").clicked() { ui.ctx().data_mut(|d| d.insert_temp(target_id, target_prop)); }
                if ui.selectable_value(&mut target_prop, 2, "Scale").clicked() { ui.ctx().data_mut(|d| d.insert_temp(target_id, target_prop)); }
                if ui.selectable_value(&mut target_prop, 3, "Opacity").clicked() { ui.ctx().data_mut(|d| d.insert_temp(target_id, target_prop)); }
            });
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Noise Type:");
        if ui.selectable_value(&mut noise_type_idx, 0, "🌊 Smooth").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(noise_type_id, noise_type_idx));
        }
        if ui.selectable_value(&mut noise_type_idx, 1, "⚡ Jagged").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(noise_type_id, noise_type_idx));
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Frequency:");
        if ui.add(egui::Slider::new(&mut freq, 0.5..=30.0).suffix(" wiggles/s")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(freq_id, freq));
        }
    });

    ui.horizontal(|ui| {
        ui.label("Magnitude:");
        if ui.add(egui::Slider::new(&mut mag, 1.0..=200.0)).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(mag_id, mag));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    if custom_widgets::ae_button(ui, "🎲 Apply Wiggle Keyframes").on_hover_text("Bake procedural noise into layer keyframes").clicked() {
        let Some(layer_idx) = app.selected_layer_idx else {
            app.toasts.error("Select a layer first");
            return;
        };

        let mut temp_proj = app.history.current().clone();
        let comp = temp_proj.active_composition_mut();
        let fps = comp.fps;
        let Some(layer) = comp.layers.get_mut(layer_idx) else { return };

        let start_f = layer.in_frame;
        let end_f = layer.out_frame;
        let noise_type = if noise_type_idx == 0 { WiggleNoiseType::Smooth } else { WiggleNoiseType::Jagged };
        let seed = layer_idx as u32 * 100 + 7;

        match target_prop {
            0 => {
                let base = layer.transform.position.evaluate(start_f);
                let kfs = crate::core::the_wiggler::generate_wiggle_vec2(
                    base, start_f, end_f, fps, freq, mag, noise_type, WiggleDimension::AllIndependent, seed,
                );
                layer.transform.position = Animatable::Animated(kfs);
            }
            1 => {
                let base = layer.transform.rotation.evaluate(start_f);
                let kfs = crate::core::the_wiggler::generate_wiggle_scalar(
                    base, start_f, end_f, fps, freq, mag, noise_type, seed,
                );
                layer.transform.rotation = Animatable::Animated(kfs);
            }
            2 => {
                let base = layer.transform.scale.evaluate(start_f);
                let kfs = crate::core::the_wiggler::generate_wiggle_vec2(
                    base, start_f, end_f, fps, freq, mag, noise_type, WiggleDimension::AllSame, seed,
                );
                layer.transform.scale = Animatable::Animated(kfs);
            }
            _ => {
                let base = layer.transform.opacity.evaluate(start_f);
                let kfs = crate::core::the_wiggler::generate_wiggle_scalar(
                    base, start_f, end_f, fps, freq, mag, noise_type, seed,
                );
                layer.transform.opacity = Animatable::Animated(kfs);
            }
        }

        app.history.commit(temp_proj);
        crate::core::frame_cache::bump_version();
        app.toasts.info("The Wiggler: Baked procedural wiggle keyframes");
    }
}
