use crate::core::editor::EditorSession;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: &mut u32) {
    let dt = ctx.input(|i| i.stable_dt);
    app.effects_animation.update(dt);

    let animated_width =
        crate::ui::panel_animation::animate_panel_width(ctx, &app.effects_animation, 350.0)
            .max(200.0);

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(240.0)
        .min_width(animated_width)
        .show(ctx, |ui| {
            const TAB_CATEGORIES: &[(&str, &[(usize, &str)])] = &[
                (
                    "Core",
                    &[
                        (30, "Effect Controls"),
                        (0, "Effects & Presets"),
                        (4, "Preview"),
                        (2, "Info"),
                    ],
                ),
                ("Audio", &[(7, "Audio"), (23, "Mixer")]),
                (
                    "Text",
                    &[(21, "Fonts"), (27, "Character"), (28, "Paragraph")],
                ),
                (
                    "Transform",
                    &[(1, "Align"), (8, "Time"), (24, "Velocity"), (6, "Markers")],
                ),
                (
                    "Layer FX",
                    &[
                        (26, "Layer Styles"),
                        (9, "Masks"),
                        (5, "Paint"),
                        (12, "Content-Aware Fill"),
                    ],
                ),
                (
                    "Color",
                    &[
                        (19, "Lumetri Color"),
                        (16, "Color (OCIO)"),
                        (20, "Libraries"),
                    ],
                ),
                (
                    "3D",
                    &[(18, "3D Views"), (25, "3D Options"), (3, "Tracker")],
                ),
                (
                    "Automation",
                    &[
                        (10, "Expressions"),
                        (14, "Scripting Console"),
                        (22, "Render Presets"),
                    ],
                ),
                (
                    "More",
                    &[
                        (11, "Essential Graphics"),
                        (13, "Metadata"),
                        (15, "Workspaces"),
                    ],
                ),
            ];
            let active_cat = TAB_CATEGORIES
                .iter()
                .position(|(_, tabs)| {
                    tabs.iter()
                        .any(|(idx, _)| *idx == app.ui_tabs.right_tab_idx)
                })
                .unwrap_or(0);
            let cat_id = egui::Id::new("right_panel_active_category");

            ui.horizontal_wrapped(|ui| {
                for (ci, (cat_name, tabs)) in TAB_CATEGORIES.iter().enumerate() {
                    let mut is_active_cat = ci == active_cat;
                    let resp = ui.toggle_value(&mut is_active_cat, *cat_name).clicked();
                    if resp {
                        ui.ctx().data_mut(|d| d.insert_temp(cat_id, ci));
                        if let Some((first_idx, _)) = tabs.first() {
                            if !tabs
                                .iter()
                                .any(|(idx, _)| *idx == app.ui_tabs.right_tab_idx)
                            {
                                app.ui_tabs.right_tab_idx = *first_idx;
                            }
                        }
                    }
                }
            });
            let active_cat = ui
                .ctx()
                .data_mut(|d| {
                    let stored: Option<usize> = d.get_temp(cat_id);
                    match stored {
                        Some(c) if c < TAB_CATEGORIES.len() => Some(c),
                        _ => None,
                    }
                })
                .and(Some(active_cat))
                .unwrap_or(0);

            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, label) in TAB_CATEGORIES[active_cat].1 {
                        ui.selectable_value(&mut app.ui_tabs.right_tab_idx, *idx, *label);
                    }
                });
            });
            ui.separator();

            let mut next_frame = None;
            let mut current_frame_reset = None;

            // Track whether a continuous interaction (slider drag) occurred.
            // Discrete actions are committed immediately via EditorSession.
            let mut slider_changed = false;

            if app.ui_tabs.right_tab_idx == 3 {
                crate::ui::tracker_panel::draw_tracker_panel(app, ui, *current_frame);
            } else if app.ui_tabs.right_tab_idx == 4 {
                let total_frames = app.history.current().active_composition().duration_frames;
                crate::ui::transport_panel::draw_transport_panel(
                    app,
                    ui,
                    current_frame,
                    total_frames,
                );
            } else if app.ui_tabs.right_tab_idx == 30 {
                draw_effect_controls(
                    app,
                    ui,
                    *current_frame,
                    &mut next_frame,
                    &mut slider_changed,
                );
            } else if app.ui_tabs.right_tab_idx == 5 {
                crate::ui::paint_panel::draw_paint_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 6 {
                crate::ui::marker_panel::draw_marker_panel(app, ui, *current_frame);
            } else if app.ui_tabs.right_tab_idx == 7 {
                crate::ui::audio_meter::draw_content(app, ui);
                ui.add_space(8.0);
                crate::ui::audio_panel::draw_audio_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 8 {
                crate::ui::time_remap_panel::draw_time_remap_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 9 {
                crate::ui::mask_panel::draw_mask_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 10 {
                crate::ui::expression_panel::draw_expression_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 11 {
                crate::ui::essential_graphics::draw_essential_graphics(app, ui);
            } else if app.ui_tabs.right_tab_idx == 12 {
                crate::ui::content_aware_fill::draw_content_aware_fill(app, ui);
            } else if app.ui_tabs.right_tab_idx == 13 {
                crate::ui::metadata_panel::draw_metadata_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 14 {
                crate::ui::scripting_console::draw_scripting_console(app, ui);
            } else if app.ui_tabs.right_tab_idx == 15 {
                crate::ui::workspace_manager::draw_workspace_manager(app, ui);
            } else if app.ui_tabs.right_tab_idx == 16 {
                crate::ui::color_management::draw_color_management(app, ui);
            } else if app.ui_tabs.right_tab_idx == 17 {
                crate::ui::flowchart_inspector::draw_flowchart_inspector(app, ui);
            } else if app.ui_tabs.right_tab_idx == 18 {
                crate::ui::camera_views::draw_camera_views(app, ui);
            } else if app.ui_tabs.right_tab_idx == 19 {
                crate::ui::lumetri_color::draw_lumetri_color(app, ui);
            } else if app.ui_tabs.right_tab_idx == 20 {
                crate::ui::cc_libraries::draw_cc_libraries(app, ui);
            } else if app.ui_tabs.right_tab_idx == 21 {
                crate::ui::font_picker::draw_font_picker(app, ui);
            } else if app.ui_tabs.right_tab_idx == 22 {
                crate::ui::render_presets::draw_render_presets(app, ui);
            } else if app.ui_tabs.right_tab_idx == 23 {
                crate::ui::audio_mixer::draw_audio_mixer(app, ui);
            } else if app.ui_tabs.right_tab_idx == 24 {
                let cf = app.playback.current_frame;
                crate::ui::speed_graph_options::draw_speed_graph_options(app, ui, cf);
            } else if app.ui_tabs.right_tab_idx == 25 {
                crate::ui::camera_light_options::draw_camera_light_options(app, ui);
            } else if app.ui_tabs.right_tab_idx == 26 {
                crate::ui::layer_styles::draw_layer_styles(app, ui);
            } else if app.ui_tabs.right_tab_idx == 27 {
                draw_character_tab(app, ui, *current_frame);
            } else if app.ui_tabs.right_tab_idx == 28 {
                crate::ui::paragraph_panel::draw_paragraph_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 1 {
                crate::ui::align_panel::draw_align_panel(app, ui);
            } else if app.ui_tabs.right_tab_idx == 2 {
                ui.heading("Info");
                ui.separator();
                if let Some(idx) = app.selection.selected_layer_idx {
                    let comp = app.history.current().active_composition();
                    if idx < comp.layers.len() {
                        let layer = &comp.layers[idx];
                        ui.label(format!("Layer: {}", layer.name));
                        ui.label(format!("ID: {}", layer.id));
                        let pos = layer.transform.position.evaluate(*current_frame);
                        let scale = layer.transform.scale.evaluate(*current_frame);
                        let rot = layer.transform.rotation.evaluate(*current_frame);
                        let op = layer.transform.opacity.evaluate(*current_frame);
                        ui.weak(format!("Position: ({:.1}, {:.1})", pos[0], pos[1]));
                        ui.weak(format!("Scale: ({:.1}%, {:.1}%)", scale[0], scale[1]));
                        ui.weak(format!("Rotation: {:.1}°", rot));
                        ui.weak(format!("Opacity: {:.1}%", op));
                    }
                } else {
                    ui.weak("No layer selected.");
                }
            } else {
                draw_effects_presets_tab(
                    app,
                    ui,
                    *current_frame,
                    &mut next_frame,
                    &mut slider_changed,
                );
            }

            ui.separator();
            ui.heading("External NLE Link");
            ui.add_space(4.0);

            if let Some(app_name) = &app.connected_app {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(
                        colors::ACCENT_GREEN,
                        format!("[ONLINE] Connected to {}", app_name),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.colored_label(colors::ACCENT_RED, "[OFFLINE] Listening on 127.0.0.1:9000");
                });
            }
            ui.add_space(8.0);

            ui.label("OTIO File Path:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.otio_path);
            });

            ui.horizontal(|ui| {
                if ui.button("Import OTIO").clicked() {
                    if let Ok(json_str) = std::fs::read_to_string(&app.otio_path) {
                        if let Ok(otio_timeline) =
                            crate::core::integration::OtioTimeline::from_json(&json_str)
                        {
                            let new_comp = otio_timeline.to_composition();
                            let mut session = EditorSession::new(&mut app.history, "Import OTIO");
                            let comp = session.current_mut().active_composition_mut();
                            comp.name = new_comp.name;
                            comp.width = new_comp.width;
                            comp.height = new_comp.height;
                            comp.fps = new_comp.fps;
                            comp.duration_frames = new_comp.duration_frames;
                            comp.layers = new_comp.layers;
                            session.commit();
                            current_frame_reset = Some(0);
                            log::info!("Successfully imported OTIO composition");
                        } else {
                            log::error!("Failed to parse OTIO JSON");
                        }
                    } else {
                        log::error!("Failed to read OTIO file from path: {}", app.otio_path);
                    }
                }
                if ui.button("Export OTIO").clicked() {
                    let active_comp = app.history.current().active_composition();
                    let otio_timeline =
                        crate::core::integration::OtioTimeline::from_composition(active_comp);
                    if let Ok(json_str) = otio_timeline.to_json() {
                        if std::fs::write(&app.otio_path, json_str).is_ok() {
                            log::info!(
                                "Successfully exported OTIO composition to: {}",
                                app.otio_path
                            );
                        } else {
                            log::error!("Failed to write OTIO file to path: {}", app.otio_path);
                        }
                    }
                }
            });

            // Continuous interaction commit: slider drags and other live parameter
            // edits go through the drag transaction API (begin on drag start,
            // commit on drag end). This produces exactly one undo entry per drag
            // regardless of how many intermediate value updates occur.
            if slider_changed {
                let is_pointer_down = ui.input(|i| i.pointer.any_down());
                if is_pointer_down {
                    if !app.drag_active() {
                        let snapshot = app.history.current().clone();
                        app.begin_drag_with_snapshot(snapshot, "Effect Parameter Edit");
                    }
                } else if app.drag_active() {
                    app.commit_drag();
                }
            }
            if let Some(nf) = next_frame {
                *current_frame = nf;
            }
            if let Some(cf) = current_frame_reset {
                app.playback.current_frame = cf;
                *current_frame = cf;
            }

            crate::ui::effects_controls::draw_particle_emitter_controls(app, ui);
        });
}

/// Effect Controls panel (tab 30).
///
/// Discrete actions (toggle, reorder, delete, duplicate, drop, preset) use
/// `EditorSession` for immediate commit — each produces exactly one undo entry.
///
/// Continuous interactions (slider drag via `draw_effect_type_ui`) use the drag
/// transaction API for end-of-frame commit.
fn draw_effect_controls(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    next_frame: &mut Option<u32>,
    slider_changed: &mut bool,
) {
    ui.heading("Effect Controls");
    ui.separator();

    let Some(idx) = app.selection.selected_layer_idx else {
        ui.weak("No layer selected.");
        return;
    };

    // Immutable read for layer name and effect count
    let layer_info = app
        .history
        .current()
        .active_composition()
        .layers
        .get(idx)
        .map(|l| (l.name.clone(), l.effects.len(), l.id.clone()));

    let Some((layer_name, _effect_count, _layer_id)) = layer_info else {
        ui.weak("Select a layer.");
        return;
    };

    let drag_info = app.dragging_effect.clone();
    ui.label(
        egui::RichText::new(format!("Layer: {}", layer_name))
            .strong()
            .color(colors::ACCENT_CYAN),
    );
    ui.add_space(4.0);

    // ── Drop zone for effects ──
    if let Some((ref effect_name, _)) = drag_info {
        let drop_rect = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 40.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(format!("Drop '{}' here", effect_name))
                            .small()
                            .color(colors::TEXT_ACCENT),
                    );
                },
            )
            .response;
        ui.painter()
            .rect_filled(drop_rect.rect, 4.0, colors::TIMELINE_SELECTION);
        ui.painter().rect_stroke(
            drop_rect.rect,
            4.0,
            egui::Stroke::new(1.5_f32, colors::TEXT_ACCENT),
        );
        let is_hovered = ui.rect_contains_pointer(drop_rect.rect);
        if is_hovered && ui.input(|i| i.pointer.any_released()) {
            let presets = crate::ui::effects_controls::get_all_effect_presets();
            if let Some(preset) = presets.iter().find(|p| p.name == effect_name) {
                let mut session = EditorSession::new(&mut app.history, "Drop Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() {
                    let effect = (preset.create_fn)(comp.layers[idx].effects.len());
                    comp.layers[idx].effects.push(effect);
                    session.commit();
                    app.toasts
                        .info(format!("Applied '{}' to '{}'", effect_name, layer_name));
                }
            }
            app.dragging_effect = None;
        }
    }

    // ── Effect list with discrete actions via EditorSession ──
    let effect_count = app
        .history
        .current()
        .active_composition()
        .layers
        .get(idx)
        .map_or(0, |l| l.effects.len());

    if effect_count == 0 && drag_info.is_none() {
        ui.weak("No effects applied. Drag an effect from 'Effects & Presets' tab.");
    } else {
        // Collect discrete actions during rendering, apply after.
        let mut fx_move_up: Option<usize> = None;
        let mut fx_move_down: Option<usize> = None;
        let mut fx_dup: Option<usize> = None;
        let mut fx_del: Option<usize> = None;
        let mut fx_toggle: Option<(usize, bool)> = None;

        // Render effect list in a block scope so the mutable borrow on
        // app.history is released before we create EditorSessions below.
        {
            let temp_project = app.history.current_mut();
            let comp = temp_project.active_composition_mut();

            if idx < comp.layers.len() {
                let layer = &mut comp.layers[idx];
                let total_effects = layer.effects.len();

                for (e_idx, fx) in layer.effects.iter_mut().enumerate() {
                    ui.collapsing(format!("fx {} - {}", e_idx + 1, fx.name), |ui| {
                        ui.horizontal(|ui| {
                            let was_enabled = fx.enabled;
                            ui.checkbox(&mut fx.enabled, "Enabled (fx)");
                            if was_enabled != fx.enabled {
                                fx_toggle = Some((e_idx, fx.enabled));
                            }
                            if e_idx > 0
                                && ui
                                    .small_button("▲")
                                    .on_hover_text("Move effect up in render order")
                                    .clicked()
                            {
                                fx_move_up = Some(e_idx);
                            }
                            if e_idx + 1 < total_effects
                                && ui
                                    .small_button("▼")
                                    .on_hover_text("Move effect down in render order")
                                    .clicked()
                            {
                                fx_move_down = Some(e_idx);
                            }
                            if ui
                                .small_button("Dup")
                                .on_hover_text("Duplicate effect")
                                .clicked()
                            {
                                fx_dup = Some(e_idx);
                            }
                            if ui
                                .small_button("Del")
                                .on_hover_text("Remove effect")
                                .clicked()
                            {
                                fx_del = Some(e_idx);
                            }
                        });

                        if ui.small_button("Save as Preset").clicked() {
                            let preset = crate::core::effect_presets::EffectPreset::from_effect(
                                fx,
                                fx.name.clone(),
                            );
                            let preset_dir = crate::core::effect_presets::default_preset_dir();
                            let _ = std::fs::create_dir_all(&preset_dir);
                            let filename = format!(
                                "{}.kagari-preset.json",
                                fx.name.replace(' ', "_").to_lowercase()
                            );
                            let path = preset_dir.join(&filename);
                            match preset.save_to_file(&path) {
                                Ok(()) => {
                                    app.toasts.info(format!("Preset saved: {}", filename));
                                }
                                Err(e) => {
                                    app.toasts.error(format!("Failed to save preset: {}", e));
                                }
                            }
                        }
                    });

                    // Continuous interaction: parameter edits via draw_effect_type_ui.
                    ui.collapsing("Parameters", |ui| {
                        if let crate::core::timeline::EffectType::LensFlare {
                            link_to_light, ..
                        } = &mut fx.effect_type
                        {
                            let light_names: Vec<String> =
                                comp.lights.iter().map(|l| l.name.clone()).collect();
                            let current = link_to_light
                                .clone()
                                .unwrap_or_else(|| "(manual position)".into());
                            egui::ComboBox::from_id_salt(("flare_link", fx.id.as_str()))
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            link_to_light.is_none(),
                                            "(manual position)",
                                        )
                                        .clicked()
                                    {
                                        *link_to_light = None;
                                        *slider_changed = true;
                                    }
                                    for ln in &light_names {
                                        if ui
                                            .selectable_label(
                                                link_to_light.as_deref() == Some(ln.as_str()),
                                                ln,
                                            )
                                            .clicked()
                                        {
                                            *link_to_light = Some(ln.clone());
                                            *slider_changed = true;
                                        }
                                    }
                                });
                            if link_to_light.is_some() {
                                ui.label(
                                    egui::RichText::new("tracking selected light")
                                        .small()
                                        .color(colors::TEXT_SECONDARY),
                                );
                            }
                        }
                        crate::ui::effects_controls::draw_effect_type_ui(
                            &mut fx.effect_type,
                            ui,
                            current_frame,
                            slider_changed,
                            next_frame,
                        );
                    });
                }
            }
        } // <-- mutable borrow on app.history released here

        // Apply collected discrete actions via EditorSession.
        if let Some(i) = fx_move_up {
            let mut session = EditorSession::new(&mut app.history, "Move Effect Up");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                comp.layers[idx].effects.swap(i, i - 1);
            }
            session.commit();
        }
        if let Some(i) = fx_move_down {
            let mut session = EditorSession::new(&mut app.history, "Move Effect Down");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                comp.layers[idx].effects.swap(i, i + 1);
            }
            session.commit();
        }
        if let Some(i) = fx_dup {
            let mut session = EditorSession::new(&mut app.history, "Duplicate Effect");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                let mut cloned = comp.layers[idx].effects[i].clone();
                cloned.id = format!("{}_copy", cloned.id);
                comp.layers[idx].effects.insert(i + 1, cloned);
            }
            session.commit();
        }
        if let Some(i) = fx_del {
            let mut session = EditorSession::new(&mut app.history, "Delete Effect");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() {
                comp.layers[idx].effects.remove(i);
            }
            session.commit();
        }
        if let Some((e_idx, new_enabled)) = fx_toggle {
            let mut session = EditorSession::new(&mut app.history, "Toggle Effect");
            let comp = session.current_mut().active_composition_mut();
            if idx < comp.layers.len() && e_idx < comp.layers[idx].effects.len() {
                comp.layers[idx].effects[e_idx].enabled = new_enabled;
            }
            session.commit();
        }

        // ── Preset loading (discrete actions) ──
        ui.add_space(4.0);
        ui.separator();
        if ui.button("📂 Load Preset from File...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Effect Preset", &["json", "kagari-preset"])
                .pick_file()
            {
                match crate::core::effect_presets::EffectPreset::load_from_file(&path) {
                    Ok(preset) => {
                        let mut session = EditorSession::new(&mut app.history, "Load Preset");
                        let comp = session.current_mut().active_composition_mut();
                        if idx < comp.layers.len() {
                            preset.apply_to_layer(&mut comp.layers[idx]);
                            session.commit();
                        }
                        app.toasts.info(format!("Loaded preset: {}", preset.name));
                    }
                    Err(e) => {
                        app.toasts.error(format!("Failed to load preset: {}", e));
                    }
                }
            }
        }

        // Show saved presets from default directory
        let preset_dir = crate::core::effect_presets::default_preset_dir();
        if preset_dir.is_dir() {
            let presets = crate::core::effect_presets::discover_presets_in_dir(&preset_dir);
            if !presets.is_empty() {
                ui.collapsing(format!("Saved Presets ({})", presets.len()), |ui| {
                    for (name, path) in &presets {
                        if ui.selectable_label(false, name.to_string()).clicked() {
                            match crate::core::effect_presets::EffectPreset::load_from_file(path) {
                                Ok(preset) => {
                                    let mut session =
                                        EditorSession::new(&mut app.history, "Load Preset");
                                    let comp = session.current_mut().active_composition_mut();
                                    if idx < comp.layers.len() {
                                        preset.apply_to_layer(&mut comp.layers[idx]);
                                        session.commit();
                                    }
                                    app.toasts.info(format!("Loaded: {}", name));
                                }
                                Err(e) => {
                                    app.toasts.error(format!("Failed to load: {}", e));
                                }
                            }
                        }
                    }
                });
            }
        }
    }
}

/// Effects & Presets tab: browse and apply effects.
/// All mutations use EditorSession for immediate commit.
fn draw_effects_presets_tab(
    app: &mut KagariApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    next_frame: &mut Option<u32>,
    slider_changed: &mut bool,
) {
    ui.heading("Effects & Presets");
    ui.separator();

    let presets = crate::ui::effects_controls::get_all_effect_presets();
    let search_q = app.ui_tabs.effects_search_query.to_lowercase();

    fn category_of(name: &str) -> &'static str {
        if name.contains("Key")
            || name.contains("Matte")
            || name.contains("Choker")
            || name.contains("Choke")
            || name.contains("Minimax")
            || name.contains("Alpha")
        {
            "Keying & Matte"
        } else if name.contains("Blur")
            || name.contains("Sharpen")
            || name.contains("Median")
            || name.contains("Tilt")
        {
            "Blur & Sharpen"
        } else if name.contains("Tint")
            || name.contains("Hue")
            || name.contains("LUT")
            || name.contains("Levels")
            || name.contains("Log Space")
            || name.contains("Balance")
            || name.contains("Vibrance")
            || name.contains("HSL")
            || name.contains("Curve")
            || name.contains("Channel")
            || name.contains("Colorama")
            || name.contains("Color Space")
            || name.contains("Gradient")
        {
            "Color Correction"
        } else if name.contains("Warp")
            || name.contains("Bulge")
            || name.contains("Twirl")
            || name.contains("Offset")
            || name.contains("Distort")
            || name.contains("Ripple")
            || name.contains("Spherize")
            || name.contains("Displace")
        {
            "Distort"
        } else if name.contains("Shadow")
            || name.contains("Glow")
            || name.contains("Grain")
            || name.contains("Vignette")
            || name.contains("Aberration")
            || name.contains("Scanline")
            || name.contains("CRT")
            || name.contains("Posterize")
            || name.contains("Invert")
            || name.contains("Threshold")
            || name.contains("Light Sweep")
            || name.contains("Night")
            || name.contains("Vision")
            || name.contains("Halftone")
            || name.contains("Solarize")
            || name.contains("Pixel")
            || name.contains("Emboss")
        {
            "Stylize"
        } else if name.contains("Noise")
            || name.contains("Fractal")
            || name.contains("Star")
            || name.contains("Lightning")
            || name.contains("Fire")
            || name.contains("Reflection")
            || name.contains("Perlin")
        {
            "Generate & Simulation"
        } else if name.contains("Wipe") {
            "Transition"
        } else {
            "Other"
        }
    }

    let layer_idx = app.selection.selected_layer_idx;

    ui.collapsing("Effect Browser (categorized)", |ui| {
        let categories = [
            "Blur & Sharpen",
            "Color Correction",
            "Distort",
            "Stylize",
            "Keying & Matte",
            "Generate & Simulation",
            "Transition",
            "Other",
        ];
        for cat in categories {
            let matching: Vec<_> = presets
                .iter()
                .filter(|p| category_of(p.name) == cat)
                .filter(|p| {
                    search_q.is_empty()
                        || p.search_key.contains(&search_q)
                        || p.name.to_lowercase().contains(&search_q)
                })
                .collect();
            if matching.is_empty() {
                continue;
            }
            ui.collapsing(format!("{} ({})", cat, matching.len()), |ui| {
                for (pi, p) in matching.iter().enumerate() {
                    let preset_idx = presets
                        .iter()
                        .position(|pp| std::ptr::eq(pp, *p))
                        .unwrap_or(pi);
                    let resp =
                        crate::ui::custom_widgets::ae_button(ui, &format!(" {}", p.button_label));

                    if resp.drag_started() {
                        app.dragging_effect = Some((p.name.to_string(), preset_idx));
                    }
                    if resp.drag_stopped() {
                        app.dragging_effect = None;
                    }

                    // Click to apply effect — discrete action via EditorSession
                    if resp.clicked() {
                        if let Some(idx) = layer_idx {
                            let mut session = EditorSession::new(&mut app.history, "Apply Effect");
                            let comp = session.current_mut().active_composition_mut();
                            if idx < comp.layers.len() {
                                let effect = (p.create_fn)(comp.layers[idx].effects.len());
                                comp.layers[idx].effects.push(effect);
                                session.commit();
                            }
                        }
                    }

                    if resp.dragged() {
                        ui.painter()
                            .rect_filled(resp.rect, 2.0, colors::TIMELINE_SELECTION);
                    }
                    resp.on_hover_text("Click to apply • Drag to layer in timeline");
                }
            });
        }
    });

    ui.label("Add Effect to Selected Layer:");
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("Motion VFX Presets")
                .strong()
                .color(colors::ACCENT_CYAN),
        );
        ui.small("Quick-add effect presets:");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.ui_tabs.effects_search_query)
                    .hint_text("e.g. Cyberpunk Neon Glow"),
            );
        });
        ui.horizontal(|ui| {
            if ui.button("Cyberpunk").clicked() {
                if let Some(idx) = layer_idx {
                    let mut session = EditorSession::new(&mut app.history, "Add Cyberpunk Neon");
                    let comp = session.current_mut().active_composition_mut();
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx]
                            .effects
                            .push(crate::core::timeline::Effect {
                                id: format!("ai_glow_{}", len),
                                name: "Cyberpunk Neon".to_string(),
                                effect_type: crate::core::timeline::EffectType::Glow {
                                    threshold: crate::core::property::Animatable::new_constant(0.2),
                                    radius: crate::core::property::Animatable::new_constant(30.0),
                                    intensity: crate::core::property::Animatable::new_constant(3.0),
                                    color: crate::core::property::Animatable::new_constant([
                                        0.0, 0.9, 1.0, 1.0,
                                    ]),
                                },
                                enabled: true,
                            });
                        session.commit();
                    }
                }
            }
            if ui.button("Motion Burn").clicked() {
                if let Some(idx) = layer_idx {
                    let mut session = EditorSession::new(&mut app.history, "Add Motion Burn");
                    let comp = session.current_mut().active_composition_mut();
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx]
                            .effects
                            .push(crate::core::timeline::Effect {
                                id: format!("ai_burn_{}", len),
                                name: "Motion Burn".to_string(),
                                effect_type: crate::core::timeline::EffectType::Glow {
                                    threshold: crate::core::property::Animatable::new_constant(0.1),
                                    radius: crate::core::property::Animatable::new_constant(45.0),
                                    intensity: crate::core::property::Animatable::new_constant(4.0),
                                    color: crate::core::property::Animatable::new_constant([
                                        1.0, 0.3, 0.0, 1.0,
                                    ]),
                                },
                                enabled: true,
                            });
                        session.commit();
                    }
                }
            }
        });
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.small("Search:");
        ui.add(
            egui::TextEdit::singleline(&mut app.ui_tabs.effects_search_query)
                .hint_text("Search effects..."),
        );
    });
    let q = app.ui_tabs.effects_search_query.to_lowercase();

    ui.vertical(|ui| {
        for preset in crate::ui::effects_controls::get_all_effect_presets() {
            if (q.is_empty() || preset.search_key.contains(&q))
                && ui.button(preset.button_label).clicked()
            {
                if let Some(idx) = layer_idx {
                    let mut session = EditorSession::new(&mut app.history, "Apply Effect");
                    let comp = session.current_mut().active_composition_mut();
                    if idx < comp.layers.len() {
                        let len = comp.layers[idx].effects.len();
                        comp.layers[idx].effects.push((preset.create_fn)(len));
                        session.commit();
                    }
                }
            }
        }
    });

    // Applied effects list
    ui.separator();
    if let Some(idx) = layer_idx {
        let effect_count = app
            .history
            .current()
            .active_composition()
            .layers
            .get(idx)
            .map_or(0, |l| l.effects.len());
        if effect_count > 0 {
            ui.label("Applied Effects:");

            let mut effect_to_remove: Option<usize> = None;
            let mut effect_to_swap: Option<(usize, usize)> = None;
            let mut effect_toggle: Option<(usize, bool)> = None;

            // Render with current_mut for combo boxes that need &mut access.
            // Use block scope so mutable borrow is released before EditorSession.
            {
                let temp_project = app.history.current_mut();
                let comp = temp_project.active_composition_mut();

                if idx < comp.layers.len() {
                    let layer = &mut comp.layers[idx];
                    let effects_count = layer.effects.len();

                    for (e_idx, effect) in layer.effects.iter_mut().enumerate() {
                        let fx_persistent_id =
                            ui.make_persistent_id(format!("ae_fx_item_{}_{}", effect.id, e_idx));
                        ui.push_id(fx_persistent_id, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button("[X]")
                                    .on_hover_text("Delete Effect")
                                    .clicked()
                                {
                                    effect_to_remove = Some(e_idx);
                                }
                                if e_idx > 0
                                    && ui.small_button("▲").on_hover_text("Move Up").clicked()
                                {
                                    effect_to_swap = Some((e_idx, e_idx - 1));
                                }
                                if e_idx + 1 < effects_count
                                    && ui.small_button("▼").on_hover_text("Move Down").clicked()
                                {
                                    effect_to_swap = Some((e_idx, e_idx + 1));
                                }
                                let fx_label = if effect.enabled { "[fx]" } else { "[fx off]" };
                                if ui
                                    .selectable_label(effect.enabled, fx_label)
                                    .on_hover_text("Toggle Effect Bypass (ON/OFF)")
                                    .clicked()
                                {
                                    effect.enabled = !effect.enabled;
                                    effect_toggle = Some((e_idx, effect.enabled));
                                }
                            });
                            ui.collapsing(&effect.name, |ui| {
                                if let crate::core::timeline::EffectType::LensFlare {
                                    link_to_light,
                                    ..
                                } = &mut effect.effect_type
                                {
                                    let light_names: Vec<String> =
                                        comp.lights.iter().map(|l| l.name.clone()).collect();
                                    let current = link_to_light
                                        .clone()
                                        .unwrap_or_else(|| "(manual position)".into());
                                    egui::ComboBox::from_id_salt((
                                        "flare_link2",
                                        effect.id.as_str(),
                                    ))
                                    .selected_text(current)
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                link_to_light.is_none(),
                                                "(manual position)",
                                            )
                                            .clicked()
                                        {
                                            *link_to_light = None;
                                        }
                                        for ln in &light_names {
                                            if ui
                                                .selectable_label(
                                                    link_to_light.as_deref() == Some(ln.as_str()),
                                                    ln,
                                                )
                                                .clicked()
                                            {
                                                *link_to_light = Some(ln.clone());
                                            }
                                        }
                                    });
                                }
                                crate::ui::effects_controls::draw_effect_type_ui(
                                    &mut effect.effect_type,
                                    ui,
                                    current_frame,
                                    slider_changed,
                                    next_frame,
                                );
                            });
                        });
                    }
                }
            } // <-- mutable borrow released here

            // Apply discrete actions via EditorSession
            if let Some(r_idx) = effect_to_remove {
                let mut session = EditorSession::new(&mut app.history, "Delete Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() {
                    comp.layers[idx].effects.remove(r_idx);
                }
                session.commit();
            }
            if let Some((a, b)) = effect_to_swap {
                let mut session = EditorSession::new(&mut app.history, "Reorder Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() {
                    comp.layers[idx].effects.swap(a, b);
                }
                session.commit();
            }
            if let Some((e_idx, new_enabled)) = effect_toggle {
                let mut session = EditorSession::new(&mut app.history, "Toggle Effect");
                let comp = session.current_mut().active_composition_mut();
                if idx < comp.layers.len() && e_idx < comp.layers[idx].effects.len() {
                    comp.layers[idx].effects[e_idx].enabled = new_enabled;
                }
                session.commit();
            }
        }
    }
}

/// Character tab (tab 27) — uses drag transaction API for continuous editing.
fn draw_character_tab(app: &mut KagariApp, ui: &mut egui::Ui, current_frame: u32) {
    let pre_snapshot = if !app.drag_active() {
        Some(app.history.current().clone())
    } else {
        None
    };
    let mut temp_proj = app.history.current().clone();
    let changed = crate::ui::character_panel::draw_character_panel(
        app,
        ui,
        temp_proj.active_composition_mut(),
        current_frame,
    );
    if changed {
        let is_pointer_down = ui.input(|i| i.pointer.any_down());
        if is_pointer_down {
            if !app.drag_active() {
                if let Some(snapshot) = pre_snapshot {
                    app.begin_drag_with_snapshot(snapshot, "Character Edit");
                }
            }
        } else if app.drag_active() {
            app.commit_drag();
        }
        crate::core::frame_cache::bump_version();
    }
}
