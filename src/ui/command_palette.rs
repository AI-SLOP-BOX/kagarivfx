use eframe::egui;
use crate::AfterEffectsApp;

pub struct PaletteCommand {
    pub name: &'static str,
    pub category: &'static str,
    pub shortcut_hint: &'static str,
    pub action: Box<dyn Fn(&mut AfterEffectsApp) + Send + Sync>,
}

pub fn get_all_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand {
            name: "Add Effect: Gaussian Blur",
            category: "Effects",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx].effects.push(crate::core::timeline::Effect {
                            id: format!("blur_{}", len),
                            name: "Gaussian Blur".to_string(),
                            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                                blur_radius: crate::core::property::Animatable::new_constant(10.0),
                            },
                            enabled: true,
                        });
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Effect: Glow / Bloom",
            category: "Effects",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx].effects.push(crate::core::timeline::Effect {
                            id: format!("glow_{}", len),
                            name: "Glow / Bloom".to_string(),
                            effect_type: crate::core::timeline::EffectType::Glow {
                                threshold: crate::core::property::Animatable::new_constant(0.7),
                                radius: crate::core::property::Animatable::new_constant(15.0),
                                intensity: crate::core::property::Animatable::new_constant(1.5),
                                color: crate::core::property::Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                            },
                            enabled: true,
                        });
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Effect: Color Tint",
            category: "Effects",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selected_layer_idx {
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx].effects.push(crate::core::timeline::Effect {
                            id: format!("tint_{}", len),
                            name: "Color Tint".to_string(),
                            effect_type: crate::core::timeline::EffectType::ColorTint {
                                color: crate::core::property::Animatable::new_constant([1.0, 0.2, 0.4, 1.0]),
                                intensity: crate::core::property::Animatable::new_constant(1.0),
                            },
                            enabled: true,
                        });
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Add Layer: New Solid Layer",
            category: "Layer",
            shortcut_hint: "Cmd+Y",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                let len = comp.layers.len();
                let dur = comp.duration_frames;
                let layer = crate::core::timeline::Layer::new(
                    format!("solid_{}", len),
                    format!("Solid Layer {}", len + 1),
                    crate::core::timeline::LayerType::Solid { color: [0.2, 0.6, 0.9, 1.0] },
                    dur,
                );
                comp.add_layer(layer);
                app.selected_layer_idx = Some(len);
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Add Layer: New Text Layer",
            category: "Layer",
            shortcut_hint: "Cmd+T",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                let len = comp.layers.len();
                let dur = comp.duration_frames;
                let layer = crate::core::timeline::Layer::new(
                    format!("text_{}", len),
                    format!("Text Layer {}", len + 1),
                    crate::core::timeline::LayerType::new_text("New Text", 48, [1.0, 1.0, 1.0, 1.0]),

                    dur,
                );
                comp.add_layer(layer);
                app.selected_layer_idx = Some(len);
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Layer: Reset Transform",
            category: "Transform",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                if let Some(idx) = app.selected_layer_idx {
                    if idx < comp.layers.len() {
                        comp.layers[idx].transform = crate::core::timeline::Transform2D::default();
                        crate::core::frame_cache::bump_version();
                    }
                }
            }),
        },
        PaletteCommand {
            name: "Composition: Settings",
            category: "Composition",
            shortcut_hint: "Cmd+K",
            action: Box::new(|app| {
                app.show_comp_settings = true;
            }),
        },
        PaletteCommand {
            name: "Export: Render Video (Async)",
            category: "Export",
            shortcut_hint: "Cmd+M",
            action: Box::new(|app| {
                app.show_export_dialog = true;
            }),
        },
        PaletteCommand {
            name: "View: Toggle Composition Grid",
            category: "Viewport",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_grid = !app.show_grid;
            }),
        },
        PaletteCommand {
            name: "View: Toggle Safe Guides",
            category: "Viewport",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_guides = !app.show_guides;
            }),
        },
        PaletteCommand {
            name: "Keyframe Assistant: Convert Audio to Keyframes",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                let comp = app.history.current_mut().active_composition_mut();
                let keyframes = crate::core::audio_engine::convert_audio_to_keyframes(comp);
                
                let dur = comp.duration_frames;
                let mut null_layer = crate::core::timeline::Layer::new_null(
                    format!("audio_amp_{}", comp.layers.len()),
                    "Audio Amplitude".to_string(),
                    dur,
                );

                // Populate animated slider keyframes
                let mut pos_kf = Vec::new();
                for kf in keyframes {
                    pos_kf.push(crate::core::keyframe::Keyframe {
                        frame: kf.frame,
                        value: [kf.both_amp, kf.both_amp],
                        interpolation: crate::core::keyframe::InterpolationType::Linear,
                    });
                }
                null_layer.transform.position = crate::core::property::Animatable::new_animated(pos_kf);


                comp.add_layer(null_layer);
                app.selected_layer_idx = Some(comp.layers.len() - 1);
                crate::core::frame_cache::bump_version();
            }),
        },
    ]
}


pub fn draw_command_palette(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    let mut open = app.show_command_palette;
    egui::Window::new("🔍 Command Palette (Cmd+K)")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 100.0))
        .fixed_size(egui::vec2(520.0, 340.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🔍").size(18.0));
                let search_resp = ui.add(
                    egui::TextEdit::singleline(&mut app.command_palette_search)
                        .hint_text("Type a command or effect... (e.g. Blur, Text, Export)")
                        .desired_width(450.0),
                );
                search_resp.request_focus();
            });
            ui.separator();

            let query = app.command_palette_search.to_lowercase();
            let commands = get_all_commands();

            let filtered: Vec<&PaletteCommand> = commands
                .iter()
                .filter(|cmd| {
                    query.is_empty()
                        || cmd.name.to_lowercase().contains(&query)
                        || cmd.category.to_lowercase().contains(&query)
                })
                .collect();

            let mut executed_command_idx: Option<usize> = None;

            // Handle Keyboard Navigation (Up / Down / Enter / Esc)
            let input = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                )
            });

            if input.3 {
                app.show_command_palette = false;
                return;
            }

            if !filtered.is_empty() {
                if input.0 {
                    app.command_palette_selected_idx = (app.command_palette_selected_idx + 1) % filtered.len();
                }
                if input.1 {
                    app.command_palette_selected_idx = if app.command_palette_selected_idx == 0 {
                        filtered.len() - 1
                    } else {
                        app.command_palette_selected_idx - 1
                    };
                }
                if input.2
                    && app.command_palette_selected_idx < filtered.len() {
                        executed_command_idx = Some(app.command_palette_selected_idx);
                    }
            }

            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(20.0);
                    ui.centered_and_justified(|ui| {
                        ui.weak("No matching commands found");
                    });
                } else {
                    for (idx, cmd) in filtered.iter().enumerate() {
                        let is_selected = idx == app.command_palette_selected_idx;
                        
                        let bg_color = if is_selected {
                            egui::Color32::from_rgb(0, 120, 215)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let frame = egui::Frame::none()
                            .fill(bg_color)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .rounding(4.0);

                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let category_text = egui::RichText::new(format!("[{}] ", cmd.category))
                                    .small()
                                    .color(if is_selected { egui::Color32::WHITE } else { egui::Color32::from_gray(160) });
                                ui.label(category_text);

                                let name_text = egui::RichText::new(cmd.name)
                                    .strong()
                                    .color(if is_selected { egui::Color32::WHITE } else { egui::Color32::from_gray(220) });
                                let item_resp = ui.selectable_label(is_selected, name_text);

                                if item_resp.clicked() {
                                    executed_command_idx = Some(idx);
                                }

                                if !cmd.shortcut_hint.is_empty() {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.weak(cmd.shortcut_hint);
                                    });
                                }
                            });
                        });
                    }
                }
            });

            if let Some(exec_idx) = executed_command_idx {
                if exec_idx < filtered.len() {
                    (filtered[exec_idx].action)(app);
                    app.show_command_palette = false;
                    app.command_palette_search.clear();
                    app.command_palette_selected_idx = 0;
                    app.toasts.info(format!("Executed: {}", filtered[exec_idx].name));
                }
            }
        });

    if !open {
        app.show_command_palette = false;
    }
}
