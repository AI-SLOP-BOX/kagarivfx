pub mod graph_editor;

use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{EffectType, Layer, LayerType, ShapeType};
use crate::core::property::Animatable;

fn get_kfs<T: Clone>(prop: &Animatable<T>) -> Vec<(u32, crate::core::keyframe::InterpolationType)> {
    prop.keyframes()
        .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.interpolation)).collect())
        .unwrap_or_default()
}

fn maybe_snap_frame(frame: u32, snap: bool, comp: &crate::core::timeline::Composition) -> u32 {
    if !snap {
        return frame;
    }
    let threshold = 3i32;
    for layer in &comp.layers {
        for (kf_f, _) in get_kfs(&layer.transform.position)
            .into_iter()
            .chain(get_kfs(&layer.transform.scale))
            .chain(get_kfs(&layer.transform.rotation))
            .chain(get_kfs(&layer.transform.opacity))
        {
            if (frame as i32 - kf_f as i32).abs() <= threshold {
                return kf_f;
            }
        }
    }
    frame
}

fn draw_keyframe_tick(
    ui: &mut egui::Ui,
    x: f32,
    y: f32,
    is_sub_prop: bool,
    current_frame: &mut u32,
    kf_frame: u32,
    _interpolation: Option<crate::core::keyframe::InterpolationType>,
) {
    let size = if is_sub_prop { 5.0 } else { 7.0 };
    let rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size, size));
    let color = if *current_frame == kf_frame {
        egui::Color32::from_rgb(255, 200, 50)
    } else {
        egui::Color32::from_rgb(180, 180, 180)
    };
    
    let painter = ui.painter();
    let pts = vec![
        egui::pos2(x, y - size),
        egui::pos2(x + size, y),
        egui::pos2(x, y + size),
        egui::pos2(x - size, y),
    ];
    painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));

    let response = ui.allocate_rect(rect, egui::Sense::click());
    if response.clicked() {
        *current_frame = kf_frame;
    }
}

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: &mut u32, total_frames: u32) {
    egui::TopBottomPanel::bottom("timeline_panel")
        .resizable(true)
        .default_height(280.0)
        .show(ctx, |ui| {
            ui.heading("Timeline & Tracks");
            ui.separator();

            let mut project_changed = false;
            let mut temp_project = app.history.current().clone();
            let comp = temp_project.active_composition_mut();

            ui.horizontal(|ui| {
                let fps = comp.fps.max(1);
                let secs = *current_frame / fps;
                let sub_f = *current_frame % fps;
                let mins = secs / 60;
                let hours = mins / 60;
                let tc_str = format!("{:02}:{:02}:{:02}:{:02}", hours, mins % 60, secs % 60, sub_f);

                ui.label(egui::RichText::new(format!("TC: {}  |  Frame: {} / {}", tc_str, current_frame, total_frames)).strong().color(egui::Color32::from_rgb(100, 220, 255)));
                ui.add_space(8.0);
                if ui.button("|< First").clicked() { *current_frame = 0; }
                if ui.button("< Prev").clicked() { *current_frame = current_frame.saturating_sub(1); }
                if ui.button(if app.is_playing { "|| Pause" } else { "> Play" }).clicked() {
                    app.is_playing = !app.is_playing;
                }
                if ui.button("Next >").clicked() { *current_frame = (*current_frame + 1).min(total_frames); }
                if ui.button("Last >|").clicked() { *current_frame = total_frames; }

                ui.separator();
                ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut app.timeline_zoom, 0.1..=10.0));
                ui.checkbox(&mut app.snap_to_keyframes, "Snap");
                let mode_btn_text = if app.show_graph_editor { "Graph Mode" } else { "Tracks Mode" };
                if ui.selectable_label(app.show_graph_editor, mode_btn_text).clicked() {
                    app.show_graph_editor = !app.show_graph_editor;
                }
                
                ui.add_space(15.0);
                if ui.button("+ Solid").clicked() {
                    let id = format!("layer_{}", comp.layers.len());
                    let name = format!("Solid {}", comp.layers.len());
                    let mut layer = Layer::new(id, name, LayerType::Solid { color: [0.3, 0.5, 0.7, 1.0] }, total_frames);
                    layer.transform.position = Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
                    comp.add_layer(layer);
                    project_changed = true;
                }
                if ui.button("+ Text").clicked() {
                    let id = format!("layer_{}", comp.layers.len());
                    let name = format!("Text {}", comp.layers.len());
                    let mut layer = Layer::new(id, name, LayerType::Text {
                        text: "New Text".to_string(),
                        font_size: 48,
                        color: [1.0, 1.0, 1.0, 1.0],
                    }, total_frames);
                    layer.transform.position = Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
                    comp.add_layer(layer);
                    project_changed = true;
                }
                if ui.button("+ Marker (M)").clicked() {
                    let m_idx = comp.markers.len() + 1;
                    comp.markers.push(crate::core::timeline::TimelineMarker {
                        frame: *current_frame,
                        label: format!("Marker {}", m_idx),
                        color: [1.0, 0.6, 0.1],
                    });
                    project_changed = true;
                }
                if ui.button("+ Shape").clicked() {
                    let id = format!("layer_{}", comp.layers.len());
                    let name = format!("Shape {}", comp.layers.len());
                    let mut layer = Layer::new(id, name, LayerType::Shape {
                        shape_type: ShapeType::Star,
                        color: [0.9, 0.4, 0.2, 1.0],
                    }, total_frames);
                    layer.transform.position = Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
                    comp.add_layer(layer);
                    project_changed = true;
                }
                if ui.button("+ Audio").clicked() {
                    let id = format!("layer_{}", comp.layers.len());
                    let name = format!("Audio {}", comp.layers.len());
                    let mut layer = Layer::new(id, name, LayerType::Audio {
                        path: "audio_track_01.wav".to_string(),
                        volume: Animatable::new_constant(1.0),
                    }, total_frames);
                    layer.label = crate::core::timeline::LabelColor::Aqua;
                    comp.add_layer(layer);
                    project_changed = true;
                }
                if ui.button("+ Null").clicked() {
                    let id = format!("layer_{}", comp.layers.len());
                    let name = format!("Null {}", comp.layers.len());
                    let layer = Layer::new_null(id, name, total_frames);
                    comp.add_layer(layer);
                    project_changed = true;
                }
                if ui.button("+ Adjustment Layer").clicked() {
                    let id = format!("layer_{}", comp.layers.len());
                    let name = format!("Adjustment Layer {}", comp.layers.len());
                    let layer = Layer::new_adjustment(id, name, total_frames);
                    comp.add_layer(layer);
                    project_changed = true;
                }

                ui.add_space(8.0);
                let add_marker_clicked = ui.button("+ Marker (M)").clicked() ||
                    ui.input(|i| i.key_pressed(egui::Key::M));
                if add_marker_clicked {
                    let marker_idx = comp.markers.len() + 1;
                    comp.markers.push(crate::core::timeline::TimelineMarker {
                        frame: *current_frame,
                        label: format!("Marker {}", marker_idx),
                        color: [1.0, 0.6, 0.1],
                    });
                    project_changed = true;
                }
            });

            ui.add_space(4.0);

            // ── RAM Preview & Marker Ruler Bar ──
            {
                use crate::core::frame_cache;
                let cur_version = frame_cache::current_version();
                let bar_height = 14.0;
                let avail_w = ui.available_width();
                let (bar_rect, bar_response) = ui.allocate_exact_size(
                    egui::vec2(avail_w, bar_height),
                    egui::Sense::click(),
                );

                // Dark background
                ui.painter().rect_filled(
                    bar_rect,
                    2.0,
                    egui::Color32::from_gray(28),
                );

                // Green segments for cached frames
                if total_frames > 0 {
                    let frame_w = bar_rect.width() / total_frames as f32;
                    for f in 0..total_frames {
                        if app.frame_cache.is_cached(f) {
                            let x = bar_rect.left() + f as f32 * frame_w;
                            let seg = egui::Rect::from_min_size(
                                egui::pos2(x, bar_rect.top()),
                                egui::vec2(frame_w.max(1.0), bar_height * 0.5),
                            );
                            ui.painter().rect_filled(
                                seg,
                                0.0,
                                egui::Color32::from_rgb(80, 210, 80),
                            );
                        }
                    }

                    // Render Composition Markers on the Ruler Bar
                    for marker in &comp.markers {
                        let mx = bar_rect.left() + (marker.frame as f32 / total_frames as f32) * bar_rect.width();
                        let tri_pts = vec![
                            egui::pos2(mx, bar_rect.top()),
                            egui::pos2(mx + 5.0, bar_rect.bottom()),
                            egui::pos2(mx - 5.0, bar_rect.bottom()),
                        ];
                        let marker_color = egui::Color32::from_rgb(
                            (marker.color[0] * 255.0) as u8,
                            (marker.color[1] * 255.0) as u8,
                            (marker.color[2] * 255.0) as u8,
                        );
                        ui.painter().add(egui::Shape::convex_polygon(tri_pts, marker_color, egui::Stroke::new(1.0, egui::Color32::WHITE)));
                    }
                }

                if bar_response.clicked() {
                    if let Some(ptr) = bar_response.interact_pointer_pos() {
                        let norm = ((ptr.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                        let raw_f = (norm * total_frames as f32) as u32;
                        *current_frame = maybe_snap_frame(raw_f, app.snap_to_keyframes, comp);
                    }
                }

                // Label and Clear button
                ui.horizontal(|ui| {
                    let cached_count = (0..total_frames)
                        .filter(|&f| app.frame_cache.is_cached(f))
                        .count();
                    ui.small(format!(
                        "RAM Preview: {}/{} frames cached (v{})",
                        cached_count, total_frames, cur_version
                    ));
                    if ui.small_button("Clear").clicked() {
                        app.frame_cache.invalidate_all();
                    }
                });
            }

            ui.add_space(4.0);

            if app.show_graph_editor {
                graph_editor::draw_graph_editor(app, ui, comp, current_frame, total_frames);
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let zoom_span = total_frames as f32 / app.timeline_zoom;
                    let start_frame = 0.0;

                for i in 0..comp.layers.len() {
                    ui.horizontal(|ui| {
                        ui.allocate_ui(egui::vec2(480.0, 24.0), |ui| {
                            ui.horizontal(|ui| {
                                let is_expanded = app.expanded_layers.contains(&i);
                                let arrow = if is_expanded { "▼" } else { "▶" };
                                if ui.selectable_label(is_expanded, arrow).clicked() {
                                    if is_expanded {
                                        app.expanded_layers.remove(&i);
                                    } else {
                                        app.expanded_layers.insert(i);
                                    }
                                }

                                // ── AE Layer Color Label Square Picker ──
                                use crate::core::timeline::LabelColor;
                                let label_rgb = comp.layers[i].label.to_rgb();
                                let label_c32 = egui::Color32::from_rgb(
                                    (label_rgb[0] * 255.0) as u8,
                                    (label_rgb[1] * 255.0) as u8,
                                    (label_rgb[2] * 255.0) as u8,
                                );
                                let (lbl_rect, lbl_resp) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
                                ui.painter().rect_filled(lbl_rect, 1.0, label_c32);
                                lbl_resp.context_menu(|ui| {
                                    ui.label("Label Color:");
                                    for label in [
                                        LabelColor::Red, LabelColor::Yellow, LabelColor::Aqua,
                                        LabelColor::Pink, LabelColor::Lavender, LabelColor::Peach,
                                        LabelColor::Sea, LabelColor::Blue, LabelColor::Purple
                                    ] {
                                        if ui.button(format!("{:?}", label)).clicked() {
                                            comp.layers[i].label = label;
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                    }
                                });

                                let vis = comp.layers[i].visible;
                                let eye_svg = if vis { crate::ui::icons::SVG_EYE_OPEN } else { crate::ui::icons::SVG_EYE_CLOSED };
                                let eye_btn = ui.button(egui::WidgetText::from("")).rect;
                                crate::ui::icons::render_svg_bytes(ui, &format!("eye_{}", i), eye_svg, egui::vec2(14.0, 14.0), egui::Color32::WHITE);
                                if ui.interact(eye_btn, ui.id().with(format!("eye_act_{}", i)), egui::Sense::click()).clicked() || ui.small_button(if vis { "V" } else { "v" }).clicked() {
                                    comp.layers[i].visible = !vis;
                                    project_changed = true;
                                }

                                let solo = comp.layers[i].solo;
                                if ui.selectable_label(solo, "S").clicked() {
                                    comp.layers[i].solo = !solo;
                                    project_changed = true;
                                }

                                let lkd = comp.layers[i].locked;
                                let lock_svg = if lkd { crate::ui::icons::SVG_LOCK } else { crate::ui::icons::SVG_UNLOCK };
                                crate::ui::icons::render_svg_bytes(ui, &format!("lock_{}", i), lock_svg, egui::vec2(14.0, 14.0), egui::Color32::WHITE);
                                if ui.selectable_label(lkd, "L").clicked() {
                                    comp.layers[i].locked = !lkd;
                                    project_changed = true;
                                }

                                let mb = comp.layers[i].motion_blur;
                                if ui.selectable_label(mb, "M").on_hover_text("Motion Blur Switch").clicked() {
                                    comp.layers[i].motion_blur = !mb;
                                    project_changed = true;
                                }

                                let is_3d = comp.layers[i].is_3d;
                                if ui.selectable_label(is_3d, "3D").on_hover_text("3D Layer Switch").clicked() {
                                    comp.layers[i].is_3d = !is_3d;
                                    project_changed = true;
                                }

                                let is_selected = app.selected_layers.contains(&i) || app.selected_layer_idx == Some(i);
                                let label_rgb = comp.layers[i].label.to_rgb();
                                let text_color = egui::Color32::from_rgb(
                                    (label_rgb[0] * 255.0) as u8,
                                    (label_rgb[1] * 255.0) as u8,
                                    (label_rgb[2] * 255.0) as u8,
                                );
                                
                                ui.style_mut().visuals.override_text_color = Some(text_color);
                                let click_resp = ui.selectable_label(is_selected, &comp.layers[i].name);
                                if click_resp.clicked() {
                                    let modifiers = ui.input(|inp| inp.modifiers);
                                    if modifiers.shift || modifiers.command || modifiers.ctrl {
                                        if app.selected_layers.contains(&i) {
                                            app.selected_layers.remove(&i);
                                            if app.selected_layer_idx == Some(i) {
                                                app.selected_layer_idx = app.selected_layers.iter().next().copied();
                                            }
                                        } else {
                                            app.selected_layers.insert(i);
                                            app.selected_layer_idx = Some(i);
                                        }
                                    } else {
                                        app.selected_layers.clear();
                                        app.selected_layers.insert(i);
                                        app.selected_layer_idx = Some(i);
                                    }
                                }
                                ui.style_mut().visuals.override_text_color = None;

                                // ── Blend Mode Dropdown ──
                                use crate::core::timeline::BlendMode;
                                let bm_text = format!("{:?}", comp.layers[i].blend_mode);
                                egui::ComboBox::from_id_source(format!("tl_blend_{}", i))
                                    .selected_text(format!("Blend: {}", bm_text))
                                    .show_ui(ui, |ui| {
                                        for bm in [
                                            BlendMode::Normal,
                                            BlendMode::Multiply,
                                            BlendMode::Screen,
                                            BlendMode::Overlay,
                                            BlendMode::Add,
                                            BlendMode::Darken,
                                            BlendMode::Lighten,
                                        ] {
                                            if ui.selectable_label(comp.layers[i].blend_mode == bm, format!("{:?}", bm)).clicked() {
                                                comp.layers[i].blend_mode = bm;
                                                project_changed = true;
                                            }
                                        }
                                    });

                                // ── Track Matte Dropdown ──
                                use crate::core::timeline::TrackMatteMode;
                                let tm_text = match comp.layers[i].track_matte {
                                    TrackMatteMode::None => "None",
                                    TrackMatteMode::AlphaMatte => "Alpha",
                                    TrackMatteMode::AlphaMatteInverted => "Alpha Inv",
                                    TrackMatteMode::LumaMatte => "Luma",
                                    TrackMatteMode::LumaMatteInverted => "Luma Inv",
                                };
                                egui::ComboBox::from_id_source(format!("tl_matte_{}", i))
                                    .selected_text(format!("Matte: {}", tm_text))
                                    .show_ui(ui, |ui| {
                                        for (mode, label) in [
                                            (TrackMatteMode::None, "None"),
                                            (TrackMatteMode::AlphaMatte, "Alpha Matte"),
                                            (TrackMatteMode::AlphaMatteInverted, "Alpha Matte Inverted"),
                                            (TrackMatteMode::LumaMatte, "Luma Matte"),
                                            (TrackMatteMode::LumaMatteInverted, "Luma Matte Inverted"),
                                        ] {
                                            if ui.selectable_label(comp.layers[i].track_matte == mode, label).clicked() {
                                                comp.layers[i].track_matte = mode;
                                                project_changed = true;
                                            }
                                        }
                                    });

                                // ── Parenting Dropdown ──
                                let parent_text = comp.layers[i].parent_id.as_deref().unwrap_or("None");
                                egui::ComboBox::from_id_source(format!("tl_parent_{}", i))
                                    .selected_text(format!("Parent: {}", parent_text))
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(comp.layers[i].parent_id.is_none(), "None").clicked() {
                                            comp.layers[i].parent_id = None;
                                            project_changed = true;
                                        }
                                        for p_other in 0..comp.layers.len() {
                                            if p_other != i {
                                                let p_id = comp.layers[p_other].id.clone();
                                                let is_p = comp.layers[i].parent_id.as_deref() == Some(&p_id);
                                                if ui.selectable_label(is_p, &comp.layers[p_other].name).clicked() {
                                                    comp.layers[i].parent_id = Some(p_id);
                                                    project_changed = true;
                                                }
                                            }
                                        }
                                    });
                            });
                        });

                        ui.separator();

                        let ruler_width = ui.available_width() - 20.0;
                        let (track_rect, track_response) = ui.allocate_exact_size(
                            egui::vec2(ruler_width, 24.0),
                            egui::Sense::click_and_drag(),
                        );

                        ui.painter().rect_filled(track_rect, 2.0, egui::Color32::from_gray(40));

                        let in_x = track_rect.left() + ((comp.layers[i].in_frame as f32 - start_frame) / zoom_span) * track_rect.width();
                        let out_x = track_rect.left() + ((comp.layers[i].out_frame as f32 - start_frame) / zoom_span) * track_rect.width();

                        let span_rect = egui::Rect::from_x_y_ranges(
                            in_x.max(track_rect.left())..=out_x.min(track_rect.right()),
                            (track_rect.top() + 4.0)..=(track_rect.bottom() - 4.0),
                        );
                        
                        let base_color = match comp.layers[i].layer_type {
                            LayerType::Null => egui::Color32::from_rgb(180, 100, 100),
                            LayerType::Audio { .. } => egui::Color32::from_rgb(40, 140, 110),
                            _ => egui::Color32::from_rgb(80, 110, 160),
                        };
                        ui.painter().rect_filled(span_rect, 2.0, base_color);

                        // ── Audio Waveform Visualization ──
                        if matches!(comp.layers[i].layer_type, LayerType::Audio { .. }) && span_rect.width() > 10.0 {
                            let wave_color = egui::Color32::from_rgba_unmultiplied(180, 255, 220, 180);
                            let mid_y = span_rect.center().y;
                            let step_px = 3.0;
                            let mut x = span_rect.left() + 2.0;
                            let mut step_idx = 0;
                            while x < span_rect.right() - 2.0 {
                                let phase = step_idx as f32 * 0.18;
                                let env = (phase.sin().abs() * 0.65 + (phase * 2.7).cos().abs() * 0.35) * (span_rect.height() * 0.42);
                                ui.painter().line_segment(
                                    [egui::pos2(x, mid_y - env), egui::pos2(x, mid_y + env)],
                                    egui::Stroke::new(1.2, wave_color),
                                );
                                x += step_px;
                                step_idx += 1;
                            }
                        }

                        let handle_w = 4.0;
                        if in_x >= track_rect.left() && in_x <= track_rect.right() {
                            ui.painter().rect_filled(
                                egui::Rect::from_x_y_ranges(in_x..=(in_x + handle_w), track_rect.top()..=track_rect.bottom()),
                                1.0, egui::Color32::from_rgb(120, 180, 255),
                            );
                        }
                        if out_x >= track_rect.left() && out_x <= track_rect.right() {
                            ui.painter().rect_filled(
                                egui::Rect::from_x_y_ranges((out_x - handle_w)..=out_x, track_rect.top()..=track_rect.bottom()),
                                1.0, egui::Color32::from_rgb(120, 180, 255),
                            );
                        }

                        if let Some(ptr) = track_response.interact_pointer_pos() {
                            let ptr_frame = (start_frame + (ptr.x - track_rect.left()) / track_rect.width() * zoom_span)
                                .clamp(0.0, total_frames as f32) as u32;

                            let near_in  = (ptr.x - in_x).abs() < 8.0;
                            let near_out = (ptr.x - out_x).abs() < 8.0;

                            if track_response.drag_started() && (near_in || near_out) {
                                app.selected_layer_idx = Some(i);
                            }

                            if track_response.dragged() {
                                let in_px_start = track_rect.left()
                                    + ((comp.layers[i].in_frame as f32 - start_frame) / zoom_span) * track_rect.width();
                                let out_px_start = track_rect.left()
                                    + ((comp.layers[i].out_frame as f32 - start_frame) / zoom_span) * track_rect.width();
                                let drag_origin = ptr - track_response.drag_delta();
                                let near_in_start  = (drag_origin.x - in_px_start).abs() < 12.0;
                                let near_out_start = (drag_origin.x - out_px_start).abs() < 12.0;

                                if near_in_start && !near_out_start {
                                    comp.layers[i].in_frame = ptr_frame.min(comp.layers[i].out_frame.saturating_sub(1));
                                    project_changed = true;
                                } else if near_out_start {
                                    comp.layers[i].out_frame = ptr_frame.max(comp.layers[i].in_frame + 1).min(total_frames);
                                    project_changed = true;
                                } else if !near_in_start && !near_out_start {
                                    let dur = comp.layers[i].out_frame - comp.layers[i].in_frame;
                                    let new_in = (ptr_frame as i64 - dur as i64 / 2).clamp(0, (total_frames - dur) as i64) as u32;
                                    comp.layers[i].in_frame  = new_in;
                                    comp.layers[i].out_frame = new_in + dur;
                                    project_changed = true;
                                }
                            }

                            if near_in || near_out {
                                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                            }
                        }

                        let layer = &comp.layers[i];
                        let mut kf_frames = std::collections::HashSet::new();
                        for (f, _) in get_kfs(&layer.transform.position) { kf_frames.insert(f); }
                        for (f, _) in get_kfs(&layer.transform.scale) { kf_frames.insert(f); }
                        for (f, _) in get_kfs(&layer.transform.rotation) { kf_frames.insert(f); }
                        for (f, _) in get_kfs(&layer.transform.opacity) { kf_frames.insert(f); }

                        for f in kf_frames {
                            let x = track_rect.left() + ((f as f32 - start_frame) / zoom_span) * track_rect.width();
                            if x >= track_rect.left() && x <= track_rect.right() {
                                draw_keyframe_tick(ui, x, track_rect.center().y, false, current_frame, f, None);
                            }
                        }

                        let playhead_x = track_rect.left() + ((*current_frame as f32 - start_frame) / zoom_span) * track_rect.width();
                        if playhead_x >= track_rect.left() && playhead_x <= track_rect.right() {
                            ui.painter().line_segment(
                                [egui::pos2(playhead_x, track_rect.top()), egui::pos2(playhead_x, track_rect.bottom())],
                                egui::Stroke::new(1.0, egui::Color32::RED),
                            );
                        }
                    });

                    if app.expanded_layers.contains(&i) {
                        let draw_prop_row = |ui: &mut egui::Ui, label: &str, kf_data: &[(u32, crate::core::keyframe::InterpolationType)], current_frame: &mut u32| {
                            ui.horizontal(|ui| {
                                ui.allocate_ui(egui::vec2(220.0, 16.0), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add_space(20.0);
                                        ui.weak(label);
                                    });
                                });

                                ui.separator();

                                let ruler_width = ui.available_width() - 20.0;
                                let (prop_rect, _prop_response) = ui.allocate_exact_size(
                                    egui::vec2(ruler_width, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(prop_rect, 1.0, egui::Color32::from_gray(30));

                                for &(frame, interpolation) in kf_data {
                                    let x = prop_rect.left() + (frame as f32 / total_frames as f32) * prop_rect.width();
                                    draw_keyframe_tick(ui, x, prop_rect.center().y, true, current_frame, frame, Some(interpolation));
                                }

                                let playhead_x = prop_rect.left() + (*current_frame as f32 / total_frames as f32) * prop_rect.width();
                                ui.painter().line_segment(
                                    [egui::pos2(playhead_x, prop_rect.top()), egui::pos2(playhead_x, prop_rect.bottom())],
                                    egui::Stroke::new(1.0, egui::Color32::RED),
                                );
                            });
                        };

                        let layer = &comp.layers[i];
                        
                        let pos_kfs = get_kfs(&layer.transform.position);
                        let scale_kfs = get_kfs(&layer.transform.scale);
                        let rot_kfs = get_kfs(&layer.transform.rotation);
                        let op_kfs = get_kfs(&layer.transform.opacity);

                        draw_prop_row(ui, "  Position", &pos_kfs, current_frame);
                        draw_prop_row(ui, "  Scale", &scale_kfs, current_frame);
                        draw_prop_row(ui, "  Rotation", &rot_kfs, current_frame);
                        draw_prop_row(ui, "  Opacity", &op_kfs, current_frame);

                        for effect in &layer.effects {
                            match &effect.effect_type {
                                EffectType::GaussianBlur { blur_radius } => {
                                    let blur_kfs = get_kfs(blur_radius);
                                    draw_prop_row(ui, &format!("  [{}] Radius", effect.name), &blur_kfs, current_frame);
                                }
                                EffectType::ColorTint { color, intensity } => {
                                    let color_kfs = get_kfs(color);
                                    let intensity_kfs = get_kfs(intensity);
                                    draw_prop_row(ui, &format!("  [{}] Color", effect.name), &color_kfs, current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Intensity", effect.name), &intensity_kfs, current_frame);
                                }
                                EffectType::DropShadow { color, opacity, direction, distance, softness } => {
                                    let color_kfs = get_kfs(color);
                                    let opacity_kfs = get_kfs(opacity);
                                    let direction_kfs = get_kfs(direction);
                                    let distance_kfs = get_kfs(distance);
                                    let softness_kfs = get_kfs(softness);
                                    draw_prop_row(ui, &format!("  [{}] Shadow Color", effect.name), &color_kfs, current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Opacity", effect.name), &opacity_kfs, current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Direction", effect.name), &direction_kfs, current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Distance", effect.name), &distance_kfs, current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Softness", effect.name), &softness_kfs, current_frame);
                                }
                                EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
                                    draw_prop_row(ui, &format!("  [{}] Red Shift", effect.name), &get_kfs(shift_r), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Blue Shift", effect.name), &get_kfs(shift_b), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Edge Falloff", effect.name), &get_kfs(edge_falloff), current_frame);
                                }
                                EffectType::Vignette { intensity, roundness, feather, color } => {
                                    draw_prop_row(ui, &format!("  [{}] Intensity", effect.name), &get_kfs(intensity), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Roundness", effect.name), &get_kfs(roundness), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Feather", effect.name), &get_kfs(feather), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Color", effect.name), &get_kfs(color), current_frame);
                                }
                                EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
                                    draw_prop_row(ui, &format!("  [{}] Input Black", effect.name), &get_kfs(input_black), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Input White", effect.name), &get_kfs(input_white), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Gamma", effect.name), &get_kfs(gamma), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Output Black", effect.name), &get_kfs(output_black), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Output White", effect.name), &get_kfs(output_white), current_frame);
                                }
                                EffectType::HueSaturation { hue_shift, saturation, lightness } => {
                                    draw_prop_row(ui, &format!("  [{}] Hue Shift", effect.name), &get_kfs(hue_shift), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Saturation", effect.name), &get_kfs(saturation), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Lightness", effect.name), &get_kfs(lightness), current_frame);
                                }
                                EffectType::Glow { threshold, radius, intensity, color } => {
                                    draw_prop_row(ui, &format!("  [{}] Threshold", effect.name), &get_kfs(threshold), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Radius", effect.name), &get_kfs(radius), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Intensity", effect.name), &get_kfs(intensity), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Color", effect.name), &get_kfs(color), current_frame);
                                }
                                EffectType::MotionBlur { shutter_angle, .. } => {
                                    draw_prop_row(ui, &format!("  [{}] Shutter Angle", effect.name), &get_kfs(shutter_angle), current_frame);
                                }
                                EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
                                    draw_prop_row(ui, &format!("  [{}] Top Left", effect.name), &get_kfs(top_left), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Top Right", effect.name), &get_kfs(top_right), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Bottom Left", effect.name), &get_kfs(bottom_left), current_frame);
                                    draw_prop_row(ui, &format!("  [{}] Bottom Right", effect.name), &get_kfs(bottom_right), current_frame);
                                }
                                EffectType::ColorGradeLUT { intensity, .. } => {
                                    draw_prop_row(ui, &format!("  [{}] LUT Intensity", effect.name), &get_kfs(intensity), current_frame);
                                }
                                EffectType::ColorSpaceConvert { .. } => {
                                    // No animated properties for simple color space converter
                                }
                                EffectType::FilmGrain { intensity, .. } => {
                                    draw_prop_row(ui, &format!("  [{}] Grain Intensity", effect.name), &get_kfs(intensity), current_frame);
                                }
                            }
                        }
                    }
                }
            });
            }

            if project_changed {
                let is_pointer_down = ui.input(|i| i.pointer.any_down());
                if !is_pointer_down {
                    app.history.commit(temp_project);
                } else {
                    *app.history.current_mut() = temp_project;
                }
                crate::core::frame_cache::bump_version();
            }
        });
}
