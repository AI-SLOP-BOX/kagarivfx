use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_comp_settings_dialog(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_comp_settings {
        return;
    }

    let mut open = app.show_comp_settings;
    egui::Window::new("⚙ Composition Settings (Cmd+K)")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            let mut temp_proj = app.history.current().clone();
            let comp = temp_proj.active_composition_mut();

            ui.heading("Basic Composition Settings");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Composition Name:");
                ui.text_edit_singleline(&mut comp.name);
            });

            ui.add_space(6.0);
            ui.label("Preset:");
            let preset_id = egui::Id::new("ae_comp_preset_choice");
            let mut preset_choice = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(preset_id, || 0));

            egui::ComboBox::from_id_source("comp_preset_combo")
                .selected_text(match preset_choice {
                    0 => "HDTV 1080 29.97 (1920 x 1080)",
                    1 => "4K UHD 60fps (3840 x 2160)",
                    2 => "720p HD 24fps (1280 x 720)",
                    _ => "Custom",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut preset_choice, 0, "HDTV 1080 30fps (1920 x 1080)").clicked() {
                        comp.width = 1920;
                        comp.height = 1080;
                        comp.fps = 30;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                    if ui.selectable_value(&mut preset_choice, 1, "4K UHD 60fps (3840 x 2160)").clicked() {
                        comp.width = 3840;
                        comp.height = 2160;
                        comp.fps = 60;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                    if ui.selectable_value(&mut preset_choice, 2, "720p HD 24fps (1280 x 720)").clicked() {
                        comp.width = 1280;
                        comp.height = 720;
                        comp.fps = 24;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut comp.width).speed(1.0).suffix(" px"));
                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut comp.height).speed(1.0).suffix(" px"));
            });

            ui.horizontal(|ui| {
                ui.label("Frame Rate (FPS):");
                ui.add(egui::DragValue::new(&mut comp.fps).speed(1).clamp_range(1..=120));
            });

            ui.horizontal(|ui| {
                ui.label("Duration (Frames):");
                ui.add(egui::DragValue::new(&mut comp.duration_frames).speed(1.0).clamp_range(1..=100000));
                let seconds = comp.duration_frames as f64 / comp.fps as f64;
                ui.small(format!("({:.2} seconds)", seconds));
            });

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.show_comp_settings = false;
                }
                if ui.button("Cancel").clicked() {
                    app.show_comp_settings = false;
                }
            });
        });

    if !open {
        app.show_comp_settings = false;
    }
}
