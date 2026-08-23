use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

pub fn draw_tracker_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading("Tracker");
    ui.separator();

    let sel_idx = app.selected_layer_idx;
    let (layer_name, tracker_count, has_media, layer_pos_at_head, _masks_len) = {
        let comp = app.history.current().active_composition();
        match sel_idx.and_then(|i| comp.layers.get(i)) {
            Some(l) => (
                l.name.clone(),
                l.trackers.len(),
                matches!(l.layer_type, crate::core::timeline::LayerType::Video { .. } | crate::core::timeline::LayerType::Image { .. }),
                l.transform.position.evaluate(current_frame),
                l.masks.len(),
            ),
            None => return,
        }
    };

    if let Some(idx) = sel_idx {
        if idx < app.history.current().active_composition().layers.len() {
            ui.label(format!("Motion Source: {}", layer_name));

            ui.add_space(4.0);
            // ── Tracker point management ──
            ui.label(format!("Tracker points: {}", tracker_count));
            if custom_widgets::ae_button_accent(ui, "+ Add Tracker Point").on_hover_text("Add a track point at the playhead position").clicked() {
                let pos = layer_pos_at_head;
                let tp = crate::core::timeline::TrackerPoint {
                    id: format!("tracker_{}", tracker_count),
                    name: format!("Tracker {}", tracker_count + 1),
                    position: crate::core::property::Animatable::new_constant(pos),
                    search_size: 32.0,
                    feature_size: 16.0,
                    reference_pattern: None,
                };
                app.modify_project(|p| {
                    p.active_composition_mut().layers[idx].trackers.push(tp);
                });
            }

            if !has_media {
                ui.label(
                    egui::RichText::new("Note: pixel tracking needs a Video or Image layer; others extrapolate transform velocity.")
                        .small()
                        .color(colors::ACCENT_YELLOW),
                );
            }

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, "Analyze Forward (Work Area)").on_hover_text("Track the feature through the work area using real SAD matching + subpixel refinement").clicked() {
                    let wa_out = app.work_area_out.unwrap_or_else(|| {
                        app.history.current().active_composition().duration_frames.saturating_sub(1)
                    });
                    let start = current_frame.max(1).saturating_sub(1);
                    if wa_out > start {
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                                comp, idx, 0, start, wa_out, None,
                            );
                        });
                        app.toasts.info(format!("Tracked frames {}..{} on '{}'", start, wa_out, layer_name));
                    } else {
                        app.toasts.error("Nothing to analyze: extend the work area past the playhead");
                    }
                }
            });

            // ── Perspective Corner Pin (4-point) tracking ──
            ui.add_space(8.0);
            ui.separator();
            ui.label(
                egui::RichText::new("Perspective Corner Pin (4-point)")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.label("Corners = Tracker 1..4 in order: TL, TR, BR, BL.");
            let quad_conf_id = egui::Id::new("aevfx_quad_conf");
            if let Some(prev) = ui.memory(|m| m.data.get_temp::<String>(quad_conf_id)) {
                ui.label(
                    egui::RichText::new(format!("Last run lock quality: {prev}"))
                        .small()
                        .color(colors::ACCENT_GREEN),
                );
            }
            if ui
                .add_enabled(
                    tracker_count >= 4,
                    egui::Button::new("Analyze Quad (Work Area)"),
                )
                .on_disabled_hover_text("Add at least 4 tracker points first")
                .clicked()
            {
                let wa_out = app.work_area_out.unwrap_or_else(|| {
                    app.history
                        .current()
                        .active_composition()
                        .duration_frames
                        .saturating_sub(1)
                });
                let mut ran = false;
                let mut avg = 0.0f32;
                let mut frames = 0u32;
                if wa_out > current_frame {
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        // 1) Seed rectangle from the four trackers at the start frame.
                        let Some(layer0) = comp.layers.get(idx) else { return };
                        if layer0.trackers.len() < 4 {
                            return;
                        }
                        let src_rect: [[f32; 2]; 4] = {
                            let mut r = [[0.0f32; 2]; 4];
                            for (s, rslot) in r.iter_mut().enumerate() {
                                *rslot = layer0.trackers[s].position.evaluate(current_frame);
                            }
                            r
                        };
                        // 2) Track all four corners over the work area.
                        let track =
                            crate::core::tracker_engine::TrackerEngine::analyze_quad_track(
                                comp,
                                idx,
                                [0, 1, 2, 3],
                                current_frame,
                                wa_out,
                            );
                        avg = crate::core::tracker_engine::quad_track_confidence(
                            &track,
                            src_rect,
                        )
                        .iter()
                        .sum::<f32>()
                            / track.frames.len().max(1) as f32;
                        frames = track.frames.len() as u32;
                        // 3) Bake per-corner keyframes back onto the trackers.
                        let Some(layer) = comp.layers.get_mut(idx) else { return };
                        for slot in 0..4usize {
                            let kfs: Vec<crate::core::keyframe::Keyframe<[f32; 2]>> = track
                                .frames
                                .iter()
                                .zip(&track.corners)
                                .map(|(f, c)| {
                                    crate::core::keyframe::Keyframe::new(
                                        *f,
                                        c[slot],
                                        crate::core::keyframe::InterpolationType::Linear,
                                    )
                                })
                                .collect();
                            layer.trackers[slot].position =
                                crate::core::property::Animatable::Animated(kfs);
                        }
                        ran = true;
                    });
                }
                if ran {
                    crate::core::frame_cache::bump_version();
                    let pct = format!("{:.0}% over {} frames", avg * 100.0, frames);
                    ui.memory_mut(|m| m.data.insert_temp(quad_conf_id, pct.clone()));
                    app.toasts.info(format!("Quad tracking complete — avg lock {pct}"));
                } else {
                    app.toasts.error("Nothing to analyze: extend the work area past the playhead");
                }
            }

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, "◀◀ 1f").on_hover_text("Analyze 1 Frame Backward").clicked() && current_frame > 0 {
                    let f = current_frame - 1;
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(comp, idx, 0, f.saturating_sub(1), f, None);
                    });
                }
                if custom_widgets::ae_button(ui, "1f ▶▶").on_hover_text("Analyze 1 Frame Forward").clicked() {
                    let total = app.history.current().active_composition().duration_frames;
                    let f = (current_frame + 1).min(total.saturating_sub(1));
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(comp, idx, 0, current_frame, f, None);
                    });
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Apply:");

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, "Reset Track").on_hover_text("Remove all tracked keyframes from this tracker").clicked() {
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        if let Some(tp) = comp.layers.get_mut(idx) {
                            for t in tp.trackers.iter_mut() {
                                t.position = crate::core::property::Animatable::new_constant(t.position.evaluate(current_frame));
                            }
                        }
                    });
                    app.toasts.info("Tracker keyframes reset");
                }
                if custom_widgets::ae_button(ui, "Apply to Position").on_hover_text("Bake tracker motion into a target layer's position (pick below)").clicked() {
                    app.toasts.info("Select the target layer in 'Apply to Layer' dropdown");
                }
            });

            // Target picker + apply
            egui::ComboBox::from_id_salt("tracker_apply_target")
                .selected_text(app.tracker_apply_target.map(|i| format!("Layer {}", i + 1)).unwrap_or_else(|| "Target layer...".into()))
                .show_ui(ui, |ui| {
                    let names: Vec<(usize, String)> = app.history.current().active_composition()
                        .layers.iter().enumerate()
                        .map(|(i, l)| (i, l.name.clone())).collect();
                    for (ti, tname) in names {
                        ui.selectable_value(&mut app.tracker_apply_target, Some(ti), format!("{}. {}", ti + 1, tname));
                    }
                });
            if let Some(target_idx) = app.tracker_apply_target {
                if custom_widgets::ae_button_accent(ui, "Apply Motion → Target").clicked() {
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(comp, idx, 0, target_idx, true, false);
                    });
                    app.toasts.info(format!("Applied tracking to layer {}", target_idx + 1));
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("✨ AI Auto-Mask & Roto Generator").strong().color(colors::ACCENT_CYAN));
            ui.horizontal(|ui| {
                if custom_widgets::ae_button_accent(ui, "🎯 Auto-Generate Mask").on_hover_text("Auto-create 4-vertex Bezier Mask around tracked feature").clicked() {
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    if idx < comp_mut.layers.len() {
                        let target_pos = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                        let (cx, cy) = (target_pos[0], target_pos[1]);
                        let (hw, hh) = (60.0f32, 60.0f32);

                        let vertices = vec![
                            [cx - hw, cy - hh], // Top-Left
                            [cx + hw, cy - hh], // Top-Right
                            [cx + hw, cy + hh], // Bottom-Right
                            [cx - hw, cy + hh], // Bottom-Left
                        ];

                        let auto_mask_count = comp_mut.layers[idx].masks.len();
                            let mask = crate::core::mask::Mask {
                            id: format!("auto_mask_{}", auto_mask_count),
                            name: format!("Auto Track Mask {}", auto_mask_count + 1),
                            mode: crate::core::mask::MaskMode::Add,
                            inverted: false,
                            path: crate::core::mask::MaskPath {
                                vertices: crate::core::property::Animatable::new_constant(vertices),
                                tangents: None,
                                is_closed: true,
                            },
                            feather: crate::core::property::Animatable::new_constant(5.0),
                            opacity: crate::core::property::Animatable::new_constant(100.0),
                            expansion: crate::core::property::Animatable::new_constant(0.0),
                            enabled: true,
                        };

                        comp_mut.layers[idx].masks.push(mask);
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Auto-generated Bezier Mask on {}", layer_name));
                    }
                }
            });

        } else {
            ui.weak("Select a layer to perform motion tracking.");
        }
    } else {
        ui.weak("No layer selected. Select a layer in timeline.");
    }
}
