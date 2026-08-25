use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_audio_mixer(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Multi-Track Audio Mixer");
    ui.separator();

    let layer_count = app.history.current().active_composition().layers.len();

    if app.audio_mixer_channels.len() != layer_count {
        app.audio_mixer_channels.resize(layer_count, Default::default());
    }

    let comp_name = app.history.current().active_composition().name.clone();
    ui.label(egui::RichText::new(format!("Composition: {}", comp_name)).small().color(colors::TEXT_SECONDARY));

    ui.add_space(4.0);

    // ── Toolbar: Mute All / Solo All / Reset All ──
    ui.horizontal(|ui| {
        if ui.small_button("Reset All").on_hover_text("Reset all channels to defaults").clicked() {
            for ch in app.audio_mixer_channels.iter_mut() {
                ch.gain_db = 0.0;
                ch.pan = 0.0;
                ch.mute = false;
                ch.solo = false;
            }
        }
        let has_solo = app.audio_mixer_channels.iter().any(|c| c.solo);
        if has_solo && ui.small_button("Clear Solo").clicked() {
            for ch in app.audio_mixer_channels.iter_mut() {
                ch.solo = false;
            }
        }
        let has_mute = app.audio_mixer_channels.iter().any(|c| c.mute);
        if has_mute && ui.small_button("Clear Mute").clicked() {
            for ch in app.audio_mixer_channels.iter_mut() {
                ch.mute = false;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{} tracks", layer_count)).small().color(colors::TEXT_MUTED));
        });
    });

    ui.add_space(4.0);

    // ── Track Strips + Output Meter + Master ──
    egui::ScrollArea::horizontal().max_height(280.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            let layer_info: Vec<(String, bool, bool)> = app
                .history.current().active_composition()
                .layers.iter()
                .map(|l| {
                    let has_audio = matches!(
                        l.layer_type,
                        crate::core::timeline::LayerType::Audio { .. }
                            | crate::core::timeline::LayerType::Video { audio_wav: Some(_), .. }
                    );
                    let has_video = matches!(
                        l.layer_type,
                        crate::core::timeline::LayerType::Video { .. }
                            | crate::core::timeline::LayerType::Image { .. }
                            | crate::core::timeline::LayerType::Text { .. }
                            | crate::core::timeline::LayerType::Shape { .. }
                            | crate::core::timeline::LayerType::Solid { .. }
                            | crate::core::timeline::LayerType::PreComp { .. }
                            | crate::core::timeline::LayerType::Particle { .. }
                    );
                    (l.name.clone(), has_audio, has_video)
                })
                .collect();

            for (idx, (name, has_audio, _has_video)) in layer_info.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(76.0);

                    // ── Track header: icon + name ──
                    let type_icon = if *has_audio { "🔊" } else { "🔇" };
                    let name_color = if *has_audio { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(type_icon).small());
                        ui.label(egui::RichText::new(format!("{:02}", idx + 1)).small().strong().color(colors::TEXT_SECONDARY));
                    });
                    ui.label(egui::RichText::new(name).small().color(name_color));

                    ui.add_space(4.0);

                    // ── Mute / Solo / Reset ──
                    let ch = &mut app.audio_mixer_channels[idx];
                    ui.horizontal(|ui| {
                        let mute_color = if ch.mute { colors::ACCENT_RED } else { colors::TEXT_MUTED };
                        if ui.add(egui::Button::new(
                            egui::RichText::new("M").strong().color(mute_color)
                        ).min_size(egui::vec2(22.0, 16.0))).clicked() {
                            ch.mute = !ch.mute;
                        }
                        let solo_color = if ch.solo { colors::ACCENT_YELLOW } else { colors::TEXT_MUTED };
                        if ui.add(egui::Button::new(
                            egui::RichText::new("S").strong().color(solo_color)
                        ).min_size(egui::vec2(22.0, 16.0))).clicked() {
                            ch.solo = !ch.solo;
                        }
                        let reset_btn = ui.add(egui::Button::new(
                            egui::RichText::new("R").small().color(colors::TEXT_MUTED)
                        ).min_size(egui::vec2(18.0, 16.0)));
                        if reset_btn.on_hover_text("Reset to defaults").clicked() {
                            ch.gain_db = 0.0;
                            ch.pan = 0.0;
                        }
                    });

                    ui.add_space(4.0);

                    // ── Gain fader (vertical) with dB readout ──
                    ui.add(egui::Slider::new(&mut ch.gain_db, -60.0..=12.0)
                        .vertical()
                        .step_by(0.5)
                        .custom_formatter(|v, _| {
                            if v <= -60.0 { "-∞ dB".into() } else { format!("{:.1} dB", v) }
                        }));

                    ui.add_space(2.0);

                    // ── Pan knob (horizontal) with L/C/R labels ──
                    let pan_label = if ch.pan < -10.0 {
                        format!("L{:.0}", ch.pan.abs())
                    } else if ch.pan > 10.0 {
                        format!("R{:.0}", ch.pan)
                    } else {
                        "C".into()
                    };
                    ui.add(egui::Slider::new(&mut ch.pan, -100.0..=100.0)
                        .step_by(1.0)
                        .show_value(false));
                    ui.label(egui::RichText::new(format!("Pan {}", pan_label)).small().color(colors::TEXT_SECONDARY));

                    // ── Mini VU meter (per-track) ──
                    // Use the global meter as a rough proxy; per-track metering
                    // would require separate mix buffers per track.
                    let level = if *has_audio && !ch.mute {
                        let gain_linear = 10f32.powf(ch.gain_db / 20.0);
                        (app.audio_meter.0.max(app.audio_meter.1) * gain_linear).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let (m_h, m_w) = (60.0, 8.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(m_w, m_h), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, colors::BG_DEEPEST);
                    let filled = (rect.height() * level).min(rect.height());
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), rect.bottom() - filled),
                        egui::vec2(rect.width(), filled),
                    );
                    let color = if level > 0.95 { colors::ACCENT_RED }
                        else if level > 0.7 { colors::ACCENT_YELLOW }
                        else { colors::ACCENT_GREEN };
                    ui.painter().rect_filled(bar, 1.0, color);
                });
                ui.separator();
            }

            // ── Output Meters (L/R) ──
            ui.vertical(|ui| {
                ui.set_width(60.0);
                ui.label(egui::RichText::new("OUT").strong().small().color(colors::TEXT_SECONDARY));
                let (meter_h, meter_w) = (140.0, 16.0);
                ui.horizontal(|ui| {
                    for (label, level) in [("L", app.audio_meter.0), ("R", app.audio_meter.1)] {
                        ui.vertical(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(meter_w, meter_h), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, colors::BG_DEEPEST);

                            // Segment lines
                            for seg in 0..12 {
                                let y = rect.top() + rect.height() * (seg as f32 / 12.0);
                                ui.painter().line_segment(
                                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                                    egui::Stroke::new(0.5, colors::BORDER_SUBTLE),
                                );
                            }

                            let filled = (rect.height() * level.clamp(0.0, 1.0)).min(rect.height());
                            let bar = egui::Rect::from_min_size(
                                egui::pos2(rect.left(), rect.bottom() - filled),
                                egui::vec2(rect.width(), filled),
                            );
                            let color = if level > 0.95 { colors::ACCENT_RED }
                                else if level > 0.7 { colors::ACCENT_YELLOW }
                                else { colors::ACCENT_GREEN };
                            ui.painter().rect_filled(bar, 1.0, color);

                            // Peak dot
                            let peak_y = rect.bottom() - filled;
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(rect.left(), peak_y - 2.0),
                                    egui::vec2(rect.width(), 2.0),
                                ),
                                0.0,
                                color,
                            );

                            ui.label(egui::RichText::new(label).small().color(colors::TEXT_SECONDARY));
                        });
                    }
                });
                // dB readout
                let db_l = if app.audio_meter.0 > 0.0 { 20.0 * app.audio_meter.0.log10() } else { -96.0 };
                let db_r = if app.audio_meter.1 > 0.0 { 20.0 * app.audio_meter.1.log10() } else { -96.0 };
                ui.label(egui::RichText::new(format!("{:.0} / {:.0} dB", db_l, db_r)).small().monospace().color(colors::TEXT_SECONDARY));
            });
            ui.separator();

            // ── Master Channel Strip ──
            ui.vertical(|ui| {
                ui.set_width(80.0);
                ui.label(egui::RichText::new("MASTER").strong().small().color(colors::ACCENT_BLUE));
                ui.add_space(4.0);

                // Master volume fader
                ui.add(egui::Slider::new(&mut app.master_volume, 0.0..=1.0)
                    .vertical()
                    .step_by(0.01)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));

                ui.add_space(2.0);

                // dB readout
                let master_db = if app.master_volume > 0.0 { 20.0 * app.master_volume.log10() } else { -96.0 };
                let db_color = if master_db < -3.0 { colors::TEXT_SECONDARY }
                    else if master_db < 0.0 { colors::ACCENT_YELLOW }
                    else { colors::ACCENT_RED };
                ui.label(egui::RichText::new(format!("{:.1} dB", master_db)).small().monospace().color(db_color));

                ui.add_space(4.0);
                ui.label(egui::RichText::new("Vol").small().color(colors::TEXT_SECONDARY));
            });
        });
    });

    // ── Master DSP: EQ + Compressor ──
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.strong("Master DSP");
    });
    ui.horizontal(|ui| {
        // EQ controls
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("EQ").small().strong().color(colors::ACCENT_BLUE));
            ui.add(egui::Slider::new(&mut app.master_eq_highpass, 20.0..=500.0)
                .text("HPF Hz")
                .logarithmic(true));
            ui.add(egui::Slider::new(&mut app.master_eq_lowpass, 2000.0..=20000.0)
                .text("LPF Hz")
                .logarithmic(true));
            ui.add(egui::Slider::new(&mut app.master_eq_mid_gain, -12.0..=12.0)
                .text("Mid dB"));
            ui.add(egui::Slider::new(&mut app.master_eq_mid_freq, 200.0..=8000.0)
                .text("Mid Hz")
                .logarithmic(true));
        });
        ui.separator();
        // Compressor controls
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Compressor").small().strong().color(colors::ACCENT_BLUE));
            ui.add(egui::Slider::new(&mut app.master_comp_threshold, -40.0..=0.0)
                .text("Thresh dB"));
            ui.add(egui::Slider::new(&mut app.master_comp_ratio, 1.0..=20.0)
                .text("Ratio"));
            ui.add(egui::Slider::new(&mut app.master_comp_attack, 0.1..=50.0)
                .text("Attack ms"));
            ui.add(egui::Slider::new(&mut app.master_comp_release, 10.0..=500.0)
                .text("Release ms"));
            ui.add(egui::Slider::new(&mut app.master_comp_makeup, 0.0..=24.0)
                .text("Makeup dB"));
        });
    });
}
