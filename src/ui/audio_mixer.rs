use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_audio_mixer(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Multi-Track Audio Mixer");
    ui.separator();

    let layer_count = app.history.current().active_composition().layers.len();

    // ── Ensure channel state vec is sized correctly ──
    // Grow or shrink to match layer count without reallocating unnecessarily.
    if app.audio_mixer_channels.len() != layer_count {
        app.audio_mixer_channels.resize(layer_count, (0.0_f32, 0.0_f32));
    }

    let comp_name = app.history.current().active_composition().name.clone();
    ui.label(format!("Active Composition: {}", comp_name));

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        egui::ScrollArea::horizontal().max_height(240.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                let layer_names: Vec<String> = app
                    .history.current().active_composition()
                    .layers.iter()
                    .enumerate()
                    .map(|(i, l)| format!("{:02}. {}", i + 1, l.name))
                    .collect();

                // Render per-layer track strip
                for (idx, name) in layer_names.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(70.0);
                        ui.label(egui::RichText::new(name).small());
                        ui.add_space(4.0);

                        // gain_db slider ── app field, zero ctx.data_mut ──
                        let (gain_db, pan) = &mut app.audio_mixer_channels[idx];
                        ui.add(egui::Slider::new(gain_db, -60.0..=12.0).vertical().suffix(" dB"));

                        ui.add_space(4.0);
                        ui.add(egui::Slider::new(pan, -100.0..=100.0));
                        ui.label(egui::RichText::new("Pan").small());
                    });
                    ui.separator();
                }

                // Live output meter (L/R), driven by the real mix each frame
                ui.vertical(|ui| {
                    ui.set_width(56.0);
                    ui.label(egui::RichText::new("OUT").strong().small());
                    let (meter_h, meter_w) = (140.0, 12.0);
                    for level in [app.audio_meter.0, app.audio_meter.1] {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(meter_w, meter_h), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_gray(20));
                        let filled = (rect.height() * level.clamp(0.0, 1.0)).min(rect.height());
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(rect.left(), rect.bottom() - filled),
                            egui::vec2(rect.width(), filled),
                        );
                        let color = if level > 0.95 {
                            egui::Color32::from_rgb(230, 70, 70)
                        } else if level > 0.7 {
                            egui::Color32::from_rgb(240, 200, 60)
                        } else {
                            egui::Color32::from_rgb(80, 220, 120)
                        };
                        ui.painter().rect_filled(bar, 1.0, color);
                        ui.add_space(2.0);
                    }
                    ui.label(egui::RichText::new("L / R").small());
                });
                ui.separator();

                // Master Channel Strip
                ui.vertical(|ui| {
                    ui.set_width(80.0);
                    ui.label(egui::RichText::new("MASTER").strong());
                    ui.add_space(4.0);
                    ui.add(egui::Slider::new(&mut app.master_volume, 0.0..=1.0).vertical());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Master Vol").small());
                });
            });
        });
    });
}
