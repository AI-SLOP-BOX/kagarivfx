use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

pub fn draw_comp_settings_dialog(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_comp_settings {
        return;
    }

    let mut open = app.show_comp_settings;
    egui::Window::new("⚙ Composition Settings (Cmd+K)")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            let mut temp_proj = app.history.current().clone();
            let comp = temp_proj.active_composition_mut();

            ui.heading("Basic Composition Settings");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Composition Name:");
                ui.text_edit_singleline(&mut comp.name);
            });

            ui.add_space(6.0);
            ui.label("Preset:");
            let preset_id = egui::Id::new("ae_comp_preset_choice");
            let mut preset_choice = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(preset_id, || 0));

            egui::ComboBox::from_id_salt("comp_preset_combo")
                .selected_text(match preset_choice {
                    0 => "HDTV 1080 29.97 (1920 x 1080)",
                    1 => "4K UHD 60fps (3840 x 2160)",
                    2 => "720p HD 24fps (1280 x 720)",
                    _ => "Custom",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut preset_choice, 0, "HDTV 1080 30fps (1920 x 1080)").clicked() {
                        comp.width = 1920;
                        comp.height = 1080;
                        comp.fps = 30;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                    if ui.selectable_value(&mut preset_choice, 1, "4K UHD 60fps (3840 x 2160)").clicked() {
                        comp.width = 3840;
                        comp.height = 2160;
                        comp.fps = 60;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                    if ui.selectable_value(&mut preset_choice, 2, "720p HD 24fps (1280 x 720)").clicked() {
                        comp.width = 1280;
                        comp.height = 720;
                        comp.fps = 24;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                    if ui.selectable_value(&mut preset_choice, 3, "📱 TikTok / Shorts / Reels (1080 x 1920 9:16)").clicked() {
                        comp.width = 1080;
                        comp.height = 1920;
                        comp.fps = 30;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                    if ui.selectable_value(&mut preset_choice, 4, "🔳 Instagram Feed (1080 x 1080 1:1)").clicked() {
                        comp.width = 1080;
                        comp.height = 1080;
                        comp.fps = 30;
                        ui.ctx().data_mut(|d| d.insert_temp(preset_id, preset_choice));
                    }
                });

            ui.add_space(6.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("📱 One-Tap Smart SNS Reframer").strong().color(colors::ACCENT_CYAN));
                ui.small("Auto-remap layer positions to new Aspect Ratio:");
                ui.horizontal(|ui| {
                    if custom_widgets::ae_button(ui, "📱 Shorts 9:16").on_hover_text("Vertical (1080 x 1920) for TikTok/Reels/Shorts").clicked() {
                        let old_w = comp.width;
                        let old_h = comp.height;
                        comp.resize_and_remap(1080, 1920, old_w, old_h);
                    }
                    if custom_widgets::ae_button(ui, "🔳 Square 1:1").on_hover_text("Square (1080 x 1080) for Instagram Feed").clicked() {
                        let old_w = comp.width;
                        let old_h = comp.height;
                        comp.resize_and_remap(1080, 1080, old_w, old_h);
                    }
                    if custom_widgets::ae_button(ui, "🎬 Cinema 21:9").on_hover_text("Ultrawide Cinematic (2560 x 1080)").clicked() {
                        let old_w = comp.width;
                        let old_h = comp.height;
                        comp.resize_and_remap(2560, 1080, old_w, old_h);
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut comp.width).speed(1.0).suffix(" px").range(16..=16384));
                ui.label("Height:");
                ui.add(egui::DragValue::new(&mut comp.height).speed(1.0).suffix(" px").range(16..=16384));
            });

            comp.width = comp.width.max(16);
            comp.height = comp.height.max(16);

            ui.horizontal(|ui| {
                ui.label("Frame Rate (FPS):");
                ui.add(egui::DragValue::new(&mut comp.fps).speed(1).range(1..=120));
                
                let fps_id = ui.make_persistent_id("ae_fps_preset_choice");
                let mut fps_choice: usize = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(fps_id, || 0));
                egui::ComboBox::from_id_salt(fps_id)
                    .selected_text(format!("Preset ({} fps)", comp.fps))
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut fps_choice, 0, "Film (24 fps)").clicked() { comp.fps = 24; }
                        if ui.selectable_value(&mut fps_choice, 1, "PAL (25 fps)").clicked() { comp.fps = 25; }
                        if ui.selectable_value(&mut fps_choice, 2, "NTSC Video (30 fps)").clicked() { comp.fps = 30; }
                        if ui.selectable_value(&mut fps_choice, 3, "Smooth (60 fps)").clicked() { comp.fps = 60; }
                        if ui.selectable_value(&mut fps_choice, 4, "High-Speed (120 fps)").clicked() { comp.fps = 120; }
                    });
            });
            comp.fps = comp.fps.max(1);

            ui.horizontal(|ui| {
                ui.label("Duration (Frames):");
                ui.add(egui::DragValue::new(&mut comp.duration_frames).speed(1.0).range(1..=100000));
                comp.duration_frames = comp.duration_frames.max(1);
                let seconds = comp.duration_frames as f64 / comp.fps as f64;
                ui.small(format!("({:.2} seconds)", seconds));
            });

            ui.add_space(4.0);
            ui.collapsing("🌈 Blending", |ui| {
                let before = comp.blend_linear;
                ui.checkbox(&mut comp.blend_linear,
                    "Blend colors in linear light (1.0 gamma)")
                    .on_hover_text("Add / Screen / Glow blends compute in linear space — brighter, physically-plausible results. Matches AE's 'Blend Colors Using 1.0 Gamma'.");
                if comp.blend_linear != before {
                    crate::core::frame_cache::bump_version();
                }
                ui.label(egui::RichText::new("Legacy projects default to OFF (gamma-space) for identical output.").small().color(colors::TEXT_MUTED));
                ui.add_space(2.0);
                ui.checkbox(&mut comp.dither_output, "Output dithering (smooth gradients)")
                    .on_hover_text("Triangular-PDF noise ±1/255 breaks banding in dark gradients — imperceptible, deterministic");
            });

            ui.add_space(4.0);
            ui.collapsing("🎨 Color Management & Working Space", |ui| {
                let color_space_id = ui.make_persistent_id("ae_color_space_choice");
                let mut cs_idx: usize = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(color_space_id, || 0));
                let cs_label = match cs_idx {
                    0 => "sRGB (Web / Standard)",
                    1 => "Rec.709 (HDTV Broadcast)",
                    2 => "ACEScg (VFX Cinema Standard)",
                    _ => "Display P3 (Apple Wide Color)",
                };
                ui.horizontal(|ui| {
                    ui.label("Working Color Space:");
                    egui::ComboBox::from_id_salt(color_space_id)
                        .selected_text(cs_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut cs_idx, 0, "sRGB (Web / Standard)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(color_space_id, cs_idx)); }
                            if ui.selectable_value(&mut cs_idx, 1, "Rec.709 (HDTV Broadcast)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(color_space_id, cs_idx)); }
                            if ui.selectable_value(&mut cs_idx, 2, "ACEScg (VFX Cinema Standard)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(color_space_id, cs_idx)); }
                            if ui.selectable_value(&mut cs_idx, 3, "Display P3 (Apple Wide Color)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(color_space_id, cs_idx)); }
                        });
                });

                let bit_depth_id = ui.make_persistent_id("ae_color_bit_depth");
                let mut depth_idx: usize = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(bit_depth_id, || 0));
                let depth_label = match depth_idx {
                    0 => "8-bit per channel (8-bpc)",
                    1 => "16-bit per channel (16-bpc)",
                    _ => "32-bit Float (HDR / 32-bpc)",
                };
                ui.horizontal(|ui| {
                    ui.label("Depth:");
                    egui::ComboBox::from_id_salt(bit_depth_id)
                        .selected_text(depth_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut depth_idx, 0, "8-bit per channel (8-bpc)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(bit_depth_id, depth_idx)); }
                            if ui.selectable_value(&mut depth_idx, 1, "16-bit per channel (16-bpc)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(bit_depth_id, depth_idx)); }
                            if ui.selectable_value(&mut depth_idx, 2, "32-bit Float (HDR / 32-bpc)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(bit_depth_id, depth_idx)); }
                        });
                });
            });
            ui.horizontal(|ui| {
                ui.label("🌀 Motion Blur Shutter Angle:");
                ui.add(egui::Slider::new(&mut comp.motion_blur_shutter_angle, 0.0..=720.0).suffix("°"));
            });

            ui.add_space(10.0);
            ui.separator();

            let mut should_commit = false;
            let mut should_close = false;

            ui.horizontal(|ui| {
                if custom_widgets::ae_button_accent(ui, "OK").clicked() {
                    should_commit = true;
                    should_close = true;
                }
                if custom_widgets::ae_button(ui, "Cancel").clicked() {
                    should_close = true;
                }
            });

            if should_commit {
                let (old_w, old_h) = {
                    let orig = app.history.current().active_composition();
                    (orig.width as f32, orig.height as f32)
                };

                let comp = temp_proj.active_composition_mut();
                let (new_w, new_h) = (comp.width as f32, comp.height as f32);

                if (old_w - new_w).abs() > 0.1 || (old_h - new_h).abs() > 0.1 {
                    for layer in &mut comp.layers {
                        let pos_now = layer.transform.position.evaluate(0);
                        let remapped = layer.constraints.remap_position(pos_now, old_w, old_h, new_w, new_h);
                        layer.transform.position = crate::core::property::Animatable::new_constant(remapped);
                    }
                }

                app.history.commit(temp_proj);
                crate::core::frame_cache::bump_version();
            }

            if should_close {
                app.show_comp_settings = false;
            }


        });

    if !open {
        app.show_comp_settings = false;
    }
}
