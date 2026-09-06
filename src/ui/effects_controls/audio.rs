use crate::core::timeline::EffectType;
use eframe::egui;

use super::common::draw_prop;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::AudioSpectrum {
            enabled: _,
            bands,
            opacity,
            color_start: _,
            color_end: _,
            position_x,
            position_y,
            width: spec_w,
            height: spec_h,
        } => {
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bands",
                bands,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=5.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Opacity",
                opacity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position X",
                position_x,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Position Y",
                position_y,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Width",
                spec_w,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.01..=1.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Height",
                spec_h,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.01..=1.0));
                },
            );
        }
        EffectType::BassTreble {
            bass_gain,
            treble_gain,
            crossover_freq,
        } => {
            ui.label("Bass & Treble");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Bass (dB)",
                bass_gain,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -24.0..=24.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Treble (dB)",
                treble_gain,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -24.0..=24.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Crossover (Hz)",
                crossover_freq,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 80.0..=5000.0));
                },
            );
        }
        EffectType::Flanger {
            max_delay_ms,
            lfo_rate,
            feedback,
            wet_dry,
        } => {
            ui.label("Flanger");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Max Delay (ms)",
                max_delay_ms,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=10.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "LFO Rate (Hz)",
                lfo_rate,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.1..=10.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Feedback",
                feedback,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=0.95));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Wet/Dry",
                wet_dry,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
        }
        EffectType::Chorus {
            delay_ms,
            depth_ms,
            rate_hz,
            voices,
            feedback,
        } => {
            ui.label("Chorus");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Delay (ms)",
                delay_ms,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 1.0..=30.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Depth (ms)",
                depth_ms,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.5..=10.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Rate (Hz)",
                rate_hz,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.1..=6.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Voices",
                voices,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 2.0..=8.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Feedback",
                feedback,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=0.9));
                },
            );
        }
        EffectType::ParametricEQ {
            freq_hz,
            gain_db,
            q_factor,
        } => {
            ui.label("Parametric EQ");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Frequency (Hz)",
                freq_hz,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 60.0..=18000.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Gain (dB)",
                gain_db,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -24.0..=24.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Q Factor",
                q_factor,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.5..=12.0));
                },
            );
        }
        EffectType::Echo {
            echo_time_seconds,
            num_echoes,
            starting_intensity,
            decay,
            operator,
        } => {
            ui.label("⏱️ Echo");
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Echo Time (s)",
                echo_time_seconds,
                |ui, v| {
                    ui.add(egui::Slider::new(v, -2.0..=2.0).suffix(" s"));
                },
            );
            ui.horizontal(|ui| {
                ui.label("Number of Echoes:");
                let mut n_i32 = *num_echoes as i32;
                if ui.add(egui::Slider::new(&mut n_i32, 1..=30)).changed() {
                    *num_echoes = n_i32.max(1) as u32;
                    *project_changed = true;
                }
            });
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Starting Intensity",
                starting_intensity,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=2.0));
                },
            );
            draw_prop(
                ui,
                current_frame,
                project_changed,
                next_frame,
                "Decay",
                decay,
                |ui, v| {
                    ui.add(egui::Slider::new(v, 0.0..=1.0));
                },
            );
            ui.horizontal(|ui| {
                ui.label("Echo Operator:");
                egui::ComboBox::from_id_salt("echo_operator")
                    .selected_text(format!("{:?}", operator))
                    .show_ui(ui, |ui| {
                        for op in [
                            crate::core::echo_effect::EchoOperator::Add,
                            crate::core::echo_effect::EchoOperator::Screen,
                            crate::core::echo_effect::EchoOperator::Maximum,
                            crate::core::echo_effect::EchoOperator::Minimum,
                            crate::core::echo_effect::EchoOperator::CompositeInBack,
                            crate::core::echo_effect::EchoOperator::CompositeInFront,
                            crate::core::echo_effect::EchoOperator::Blend,
                        ] {
                            if ui
                                .selectable_value(operator, op, format!("{:?}", op))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                    });
            });
        }
        _ => {}
    }
}
