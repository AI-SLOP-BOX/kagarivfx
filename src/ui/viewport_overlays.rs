#![allow(clippy::too_many_arguments)]

use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_viewport_overlays(
    ui: &mut egui::Ui,
    app: &AfterEffectsApp,
    ctx: &egui::Context,
    current_frame: u32,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
    rendered_gpu: bool,
) {
    // ── Grid Overlay ──
    if app.show_grid {
        let grid_step = draw_w / 16.0;
        let grid_stroke = egui::Stroke::new(0.5, colors::GRID_LINE);
        let mut gx = origin_x + grid_step;
        while gx < origin_x + draw_w {
            ui.painter().line_segment([egui::pos2(gx, origin_y), egui::pos2(gx, origin_y + draw_h)], grid_stroke);
            gx += grid_step;
        }
        let mut gy = origin_y + grid_step;
        while gy < origin_y + draw_h {
            ui.painter().line_segment([egui::pos2(origin_x, gy), egui::pos2(origin_x + draw_w, gy)], grid_stroke);
            gy += grid_step;
        }
    }

    // 📍 Live Motion Path Trajectory Overlay (AE Standard)
    let comp = app.history.current().active_composition();
    if let Some(idx) = app.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer = &comp.layers[idx];
            if let Some(kfs) = layer.transform.position.keyframes() {
                if kfs.len() >= 2 {
                    let path_stroke = egui::Stroke::new(1.5, colors::MOTION_PATH);
                    let to_screen = |v: [f32; 2]| {
                        let px = origin_x + (v[0] / comp_w) * draw_w;
                        let py = origin_y + (v[1] / comp_h) * draw_h;
                        egui::pos2(px, py)
                    };

                    // Sample each segment with the keyframe's real easing
                    // (Hold / Linear / Bezier) so the polyline matches what the
                    // renderer actually evaluates frame by frame.
                    for seg in kfs.windows(2) {
                        let (a, b) = (&seg[0], &seg[1]);
                        let span = (b.frame - a.frame).max(1);
                        let steps = ((span as usize) / 4).clamp(4, 32);
                        let mut prev = to_screen(a.value);
                        for s in 1..=steps {
                            let raw_t = s as f32 / steps as f32;
                            let eased_t = match &a.interpolation {
                                crate::core::keyframe::InterpolationType::Hold => 0.0,
                                crate::core::keyframe::InterpolationType::Linear => raw_t,
                                crate::core::keyframe::InterpolationType::Bezier { custom_bezier, .. } => {
                                    let c = custom_bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]);
                                    crate::core::keyframe::solve_bezier_eased_time(raw_t, c[0], c[1], c[2], c[3])
                                }
                            };
                            let val = [
                                a.value[0] + (b.value[0] - a.value[0]) * eased_t,
                                a.value[1] + (b.value[1] - a.value[1]) * eased_t,
                            ];
                            let p = to_screen(val);
                            ui.painter().line_segment([prev, p], path_stroke);
                            prev = p;
                        }
                    }

                    // Keyframe Anchor Point Markers (📍)
                    for kf in kfs {
                        let p = to_screen(kf.value);
                        ui.painter().circle_filled(p, 4.0, colors::KEYFRAME_DOT);
                        ui.painter().circle_stroke(p, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                    }

                    // ── Spatial Bezier Tangent Handles ──
                    // Draw outgoing/incoming tangent lines for each keyframe pair
                    // so users can see the spatial curve control points in the viewport.
                    for seg in kfs.windows(2) {
                        let (a, b) = (&seg[0], &seg[1]);
                        if let crate::core::keyframe::InterpolationType::Bezier { custom_bezier, .. } = &a.interpolation {
                            let c = custom_bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]);
                            let val_delta = [b.value[0] - a.value[0], b.value[1] - a.value[1]];

                            // Outgoing handle from keyframe a
                            let out_pos = [
                                a.value[0] + c[0] * val_delta[0],
                                a.value[1] + c[1] * val_delta[1],
                            ];
                            let a_screen = to_screen(a.value);
                            let out_screen = to_screen(out_pos);
                            let handle_stroke = egui::Stroke::new(1.0, colors::MOTION_PATH.linear_multiply(0.5));
                            ui.painter().line_segment([a_screen, out_screen], handle_stroke);
                            ui.painter().circle_filled(out_screen, 2.5, colors::MOTION_PATH.linear_multiply(0.6));

                            // Incoming handle for keyframe b
                            let in_pos = [
                                b.value[0] - (1.0 - c[2]) * val_delta[0],
                                b.value[1] - (1.0 - c[3]) * val_delta[1],
                            ];
                            let b_screen = to_screen(b.value);
                            let in_screen = to_screen(in_pos);
                            ui.painter().line_segment([b_screen, in_screen], handle_stroke);
                            ui.painter().circle_filled(in_screen, 2.5, colors::MOTION_PATH.linear_multiply(0.6));
                        }
                    }

                    // Current playhead position marker on the path
                    let cur = layer.transform.position.evaluate(app.current_frame);
                    let cp = to_screen(cur);
                    ui.painter().circle_filled(cp, 3.0, colors::TIMELINE_PLAYHEAD);
                    ui.painter().circle_stroke(cp, 3.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                }
            }
        }
    }

    // ── Action & Title Safe Guides Overlay ──
    if app.show_guides {
        let guide_stroke = egui::Stroke::new(1.0, colors::GUIDE_LINE);
        let cx = origin_x + draw_w * 0.5;
        let cy = origin_y + draw_h * 0.5;

        // Action Safe (90%)
        let as_w = draw_w * 0.9;
        let as_h = draw_h * 0.9;
        let as_rect = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(as_w, as_h));
        ui.painter().rect_stroke(as_rect, 0.0, guide_stroke);

        // Title Safe (80%)
        let ts_w = draw_w * 0.8;
        let ts_h = draw_h * 0.8;
        let ts_rect = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(ts_w, ts_h));
        ui.painter().rect_stroke(ts_rect, 0.0, guide_stroke);

        // Center Crosshair
        ui.painter().line_segment([egui::pos2(cx - 10.0, cy), egui::pos2(cx + 10.0, cy)], guide_stroke);
        ui.painter().line_segment([egui::pos2(cx, cy - 10.0), egui::pos2(cx, cy + 10.0)], guide_stroke);
    }

    // ── HUD Overlay Badges ──
    let backend_text = if rendered_gpu { "[GPU] WGPU Acceleration" } else { "[CPU] Software Canvas" };
    let backend_color = if rendered_gpu { colors::ACCENT_GREEN } else { colors::ACCENT_ORANGE };
    
    // Top Left Performance & FPS HUD Overlay
    let dt = ctx.input(|i| i.stable_dt.max(0.001));
    let real_fps = (1.0 / dt).clamp(1.0, 240.0);
    let render_ms = dt * 1000.0;
    let cached_frames = app.frame_cache.cached_count();
    let total_comp_frames = app.history.current().active_composition().duration_frames.max(1);
    let _cache_pct = (cached_frames as f32 / total_comp_frames as f32 * 100.0).clamp(0.0, 100.0);

    // Adaptive quality indicator
    let quality_pct = (app.adaptive_preview_factor * 100.0).round();
    let quality_label = if app.adaptive_preview_factor >= 1.0 {
        "FULL"
    } else if app.adaptive_preview_factor >= 0.5 {
        "½"
    } else {
        "¼"
    };

    let fps_text = format!(
        "⚡ {:.1}ms ({:.0} FPS) | Q:{}% ({}) | RAM:{}/{}",
        render_ms, real_fps, quality_pct, quality_label, cached_frames, total_comp_frames
    );
    let fps_rect = egui::Rect::from_min_size(
        egui::pos2(origin_x + 10.0, origin_y + 10.0),
        egui::vec2(320.0, 24.0),
    );
    ui.painter().rect_filled(fps_rect, 4.0, colors::HUD_BG);
    let stroke_c = if render_ms > 33.3 {
        colors::FPS_BAD
    } else {
        colors::FPS_GOOD
    };
    ui.painter().rect_stroke(fps_rect, 4.0, egui::Stroke::new(1.0, stroke_c));
    ui.painter().text(fps_rect.center(), egui::Align2::CENTER_CENTER, fps_text, egui::FontId::monospace(10.5), colors::HUD_TEXT);

    // Top Right Engine Badge
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(origin_x + draw_w - 180.0, origin_y + 10.0),
        egui::vec2(170.0, 24.0),
    );
    ui.painter().rect_filled(badge_rect, 4.0, egui::Color32::from_rgba_unmultiplied(20, 20, 20, 210));
    ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0, backend_color));
    ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, backend_text, egui::FontId::proportional(11.0), backend_color);

    // ── CPU-only Feature Notice ──
    // Masks, Text Animator, Layer Styles and DOF composite correctly only in
    // the software (export/CPU) path today. Surface one honest badge when any
    // of them is active so viewport vs export differences aren't mysterious.
    if rendered_gpu {
        let comp_now = app.history.current().active_composition();
        let mut reasons: Vec<String> = Vec::new();

        let masked = comp_now.layers.iter()
            .filter(|l| l.is_active(current_frame) && l.masks.iter().any(|m| m.enabled && m.mode != crate::core::mask::MaskMode::None))
            .count();
        if masked > 0 {
            reasons.push(format!("{} mask{}", masked, if masked > 1 { "s" } else { "" }));
        }

        let animated_text = comp_now.layers.iter()
            .filter(|l| l.is_active(current_frame))
            .filter(|l| l.text_animator.as_ref().map(|a| a.enabled).unwrap_or(false))
            .count();
        if animated_text > 0 {
            reasons.push(format!("text animator ×{}", animated_text));
        }

        let styled = comp_now.layers.iter()
            .filter(|l| l.is_active(current_frame))
            .filter(|l| l.style.drop_shadow.enabled || l.style.outer_glow.enabled || l.style.stroke.enabled)
            .count();
        if styled > 0 {
            reasons.push(format!("layer styles ×{}", styled));
        }

        if comp_now.active_camera.dof_enabled
            && comp_now.layers.iter().any(|l| l.is_active(current_frame) && l.is_3d)
        {
            reasons.push("3D DOF".to_string());
        }

        if !reasons.is_empty() {
            let warn_text = format!("⚠ {} → export only", reasons.join(", "));
            let warn_w = (warn_text.len() as f32 * 6.2 + 20.0).min(draw_w - 20.0);
            let warn_rect = egui::Rect::from_min_size(
                egui::pos2(origin_x + draw_w - warn_w - 10.0, origin_y + 40.0),
                egui::vec2(warn_w, 22.0),
            );
            ui.painter().rect_filled(warn_rect, 4.0, egui::Color32::from_rgba_unmultiplied(30, 24, 8, 210));
            ui.painter().rect_stroke(warn_rect, 4.0, egui::Stroke::new(1.0, colors::ACCENT_ORANGE));
            ui.painter().text(warn_rect.center(), egui::Align2::CENTER_CENTER, &warn_text, egui::FontId::proportional(10.5), colors::ACCENT_ORANGE);
        }
    }

    // Bottom Left Selection HUD Status
    if let Some(s_idx) = app.selected_layer_idx {
        let comp = app.history.current().active_composition();
        if s_idx < comp.layers.len() {
            let s_layer = &comp.layers[s_idx];
            let pos = s_layer.transform.position.evaluate(current_frame);
            let scale = s_layer.transform.scale.evaluate(current_frame);
            let rot = s_layer.transform.rotation.evaluate(current_frame);
            
            let status_str = format!(
                "SELECTED: Layer {} ({}) | Pos: ({:.0}, {:.0}) | Scale: {:.0}% | Rot: {:.1}°",
                s_idx + 1, s_layer.name, pos[0], pos[1], scale[0], rot
            );
            
            let status_rect = egui::Rect::from_min_size(
                egui::pos2(origin_x + 10.0, origin_y + draw_h - 34.0),
                egui::vec2(status_str.len() as f32 * 6.8 + 20.0, 24.0),
            );
            ui.painter().rect_filled(status_rect, 4.0, colors::HUD_BG);
            ui.painter().rect_stroke(status_rect, 4.0, egui::Stroke::new(1.0, colors::BORDER_ACTIVE));
            ui.painter().text(
                status_rect.center(),
                egui::Align2::CENTER_CENTER,
                status_str,
                egui::FontId::proportional(11.0),
                colors::HUD_STATUS_TEXT,
            );
        }
    }

    // ── 3D Axis Transform Gizmo Overlay ──
    if app.show_handles {
        if let Some(s_idx) = app.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if s_idx < comp.layers.len() {
                let s_layer = &comp.layers[s_idx];
                let (pos, _scale, _rot, _op) = comp.resolve_world_transform(s_layer, current_frame);
                let rx = origin_x + (pos[0] / comp_w) * draw_w;
                let ry = origin_y + (pos[1] / comp_h) * draw_h;
                let center = egui::pos2(rx, ry);

                // X-Axis (Red)
                let x_end = egui::pos2(rx + 65.0, ry);
                ui.painter().line_segment([center, x_end], egui::Stroke::new(2.5, colors::GIZMO_X));
                ui.painter().text(egui::pos2(x_end.x + 8.0, x_end.y), egui::Align2::LEFT_CENTER, "X", egui::FontId::proportional(12.0), colors::GIZMO_X);

                // Y-Axis (Green)
                let y_end = egui::pos2(rx, ry + 65.0);
                ui.painter().line_segment([center, y_end], egui::Stroke::new(2.5, colors::GIZMO_Y));
                ui.painter().text(egui::pos2(y_end.x, y_end.y + 8.0), egui::Align2::CENTER_TOP, "Y", egui::FontId::proportional(12.0), colors::GIZMO_Y);

                // Z-Axis (Blue Diagonal)
                let z_end = egui::pos2(rx - 45.0, ry + 45.0);
                ui.painter().line_segment([center, z_end], egui::Stroke::new(2.5, colors::GIZMO_Z));
                ui.painter().text(egui::pos2(z_end.x - 8.0, z_end.y + 4.0), egui::Align2::RIGHT_TOP, "Z", egui::FontId::proportional(12.0), colors::GIZMO_Z);

                // 8-Point Bounding Box Transform Handles
                let scale = s_layer.transform.scale.evaluate(current_frame);
                let hw = (scale[0].abs() / 100.0 * 100.0 * 0.5) * (draw_w / comp_w);
                let hh = (scale[1].abs() / 100.0 * 100.0 * 0.5) * (draw_h / comp_h);
                let bbox_corners = [
                    egui::pos2(rx - hw, ry - hh), // Top-Left
                    egui::pos2(rx, ry - hh),      // Top-Center
                    egui::pos2(rx + hw, ry - hh), // Top-Right
                    egui::pos2(rx + hw, ry),      // Mid-Right
                    egui::pos2(rx + hw, ry + hh), // Bottom-Right
                    egui::pos2(rx, ry + hh),      // Bottom-Center
                    egui::pos2(rx - hw, ry + hh), // Bottom-Left
                    egui::pos2(rx - hw, ry),      // Mid-Left
                ];

                let bbox_rect = egui::Rect::from_center_size(center, egui::vec2(hw * 2.0, hh * 2.0));
                ui.painter().rect_stroke(bbox_rect, 0.0, egui::Stroke::new(1.5, colors::BBOX_STROKE));

                let hover_pos = ctx.input(|i| i.pointer.hover_pos());

                for hp in bbox_corners {
                    let dist = hover_pos.map_or(999.0, |p| p.distance(hp));
                    let is_hovered = dist <= 14.0;

                    let h_size = if is_hovered { 12.0 } else { 7.0 };
                    let fill_c = if is_hovered { colors::HANDLE_HOVER_FILL } else { colors::HANDLE_NORMAL };
                    let stroke_c = if is_hovered { colors::HANDLE_HOVER_STROKE } else { colors::BORDER_ACTIVE };

                    let h_rect = egui::Rect::from_center_size(hp, egui::vec2(h_size, h_size));
                    ui.painter().rect_filled(h_rect, 1.0, fill_c);
                    ui.painter().rect_stroke(h_rect, 1.0, egui::Stroke::new(if is_hovered { 2.0 } else { 1.0 }, stroke_c));
                }

                // Center Target Circle with Magnetic Hover
                let center_dist = hover_pos.map_or(999.0, |p| p.distance(center));
                let is_center_hovered = center_dist <= 14.0;
                let c_radius = if is_center_hovered { 8.0 } else { 5.0 };

                ui.painter().circle_filled(center, c_radius, colors::CENTER_DOT);
                ui.painter().circle_stroke(center, c_radius, egui::Stroke::new(1.5, egui::Color32::BLACK));
                if is_center_hovered {
                    ui.painter().circle_stroke(center, 30.0, egui::Stroke::new(1.5, colors::CENTER_HOVER_RING));
                }

                // ── Vector Mask Bezier Tangents & Path Overlay ──
                for mask in &s_layer.masks {
                    if mask.enabled {
                        let verts = mask.path.vertices_at_frame(current_frame);
                        for v in &verts {
                            let vx = origin_x + (v[0] / comp_w) * draw_w;
                            let vy = origin_y + (v[1] / comp_h) * draw_h;
                            let v_pos = egui::pos2(vx, vy);

                            // Draw Mask Vertex Anchor Point
                            ui.painter().rect_filled(
                                egui::Rect::from_center_size(v_pos, egui::vec2(7.0, 7.0)),
                                1.0,
                                colors::KEYFRAME_DOT,
                            );
                            ui.painter().rect_stroke(
                                egui::Rect::from_center_size(v_pos, egui::vec2(7.0, 7.0)),
                                1.0,
                                egui::Stroke::new(1.0, egui::Color32::BLACK),
                            );
                        }
                    }
                }

                // Dragging Floating HUD Badge next to cursor
                if app.viewport_drag_state.is_some() || app.viewport_mask_drag_state.is_some() {
                    if let Some(ptr) = hover_pos {
                        let hud_pos = egui::pos2(ptr.x + 16.0, ptr.y + 16.0);
                        let hud_rect = egui::Rect::from_min_size(hud_pos, egui::vec2(130.0, 22.0));
                        ui.painter().rect_filled(hud_rect, 4.0, colors::HUD_BG);
                        ui.painter().rect_stroke(hud_rect, 4.0, egui::Stroke::new(1.0, colors::HUD_STROKE));
                        ui.painter().text(
                            hud_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Scale: ({:.0}%, {:.0}%)", scale[0], scale[1]),
                            egui::FontId::monospace(10.0),
                            colors::HUD_TEXT,
                        );
                    }
                }

                // Render 3D Vector Spatial Gizmo Overlay if layer is 3D
                if s_layer.is_3d {
                    let axis_len = 65.0f32;
                    // Red X-Axis Line
                    ui.painter().line_segment(
                        [center, egui::pos2(center.x + axis_len, center.y)],
                        egui::Stroke::new(2.5, colors::GIZMO_X),
                    );
                    ui.painter().text(
                        egui::pos2(center.x + axis_len + 8.0, center.y),
                        egui::Align2::LEFT_CENTER,
                        "X",
                        egui::FontId::monospace(11.0),
                        colors::GIZMO_X,
                    );

                        // Green Y-Axis Line
                        ui.painter().line_segment(
                            [center, egui::pos2(center.x, center.y - axis_len)],
                            egui::Stroke::new(2.5, colors::GIZMO_Y),
                        );
                        ui.painter().text(
                            egui::pos2(center.x, center.y - axis_len - 8.0),
                            egui::Align2::CENTER_BOTTOM,
                            "Y",
                            egui::FontId::monospace(11.0),
                            colors::GIZMO_Y,
                        );

                        // Blue Z-Axis Line (Diagonal Depth)
                        let z_end = egui::pos2(center.x - axis_len * 0.6, center.y + axis_len * 0.6);
                        ui.painter().line_segment(
                            [center, z_end],
                            egui::Stroke::new(2.5, colors::GIZMO_Z),
                        );
                        ui.painter().text(
                            egui::pos2(z_end.x - 8.0, z_end.y + 4.0),
                            egui::Align2::RIGHT_TOP,
                            "Z",
                            egui::FontId::monospace(11.0),
                            colors::GIZMO_Z,
                        );
                    }
                }
            }
        }

    // ── Snapshot A/B Interactive Split Wipe Line Overlay ──
    if crate::ui::viewport_state::is_comparing(ctx) {
        let wipe_pos = crate::ui::viewport_state::wipe_pos(ctx);
        let wipe_x = origin_x + wipe_pos * draw_w;
        let handle_center = egui::pos2(wipe_x, origin_y + draw_h * 0.5);

        ui.painter().line_segment(
            [egui::pos2(wipe_x, origin_y), egui::pos2(wipe_x, origin_y + draw_h)],
            egui::Stroke::new(2.5, colors::ACCENT_CYAN),
        );
        ui.painter().circle_filled(handle_center, 10.0, colors::ACCENT_CYAN);
        ui.painter().circle_stroke(handle_center, 10.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
        ui.painter().text(
            egui::pos2(wipe_x - 15.0, origin_y + 15.0),
            egui::Align2::RIGHT_TOP,
            "[Snap A]",
            egui::FontId::proportional(11.0),
            colors::ACCENT_CYAN,
        );
        ui.painter().text(
            egui::pos2(wipe_x + 15.0, origin_y + 15.0),
            egui::Align2::LEFT_TOP,
            "[Live Frame]",
            egui::FontId::proportional(11.0),
            colors::ACCENT_ORANGE,
        );

        let handle_rect = egui::Rect::from_center_size(handle_center, egui::vec2(24.0, draw_h.max(24.0)));
        let wipe_response = ui.interact(handle_rect, ui.id().with("viewport_wipe_drag"), egui::Sense::drag());
        if wipe_response.dragged() {
            if let Some(ptr) = wipe_response.interact_pointer_pos() {
                crate::ui::viewport_state::set_wipe_pos(
                    ctx,
                    ((ptr.x - origin_x) / draw_w).clamp(0.05, 0.95),
                );
            }
        }
    }
}
