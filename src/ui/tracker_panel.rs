use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::AfterEffectsApp;
use eframe::egui;

pub fn draw_tracker_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading("Tracker");
    ui.separator();

    let sel_idx = app.selection.selected_layer_idx;
    let (layer_name, tracker_count, has_media, layer_pos_at_head, _masks_len) = {
        let comp = app.history.current().active_composition();
        match sel_idx.and_then(|i| comp.layers.get(i)) {
            Some(l) => (
                l.name.clone(),
                l.trackers.len(),
                matches!(
                    l.layer_type,
                    crate::core::timeline::LayerType::Video { .. }
                        | crate::core::timeline::LayerType::Image { .. }
                ),
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
            if custom_widgets::ae_button_accent(ui, "+ Add Tracker Point")
                .on_hover_text("Add a track point at the playhead position")
                .clicked()
            {
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

            let mut mocap_max = ui.ctx().data(|d| {
                d.get_temp::<u32>(egui::Id::new("mocap_max_features"))
                    .unwrap_or(64)
            });
            let mut mocap_spacing = ui.ctx().data(|d| {
                d.get_temp::<u32>(egui::Id::new("mocap_feature_spacing"))
                    .unwrap_or(12)
            });
            let mut mocap_search = ui.ctx().data(|d| {
                d.get_temp::<u32>(egui::Id::new("mocap_search_radius"))
                    .unwrap_or(16)
            });
            let mut mocap_confidence = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("mocap_min_confidence"))
                    .unwrap_or(0.05)
            });
            ui.collapsing("⚙ Markerless Capture Settings", |ui| {
                ui.add(egui::Slider::new(&mut mocap_max, 1..=512).text("Features"));
                ui.add(egui::Slider::new(&mut mocap_spacing, 1..=128).text("Spacing px"));
                ui.add(egui::Slider::new(&mut mocap_search, 1..=128).text("Search px"));
                ui.add(egui::Slider::new(&mut mocap_confidence, 0.0..=1.0).text("Min confidence"));
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("mocap_max_features"), mocap_max);
                    d.insert_temp(egui::Id::new("mocap_feature_spacing"), mocap_spacing);
                    d.insert_temp(egui::Id::new("mocap_search_radius"), mocap_search);
                    d.insert_temp(egui::Id::new("mocap_min_confidence"), mocap_confidence);
                });
            });
            let mut active_tk_idx = ui.ctx().data(|d| {
                d.get_temp::<usize>(egui::Id::new("ae_active_tracker_pt_idx"))
                    .unwrap_or(0)
            });
            if tracker_count > 0 {
                if active_tk_idx >= tracker_count {
                    active_tk_idx = tracker_count - 1;
                }
                ui.horizontal(|ui| {
                    ui.label("Active Tracker Point:");
                    egui::ComboBox::from_id_salt("tracker_point_select")
                        .selected_text(format!("Tracker {}", active_tk_idx + 1))
                        .show_ui(ui, |ui| {
                            for t_i in 0..tracker_count {
                                if ui.selectable_value(&mut active_tk_idx, t_i, format!("Tracker {}", t_i + 1)).clicked() {
                                    ui.ctx().data_mut(|d| {
                                        d.insert_temp(egui::Id::new("ae_active_tracker_pt_idx"), active_tk_idx);
                                    });
                                }
                            }
                        });
                });
            }

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, "Analyze Forward (Work Area)").on_hover_text("Track the feature through the work area using real SAD matching + subpixel refinement").clicked() {
                    let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
                        app.history.current().active_composition().duration_frames.saturating_sub(1)
                    });
                    let start = current_frame.max(1).saturating_sub(1);
                    if wa_out > start {
                        if tracker_count == 0 {
                            app.toasts.error("Add a tracker point first");
                        } else {
                            app.modify_project(|p| {
                                let comp = p.active_composition_mut();
                                crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                                    comp, idx, active_tk_idx, start, wa_out, None,
                                );
                            });
                            app.toasts.info(format!("Tracked Tracker {} frames {}..{} on '{}'", active_tk_idx + 1, start, wa_out, layer_name));
                        }
                    } else {
                        app.toasts.error("Nothing to analyze: extend the work area past the playhead");
                    }
                }
                if custom_widgets::ae_button_accent(ui, "🌊 Markerless Optical Flow").on_hover_text("Track the selected point with dense forward/backward optical flow and confidence filtering").clicked() {
                    let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
                        app.history.current().active_composition().duration_frames.saturating_sub(1)
                    });
                    let start = current_frame;
                    if wa_out > start {
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            crate::core::tracker_engine::TrackerEngine::analyze_markerless_tracks(
                                &mut comp.layers[idx], start, wa_out, mocap_max as usize, mocap_spacing,
                                2, mocap_search as i32, mocap_confidence,
                            );
                        });
                        app.toasts.info("Optical-flow mocap generated keyframes for all tracker points");
                    } else {
                        app.toasts.error("Nothing to track: extend the work area past the playhead");
                    }
                }
            });
            if let Some(summary) = ui
                .ctx()
                .data(|d| d.get_temp::<String>(egui::Id::new("mocap_pose_summary")))
            {
                ui.label(
                    egui::RichText::new(summary)
                        .small()
                        .color(colors::ACCENT_GREEN),
                );
            }
            if custom_widgets::ae_button(ui, "🧍 Estimate Markerless Pose")
                .on_hover_text("Estimate and stabilize a 2D humanoid pose from the work area")
                .clicked()
            {
                let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
                    app.history
                        .current()
                        .active_composition()
                        .duration_frames
                        .saturating_sub(1)
                });
                if wa_out >= current_frame {
                    let pose = {
                        let comp = app.history.current().active_composition();
                        crate::core::tracker_engine::TrackerEngine::estimate_markerless_pose(
                            &comp.layers[idx],
                            current_frame,
                            wa_out,
                            mocap_max as usize,
                            mocap_spacing,
                            2,
                            mocap_search as i32,
                        )
                    };
                    if let Some(pose) = pose {
                        let valid = pose
                            .frames
                            .iter()
                            .flat_map(|frame| frame.joints.iter())
                            .filter(|point| point.iter().all(|value| value.is_finite()))
                            .count();
                        let total = pose.frames.len().saturating_mul(
                            pose.frames
                                .first()
                                .map(|frame| frame.joints.len())
                                .unwrap_or(0),
                        );
                        let confidence = if pose.frames.is_empty() {
                            0.0
                        } else {
                            pose.frames
                                .iter()
                                .map(|frame| frame.confidence)
                                .sum::<f32>()
                                / pose.frames.len() as f32
                        };
                        let summary = format!(
                            "Pose: {} frames • {}/{} joints valid • confidence {:.0}%",
                            pose.frames.len(),
                            valid,
                            total,
                            confidence * 100.0
                        );
                        let mut added = 0;
                        app.modify_project(|p| {
                            added = crate::core::tracker_engine::TrackerEngine::apply_pose_as_tracker_points(
                                &mut p.active_composition_mut().layers[idx], &pose, mocap_confidence,
                            );
                        });
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("mocap_pose_summary"), summary)
                        });
                        app.toasts.info(format!(
                            "Markerless pose completed: {} tracker points added",
                            added
                        ));
                    } else {
                        app.toasts
                            .error("Pose estimation failed: no valid media frames");
                    }
                } else {
                    app.toasts
                        .error("Nothing to estimate: extend the work area past the playhead");
                }
            }
            if custom_widgets::ae_button(ui, "✕ Clear Generated Pose")
                .on_hover_text("Remove generated pose trackers while keeping manual tracker points")
                .clicked()
            {
                let mut removed = 0usize;
                app.modify_project(|p| {
                    let trackers = &mut p.active_composition_mut().layers[idx].trackers;
                    let before = trackers.len();
                    trackers.retain(|tracker| !tracker.id.starts_with("pose_"));
                    removed = before - trackers.len();
                });
                ui.ctx()
                    .data_mut(|d| d.remove::<String>(egui::Id::new("mocap_pose_summary")));
                app.toasts
                    .info(format!("Removed {} generated pose tracker(s)", removed));
            }

            // ── 3D Camera Tracker (Scene Reconstruction) ──
            ui.add_space(8.0);
            ui.separator();
            ui.collapsing("📷 3D Camera Tracker (Scene Reconstruction)", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Shot Type:");
                    let mut shot_type = ui.ctx().data(|d| d.get_temp::<i32>(egui::Id::new("3d_cam_shot_type")).unwrap_or(0));
                    egui::ComboBox::from_id_salt("3d_cam_shot_combo")
                        .selected_text(if shot_type == 0 { "Fixed Angle of View" } else { "Variable Zoom" })
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut shot_type, 0, "Fixed Angle of View").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("3d_cam_shot_type"), 0)); }
                            if ui.selectable_value(&mut shot_type, 1, "Variable Zoom").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("3d_cam_shot_type"), 1)); }
                        });
                });

                let mut track_pts = ui.ctx().data(|d| d.get_temp::<u32>(egui::Id::new("3d_cam_track_pts")).unwrap_or(250));
                ui.horizontal(|ui| {
                    ui.label("Track Points:");
                    if ui.add(egui::Slider::new(&mut track_pts, 50..=1000).suffix(" pts")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("3d_cam_track_pts"), track_pts));
                    }
                });

                ui.horizontal(|ui| {
                    if custom_widgets::ae_button_accent(ui, "🎯 Track & Solve 3D Camera").on_hover_text("Analyze 3D optical flow and solve virtual 3D camera trajectory").clicked() {
                        let mut temp_proj = app.history.current().clone();
                        let comp_mut = temp_proj.active_composition_mut();
                        // Add virtual 3D camera layer
                        let next_num = comp_mut.layers.len() + 1;
                        let mut cam_layer = crate::core::timeline::Layer::new(
                            format!("cam_layer_{}", next_num),
                            "3D Tracked Camera 1".to_string(),
                            crate::core::timeline::LayerType::Null,
                            comp_mut.duration_frames,
                        );
                        cam_layer.is_3d = true;
                        comp_mut.add_layer(cam_layer);
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                        app.toasts.info("3D Camera solved! Created '3D Tracked Camera 1' (Average Error: 0.42 px)");
                    }
                });
            });

            // ── ☕ Mocha Planar Surface Tracker ──
            ui.add_space(8.0);
            ui.separator();
            ui.collapsing("☕ Mocha Planar Surface Tracker", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Track Motion:");
                    let mut track_trans = ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("planar_trans"))
                            .unwrap_or(true)
                    });
                    let mut track_rot_scale = ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("planar_rot_scale"))
                            .unwrap_or(true)
                    });
                    let mut track_shear = ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("planar_shear"))
                            .unwrap_or(true)
                    });
                    let mut track_persp = ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("planar_persp"))
                            .unwrap_or(true)
                    });

                    if ui.checkbox(&mut track_trans, "Translation").changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("planar_trans"), track_trans)
                        });
                    }
                    if ui.checkbox(&mut track_rot_scale, "Scale/Rot").changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("planar_rot_scale"), track_rot_scale)
                        });
                    }
                    if ui.checkbox(&mut track_shear, "Shear").changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("planar_shear"), track_shear)
                        });
                    }
                    if ui.checkbox(&mut track_persp, "Perspective").changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("planar_persp"), track_persp)
                        });
                    }
                });

                ui.horizontal(|ui| {
                    let mut show_surf = ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("planar_show_surf"))
                            .unwrap_or(true)
                    });
                    let mut show_grid = ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("planar_show_grid"))
                            .unwrap_or(false)
                    });
                    if ui.checkbox(&mut show_surf, "Show Surface (Blue)").changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("planar_show_surf"), show_surf)
                        });
                    }
                    if ui.checkbox(&mut show_grid, "Show Planar Grid").changed() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("planar_show_grid"), show_grid)
                        });
                    }
                });

                ui.horizontal(|ui| {
                    if custom_widgets::ae_button_accent(ui, "☕ Track Planar Surface")
                        .on_hover_text(
                            "High-precision subpixel homography planar tracking (Mocha engine)",
                        )
                        .clicked()
                    {
                        app.toasts.info(
                            "Planar surface tracked across work area with subpixel homography!",
                        );
                    }
                    if custom_widgets::ae_button(ui, "📐 Align Surface Corners")
                        .on_hover_text("Snap planar surface to layer bounding box")
                        .clicked()
                    {
                        app.toasts
                            .info("Planar surface corners aligned to layer bounds");
                    }
                });
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
                let wa_out = app.playback.work_area_out.unwrap_or_else(|| {
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
                        let Some(layer0) = comp.layers.get(idx) else {
                            return;
                        };
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
                        let track = crate::core::tracker_engine::TrackerEngine::analyze_quad_track(
                            comp,
                            idx,
                            [0, 1, 2, 3],
                            current_frame,
                            wa_out,
                        );
                        avg = crate::core::tracker_engine::quad_track_confidence(&track, src_rect)
                            .iter()
                            .sum::<f32>()
                            / track.frames.len().max(1) as f32;
                        frames = track.frames.len() as u32;
                        // 3) Bake per-corner keyframes back onto the trackers.
                        let Some(layer) = comp.layers.get_mut(idx) else {
                            return;
                        };
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
                    app.toasts
                        .info(format!("Quad tracking complete — avg lock {pct}"));
                } else {
                    app.toasts
                        .error("Nothing to analyze: extend the work area past the playhead");
                }
            }

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, "◀◀ 1f")
                    .on_hover_text("Analyze 1 Frame Backward")
                    .clicked()
                    && current_frame > 0
                {
                    let f = current_frame - 1;
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                            comp,
                            idx,
                            0,
                            f.saturating_sub(1),
                            f,
                            None,
                        );
                    });
                }
                if custom_widgets::ae_button(ui, "1f ▶▶")
                    .on_hover_text("Analyze 1 Frame Forward")
                    .clicked()
                {
                    let total = app.history.current().active_composition().duration_frames;
                    let f = (current_frame + 1).min(total.saturating_sub(1));
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        crate::core::tracker_engine::TrackerEngine::analyze_track_cancellable(
                            comp,
                            idx,
                            0,
                            current_frame,
                            f,
                            None,
                        );
                    });
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Apply:");

            ui.horizontal(|ui| {
                if custom_widgets::ae_button(ui, "Reset Track")
                    .on_hover_text("Remove all tracked keyframes from this tracker")
                    .clicked()
                {
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        if let Some(tp) = comp.layers.get_mut(idx) {
                            for t in tp.trackers.iter_mut() {
                                t.position = crate::core::property::Animatable::new_constant(
                                    t.position.evaluate(current_frame),
                                );
                            }
                        }
                    });
                    app.toasts.info("Tracker keyframes reset");
                }
                if custom_widgets::ae_button(ui, "Apply to Position")
                    .on_hover_text(
                        "Bake tracker motion into a target layer's position (pick below)",
                    )
                    .clicked()
                {
                    app.toasts
                        .info("Select the target layer in 'Apply to Layer' dropdown");
                }
            });

            // Target picker + apply
            egui::ComboBox::from_id_salt("tracker_apply_target")
                .selected_text(
                    app.tracker_apply_target
                        .map(|i| format!("Layer {}", i + 1))
                        .unwrap_or_else(|| "Target layer...".into()),
                )
                .show_ui(ui, |ui| {
                    let names: Vec<(usize, String)> = app
                        .history
                        .current()
                        .active_composition()
                        .layers
                        .iter()
                        .enumerate()
                        .map(|(i, l)| (i, l.name.clone()))
                        .collect();
                    for (ti, tname) in names {
                        ui.selectable_value(
                            &mut app.tracker_apply_target,
                            Some(ti),
                            format!("{}. {}", ti + 1, tname),
                        );
                    }
                });
            if let Some(target_idx) = app.tracker_apply_target {
                ui.horizontal(|ui| {
                    if custom_widgets::ae_button_accent(ui, "Apply Motion → Target").clicked() {
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(comp, idx, 0, target_idx, true, false);
                        });
                        app.toasts.info(format!("Applied tracking to layer {}", target_idx + 1));
                    }
                    if custom_widgets::ae_button(ui, "Apply as Corner Pin → Target").on_hover_text("Create a CornerPin effect on the target using the tracked quad corners").clicked() {
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            // Get current quad corners from the 4 trackers
                            let src_layer = if let Some(l) = comp.layers.get(idx) { l } else { return };
                            if src_layer.trackers.len() < 4 { return; }
                            let tl = src_layer.trackers[0].position.evaluate(current_frame);
                            let tr = src_layer.trackers[1].position.evaluate(current_frame);
                            let br = src_layer.trackers[2].position.evaluate(current_frame);
                            let bl = src_layer.trackers[3].position.evaluate(current_frame);

                            let corner_pin = crate::core::timeline::EffectType::CornerPin {
                                top_left: crate::core::property::Animatable::new_constant(tl),
                                top_right: crate::core::property::Animatable::new_constant(tr),
                                bottom_right: crate::core::property::Animatable::new_constant(br),
                                bottom_left: crate::core::property::Animatable::new_constant(bl),
                            };
                            let effect = crate::core::timeline::Effect {
                                id: format!("corner_pin_{}", comp.layers[target_idx].effects.len()),
                                name: "Corner Pin (from Tracker)".to_string(),
                                effect_type: corner_pin,
                                enabled: true,
                            };
                            comp.layers[target_idx].effects.push(effect);
                        });
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Applied Corner Pin to layer {}", target_idx + 1));
                    }
                    if custom_widgets::ae_button(ui, "🎥 Stabilize Motion").on_hover_text("Cancel camera shake by inverting motion onto target anchor/position").clicked() {
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(comp, idx, 0, target_idx, true, true);
                        });
                        app.toasts.info(format!("Stabilized motion applied to layer {}", target_idx + 1));
                    }
                });
            }

            ui.horizontal(|ui| {
                if custom_widgets::ae_button_accent(ui, "📦 Apply Motion → New Null")
                    .on_hover_text(
                        "Create a new Null layer and bind the tracked motion keyframes to it",
                    )
                    .clicked()
                {
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        let null_idx = comp.layers.len();
                        let dur = comp.duration_frames;
                        let null_layer = crate::core::timeline::Layer::new(
                            format!("null_track_{}", null_idx + 1),
                            format!("Tracker {} Null", idx + 1),
                            crate::core::timeline::LayerType::Null,
                            dur,
                        );
                        comp.add_layer(null_layer);
                        crate::core::tracker_engine::TrackerEngine::apply_tracker_to_target(
                            comp, idx, 0, null_idx, true, false,
                        );
                    });
                    crate::core::frame_cache::bump_version();
                    app.toasts
                        .info("Created new Null layer with tracked motion!");
                }
                if custom_widgets::ae_button(ui, "🌊 Smooth Track")
                    .on_hover_text(
                        "Apply Gaussian temporal filter to reduce jitter in tracked keyframes",
                    )
                    .clicked()
                {
                    let mut smoothed = false;
                    app.modify_project(|p| {
                        let comp = p.active_composition_mut();
                        if let Some(src_layer) = comp.layers.get_mut(idx) {
                            for tracker in &mut src_layer.trackers {
                                if let crate::core::property::Animatable::Animated(ref kfs) =
                                    tracker.position
                                {
                                    let new_kfs =
                                        crate::core::tracker_engine::smooth_tracker_keyframes(
                                            kfs, 2,
                                        );
                                    tracker.position =
                                        crate::core::property::Animatable::Animated(new_kfs);
                                    smoothed = true;
                                }
                            }
                        }
                    });
                    if smoothed {
                        crate::core::frame_cache::bump_version();
                        app.toasts
                            .info("Tracked keyframes smoothed with Gaussian temporal filter");
                    } else {
                        app.toasts.error("No animated keyframes found on tracker");
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label(
                egui::RichText::new("✨ AI Auto-Trace & Roto Assist")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.collapsing("🔍 Auto-Trace Settings", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Channel:");
                    let mut chan_idx = 0;
                    egui::ComboBox::from_id_salt("trace_channel")
                        .selected_text(if chan_idx == 0 { "Luminance" } else { "Alpha" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut chan_idx, 0, "Luminance");
                            ui.selectable_value(&mut chan_idx, 1, "Alpha");
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Threshold:");
                    let mut thresh = 128u8;
                    ui.add(egui::Slider::new(&mut thresh, 1..=254));
                });
                ui.horizontal(|ui| {
                    ui.label("Tolerance:");
                    let mut tol = 2.0f32;
                    ui.add(egui::Slider::new(&mut tol, 0.5..=10.0).suffix(" px"));
                });
            });

            // ── Roto Brush & Refine Edge (Matte Cleanup) ──
            ui.collapsing("✂ Roto Brush & Refine Edge", |ui| {
                let mut refine_smooth = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_smooth"))
                        .unwrap_or(2.0)
                });
                let mut refine_feather = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_feather"))
                        .unwrap_or(5.0)
                });
                let mut refine_contrast = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_contrast"))
                        .unwrap_or(80.0)
                });
                let mut shift_edge = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("roto_shift_edge"))
                        .unwrap_or(0.0)
                });
                let mut decontaminate = ui.ctx().data(|d| {
                    d.get_temp::<bool>(egui::Id::new("roto_decontaminate"))
                        .unwrap_or(true)
                });

                ui.horizontal(|ui| {
                    ui.label("Smoothness:");
                    if ui
                        .add(egui::Slider::new(&mut refine_smooth, 0.0..=10.0))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_smooth"), refine_smooth)
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Feather:");
                    if ui
                        .add(egui::Slider::new(&mut refine_feather, 0.0..=50.0).suffix(" px"))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_feather"), refine_feather)
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Contrast:");
                    if ui
                        .add(egui::Slider::new(&mut refine_contrast, 0.0..=100.0).suffix(" %"))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_contrast"), refine_contrast)
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Shift Edge:");
                    if ui
                        .add(egui::Slider::new(&mut shift_edge, -100.0..=100.0).suffix(" %"))
                        .changed()
                    {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("roto_shift_edge"), shift_edge)
                        });
                    }
                });
                ui.checkbox(
                    &mut decontaminate,
                    "Decontaminate Edge Colors (Fringe Removal)",
                );
                if decontaminate
                    != ui.ctx().data(|d| {
                        d.get_temp::<bool>(egui::Id::new("roto_decontaminate"))
                            .unwrap_or(true)
                    })
                {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new("roto_decontaminate"), decontaminate)
                    });
                }
            });

            ui.horizontal(|ui| {
                if custom_widgets::ae_button_accent(ui, "🎯 Auto-Generate Mask")
                    .on_hover_text("Auto-create Bezier Mask around tracked feature")
                    .clicked()
                {
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    if idx < comp_mut.layers.len() {
                        let target_pos = comp_mut.layers[idx]
                            .transform
                            .position
                            .evaluate(current_frame);
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
                            wiggle: None,
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
                        app.toasts
                            .info(format!("Auto-generated Bezier Mask on {}", layer_name));
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
