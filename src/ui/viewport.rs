use crate::AfterEffectsApp;
use crate::core::timeline::Layer;
use crate::core::property::Animatable;
use crate::ViewportMode;
use eframe::egui;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: u32) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // ── AE Composition Viewport Tab Bar (TOP) ──────────────────────────────────
        let active_comp_name = app.history.current().active_composition().name.clone();
        ui.horizontal(|ui| {
            let tab_frame = egui::Frame::none()
                .fill(egui::Color32::from_rgb(34, 38, 46))
                .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 65, 80)));
            
            tab_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Composition: {}", active_comp_name)).strong().color(egui::Color32::WHITE));
                    if ui.small_button("×").clicked() {
                        log::info!("Composition tab active");
                    }
                });
            });

            ui.add_space(8.0);
            let mode_2d = app.viewport_mode == ViewportMode::Comp2D;
            if ui.selectable_label(mode_2d, "2D").clicked() {
                app.viewport_mode = ViewportMode::Comp2D;
            }
            if ui.selectable_label(!mode_2d, "3D Camera").clicked() {
                app.viewport_mode = ViewportMode::Camera3D;
            }
        });
        ui.separator();

        // ── Comp Settings Modal ──────────────────────────────────────────────
        crate::ui::comp_settings_dialog::draw_comp_settings_dialog(app, ctx);

        // ── AE Viewport Composition Top Tabs & Breadcrumbs Bar ──
        let active_comp_idx = app.history.current().active_composition_idx;
        let comps_count = app.history.current().compositions.len();
        ui.horizontal(|ui| {
            for idx in 0..comps_count {
                let is_active = idx == active_comp_idx;
                let c_name = app.history.current().compositions[idx].name.clone();
                let tab_text = format!("🎞 Composition: {} {}", c_name, if is_active { "x" } else { "" });
                if ui.selectable_label(is_active, tab_text).clicked() {
                    let mut p = app.history.current().clone();
                    p.active_composition_idx = idx;
                    app.history.commit(p);
                }
            }
            ui.separator();
            ui.small(egui::RichText::new("Composition Flow: Main Comp > Active Layer").color(egui::Color32::from_gray(140)));
        });
        ui.separator();

        let size = ui.available_size();
        let (rect, viewport_response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

        // ── Viewport zoom: scroll wheel scales magnification anchored at the pointer ──
        if viewport_response.hovered() {
            let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_y != 0.0 && !ui.input(|i| i.modifiers.command) {
                // Fit mode (0.0) resolves to a concrete ratio first so zooming feels continuous
                let current = if app.viewport_mag_ratio == 0.0 { 1.0 } else { app.viewport_mag_ratio };
                let factor = if scroll_y > 0.0 { 1.1 } else { 1.0 / 1.1 };
                let new_mag = (current * factor).clamp(0.05, 8.0);

                // Keep the content point under the cursor stationary:
                // screen_delta = content_pos * (new_scale - old_scale), where scale
                // is measured relative to the fit size.
                if let Some(pointer) = viewport_response.hover_pos() {
                    let aspect = {
                        let comp = app.history.current().active_composition();
                        comp.width as f32 / comp.height as f32
                    };
                    let (fit_w, fit_h) = {
                        let mut fw = rect.width();
                        let mut fh = fw / aspect;
                        if fh > rect.height() {
                            fh = rect.height();
                            fw = fh * aspect;
                        }
                        (fw, fh)
                    };
                    let center = rect.center();
                    // Content coordinate under the pointer at the OLD zoom
                    let old_scale = if app.viewport_mag_ratio == 0.0 { 1.0 } else { app.viewport_mag_ratio };
                    let content = egui::vec2(
                        (pointer.x - center.x) / (fit_w * old_scale),
                        (pointer.y - center.y) / (fit_h * old_scale),
                    );
                    // Pan adjustment keeps that content point at the pointer
                    app.viewport_pan.x += content.x * fit_w * (old_scale - new_mag);
                    app.viewport_pan.y += content.y * fit_h * (old_scale - new_mag);
                }
                app.viewport_mag_ratio = new_mag;
            }
            // Middle-drag pans regardless of active tool (AE behavior)
            if ui.input(|i| i.pointer.middle_down()) {
                app.active_tool = crate::ui::toolbar::ActiveTool::Hand;
            }
        }

        // Render background
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(20));

        let comp = app.history.current().active_composition();
        let aspect = comp.width as f32 / comp.height as f32;
        let (origin_x, origin_y, draw_w, draw_h) =
            crate::ui::viewport_state::compute_draw_layout_pan(rect, aspect, app.viewport_mag_ratio, app.viewport_pan);
        let draw_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x, origin_y),
            egui::vec2(draw_w, draw_h),
        );
        let comp_w = comp.width as f32;
        let comp_h = comp.height as f32;

        let snap_frame_opt = crate::ui::viewport_state::snap_frame(ctx);
        let is_comparing = crate::ui::viewport_state::is_comparing(ctx);
        let wipe_pos = crate::ui::viewport_state::wipe_pos(ctx);

        // -- GPU Render Path --
        #[allow(unused_mut)]
        let mut rendered_gpu = false;
        #[cfg(feature = "wgpu")]
        if let Some(renderer) = &mut app.renderer {
            if let Some(wgpu_state) = &app.wgpu_state {
                let exp_id = egui::Id::new("ae_exposure_ev");
                let exposure_ev = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(exp_id, || 0.0f32));
                let lut_id = egui::Id::new("ae_colorspace_lut");
                let lut_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(lut_id, || 0usize));

                let snap_id = egui::Id::new(crate::ui::viewport_state::SNAP_FRAME_ID);
                let is_comparing_id = egui::Id::new(crate::ui::viewport_state::IS_COMPARING_ID);
                let wipe_id = egui::Id::new(crate::ui::viewport_state::WIPE_POS_ID);

                let snap_frame_opt = ctx.data_mut(|d| d.get_temp::<u32>(snap_id));
                let is_comparing = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(is_comparing_id, || false));
                let wipe_pos = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(wipe_id, || 0.5f32));

                // Render at display resolution: a 4K comp shown in an 800px
                // viewport renders at ~800px wide (4-16x less fill rate).
                // While playing, the adaptive factor further reduces resolution when
                // frames exceed the playback budget (AE-style auto quality drop).
                let display_px = (draw_w * ctx.pixels_per_point()).ceil();
                let preview_px = ((display_px * app.adaptive_preview_factor) as u32).clamp(64, 4096);
                renderer.set_preview_max_width(Some(preview_px));

                // ── RAM preview: incremental pre-pass, a few frames per UI frame ──
                // Rendering everything up front would freeze the UI on heavy comps;
                // instead we chip away at the work area while playback runs.
                const RAM_PREPASS_FRAMES_PER_TICK: u32 = 6;
                const RAM_PREPASS_MAX_FRAMES: u32 = 300;
                if app.is_playing && !app.was_playing_last_frame {
                    let wa_in = app.work_area_in.unwrap_or(0);
                    let wa_out = app
                        .work_area_out
                        .unwrap_or(comp.duration_frames)
                        .min(comp.duration_frames.saturating_sub(1));
                    app.ram_texture_ids.clear();
                    app.ram_prepass_cursor = wa_in;
                    app.ram_prepass_end = wa_in + (wa_out - wa_in).min(RAM_PREPASS_MAX_FRAMES);
                }
                if app.is_playing && app.ram_prepass_cursor <= app.ram_prepass_end {
                    let batch_end = app.ram_prepass_cursor + RAM_PREPASS_FRAMES_PER_TICK - 1;
                    let batch_end = batch_end.min(app.ram_prepass_end);
                    renderer.render_ram_preview_range(
                        comp,
                        app.ram_prepass_cursor,
                        batch_end,
                        exposure_ev,
                        lut_idx as u32,
                        app.ram_prepass_end - app.ram_prepass_cursor + 1,
                    );
                    for f in app.ram_prepass_cursor..=batch_end {
                        if let Some(view) = renderer.ram_frame_view(f) {
                            let id = wgpu_state.renderer.write().register_native_texture(
                                &wgpu_state.device,
                                view,
                                wgpu::FilterMode::Linear,
                            );
                            app.ram_texture_ids.push((f, id));
                        }
                    }
                    app.ram_prepass_cursor = batch_end.saturating_add(1);
                }
                if !app.is_playing && app.was_playing_last_frame {
                    // Playback stopped: free ring textures and egui ids
                    for (_, id) in app.ram_texture_ids.drain(..) {
                        wgpu_state.renderer.write().free_texture(&id);
                    }
                    renderer.clear_ram_preview();
                }
                app.was_playing_last_frame = app.is_playing;

                // During playback, prefer a cached RAM frame over a live render
                let ram_id = if app.is_playing {
                    app.ram_texture_ids
                        .iter()
                        .find(|(f, _)| *f == current_frame)
                        .map(|(_, id)| *id)
                } else {
                    None
                };

                let render_started = std::time::Instant::now();
                if let Some(id) = ram_id {
                    // Cached frame: swap the displayed texture, no GPU re-render
                    if app.viewport_texture_id != Some(id) {
                        if let Some(old_id) = app.viewport_texture_id {
                            wgpu_state.renderer.write().free_texture(&old_id);
                        }
                        app.viewport_texture_id = Some(id);
                    }
                    rendered_gpu = true;
                }
                let (texture_view, recreated) = if ram_id.is_some() {
                    // Skip live rendering entirely this frame
                    (None, false)
                } else {
                    let (view, rec) = renderer.render(comp, current_frame, exposure_ev, lut_idx as u32);
                    (Some(view), rec)
                };
                let render_ms = render_started.elapsed().as_secs_f32() * 1000.0;
                app.preview_render_ema_ms = if app.preview_render_ema_ms <= 0.0 {
                    render_ms
                } else {
                    app.preview_render_ema_ms * 0.9 + render_ms * 0.1
                };

                // Adapt quality only during playback; idle renders restore full quality
                if app.is_playing {
                    let budget_ms = 1000.0 / comp.fps.max(1) as f32 * 0.8;
                    if app.preview_render_ema_ms > budget_ms {
                        app.adaptive_preview_factor = (app.adaptive_preview_factor * 0.8).max(0.125);
                    } else if app.preview_render_ema_ms < budget_ms * 0.5 {
                        app.adaptive_preview_factor = (app.adaptive_preview_factor * 1.15).min(1.0);
                    }
                } else {
                    app.adaptive_preview_factor = app.adaptive_preview_factor.max(0.9); // drift back to full
                }
                if let Some(view) = texture_view {
                    if app.viewport_texture_id.is_none() || recreated {
                        if let Some(old_id) = app.viewport_texture_id {
                            wgpu_state.renderer.write().free_texture(&old_id);
                        }
                        let texture_id = wgpu_state.renderer.write().register_native_texture(
                            &wgpu_state.device,
                            view,
                            wgpu::FilterMode::Linear,
                        );
                        app.viewport_texture_id = Some(texture_id);
                    }
                }

                let mut snap_texture_id_val = None;
                if is_comparing {
                    if let Some(snap_frame) = snap_frame_opt {
                        let (snap_view, snap_recreated) = renderer.render_snapshot_frame(comp, snap_frame, exposure_ev, lut_idx as u32);
                        if app.viewport_snapshot_texture_id.is_none() || snap_recreated {
                            if let Some(old_id) = app.viewport_snapshot_texture_id {
                                wgpu_state.renderer.write().free_texture(&old_id);
                            }
                            let texture_id = wgpu_state.renderer.write().register_native_texture(
                                &wgpu_state.device,
                                snap_view,
                                wgpu::FilterMode::Linear,
                            );
                            app.viewport_snapshot_texture_id = Some(texture_id);
                        }
                        snap_texture_id_val = app.viewport_snapshot_texture_id;
                    }
                }

                if let (true, Some(cur_tex), Some(snap_tex)) = (is_comparing, app.viewport_texture_id, snap_texture_id_val) {
                    // Left Side: Current Frame
                    let left_rect = egui::Rect::from_min_max(
                        draw_rect.min,
                        egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.max.y),
                    );
                    let left_img = egui::Image::new(egui::load::SizedTexture::new(cur_tex, left_rect.size()))
                        .uv(egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(wipe_pos, 1.0)));
                    ui.put(left_rect, left_img);

                    // Right Side: Snapshot Frame (Compare)
                    let right_rect = egui::Rect::from_min_max(
                        egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.min.y),
                        draw_rect.max,
                    );
                    let right_img = egui::Image::new(egui::load::SizedTexture::new(snap_tex, right_rect.size()))
                        .uv(egui::Rect::from_min_max(egui::pos2(wipe_pos, 0.0), egui::pos2(1.0, 1.0)));
                    ui.put(right_rect, right_img);

                    rendered_gpu = true;
                } else if let Some(texture_id) = app.viewport_texture_id {
                    ui.put(draw_rect, egui::Image::new(egui::load::SizedTexture::new(texture_id, draw_rect.size())));
                    rendered_gpu = true;
                }
            }
        }

        // -- CPU Fallback Render (2D) --
        // -- 3D Camera View --
        if app.viewport_mode == ViewportMode::Camera3D {
            crate::ui::viewport_camera_3d::draw_camera_3d_viewport(
                ui, app, ctx, current_frame, &viewport_response, rect, comp_w, comp_h, draw_w, draw_h,
            );
        }

        if app.viewport_mode == ViewportMode::Comp2D && !rendered_gpu && is_comparing {
            if let Some(snap_frame) = snap_frame_opt {
                let left_rect = egui::Rect::from_min_max(
                    draw_rect.min,
                    egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.max.y),
                );
                let right_rect = egui::Rect::from_min_max(
                    egui::pos2(draw_rect.min.x + draw_rect.width() * wipe_pos, draw_rect.min.y),
                    draw_rect.max,
                );
                crate::ui::viewport_canvas::draw_software_canvas(
                    ui, app, current_frame, draw_rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h,
                    Some(left_rect),
                );
                crate::ui::viewport_canvas::draw_software_canvas(
                    ui, app, snap_frame, draw_rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h,
                    Some(right_rect),
                );
            } else {
                crate::ui::viewport_canvas::draw_software_canvas(
                    ui, app, current_frame, draw_rect, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h,
                    None,
                );
            }
        }

        // ── Viewport Overlays (Grid, Safe Guides, HUD Badges, 3D Gizmo, Snapshot Wipe) ──
        crate::ui::viewport_overlays::draw_viewport_overlays(
            ui, app, ctx, current_frame, origin_x, origin_y, draw_w, draw_h, comp_w, comp_h, rendered_gpu,
        );

        // ── Interactive Layer & Mask Drag ──────────────────────────
        if let Some(pointer_pos) = viewport_response.interact_pointer_pos() {
            let comp_px = (pointer_pos.x - origin_x) / draw_w * comp_w;
            let comp_py = (pointer_pos.y - origin_y) / draw_h * comp_h;

            if viewport_response.drag_started() {
                // ── Transactional Drag: capture pre-drag snapshot ONCE ──
                app.begin_drag("Viewport Transform");

                let comp_state = app.history.current().active_composition();
                let mut mask_hit: Option<(usize, usize, usize)> = None;

                // 1. Check if clicking on selected layer's mask vertices
                if let Some(sel_li) = app.selected_layer_idx {
                    if sel_li < comp_state.layers.len() {
                        let l = &comp_state.layers[sel_li];
                        for (mi, mask) in l.masks.iter().enumerate() {
                            if mask.enabled {
                                let verts = mask.path.vertices_at_frame(current_frame);
                                for (vi, vertex_pt) in verts.iter().enumerate() {
                                    let vx = vertex_pt[0];
                                    let vy = vertex_pt[1];
                                    
                                    // Calculate viewport screen position
                                    let screen_x = origin_x + (vx / comp_w) * draw_w;
                                    let screen_y = origin_y + (vy / comp_h) * draw_h;
                                    let dist = ((pointer_pos.x - screen_x).powi(2) + (pointer_pos.y - screen_y).powi(2)).sqrt();
                                    if dist <= 12.0 {
                                        mask_hit = Some((sel_li, mi, vi));
                                        break;
                                    }
                                }
                            }
                            if mask_hit.is_some() { break; }
                        }
                    }
                }

                if let Some((l_idx, m_idx, v_idx)) = mask_hit {
                    let verts = comp_state.layers[l_idx].masks[m_idx].path.vertices_at_frame(current_frame);
                    let start_vertex_pos = if v_idx < verts.len() { verts[v_idx] } else { [0.0, 0.0] };
                    app.viewport_mask_drag_state = Some((l_idx, m_idx, v_idx, start_vertex_pos, pointer_pos));
                    app.viewport_drag_state = None;
                } else {
                    app.viewport_mask_drag_state = None;

                    // 2. Fallback to Layer translation/rotation drag
                    let mut hit_idx: Option<usize> = None;
                    for (i, layer) in comp_state.layers.iter().enumerate().rev() {
                        let l: &Layer = layer;
                        if !l.is_active(current_frame) || l.locked { continue; }
                        let pos = l.transform.position.evaluate(current_frame);
                        let scale = l.transform.scale.evaluate(current_frame);
                        let hw = scale[0].abs() * 0.6;
                        let hh = scale[1].abs() * 0.6;
                        if (comp_px - pos[0]).abs() <= hw && (comp_py - pos[1]).abs() <= hh {
                            hit_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(idx) = hit_idx {
                        let pos_now = comp_state.layers[idx].transform.position.evaluate(current_frame);
                        app.viewport_drag_state = Some((idx, pos_now, pointer_pos));
                        app.selected_layer_idx = Some(idx);
                    } else {
                        app.viewport_drag_state = None;
                    }
                }
            }

            if viewport_response.dragged() {
                if let Some((l_idx, m_idx, v_idx, start_vertex_pos, start_ptr)) = app.viewport_mask_drag_state {
                    let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                    let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
                    
                    let comp_mut = app.history.current_mut().active_composition_mut();
                    if l_idx < comp_mut.layers.len() {
                        let layer = &mut comp_mut.layers[l_idx];
                        if m_idx < layer.masks.len() {
                            let mask = &mut layer.masks[m_idx];
                            let new_pos = [
                                start_vertex_pos[0] + delta_x,
                                start_vertex_pos[1] + delta_y,
                            ];
                            mask.path.set_vertex_at_frame(current_frame, v_idx, new_pos);
                        }
                    }
                } else if let Some((drag_idx, start_pos, start_ptr)) = app.viewport_drag_state {
                    let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                    let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
                    let new_pos = [start_pos[0] + delta_x, start_pos[1] + delta_y];

                    let comp_mut = app.history.current_mut().active_composition_mut();
                    if drag_idx < comp_mut.layers.len() {
                        let layer = &mut comp_mut.layers[drag_idx];
                        use crate::ui::toolbar::ActiveTool;
                        match app.active_tool {
                            ActiveTool::Rotation => {
                                let rot_delta = delta_x * 0.5;
                                let current_r = layer.transform.rotation.evaluate(current_frame);
                                match &mut layer.transform.rotation {
                                    Animatable::Constant(ref mut r) => *r += rot_delta,
                                    Animatable::Animated(ref mut kfs) => {
                                        if let Some(kf) = kfs.iter_mut().find(|k| k.frame == current_frame) {
                                            kf.value += rot_delta;
                                        } else {
                                            kfs.push(crate::core::keyframe::Keyframe::new(current_frame, current_r + rot_delta, crate::core::keyframe::InterpolationType::Linear));
                                            kfs.sort_by_key(|k| k.frame);
                                        }
                                    }
                                }
                            }
                            ActiveTool::AnchorPoint => {
                                let cur_ap = layer.transform.anchor_point.evaluate(current_frame);
                                match &mut layer.transform.anchor_point {
                                    Animatable::Constant(ref mut ap) => {
                                        ap[0] += delta_x;
                                        ap[1] += delta_y;
                                    }
                                    Animatable::Animated(ref mut kfs) => {
                                        if let Some(kf) = kfs.iter_mut().find(|k| k.frame == current_frame) {
                                            kf.value[0] += delta_x;
                                            kf.value[1] += delta_y;
                                        } else {
                                            kfs.push(crate::core::keyframe::Keyframe::new(current_frame, [cur_ap[0] + delta_x, cur_ap[1] + delta_y], crate::core::keyframe::InterpolationType::Linear));
                                            kfs.sort_by_key(|k| k.frame);
                                        }
                                    }
                                }
                            }
                            _ => {
                                match &mut layer.transform.position {
                                    Animatable::Constant(ref mut pos) => {
                                        *pos = new_pos;
                                    }
                                    Animatable::Animated(ref mut keyframes) => {
                                        let existing_idx = keyframes.iter().position(|kf| kf.frame == current_frame);
                                        if let Some(idx) = existing_idx {
                                            keyframes[idx].value = new_pos;
                                        } else {
                                            keyframes.push(crate::core::keyframe::Keyframe::new(
                                                current_frame,
                                                new_pos,
                                                crate::core::keyframe::InterpolationType::Linear,
                                            ));
                                            keyframes.sort_by_key(|kf| kf.frame);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if viewport_response.drag_stopped() {
                let was_dragging = app.viewport_drag_state.is_some()
                    || app.viewport_mask_drag_state.is_some();
                if was_dragging {
                    // Commit the single Undo entry for the entire drag gesture
                    app.commit_drag();
                }
                app.viewport_drag_state = None;
                app.viewport_mask_drag_state = None;
            }
        }

        // ── AE Viewport Controls Toolbar (BOTTOM OF CANVAS) ──────────────────────────
        ui.separator();
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 4.0;
            // AE Magnification Ratio Dropdown
            let mag_val = app.viewport_mag_ratio;
            egui::ComboBox::from_id_salt("mag_combo_bottom")
                .selected_text(if mag_val == 4.0 { "400%" } else if mag_val == 2.0 { "200%" } else if mag_val == 1.0 { "100%" } else if mag_val == 0.5 { "50%" } else if mag_val == 0.25 { "25%" } else { "Fit" })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(app.viewport_mag_ratio == 0.0, "Fit").clicked() { app.viewport_mag_ratio = 0.0; }
                    if ui.selectable_label(app.viewport_mag_ratio == 0.25, "25%").clicked() { app.viewport_mag_ratio = 0.25; }
                    if ui.selectable_label(app.viewport_mag_ratio == 0.5, "50%").clicked() { app.viewport_mag_ratio = 0.5; }
                    if ui.selectable_label(app.viewport_mag_ratio == 1.0, "100%").clicked() { app.viewport_mag_ratio = 1.0; }
                    if ui.selectable_label(app.viewport_mag_ratio == 2.0, "200%").clicked() { app.viewport_mag_ratio = 2.0; }
                    if ui.selectable_label(app.viewport_mag_ratio == 4.0, "400%").clicked() { app.viewport_mag_ratio = 4.0; }
                });

            ui.separator();
            ui.checkbox(&mut app.show_grid, "Grid");
            ui.checkbox(&mut app.show_guides, "Safe");
            ui.checkbox(&mut app.show_handles, "Handles");

            ui.separator();
            // AE Camera View Selector
            egui::ComboBox::from_id_salt("cam_view_combo_bottom")
                .selected_text(match app.viewport_cam_view {
                    0 => "Active Camera",
                    1 => "Front",
                    2 => "Left",
                    3 => "Top",
                    _ => "Custom View 1",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut app.viewport_cam_view, 0, "Active Camera").clicked() {
                        app.viewport_mode = ViewportMode::Comp2D;
                    }
                    if ui.selectable_value(&mut app.viewport_cam_view, 1, "Front").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (0.0, 0.0, 1000.0);
                    }
                    if ui.selectable_value(&mut app.viewport_cam_view, 2, "Left").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (-90.0, 0.0, 1000.0);
                    }
                    if ui.selectable_value(&mut app.viewport_cam_view, 3, "Top").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (0.0, -89.0, 1000.0);
                    }
                });

            ui.separator();
            // AE Render Quality / Downsample Resolution
            egui::ComboBox::from_id_salt("res_combo_bottom")
                .selected_text(match app.viewport_render_resolution {
                    0 => "Full",
                    1 => "Half",
                    2 => "Third",
                    _ => "Quarter",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.viewport_render_resolution, 0, "Full");
                    ui.selectable_value(&mut app.viewport_render_resolution, 1, "Half");
                    ui.selectable_value(&mut app.viewport_render_resolution, 2, "Third");
                    ui.selectable_value(&mut app.viewport_render_resolution, 3, "Quarter");
                });

            ui.separator();
            // AE Color Channels
            egui::ComboBox::from_id_salt("chan_combo_bottom")
                .selected_text(match app.viewport_color_channel {
                    0 => "RGB Color",
                    1 => "Red",
                    2 => "Green",
                    3 => "Blue",
                    _ => "Alpha",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.viewport_color_channel, 0, "RGB Color");
                    ui.selectable_value(&mut app.viewport_color_channel, 1, "Red");
                    ui.selectable_value(&mut app.viewport_color_channel, 2, "Green");
                    ui.selectable_value(&mut app.viewport_color_channel, 3, "Blue");
                    ui.selectable_value(&mut app.viewport_color_channel, 4, "Alpha");
                });

            ui.separator();
            // AE Snapshot & Fast Previews
            if ui.button("📷").on_hover_text("Take Snapshot (Cmd+F5)").clicked() {
                crate::ui::viewport_state::set_snap_frame(ctx, current_frame);
                app.toasts.info(format!("Snapshot saved at frame {}", current_frame));
            }
            let comparing = crate::ui::viewport_state::is_comparing(ctx);
            if ui.selectable_label(comparing, "👁").on_hover_text("Show Snapshot (F5)").clicked() {
                if crate::ui::viewport_state::snap_frame(ctx).is_some() {
                    crate::ui::viewport_state::toggle_comparing(ctx);
                } else {
                    app.toasts.warning("Take a snapshot first (📷)");
                }
            }

            ui.separator();
            egui::ComboBox::from_id_salt("fast_preview_combo")
                .selected_text(match app.viewport_fast_preview {
                    0 => "Off (Final Quality)",
                    1 => "Adaptive Resolution",
                    2 => "Fast Draft",
                    _ => "Wireframe",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.viewport_fast_preview, 0, "Off (Final Quality)");
                    ui.selectable_value(&mut app.viewport_fast_preview, 1, "Adaptive Resolution");
                    ui.selectable_value(&mut app.viewport_fast_preview, 2, "Fast Draft");
                    ui.selectable_value(&mut app.viewport_fast_preview, 3, "Wireframe");
                });
        });
    });
}
