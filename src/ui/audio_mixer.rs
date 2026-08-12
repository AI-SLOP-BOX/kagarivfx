use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_audio_mixer(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Multi-Track Audio Mixer");
    ui.separator();

    let comp = app.history.current().active_composition();
    ui.label(format!("Active Composition: {}", comp.name));

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        egui::ScrollArea::horizontal().max_height(240.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Render per-layer track strip
                for (idx, layer) in comp.layers.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(70.0);
                        ui.label(egui::RichText::new(format!("{:02}. {}", idx + 1, layer.name)).small());

                        ui.add_space(4.0);
                        let gain_id = egui::Id::new(format!("ae_audio_strip_gain_{}", idx));
                        let mut gain_db: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(gain_id, || 0.0));

                        ui.add(egui::Slider::new(&mut gain_db, -60.0..=12.0).vertical().suffix(" dB"));
                        ui.ctx().data_mut(|d| d.insert_temp(gain_id, gain_db));

                        ui.add_space(4.0);
                        let pan_id = egui::Id::new(format!("ae_audio_strip_pan_{}", idx));
                        let mut pan: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(pan_id, || 0.0));
                        if ui.add(egui::Slider::new(&mut pan, -100.0..=100.0)).changed() {
                            ui.ctx().data_mut(|d| d.insert_temp(pan_id, pan));
                        }
                        ui.label(egui::RichText::new("Pan").small());
                    });
                    ui.separator();
                }

                // Master Channel Strip
                ui.vertical(|ui| {
                    ui.set_width(80.0);
                    ui.label(egui::RichText::new("MASTER").strong());
                    ui.add_space(4.0);
                    let mut master_gain = app.master_volume;
                    if ui.add(egui::Slider::new(&mut master_gain, 0.0..=1.0).vertical()).changed() {
                        app.master_volume = master_gain;
                    }
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Master Vol").small());
                });
            });
        });
    });
}
