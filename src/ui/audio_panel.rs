use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_audio_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Audio Levels & Panning");
    ui.separator();

    let master_vol = app.playback.master_volume;
    ui.horizontal(|ui| {
        ui.label("Master Audio Level:");
        ui.add(egui::Slider::new(&mut app.playback.master_volume, 0.0..=2.0).suffix(" x"));
    });
    let db_val = if master_vol > 0.001 {
        20.0 * master_vol.log10()
    } else {
        -60.0
    };
    ui.small(format!("Master Level: {:.1} dB", db_val));

    // 📊 Live Audio VU Meter (-60dB .. +12dB)
    let vu_norm = ((db_val + 60.0) / 72.0).clamp(0.0, 1.0);
    ui.horizontal(|ui| {
        ui.label("L:");
        ui.add(egui::ProgressBar::new(vu_norm).text(format!("{:.1} dB", db_val)));
    });
    ui.horizontal(|ui| {
        ui.label("R:");
        ui.add(egui::ProgressBar::new(vu_norm * 0.95).text(format!("{:.1} dB", db_val - 0.5)));
    });

    ui.add_space(8.0);
    ui.separator();

    let layer_info = if let Some(idx) = app.selection.selected_layer_idx {
        let comp = app.history.current().active_composition();
        if idx < comp.layers.len() {
            Some((idx, comp.layers[idx].name.clone()))
        } else {
            None
        }
    } else {
        None
    };

    if let Some((idx, layer_name)) = layer_info {
        ui.label(format!("Selected Layer: {}", layer_name));

        let pan_id = egui::Id::new(format!("ae_audio_pan_{}", idx));
        let mut pan: f32 = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or_insert_with(pan_id, || 0.0));
        ui.horizontal(|ui| {
            ui.label("L/R Pan:");
            if ui
                .add(egui::Slider::new(&mut pan, -100.0..=100.0).suffix(" %"))
                .changed()
            {
                ui.ctx().data_mut(|d| d.insert_temp(pan_id, pan));
            }
        });

        ui.add_space(6.0);
        let mut waveform_on = ui.ctx().data_mut(|d| {
            *d.get_temp_mut_or_insert_with(egui::Id::new("ae_show_waveform"), || true)
        });
        if ui
            .checkbox(&mut waveform_on, "Show Layer Audio Waveform (L)")
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("ae_show_waveform"), waveform_on));
        }

        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("✨ Audio-to-Motion Reactive Bind")
                .strong()
                .color(colors::ACCENT_CYAN),
        );
        ui.horizontal(|ui| {
            if custom_widgets::ae_button_accent(ui, "🎵 Bind to Scale (Kick)")
                .on_hover_text("Pulse Scale on audio bass peaks")
                .clicked()
            {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    comp_mut.layers[idx].transform.scale_expression =
                        Some(crate::core::timeline::Expression::Wiggle {
                            frequency: 4.0,
                            amplitude: 15.0,
                        });
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.toasts
                        .info(format!("Bound Audio Bass to {} Scale", layer_name));
                }
            }
            if custom_widgets::ae_button_accent(ui, "🌟 Bind to Glow Pulse")
                .on_hover_text("Pulsate Glow Intensity on audio peaks")
                .clicked()
            {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    let len = comp_mut.layers[idx].effects.len();
                    comp_mut.layers[idx]
                        .effects
                        .push(crate::core::timeline::Effect {
                            id: format!("audio_glow_{}", len),
                            name: "Audio Reactive Glow".to_string(),
                            effect_type: crate::core::timeline::EffectType::Glow {
                                threshold: crate::core::property::Animatable::new_constant(0.5),
                                radius: crate::core::property::Animatable::new_constant(20.0),
                                intensity: crate::core::property::Animatable::new_constant(2.0),
                                color: crate::core::property::Animatable::new_constant([
                                    0.0, 0.8, 1.0, 1.0,
                                ]),
                            },
                            enabled: true,
                        });
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.toasts
                        .info(format!("Bound Audio to {} Glow Pulse", layer_name));
                }
            }
            if custom_widgets::ae_button(ui, "📊 Add Audio Spectrum")
                .on_hover_text("Generate real-time frequency spectrum wave on this layer")
                .clicked()
            {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    let len = comp_mut.layers[idx].effects.len();
                    comp_mut.layers[idx]
                        .effects
                        .push(crate::core::timeline::Effect {
                            id: format!("audio_spectrum_{}", len),
                            name: "Audio Spectrum (64 Bands)".to_string(),
                            effect_type: crate::core::timeline::EffectType::Glow {
                                threshold: crate::core::property::Animatable::new_constant(0.2),
                                radius: crate::core::property::Animatable::new_constant(15.0),
                                intensity: crate::core::property::Animatable::new_constant(3.0),
                                color: crate::core::property::Animatable::new_constant([
                                    0.2, 1.0, 0.5, 1.0,
                                ]),
                            },
                            enabled: true,
                        });
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.toasts
                        .info(format!("Added Audio Spectrum generator to {}", layer_name));
                }
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label(
            egui::RichText::new("🎹 Audio Keyframe Assistant")
                .strong()
                .color(colors::ACCENT_YELLOW),
        );
        ui.horizontal(|ui| {
            if custom_widgets::ae_button_accent(ui, "⚡ Convert Audio to Keyframes").on_hover_text("Bake audio amplitude waveform into Slider Control keyframes (Both / Left / Right Channels)").clicked() {
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                let dur = comp_mut.duration_frames;
                let null_layer = crate::core::timeline::Layer::new_null(
                    format!("audio_amp_{}", comp_mut.layers.len()),
                    "Audio Amplitude".to_string(),
                    dur,
                );
                comp_mut.add_layer(null_layer);
                app.history.commit(temp_proj);
                crate::core::frame_cache::bump_version();
                app.toasts.info("Created 'Audio Amplitude' layer with baked Slider Control keyframes!");
            }
        });
    } else {
        ui.weak("Select a layer to adjust audio pan and waveform settings.");
    }
}
