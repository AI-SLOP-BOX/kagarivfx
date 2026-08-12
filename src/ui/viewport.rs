use crate::AfterEffectsApp;
use crate::core::timeline::{Layer, LayerType};
use crate::core::property::Animatable;
use crate::ViewportMode;
use eframe::egui;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: u32) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // ── Viewport Toolbar ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.heading("Viewport");
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Mode Toggle
            let mode_2d = app.viewport_mode == ViewportMode::Comp2D;
            if ui.selectable_label(mode_2d, "2D").clicked() {
                app.viewport_mode = ViewportMode::Comp2D;
            }
            if ui.selectable_label(!mode_2d, "3D Camera").clicked() {
                app.viewport_mode = ViewportMode::Camera3D;
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(&mut app.show_grid, "Grid");
            ui.checkbox(&mut app.show_guides, "Safe");
            ui.checkbox(&mut app.show_handles, "Handles");
            ui.add_space(8.0);

            if ui.button("Comp Settings").clicked() {
                app.show_comp_settings = !app.show_comp_settings;
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // AE Magnification Ratio Dropdown
            let mag_val = app.viewport_mag_ratio;
            egui::ComboBox::from_id_source("mag_combo")
                .selected_text(if mag_val == 4.0 { "400%" } else if mag_val == 2.0 { "200%" } else if mag_val == 1.0 { "100%" } else if mag_val == 0.5 { "50%" } else if mag_val == 0.25 { "25%" } else { "Fit" })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(app.viewport_mag_ratio == 0.0, "Fit").clicked() { app.viewport_mag_ratio = 0.0; }
                    if ui.selectable_label(app.viewport_mag_ratio == 0.25, "25%").clicked() { app.viewport_mag_ratio = 0.25; }
                    if ui.selectable_label(app.viewport_mag_ratio == 0.5, "50%").clicked() { app.viewport_mag_ratio = 0.5; }
                    if ui.selectable_label(app.viewport_mag_ratio == 1.0, "100%").clicked() { app.viewport_mag_ratio = 1.0; }
                    if ui.selectable_label(app.viewport_mag_ratio == 2.0, "200%").clicked() { app.viewport_mag_ratio = 2.0; }
                    if ui.selectable_label(app.viewport_mag_ratio == 4.0, "400%").clicked() { app.viewport_mag_ratio = 4.0; }
                });

            // AE Camera View Selector
            let cam_view_id = egui::Id::new("ae_cam_view_select");
            let mut cam_view = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(cam_view_id, || 0));
            egui::ComboBox::from_id_source("cam_view_combo")
                .selected_text(match cam_view {
                    0 => "Active Camera",
                    1 => "Front",
                    2 => "Left",
                    3 => "Top",
                    _ => "Custom View 1",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut cam_view, 0, "Active Camera").clicked() {
                        app.viewport_mode = ViewportMode::Comp2D;
                        ctx.data_mut(|d| d.insert_temp(cam_view_id, cam_view));
                    }
                    if ui.selectable_value(&mut cam_view, 1, "Front").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (0.0, 0.0, 1000.0);
                        ctx.data_mut(|d| d.insert_temp(cam_view_id, cam_view));
                    }
                    if ui.selectable_value(&mut cam_view, 2, "Left").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (-90.0, 0.0, 1000.0);
                        ctx.data_mut(|d| d.insert_temp(cam_view_id, cam_view));
                    }
                    if ui.selectable_value(&mut cam_view, 3, "Top").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (0.0, -89.0, 1000.0);
                        ctx.data_mut(|d| d.insert_temp(cam_view_id, cam_view));
                    }
                    if ui.selectable_value(&mut cam_view, 4, "Custom View 1").clicked() {
                        app.viewport_mode = ViewportMode::Camera3D;
                        app.camera_orbit = (30.0, 20.0, 1200.0);
                        ctx.data_mut(|d| d.insert_temp(cam_view_id, cam_view));
                    }
                });

            ui.separator();
            ui.add_space(4.0);

            // ── Snapshot A/B Compare Controls ──
            let snap_id = egui::Id::new("ae_viewport_snap_a");
            let is_comparing_id = egui::Id::new("ae_viewport_comparing");
            let wipe_id = egui::Id::new("ae_viewport_wipe_pos");

            let mut has_snap = ctx.data_mut(|d| d.get_temp::<u32>(snap_id).is_some());
            let mut is_comparing = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(is_comparing_id, || false));
            let mut wipe_pos = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(wipe_id, || 0.5f32));

            if ui.button(if has_snap { "[Snap A] Retake" } else { "[Snap A] Take" }).on_hover_text("Take snapshot of current frame (Shift+F5)").clicked() {
                ctx.data_mut(|d| d.insert_temp(snap_id, current_frame));
                has_snap = true;
            }

            if has_snap {
                if ui.selectable_label(is_comparing, "[Compare A]").clicked() {
                    is_comparing = !is_comparing;
                    ctx.data_mut(|d| d.insert_temp(is_comparing_id, is_comparing));
                }
                if is_comparing {
                    ui.label("Wipe:");
                    if ui.add(egui::Slider::new(&mut wipe_pos, 0.0..=1.0).show_value(false)).changed() {
                        ctx.data_mut(|d| d.insert_temp(wipe_id, wipe_pos));
                    }
                }
            }

            // AE Render Quality / Downsample Resolution
            let res_id = egui::Id::new("ae_render_resolution");
            let mut res_ratio = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(res_id, || 0));
            egui::ComboBox::from_id_source("res_combo")
                .selected_text(match res_ratio {
                    0 => "Full",
                    1 => "Half",
                    2 => "Third",
                    _ => "Quarter",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut res_ratio, 0, "Full").clicked() { ctx.data_mut(|d| d.insert_temp(res_id, res_ratio)); }
                    if ui.selectable_value(&mut res_ratio, 1, "Half").clicked() { ctx.data_mut(|d| d.insert_temp(res_id, res_ratio)); }
                    if ui.selectable_value(&mut res_ratio, 2, "Third").clicked() { ctx.data_mut(|d| d.insert_temp(res_id, res_ratio)); }
                    if ui.selectable_value(&mut res_ratio, 3, "Quarter").clicked() { ctx.data_mut(|d| d.insert_temp(res_id, res_ratio)); }
                });

            // AE Color Channels
            let chan_id = egui::Id::new("ae_color_channel");
            let mut chan_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(chan_id, || 0));
            egui::ComboBox::from_id_source("chan_combo")
                .selected_text(match chan_idx {
                    0 => "RGB Color",
                    1 => "Red",
                    2 => "Green",
                    3 => "Blue",
                    _ => "Alpha",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut chan_idx, 0, "RGB Color").clicked() { ctx.data_mut(|d| d.insert_temp(chan_id, chan_idx)); }
                    if ui.selectable_value(&mut chan_idx, 1, "Red").clicked() { ctx.data_mut(|d| d.insert_temp(chan_id, chan_idx)); }
                    if ui.selectable_value(&mut chan_idx, 2, "Green").clicked() { ctx.data_mut(|d| d.insert_temp(chan_id, chan_idx)); }
                    if ui.selectable_value(&mut chan_idx, 3, "Blue").clicked() { ctx.data_mut(|d| d.insert_temp(chan_id, chan_idx)); }
                    if ui.selectable_value(&mut chan_idx, 4, "Alpha").clicked() { ctx.data_mut(|d| d.insert_temp(chan_id, chan_idx)); }
                });

            // Viewport Color Management (Exposure EV & LUT)
            ui.separator();
            let exp_id = egui::Id::new("ae_exposure_ev");
            let mut exposure_ev = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(exp_id, || 0.0f32));
            ui.label("Exp:");
            if ui.add(egui::DragValue::new(&mut exposure_ev).speed(0.1).clamp_range(-5.0..=5.0).suffix(" EV")).changed() {
                ctx.data_mut(|d| d.insert_temp(exp_id, exposure_ev));
            }

            let lut_id = egui::Id::new("ae_colorspace_lut");
            let mut lut_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(lut_id, || 0usize));
            egui::ComboBox::from_id_source("lut_combo")
                .selected_text(match lut_idx {
                    0 => "Rec.709",
                    1 => "Linear sRGB",
                    _ => "ACEScg",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut lut_idx, 0, "Rec.709").clicked() { ctx.data_mut(|d| d.insert_temp(lut_id, lut_idx)); }
                    if ui.selectable_value(&mut lut_idx, 1, "Linear sRGB").clicked() { ctx.data_mut(|d| d.insert_temp(lut_id, lut_idx)); }
                    if ui.selectable_value(&mut lut_idx, 2, "ACEScg").clicked() { ctx.data_mut(|d| d.insert_temp(lut_id, lut_idx)); }
                });

            // Resolution Scale Selector (Full, Half, Quarter)
            ui.separator();
            let res_scale_id = egui::Id::new("ae_preview_res_scale");
            let mut res_scale_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(res_scale_id, || 0usize));
            egui::ComboBox::from_id_source("res_scale_combo")
                .selected_text(match res_scale_idx {
                    0 => "Full (1/1)",
                    1 => "Half (1/2)",
                    _ => "Quarter (1/4)",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut res_scale_idx, 0, "Full (1/1)").clicked() { ctx.data_mut(|d| d.insert_temp(res_scale_id, res_scale_idx)); }
                    if ui.selectable_value(&mut res_scale_idx, 1, "Half (1/2)").clicked() { ctx.data_mut(|d| d.insert_temp(res_scale_id, res_scale_idx)); }
                    if ui.selectable_value(&mut res_scale_idx, 2, "Quarter (1/4)").clicked() { ctx.data_mut(|d| d.insert_temp(res_scale_id, res_scale_idx)); }
                });
        });

        // ── Comp Settings Modal ──────────────────────────────────────────────
        if app.show_comp_settings {
            egui::Window::new("Composition Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let mut temp_project = app.history.current().clone();
                    let comp = temp_project.active_composition_mut();
                    let mut apply_requested = false;

                    egui::Grid::new("comp_settings_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut comp.name);
                            ui.end_row();

                            ui.label("Preset:");
                            egui::ComboBox::from_id_source("preset_res_combo")
                                .selected_text("Resolution Presets")
                                .show_ui(ui, |ui| {
                                    if ui.button("1080p Full HD (1920x1080)").clicked() {
                                        comp.width = 1920;
                                        comp.height = 1080;
                                    }
                                    if ui.button("4K UHD (3840x2160)").clicked() {
                                        comp.width = 3840;
                                        comp.height = 2160;
                                    }
                                    if ui.button("720p HD (1280x720)").clicked() {
                                        comp.width = 1280;
                                        comp.height = 720;
                                    }
                                    if ui.button("Vertical 9:16 Shorts (1080x1920)").clicked() {
                                        comp.width = 1080;
                                        comp.height = 1920;
                                    }
                                    if ui.button("Square 1:1 (1080x1080)").clicked() {
                                        comp.width = 1080;
                                        comp.height = 1080;
                                    }
                                });
                            ui.end_row();

                            ui.label("Width:");
                            ui.add(egui::DragValue::new(&mut comp.width).clamp_range(1u32..=7680));
                            ui.end_row();

                            ui.label("Height:");
                            ui.add(egui::DragValue::new(&mut comp.height).clamp_range(1u32..=4320));
                            ui.end_row();

                            ui.label("FPS:");
                            ui.add(egui::DragValue::new(&mut comp.fps).clamp_range(1u32..=240));
                            ui.end_row();

                            ui.label("Duration (frames):");
                            ui.add(egui::DragValue::new(&mut comp.duration_frames).clamp_range(1u32..=108000));
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            apply_requested = true;
                        }
                        if ui.button("Close").clicked() {
                            app.show_comp_settings = false;
                        }
                    });

                    if apply_requested {
                        app.history.commit(temp_project);
                        crate::core::frame_cache::bump_version();
                        app.show_comp_settings = false;
                    }
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
                let exp_id = egui::Id::new("ae_exposure_ev");
                let exposure_ev = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(exp_id, || 0.0f32));
                let lut_id = egui::Id::new("ae_colorspace_lut");
                let lut_idx = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(lut_id, || 0usize));

                let snap_id = egui::Id::new("ae_viewport_snap_a");
                let is_comparing_id = egui::Id::new("ae_viewport_comparing");
                let wipe_id = egui::Id::new("ae_viewport_wipe_pos");

                let snap_frame_opt = ctx.data_mut(|d| d.get_temp::<u32>(snap_id));
                let is_comparing = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(is_comparing_id, || false));
                let wipe_pos = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(wipe_id, || 0.5f32));

                let (texture_view, recreated) = renderer.render(comp, current_frame, exposure_ev, lut_idx as u32);
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

                if is_comparing && snap_texture_id_val.is_some() && app.viewport_texture_id.is_some() {
                    let cur_tex = app.viewport_texture_id.unwrap();
                    let snap_tex = snap_texture_id_val.unwrap();

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
                } else {
                    if let Some(texture_id) = app.viewport_texture_id {
                        ui.put(draw_rect, egui::Image::new(egui::load::SizedTexture::new(texture_id, draw_rect.size())));
                        rendered_gpu = true;
                    }
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
                format!("[3D CAMERA] Yaw: {:.1}°  Pitch: {:.1}°  Z: {:.0}", yaw_deg, pitch_deg, zoom),
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(140, 200, 255));
        }

        if app.viewport_mode == ViewportMode::Comp2D && !rendered_gpu {
            ui.painter().rect_filled(draw_rect, 0.0, egui::Color32::BLACK);
            let comp = app.history.current().active_composition();
            
            // Check if there are solo layers
            let has_solo = comp.layers.iter().any(|l: &Layer| l.is_active(current_frame) && l.solo);

            for (li, layer) in comp.layers.iter().enumerate() {
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
                            // Check if a mask should clip this layer geometry
                            let mut pts_to_draw = None;
                            for mask in &l.masks {
                                if mask.enabled && mask.mode != crate::core::mask::MaskMode::None {
                                    let points = mask.path.to_polygon(current_frame, 16);
                                    if points.len() >= 3 {
                                        let draw_points: Vec<egui::Pos2> = points.iter().map(|pt| {
                                            let mx = origin_x + (pt[0] / comp_w) * draw_w;
                                            let my = origin_y + (pt[1] / comp_h) * draw_h;
                                            egui::pos2(mx, my)
                                        }).collect();
                                        pts_to_draw = Some(draw_points);
                                        break;
                                    }
                                }
                            }

                            let pts = if let Some(dpts) = pts_to_draw {
                                dpts
                            } else {
                                let w = (scale[0] / 100.0) * 100.0 * (draw_w / comp_w);
                                let h = (scale[1] / 100.0) * 100.0 * (draw_h / comp_h);
                                let rad = rotation.to_radians();
                                let cos_r = rad.cos();
                                let sin_r = rad.sin();
                                let local = [(-w*0.5,-h*0.5),(w*0.5,-h*0.5),(w*0.5,h*0.5),(-w*0.5,h*0.5)];
                                let center = egui::pos2(rx, ry);
                                local.iter().map(|(px,py)| {
                                    egui::pos2(center.x + px*cos_r - py*sin_r, center.y + px*sin_r + py*cos_r)
                                }).collect()
                            };

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
                                format!("IMG :: {}", filename),
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
                                format!("COMP :: {}", comp_id),
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

                    // ── Render Active Vector Masks Overlay ──
                    for mask in &l.masks {
                        if !mask.enabled {
                            continue;
                        }
                        let points = mask.path.to_polygon(current_frame, 12);
                        if points.len() >= 2 {
                            let mut draw_points = Vec::with_capacity(points.len());
                            for pt in &points {
                                let mx = origin_x + (pt[0] / comp_w) * draw_w;
                                let my = origin_y + (pt[1] / comp_h) * draw_h;
                                draw_points.push(egui::pos2(mx, my));
                            }

                            // Draw mask path line
                            let is_selected_layer = Some(li) == app.selected_layer_idx;
                            let line_color = if is_selected_layer {
                                egui::Color32::from_rgb(255, 180, 50) // Golden yellow for selected layer mask
                            } else {
                                egui::Color32::from_rgba_unmultiplied(255, 180, 50, 100)
                            };

                            for w in draw_points.windows(2) {
                                ui.painter().line_segment([w[0], w[1]], egui::Stroke::new(1.2, line_color));
                            }
                            if mask.path.is_closed {
                                ui.painter().line_segment([draw_points[draw_points.len() - 1], draw_points[0]], egui::Stroke::new(1.2, line_color));
                            }

                            // Draw vertices if active layer
                            if is_selected_layer {
                                for (v_idx, pt) in draw_points.iter().enumerate() {
                                    let v_rect = egui::Rect::from_center_size(*pt, egui::vec2(8.0, 8.0));
                                    let is_hovered = ui.rect_contains_pointer(v_rect);
                                    let handle_color = if is_hovered { egui::Color32::YELLOW } else { egui::Color32::WHITE };
                                    ui.painter().rect_filled(v_rect, 1.0, handle_color);
                                    ui.painter().rect_stroke(v_rect, 1.0, egui::Stroke::new(1.2, egui::Color32::from_rgb(255, 100, 0)));

                                    ui.painter().text(
                                        egui::pos2(pt.x + 8.0, pt.y - 8.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        format!("V{}", v_idx + 1),
                                        egui::FontId::proportional(10.0),
                                        egui::Color32::from_rgb(255, 200, 100),
                                    );
                                }
                            }
                        }
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
        let backend_text = if rendered_gpu { "[GPU] WGPU Acceleration" } else { "[CPU] Software Canvas" };
        let backend_color = if rendered_gpu { egui::Color32::from_rgb(40, 160, 100) } else { egui::Color32::from_rgb(180, 120, 40) };
        
        // Top Left Performance & FPS HUD Overlay
        let dt = ctx.input(|i| i.stable_dt.max(0.001));
        let real_fps = (1.0 / dt).clamp(1.0, 240.0);
        let fps_text = format!("PERF: {:.0} FPS | Comp: {}x{} @ {}fps", real_fps, comp_w as u32, comp_h as u32, app.history.current().active_composition().fps);
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
                    "SELECTED: Layer {} ({}) | Pos: ({:.0}, {:.0}) | Scale: {:.0}% | Rot: {:.1}°",
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

        // ── Snapshot A/B Interactive Split Wipe Line Overlay ──
        let is_comparing_id = egui::Id::new("ae_snapshot_is_comparing");
        let is_comparing = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(is_comparing_id, || false));
        if is_comparing {
            let wipe_id = egui::Id::new("ae_snapshot_wipe_pos");
            let wipe_pos = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(wipe_id, || 0.5f32));
            let wipe_x = origin_x + wipe_pos * draw_w;

            ui.painter().line_segment(
                [egui::pos2(wipe_x, origin_y), egui::pos2(wipe_x, origin_y + draw_h)],
                egui::Stroke::new(2.5, egui::Color32::from_rgb(100, 220, 255)),
            );
            ui.painter().circle_filled(egui::pos2(wipe_x, origin_y + draw_h * 0.5), 10.0, egui::Color32::from_rgb(100, 220, 255));
            ui.painter().circle_stroke(egui::pos2(wipe_x, origin_y + draw_h * 0.5), 10.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
            ui.painter().text(
                egui::pos2(wipe_x - 15.0, origin_y + 15.0),
                egui::Align2::RIGHT_TOP,
                "[Snap A]",
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(100, 220, 255),
            );
            ui.painter().text(
                egui::pos2(wipe_x + 15.0, origin_y + 15.0),
                egui::Align2::LEFT_TOP,
                "[Live Frame]",
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(255, 200, 100),
            );
        }

        // ── Interactive Layer & Mask Drag ──────────────────────────
        if let Some(pointer_pos) = viewport_response.interact_pointer_pos() {
            let comp_px = (pointer_pos.x - origin_x) / draw_w * comp_w;
            let comp_py = (pointer_pos.y - origin_y) / draw_h * comp_h;

            if viewport_response.drag_started() {
                let comp_state = app.history.current().active_composition();
                let mut mask_hit: Option<(usize, usize, usize)> = None;

                // 1. Check if clicking on selected layer's mask vertices
                if let Some(sel_li) = app.selected_layer_idx {
                    if sel_li < comp_state.layers.len() {
                        let l = &comp_state.layers[sel_li];
                        for (mi, mask) in l.masks.iter().enumerate() {
                            if mask.enabled {
                                let verts = mask.path.to_polygon(current_frame, 16);
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
                    let verts = comp_state.layers[l_idx].masks[m_idx].path.to_polygon(current_frame, 16);
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
                            let mut verts = mask.path.to_polygon(current_frame, 16);
                            if v_idx < verts.len() {
                                verts[v_idx][0] = start_vertex_pos[0] + delta_x;
                                verts[v_idx][1] = start_vertex_pos[1] + delta_y;
                                mask.path.vertices = crate::core::property::Animatable::Constant(verts);
                            }
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
                let mut changed = false;
                if app.viewport_drag_state.is_some() {
                    app.viewport_drag_state = None;
                    changed = true;
                }
                if app.viewport_mask_drag_state.is_some() {
                    app.viewport_mask_drag_state = None;
                    changed = true;
                }
                if changed {
                    crate::core::frame_cache::bump_version();
                }
            }
        }
    });
}
