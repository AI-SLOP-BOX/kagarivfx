use crate::AfterEffectsApp;
use crate::core::timeline::{Layer, LayerType};
use crate::core::property::Animatable;
use crate::ViewportMode;
use eframe::egui;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: u32) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Viewport Toolbar ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading("🎬 Viewport");
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Mode Toggle
            let mode_2d = app.viewport_mode == ViewportMode::Comp2D;
            if ui.selectable_label(mode_2d, "📺 2D").clicked() {
                app.viewport_mode = ViewportMode::Comp2D;
            }
            if ui.selectable_label(!mode_2d, "📦 3D Camera").clicked() {
                app.viewport_mode = ViewportMode::Camera3D;
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(&mut app.show_grid, "🌐 Grid");
            ui.checkbox(&mut app.show_guides, "📐 Safe");
            ui.checkbox(&mut app.show_handles, "🎯 Handles");
            ui.add_space(8.0);

            if ui.button("⚙ Comp Settings").clicked() {
                app.show_comp_settings = !app.show_comp_settings;
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            let comp = app.history.current().active_composition();
            ui.weak(egui::RichText::new(format!("{}  {}×{}  {} fps",
                comp.name, comp.width, comp.height, comp.fps))
                .color(egui::Color32::from_rgb(140, 160, 200)));
        });

        // ── Comp Settings Modal ──────────────────────────────────────────────
        if app.show_comp_settings {
            egui::Window::new("⚙ Composition Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let mut temp_project = app.history.current().clone();
                    let comp = temp_project.active_composition_mut();
                    let mut changed = false;

                    egui::Grid::new("comp_settings_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            let n = comp.name.clone();
                            ui.text_edit_singleline(&mut comp.name);
                            if n != comp.name { changed = true; }
                            ui.end_row();

                            ui.label("Width:");
                            let old = comp.width;
                            ui.add(egui::DragValue::new(&mut comp.width).clamp_range(1u32..=7680));
                            if old != comp.width { changed = true; }
                            ui.end_row();

                            ui.label("Height:");
                            let old = comp.height;
                            ui.add(egui::DragValue::new(&mut comp.height).clamp_range(1u32..=4320));
                            if old != comp.height { changed = true; }
                            ui.end_row();

                            ui.label("FPS:");
                            let old = comp.fps;
                            ui.add(egui::DragValue::new(&mut comp.fps).clamp_range(1u32..=240));
                            if old != comp.fps { changed = true; }
                            ui.end_row();

                            ui.label("Duration (frames):");
                            let old = comp.duration_frames;
                            ui.add(egui::DragValue::new(&mut comp.duration_frames).clamp_range(1u32..=108000));
                            if old != comp.duration_frames { changed = true; }
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("✅ Apply").clicked() || changed {
                            app.history.commit(temp_project);
                            crate::core::frame_cache::bump_version();
                        }
                        if ui.button("✖ Close").clicked() {
                            app.show_comp_settings = false;
                        }
                    });
                });
        }

        ui.separator();

        let size = ui.available_size();
        let (rect, viewport_response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

        // Render background
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(20));

        let comp = app.history.current().active_composition();
        let aspect = comp.width as f32 / comp.height as f32;
        let mut draw_w = rect.width();
        let mut draw_h = draw_w / aspect;
        if draw_h > rect.height() {
            draw_h = rect.height();
            draw_w = draw_h * aspect;
        }
        let origin_x = rect.left() + (rect.width() - draw_w) * 0.5;
        let origin_y = rect.top() + (rect.height() - draw_h) * 0.5;
        let draw_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x, origin_y),
            egui::vec2(draw_w, draw_h),
        );
        let comp_w = comp.width as f32;
        let comp_h = comp.height as f32;

        // -- GPU Render Path --
        let mut rendered_gpu = false;
        #[cfg(feature = "wgpu")]
        if let Some(renderer) = &mut app.renderer {
            if let Some(wgpu_state) = &app.wgpu_state {
                let comp_ref = app.history.current().active_composition();
                let (texture_view, recreated) = renderer.render(comp_ref, current_frame);
                if app.viewport_texture_id.is_none() || recreated {
                    if let Some(old_id) = app.viewport_texture_id {
                        wgpu_state.renderer.write().free_texture(&old_id);
                    }
                    let texture_id = wgpu_state.renderer.write().register_native_texture(
                        &wgpu_state.device,
                        texture_view,
                        wgpu::FilterMode::Linear,
                    );
                    app.viewport_texture_id = Some(texture_id);
                }
                if let Some(texture_id) = app.viewport_texture_id {
                    ui.put(draw_rect, egui::Image::new(egui::load::SizedTexture::new(texture_id, draw_rect.size())));
                    rendered_gpu = true;
                }
            }
        }

        // -- CPU Fallback Render (2D) --
        // -- 3D Camera View --
        if app.viewport_mode == ViewportMode::Camera3D {
            ui.painter().rect_filled(draw_rect, 0.0, egui::Color32::from_rgb(10, 14, 24));

            // Handle orbit drag (secondary / right mouse button)
            if viewport_response.dragged_by(egui::PointerButton::Secondary) {
                if let Some(delta) = viewport_response.drag_delta().into() {
                    let d: egui::Vec2 = delta;
                    app.camera_orbit.0 += d.x * 0.5;   // yaw
                    app.camera_orbit.1 += d.y * 0.5;   // pitch
                    app.camera_orbit.1 = app.camera_orbit.1.clamp(-89.0, 89.0);
                }
            }
            // Scroll to zoom
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.5 {
                app.camera_orbit.2 = (app.camera_orbit.2 - scroll * 12.0).clamp(100.0, 4000.0);
            }

            let (yaw_deg, pitch_deg, zoom) = app.camera_orbit;
            let yaw = yaw_deg.to_radians();
            let pitch = pitch_deg.to_radians();
            let cx_world = comp_w * 0.5;
            let cy_world = comp_h * 0.5;

            // Simple perspective projection helper
            let project_3d = |px: f32, py: f32, pz: f32| -> egui::Pos2 {
                // Translate to camera-centered coords
                let dx = px - cx_world;
                let dy = py - cy_world;
                let dz = pz;
                // Apply yaw (around Y axis)
                let rx = dx * yaw.cos() + dz * yaw.sin();
                let ry_tmp = dy;
                let rz = -dx * yaw.sin() + dz * yaw.cos();
                // Apply pitch (around X axis)
                let ry = ry_tmp * pitch.cos() - rz * pitch.sin();
                let rz2 = ry_tmp * pitch.sin() + rz * pitch.cos();
                // Perspective divide
                let z_cam = rz2 + zoom;
                let fov_scale = zoom / z_cam.max(1.0);
                let sx = rect.center().x + rx * fov_scale * (draw_w / comp_w);
                let sy = rect.center().y + ry * fov_scale * (draw_h / comp_h);
                egui::pos2(sx, sy)
            };

            // Draw a wireframe floor grid in 3D
            let grid_n = 6;
            let grid_step = comp_w / grid_n as f32;
            let grid_color = egui::Color32::from_rgba_unmultiplied(60, 80, 120, 80);
            for gx in 0..=grid_n {
                let x = gx as f32 * grid_step - comp_w * 0.5 + cx_world;
                let p0 = project_3d(x, comp_h, 0.0);
                let p1 = project_3d(x, 0.0, 0.0);
                ui.painter().line_segment([p0, p1], egui::Stroke::new(0.8, grid_color));
            }
            for gy in 0..=grid_n {
                let y = gy as f32 * (comp_h / grid_n as f32);
                let p0 = project_3d(0.0, y, 0.0);
                let p1 = project_3d(comp_w, y, 0.0);
                ui.painter().line_segment([p0, p1], egui::Stroke::new(0.8, grid_color));
            }

            // Draw comp canvas border in 3D
            let corners = [
                project_3d(0.0, 0.0, 0.0),
                project_3d(comp_w, 0.0, 0.0),
                project_3d(comp_w, comp_h, 0.0),
                project_3d(0.0, comp_h, 0.0),
            ];
            for i in 0..4 {
                ui.painter().line_segment(
                    [corners[i], corners[(i + 1) % 4]],
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 130, 240)),
                );
            }

            // Draw each 3D layer as a projected billboard
            let comp = app.history.current().active_composition();
            for (li, layer) in comp.layers.iter().enumerate() {
                if !layer.is_active(current_frame) { continue; }
                let (pos, scale, _rot, opacity) = comp.resolve_world_transform(layer, current_frame);
                let z_depth = if layer.is_3d {
                    layer.transform_3d.position.evaluate(current_frame)[2]
                } else {
                    0.0
                };
                let op = (opacity / 100.0).clamp(0.0, 1.0);
                let color = match &layer.layer_type {
                    LayerType::Solid { color } | LayerType::Text { color, .. } => {
                        egui::Color32::from_rgba_premultiplied(
                            (color[0] * 255.0) as u8, (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8, (op * 200.0) as u8)
                    }
                    LayerType::Shape { color, .. } => {
                        egui::Color32::from_rgba_premultiplied(
                            (color[0] * 255.0) as u8, (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8, (op * 200.0) as u8)
                    }
                    _ => egui::Color32::from_rgba_premultiplied(100, 180, 255, (op * 160.0) as u8),
                };
                let center = project_3d(pos[0], pos[1], z_depth);
                let w = scale[0].abs() * 0.5 * (draw_w / comp_w);
                let h = scale[1].abs() * 0.5 * (draw_h / comp_h);
                let bbox = egui::Rect::from_center_size(center, egui::vec2(w, h));
                ui.painter().rect_filled(bbox, 3.0, color);
                ui.painter().rect_stroke(bbox, 3.0, egui::Stroke::new(1.0,
                    if Some(li) == app.selected_layer_idx {
                        egui::Color32::from_rgb(100, 220, 255)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60)
                    }
                ));
                ui.painter().text(egui::pos2(center.x, bbox.top() - 10.0),
                    egui::Align2::CENTER_CENTER, &layer.name,
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgba_unmultiplied(200, 220, 255, 180));
            }

            // 3D Camera HUD overlay
            let hud = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 10.0, rect.bottom() - 38.0),
                egui::vec2(250.0, 28.0),
            );
            ui.painter().rect_filled(hud, 4.0, egui::Color32::from_rgba_unmultiplied(15, 20, 35, 220));
            ui.painter().rect_stroke(hud, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 100, 200)));
            ui.painter().text(hud.center(), egui::Align2::CENTER_CENTER,
                format!("📦 3D Camera | Yaw: {:.1}°  Pitch: {:.1}°  Z: {:.0}", yaw_deg, pitch_deg, zoom),
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(140, 200, 255));
        }

        if app.viewport_mode == ViewportMode::Comp2D && !rendered_gpu {
            ui.painter().rect_filled(draw_rect, 0.0, egui::Color32::BLACK);
            let comp = app.history.current().active_composition();
            
            // Check if there are solo layers
            let has_solo = comp.layers.iter().any(|l: &Layer| l.is_active(current_frame) && l.solo);

            for layer in &comp.layers {
                let l: &Layer = layer;
                if l.is_active(current_frame) {
                    // Respect solo state: if any layer is soloed, skip non-soloed active layers
                    if has_solo && !l.solo {
                        continue;
                    }

                    // Resolve Parenting and Expressions using resolve_world_transform
                    let (pos, scale, rotation, opacity) = comp.resolve_world_transform(l, current_frame);

                    let rx = origin_x + (pos[0] / comp_w) * draw_w;
                    let ry = origin_y + (pos[1] / comp_h) * draw_h;

                    let base_color = match &l.layer_type {
                        LayerType::Solid { color } | LayerType::Text { color, .. } => *color,
                        LayerType::Shape { color, .. } => *color,
                        LayerType::Image { .. } => [0.2, 0.6, 0.9, 0.9],
                        LayerType::PreComp { .. } => [0.8, 0.3, 0.8, 0.9],
                        _ => [0.5, 0.5, 0.5, 0.5],
                    };

                    // Apply AE BlendMode tint / alpha adjustment in software rendering
                    let alpha_mult = match l.blend_mode {
                        crate::core::timeline::BlendMode::Normal => 1.0,
                        crate::core::timeline::BlendMode::Multiply => 0.85,
                        crate::core::timeline::BlendMode::Screen => 0.7,
                        crate::core::timeline::BlendMode::Overlay => 0.8,
                        crate::core::timeline::BlendMode::Add => 0.9,
                        crate::core::timeline::BlendMode::Darken => 0.8,
                        crate::core::timeline::BlendMode::Lighten => 0.8,
                    };

                    let layer_color = egui::Color32::from_rgba_unmultiplied(
                        (base_color[0] * 255.0) as u8,
                        (base_color[1] * 255.0) as u8,
                        (base_color[2] * 255.0) as u8,
                        (opacity / 100.0 * alpha_mult * 255.0) as u8,
                    );

                    match &l.layer_type {
                        LayerType::Solid { .. } | LayerType::Shape { .. } => {
                            let w = (scale[0] / 100.0) * 100.0 * (draw_w / comp_w);
                            let h = (scale[1] / 100.0) * 100.0 * (draw_h / comp_h);
                            let rad = rotation.to_radians();
                            let cos_r = rad.cos();
                            let sin_r = rad.sin();
                            let local = [(-w*0.5,-h*0.5),(w*0.5,-h*0.5),(w*0.5,h*0.5),(-w*0.5,h*0.5)];
                            let center = egui::pos2(rx, ry);
                            let pts: Vec<egui::Pos2> = local.iter().map(|(px,py)| {
                                egui::pos2(center.x + px*cos_r - py*sin_r, center.y + px*sin_r + py*cos_r)
                            }).collect();
                            ui.painter().add(egui::Shape::convex_polygon(pts, layer_color, egui::Stroke::NONE));
                        }
                        LayerType::Image { path } => {
                            let w = (scale[0] / 100.0) * 160.0 * (draw_w / comp_w);
                            let h = (scale[1] / 100.0) * 120.0 * (draw_h / comp_h);
                            let img_rect = egui::Rect::from_center_size(egui::pos2(rx, ry), egui::vec2(w, h));
                            ui.painter().rect_filled(img_rect, 6.0, layer_color);
                            ui.painter().rect_stroke(img_rect, 6.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
                            let filename = path.split('/').last().unwrap_or(path);
                            ui.painter().text(
                                img_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("🖼️ {}", filename),
                                egui::FontId::proportional(12.0),
                                egui::Color32::WHITE,
                            );
                        }
                        LayerType::PreComp { comp_id } => {
                            let w = (scale[0] / 100.0) * 200.0 * (draw_w / comp_w);
                            let h = (scale[1] / 100.0) * 140.0 * (draw_h / comp_h);
                            let comp_rect = egui::Rect::from_center_size(egui::pos2(rx, ry), egui::vec2(w, h));
                            ui.painter().rect_filled(comp_rect, 6.0, layer_color);
                            ui.painter().rect_stroke(comp_rect, 6.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(220, 100, 250)));
                            ui.painter().text(
                                comp_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("🎬 PreComp: {}", comp_id),
                                egui::FontId::proportional(12.0),
                                egui::Color32::WHITE,
                            );
                        }
                        LayerType::Text { text, font_size, .. } => {
                            ui.painter().text(
                                egui::pos2(rx, ry),
                                egui::Align2::CENTER_CENTER,
                                text,
                                egui::FontId::proportional(*font_size as f32 * (scale[1] / 100.0) * (draw_h / comp_h)),
                                layer_color,
                            );
                        }
                        _ => {} // Null & Audio layers do not render visual canvas boxes
                    }

                    // Render BlendMode badge if non-normal
                    if l.blend_mode != crate::core::timeline::BlendMode::Normal {
                        ui.painter().text(
                            egui::pos2(rx, ry - 14.0),
                            egui::Align2::CENTER_CENTER,
                            format!("[{:?}]", l.blend_mode),
                            egui::FontId::proportional(10.0),
                            egui::Color32::YELLOW,
                        );
                    }
                }
            }
        }

        // ── Grid Overlay ──
        if app.show_grid {
            let grid_step = draw_w / 16.0;
            let grid_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30));
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

        // ── Action & Title Safe Guides Overlay ──
        if app.show_guides {
            let guide_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 230));
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
        let backend_text = if rendered_gpu { "⚡ WGPU GPU Acceleration" } else { "🖥️ Software Canvas" };
        let backend_color = if rendered_gpu { egui::Color32::from_rgb(40, 160, 100) } else { egui::Color32::from_rgb(180, 120, 40) };
        
        // Top Left Performance & FPS HUD Overlay
        let fps_text = format!("📊 {:.0} FPS  |  Comp: {}x{} @ {}fps", 60.0, comp_w as u32, comp_h as u32, app.history.current().active_composition().fps);
        let fps_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x + 10.0, origin_y + 10.0),
            egui::vec2(220.0, 24.0),
        );
        ui.painter().rect_filled(fps_rect, 4.0, egui::Color32::from_rgba_unmultiplied(20, 25, 35, 210));
        ui.painter().rect_stroke(fps_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 150, 255)));
        ui.painter().text(fps_rect.center(), egui::Align2::CENTER_CENTER, fps_text, egui::FontId::proportional(11.0), egui::Color32::from_rgb(180, 220, 255));

        // Top Right Engine Badge
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(origin_x + draw_w - 180.0, origin_y + 10.0),
            egui::vec2(170.0, 24.0),
        );
        ui.painter().rect_filled(badge_rect, 4.0, egui::Color32::from_rgba_unmultiplied(20, 20, 20, 210));
        ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0, backend_color));
        ui.painter().text(badge_rect.center(), egui::Align2::CENTER_CENTER, backend_text, egui::FontId::proportional(11.0), backend_color);

        // Bottom Left Selection HUD Status
        if let Some(s_idx) = app.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if s_idx < comp.layers.len() {
                let s_layer = &comp.layers[s_idx];
                let pos = s_layer.transform.position.evaluate(current_frame);
                let scale = s_layer.transform.scale.evaluate(current_frame);
                let rot = s_layer.transform.rotation.evaluate(current_frame);
                
                let status_str = format!(
                    "🎯 Selected: Layer {} ({}) | Pos: ({:.0}, {:.0}) | Scale: {:.0}% | Rot: {:.1}°",
                    s_idx + 1, s_layer.name, pos[0], pos[1], scale[0], rot
                );
                
                let status_rect = egui::Rect::from_min_size(
                    egui::pos2(origin_x + 10.0, origin_y + draw_h - 34.0),
                    egui::vec2(status_str.len() as f32 * 6.8 + 20.0, 24.0),
                );
                ui.painter().rect_filled(status_rect, 4.0, egui::Color32::from_rgba_unmultiplied(15, 20, 30, 220));
                ui.painter().rect_stroke(status_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 140, 240)));
                ui.painter().text(
                    status_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    status_str,
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(200, 220, 255),
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
                    ui.painter().line_segment([center, x_end], egui::Stroke::new(2.5, egui::Color32::from_rgb(240, 70, 70)));
                    ui.painter().text(egui::pos2(x_end.x + 8.0, x_end.y), egui::Align2::LEFT_CENTER, "X", egui::FontId::proportional(12.0), egui::Color32::from_rgb(240, 70, 70));

                    // Y-Axis (Green)
                    let y_end = egui::pos2(rx, ry + 65.0);
                    ui.painter().line_segment([center, y_end], egui::Stroke::new(2.5, egui::Color32::from_rgb(60, 220, 80)));
                    ui.painter().text(egui::pos2(y_end.x, y_end.y + 8.0), egui::Align2::CENTER_TOP, "Y", egui::FontId::proportional(12.0), egui::Color32::from_rgb(60, 220, 80));

                    // Z-Axis (Blue Diagonal)
                    let z_end = egui::pos2(rx - 45.0, ry + 45.0);
                    ui.painter().line_segment([center, z_end], egui::Stroke::new(2.5, egui::Color32::from_rgb(60, 150, 255)));
                    ui.painter().text(egui::pos2(z_end.x - 8.0, z_end.y + 4.0), egui::Align2::RIGHT_TOP, "Z", egui::FontId::proportional(12.0), egui::Color32::from_rgb(60, 150, 255));

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
                    ui.painter().rect_stroke(bbox_rect, 0.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 180, 255)));

                    for hp in bbox_corners {
                        let h_rect = egui::Rect::from_center_size(hp, egui::vec2(7.0, 7.0));
                        ui.painter().rect_filled(h_rect, 1.0, egui::Color32::WHITE);
                        ui.painter().rect_stroke(h_rect, 1.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 120, 255)));
                    }

                    // Center Target Circle
                    ui.painter().circle_filled(center, 5.0, egui::Color32::from_rgb(255, 215, 0));
                    ui.painter().circle_stroke(center, 9.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
                }
            }
        }

        // ── Interactive Layer Drag ─────────────────────────────────
        if let Some(pointer_pos) = viewport_response.interact_pointer_pos() {
            let comp_px = (pointer_pos.x - origin_x) / draw_w * comp_w;
            let comp_py = (pointer_pos.y - origin_y) / draw_h * comp_h;

            if viewport_response.drag_started() {
                let comp_state = app.history.current().active_composition();
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

            if viewport_response.dragged() {
                if let Some((drag_idx, start_pos, start_ptr)) = app.viewport_drag_state {
                    let delta_x = (pointer_pos.x - start_ptr.x) / draw_w * comp_w;
                    let delta_y = (pointer_pos.y - start_ptr.y) / draw_h * comp_h;
                    let new_pos = [start_pos[0] + delta_x, start_pos[1] + delta_y];

                    let comp_mut = app.history.current_mut().active_composition_mut();
                    if drag_idx < comp_mut.layers.len() {
                        let layer = &mut comp_mut.layers[drag_idx];
                        layer.transform.position = Animatable::new_constant(new_pos);
                    }
                }
            }
        }
    });
}
