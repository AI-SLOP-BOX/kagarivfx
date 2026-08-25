use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

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
        PaletteCommand {
            name: "Keyframe Assistant: Rove Across Time (Position)",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            if let Some(kfs) = l.transform.position.keyframes_mut() {
                                if kfs.len() >= 3 {
                                    // AE roving: interior keyframes slide along time
                                    // so velocity stays constant across the path.
                                    let rove: Vec<usize> = (1..kfs.len() - 1).collect();
                                    crate::core::spatial_keyframe::smooth_keyframe_velocity(kfs, &rove);
                                }
                            }
                        }
                    });
                    crate::core::frame_cache::bump_version();
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Stabilize Motion (from Track)",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let mut baked_count = 0usize;
                    app.modify_project(|p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            baked_count = crate::core::stabilizer::stabilize_layer_smoothed(l, 2);
                        }
                    });
                    if baked_count > 0 {
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Stabilized: {} position keyframes baked", baked_count));
                    } else {
                        app.toasts.error("Layer has no tracked data — run the Tracker first");
                    }
                } else {
                    app.toasts.info("Select a layer first");
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Center in Comp",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let dims = { let c = app.history.current().active_composition(); (c.width as f32 / 2.0, c.height as f32 / 2.0) };
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            l.transform.position = crate::core::property::Animatable::new_constant([dims.0, dims.1]);
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Fit to Comp",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let dims = { let c = app.history.current().active_composition(); (c.width as f32, c.height as f32) };
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let bs = l.bounding_size();
                            if bs[0] > 1.0 && bs[1] > 1.0 {
                                let s = (dims.0 / bs[0]).max(dims.1 / bs[1]) * 100.0;
                                l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                                l.transform.position = crate::core::property::Animatable::new_constant([dims.0 / 2.0, dims.1 / 2.0]);
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Flip Horizontal",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let cf = app.current_frame;
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let s = l.transform.scale.evaluate(cf);
                            l.transform.scale = crate::core::property::Animatable::new_constant([-s[0], s[1]]);
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Export: PNG Image Sequence",
            category: "File",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.export_codec_idx = 3;
                app.toasts.info("Codec set to PNG Sequence — open Export to render");
            }),
        },
        PaletteCommand {
            name: "Edit: Undo History Panel",
            category: "Edit",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_history_panel = !app.show_history_panel;
            }),
        },
        PaletteCommand {
            name: "Layer: Fit to Comp Width",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let cw = app.history.current().active_composition().width as f32;
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let bs = l.bounding_size();
                            if bs[0] > 1.0 {
                                let s = cw / bs[0] * 100.0;
                                l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Layer: Fit to Comp Height",
            category: "Layer",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let ch = app.history.current().active_composition().height as f32;
                    app.modify_project(move |p| {
                        if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                            let bs = l.bounding_size();
                            if bs[1] > 1.0 {
                                let s = ch / bs[1] * 100.0;
                                l.transform.scale = crate::core::property::Animatable::new_constant([s, s]);
                            }
                        }
                    });
                }
            }),
        },
        PaletteCommand {
            name: "Tool: Puppet Pin",
            category: "Tools",
            shortcut_hint: "Cmd+P",
            action: Box::new(|app| {
                app.active_tool = crate::ui::toolbar::ActiveTool::PuppetPin;
                app.toasts.info("Puppet Pin tool — click viewport to place pins");
            }),
        },
        PaletteCommand {
            name: "Layer: Add Puppet Pin at Center",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                if let Some(idx) = app.selected_layer_idx {
                    let center = app.history.current().active_composition()
                        .layers.get(idx)
                        .map(|l| l.transform.position.evaluate(app.current_frame))
                        .unwrap_or([0.0, 0.0]);
                    let proj = app.history.current_mut().active_composition_mut();
                    if let Some(l) = proj.layers.get_mut(idx) {
                        let n = l.puppet_pins.len() + 1;
                        l.puppet_pins.push(crate::core::timeline::PuppetPin::new(
                            format!("pin_{}", n), format!("Pin {}", n), center,
                        ));
                        app.toasts.info(format!("Puppet pin {} added", n));
                    }
                } else {
                    app.toasts.info("Select a layer first");
                }
            }),
        },
        PaletteCommand {
            name: "Tool: Brush",
            category: "Tools",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.active_tool = crate::ui::toolbar::ActiveTool::Brush;
                app.toasts.info("Brush — drag in viewport to paint on the selected layer");
            }),
        },
        PaletteCommand {
            name: "Tool: Eraser (removes strokes)",
            category: "Tools",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.active_tool = crate::ui::toolbar::ActiveTool::Eraser;
            }),
        },
        PaletteCommand {
            name: "File: Save Project",
            category: "File",
            shortcut_hint: "Cmd+S",
            action: Box::new(|app| {
                let path = app.project_path.clone();
                let proj = app.history.current();
                match crate::core::project_migration::save_project_atomic(proj, &path) {
                    Ok(()) => app.toasts.info(format!("Saved: {}", path)),
                    Err(e) => app.toasts.error(format!("Save failed: {}", e)),
                }
            }),
        },
        PaletteCommand {
            name: "Composition: New Composition",
            category: "Composition",
            shortcut_hint: "Cmd+N",
            action: Box::new(|app| {
                let count = app.history.current().compositions.len();
                let new_comp = crate::core::timeline::Composition::new(
                    format!("comp_{}", count), "Composition 1".to_string(), 1920, 1080, 30, 300,
                );
                let proj = app.history.current_mut();
                proj.compositions.push(new_comp);
                proj.active_composition_idx = proj.compositions.len() - 1;
                crate::core::frame_cache::bump_version();
            }),
        },
        PaletteCommand {
            name: "Keyframe Assistant: Sequence Layers",
            category: "Animation",
            shortcut_hint: "",
            action: Box::new(|app| {
                app.show_sequence_layers = true;
            }),
        },
        PaletteCommand {
            name: "Composition: Save Frame as PNG",
            category: "Export",
            shortcut_hint: "",
            action: Box::new(|app| {
                let dir = std::env::temp_dir().join("aevfx_frames");
                let _ = std::fs::create_dir_all(&dir);
                let comp = app.history.current().active_composition().clone();
                let frame = app.current_frame;
                let out = dir.join(format!("{}_f{}.png", comp.name, frame));
                let pixels = crate::core::software_renderer::render_frame_to_pixels(
                    &comp, frame, comp.width, comp.height, 0.0, 0,
                );
                match image::save_buffer(&out, &pixels, comp.width, comp.height, image::ColorType::Rgba8) {
                    Ok(_) => app.toasts.info(format!("Frame saved: {}", out.display())),
                    Err(e) => app.toasts.error(format!("Save failed: {}", e)),
                }
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
                            colors::BG_ACTIVE
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
                                    .color(if is_selected { egui::Color32::WHITE } else { colors::TEXT_SECONDARY });
                                ui.label(category_text);

                                let name_text = egui::RichText::new(cmd.name)
                                    .strong()
                                    .color(if is_selected { egui::Color32::WHITE } else { colors::TEXT_PRIMARY });
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
