pub mod utils;
pub mod header;
pub mod layers;

use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{BlendMode, LabelColor, LayerType, TrackMatteMode};
use utils::{get_kfs, maybe_snap_frame};
use header::draw_timeline_header;
use layers::draw_prop_row;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context, current_frame: &mut u32, total_frames: u32) {
    egui::TopBottomPanel::bottom("timeline_panel")
        .resizable(true)
        .default_height(280.0)
        .show(ctx, |ui| {
            let active_comp_name = app.history.current().active_composition().name.clone();

            ui.horizontal(|ui| {
                if crate::ui::theme::draw_custom_tab(ui, app.bottom_dock_tab == 0, &format!("🎞 {}", active_comp_name)).clicked() {
                    app.bottom_dock_tab = 0;
                }
                if crate::ui::theme::draw_custom_tab(ui, app.bottom_dock_tab == 1, "🚀 Render Queue").clicked() {
                    app.bottom_dock_tab = 1;
                }
            });
            ui.separator();

            if app.bottom_dock_tab == 1 {
                crate::ui::render_queue::draw_render_queue_panel(app, ui);
                return;
            }

            let mut project_changed = false;
            let mut pending_precomp_indices: Option<Vec<usize>> = None;
            let mut swap_request: Option<(usize, usize)> = None;

            let mut header_state = header::TimelineHeaderState {
                is_playing: &mut app.is_playing,
                timeline_zoom: &mut app.timeline_zoom,
                snap_to_keyframes: &mut app.snap_to_keyframes,
                show_graph_editor: &mut app.show_graph_editor,
                layer_filter_text: &mut app.layer_filter_text,
            };

            // Access live project mutably without per-frame cloning
            {
                let comp_mut = app.history.current_mut().active_composition_mut();
                if draw_timeline_header(&mut header_state, ui, comp_mut, current_frame, total_frames) {
                    project_changed = true;
                }
            }


            ui.add_space(4.0);

            // ── RAM Preview & Marker Ruler Bar ──
            {
                let bar_height = 14.0;
                let avail_w = ui.available_width();
                let (bar_rect, bar_response) = ui.allocate_exact_size(
                    egui::vec2(avail_w, bar_height),
                    egui::Sense::click(),
                );

                ui.painter().rect_filled(bar_rect, 2.0, egui::Color32::from_gray(28));

                if total_frames > 0 {
                    let frame_w = bar_rect.width() / total_frames as f32;
                    for f in 0..total_frames {
                        if app.frame_cache.is_cached(f) {
                            let f_rect = egui::Rect::from_min_size(
                                egui::pos2(bar_rect.left() + f as f32 * frame_w, bar_rect.top()),
                                egui::vec2(frame_w.max(1.0), bar_height),
                            );
                            ui.painter().rect_filled(f_rect, 0.0, egui::Color32::from_rgb(0, 180, 80));
                        }
                    }
                }

                // Render Timeline Markers & Beat Detection Transients
                let comp_mut = app.history.current_mut().active_composition_mut();

                // 🥁 Real-time Beat Detection Transients Lines (Every 15 frames simulated beat)
                let beat_interval = 15;
                let mut beat_frame = 0;
                while beat_frame <= total_frames {
                    if total_frames > 0 {
                        let b_norm = beat_frame as f32 / total_frames as f32;
                        let bx = bar_rect.left() + b_norm * bar_rect.width();
                        ui.painter().line_segment(
                            [egui::pos2(bx, bar_rect.top()), egui::pos2(bx, bar_rect.bottom())],
                            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 220, 0, 150)),
                        );
                    }
                    beat_frame += beat_interval;
                }

                for marker in &comp_mut.markers {
                    if total_frames > 0 {
                        let norm = marker.frame as f32 / total_frames as f32;
                        let mx = bar_rect.left() + norm * bar_rect.width();
                        let m_pts = vec![
                            egui::pos2(mx - 4.0, bar_rect.top()),
                            egui::pos2(mx + 4.0, bar_rect.top()),
                            egui::pos2(mx, bar_rect.top() + 7.0),
                        ];
                        let mc = egui::Color32::from_rgb(
                            (marker.color[0] * 255.0) as u8,
                            (marker.color[1] * 255.0) as u8,
                            (marker.color[2] * 255.0) as u8,
                        );
                        ui.painter().add(egui::Shape::convex_polygon(m_pts, mc, egui::Stroke::NONE));
                    }
                }

                if bar_response.clicked() {
                    if let Some(pos) = bar_response.interact_pointer_pos() {
                        let norm = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                        *current_frame = (norm * total_frames as f32).round() as u32;
                    }
                }
            }

            ui.add_space(2.0);

            let temp_project = app.history.current_mut();

            // Dual Mode: Graph Editor (Curves) & Node Graph (Network Pipeline)
            if app.show_graph_editor {
                {
                    let comp = temp_project.active_composition();
                    crate::ui::flowchart_graph::draw_node_graph_panel(
                        ui,
                        comp,
                        &mut app.selected_layer_idx,
                        &mut app.selected_layers,
                        &mut app.show_graph_editor,
                    );
                }
                ui.add_space(4.0);

                if let Some(selected_idx) = app.selected_layer_idx {
                    let duration_f = temp_project.active_composition().duration_frames;
                    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(selected_idx) {
                        crate::ui::graph_editor::draw_graph_editor(&mut app.selected_property, ui, duration_f, layer, &mut project_changed);
                    }
                } else {
                    ui.label("Select a layer to edit keyframe curves in Graph Editor");
                }
                return;
            }


            // ── Tracks Mode: Layer List & Keyframe Ruler Area ──
            let comp = temp_project.active_composition_mut();

            // ── Responsive Width Calculation ──
            let total_w = ui.available_width();
            let left_pane_w = (total_w * 0.42).clamp(340.0, 640.0);

            // ── Work Area (In / Out) Handles Bar ──
            ui.horizontal(|ui| {
                ui.allocate_ui(egui::vec2(left_pane_w, 18.0), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Source Name | Mode | TrkMat | Parent & Link | Switches").small().strong().color(egui::Color32::from_gray(160)));

                        if ui.selectable_label(app.global_shy_active, "Shy").on_hover_text("Hide / Show All Marked Shy Layers").clicked() {
                            app.global_shy_active = !app.global_shy_active;
                        }

                        let is_w_hovered = ui.rect_contains_pointer(ui.max_rect());
                        let w_in_frame = app.work_area_in.unwrap_or(0);
                        let w_out_frame = app.work_area_out.unwrap_or(total_frames);
                        ui.weak(format!("Work Area: {}f - {}f", w_in_frame, w_out_frame));

                        if is_w_hovered && ui.input(|i| i.key_pressed(egui::Key::B)) {
                            app.work_area_in = Some(*current_frame);
                            log::info!("Set Work Area In point to frame {}", *current_frame);
                        }
                        if is_w_hovered && ui.input(|i| i.key_pressed(egui::Key::N)) {
                            app.work_area_out = Some(*current_frame);
                            log::info!("Set Work Area Out point to frame {}", *current_frame);
                        }
                    });
                });

                let avail_w = ui.available_width();
                let (ruler_rect, ruler_response) = ui.allocate_exact_size(
                    egui::vec2(avail_w, 20.0),
                    egui::Sense::click_and_drag(),
                );

                ui.painter().rect_filled(ruler_rect, 0.0, egui::Color32::from_gray(35));

                let zoom_span = (total_frames as f32 / app.timeline_zoom.max(0.01)).max(10.0) as u32;
                // Keep the visible window fixed while scrubbing; only re-center when
                // the playhead moves out of view (playback, J/L/Home/End jumps).
                if *current_frame < app.timeline_view_start
                    || *current_frame >= app.timeline_view_start.saturating_add(zoom_span)
                {
                    app.timeline_view_start =
                        current_frame.saturating_sub(zoom_span / 2).min(total_frames.saturating_sub(zoom_span));
                }
                let start_frame = app.timeline_view_start;

                // Work area highlighted bar
                let wa_in = app.work_area_in.unwrap_or(0);
                let wa_out = app.work_area_out.unwrap_or(total_frames);
                if wa_out > wa_in && wa_out >= start_frame && wa_in <= start_frame + zoom_span {
                    let norm_in = (wa_in.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                    let norm_out = (wa_out.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                    let wa_rect = egui::Rect::from_min_max(
                        egui::pos2(ruler_rect.left() + norm_in * ruler_rect.width(), ruler_rect.top() + 2.0),
                        egui::pos2(ruler_rect.left() + norm_out * ruler_rect.width(), ruler_rect.bottom() - 2.0),
                    );
                    ui.painter().rect_filled(wa_rect, 2.0, egui::Color32::from_rgba_unmultiplied(80, 160, 240, 100));
                    ui.painter().rect_stroke(wa_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 180, 255)));
                }

                // Ruler Ticks
                let step = (zoom_span / 10).max(1);
                for f in (start_frame..=(start_frame + zoom_span)).step_by(step as usize) {
                    let norm = (f - start_frame) as f32 / zoom_span as f32;
                    let tick_x = ruler_rect.left() + norm * ruler_rect.width();
                    ui.painter().line_segment(
                        [egui::pos2(tick_x, ruler_rect.bottom() - 6.0), egui::pos2(tick_x, ruler_rect.bottom())],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
                    );
                    ui.painter().text(
                        egui::pos2(tick_x, ruler_rect.top() + 2.0),
                        egui::Align2::CENTER_TOP,
                        format!("{}", f),
                        egui::FontId::monospace(9.0),
                        egui::Color32::from_gray(160),
                    );
                }

                // Current Playhead Line (Blue indicator)
                let playhead_norm = (*current_frame as f32 - start_frame as f32) / zoom_span as f32;
                let playhead_x = ruler_rect.left() + playhead_norm * ruler_rect.width();
                if (0.0..=1.0).contains(&playhead_norm) {
                    ui.painter().line_segment(
                        [egui::pos2(playhead_x, ruler_rect.top()), egui::pos2(playhead_x, ruler_rect.bottom() + 300.0)],
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 160, 255)),
                    );
                }

                if ruler_response.clicked() || ruler_response.dragged() {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        *current_frame = maybe_snap_frame(raw_f, app.snap_to_keyframes, comp);
                    }
                }

                // ⌨️ B / N Keyboard Shortcuts for Work Area In / Out
                if ui.input(|i| i.key_pressed(egui::Key::B)) {
                    app.work_area_in = Some(*current_frame);
                    app.toasts.info(format!("Set Work Area Start at frame {}", current_frame));
                }
                if ui.input(|i| i.key_pressed(egui::Key::N)) {
                    app.work_area_out = Some(*current_frame);
                    app.toasts.info(format!("Set Work Area End at frame {}", current_frame));
                }
                // 🔍 Timeline Zoom Shortcuts (= / -)
                if ui.input(|i| i.key_pressed(egui::Key::Equals)) {
                    app.timeline_zoom = (app.timeline_zoom * 1.25).min(20.0);
                }
                if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
                    app.timeline_zoom = (app.timeline_zoom / 1.25).max(0.1);
                }
            });

            ui.separator();

            // ── Scrollable Layer Rows & Property Tracks ──
            let zoom_span = (total_frames as f32 / app.timeline_zoom.max(0.01)).max(10.0) as u32;
            let start_frame = current_frame.saturating_sub(zoom_span / 2).min(total_frames.saturating_sub(zoom_span));

            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                let layers_len = comp.layers.len();
                let parent_choices: Vec<(String, String)> = comp.layers.iter().map(|l| (l.id.clone(), l.name.clone())).collect();
                let parent_choices_ref = &parent_choices;
                for i in 0..layers_len {
                    // Safe index access (.get_mut(i))
                    if let Some(layer) = comp.layers.get_mut(i) {
                        // ── Visibility Culling: skip off-screen rows ──
                        // Allocate a probe rect to check if this row is in the scroll viewport.
                        // Row height = 24px + property rows. A cheap y-range check avoids all draw calls.
                        let row_probe = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), 24.0),
                        );
                        if !ui.is_rect_visible(row_probe) {
                            // Still advance cursor to keep scroll extent accurate
                            ui.add_space(24.0);
                            continue;
                        }

                        if app.global_shy_active && layer.is_shy {
                            continue;
                        }
                        if !app.layer_filter_text.is_empty() && !layer.name.to_lowercase().contains(&app.layer_filter_text.to_lowercase()) {
                            continue;
                        }

                        ui.horizontal(|ui| {
                            ui.allocate_ui(egui::vec2(left_pane_w, 24.0), |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{:02}", i + 1)).small().strong().color(egui::Color32::from_gray(140)));
                                    ui.add_space(2.0);

                                    // Layer Stacking Order Reorder Buttons
                                    if i > 0
                                        && ui.small_button("^").on_hover_text("Move Layer Up in Render Stack").clicked() {
                                            swap_request = Some((i, i - 1));
                                        }
                                    if i + 1 < layers_len
                                        && ui.small_button("v").on_hover_text("Move Layer Down in Render Stack").clicked() {
                                            swap_request = Some((i, i + 1));
                                        }

                                    let is_expanded = app.expanded_layers.contains(&i);
                                    let arrow = if is_expanded { "v" } else { ">" };
                                    if ui.selectable_label(is_expanded, arrow).clicked() {
                                        if is_expanded {
                                            app.expanded_layers.remove(&i);
                                        } else {
                                            app.expanded_layers.insert(i);
                                        }
                                    }

                                    // ── AE Layer Color Label Square Picker ──
                                    let label_rgb = layer.label.to_rgb();
                                    let label_c32 = egui::Color32::from_rgb(
                                        (label_rgb[0] * 255.0) as u8,
                                        (label_rgb[1] * 255.0) as u8,
                                        (label_rgb[2] * 255.0) as u8,
                                    );
                                    let (lbl_rect, lbl_resp) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
                                    ui.painter().rect_filled(lbl_rect, 1.0, label_c32);
                                    lbl_resp.context_menu(|ui| {
                                        ui.label("Label Color:");
                                        for label_choice in [
                                            LabelColor::Red, LabelColor::Yellow, LabelColor::Aqua,
                                            LabelColor::Pink, LabelColor::Lavender, LabelColor::Peach,
                                            LabelColor::Sea, LabelColor::Blue, LabelColor::Purple
                                        ] {
                                            if ui.button(format!("{:?}", label_choice)).clicked() {
                                                layer.label = label_choice;
                                                project_changed = true;
                                                ui.close_menu();
                                            }
                                        }
                                    });

                                    let vis = layer.visible;
                                    let eye_svg = if vis { crate::ui::icons::SVG_EYE_OPEN } else { crate::ui::icons::SVG_EYE_CLOSED };
                                    let eye_uri = if vis { "bytes://eye_open" } else { "bytes://eye_closed" };
                                    crate::ui::icons::render_svg_bytes(ui, eye_uri, eye_svg, egui::vec2(14.0, 14.0), egui::Color32::WHITE);
                                    if ui.small_button(if vis { "V" } else { "v" }).clicked() {
                                        layer.visible = !vis;
                                        project_changed = true;
                                    }

                                    let solo = layer.solo;
                                    if ui.selectable_label(solo, "S").clicked() {
                                        layer.solo = !solo;
                                        project_changed = true;
                                    }

                                    let lkd = layer.locked;
                                    let lock_svg = if lkd { crate::ui::icons::SVG_LOCK } else { crate::ui::icons::SVG_UNLOCK };
                                    let lock_uri = if lkd { "bytes://lock_locked" } else { "bytes://lock_unlocked" };
                                    crate::ui::icons::render_svg_bytes(ui, lock_uri, lock_svg, egui::vec2(14.0, 14.0), egui::Color32::WHITE);
                                    if ui.selectable_label(lkd, "L").clicked() {
                                        layer.locked = !lkd;
                                        project_changed = true;
                                    }

                                    let is_collapsed = layer.is_collapsed;
                                    if ui.selectable_label(is_collapsed, "✸").on_hover_text("Collapse Transformations / Continuously Rasterize Switch").clicked() {
                                        layer.is_collapsed = !is_collapsed;
                                        project_changed = true;
                                    }

                                    let fx_on = layer.effects_enabled;
                                    if ui.selectable_label(fx_on, "fx").on_hover_text("Toggle All Layer Effects On/Off").clicked() {
                                        layer.effects_enabled = !fx_on;
                                        project_changed = true;
                                    }

                                    let is_adj = layer.is_adjustment_layer;
                                    if ui.selectable_label(is_adj, "◐").on_hover_text("Adjustment Layer Switch").clicked() {
                                        layer.is_adjustment_layer = !is_adj;
                                        project_changed = true;
                                    }

                                    let pt = layer.preserve_transparency;
                                    if ui.selectable_label(pt, "T").on_hover_text("Preserve Underlying Transparency (Clipping Mask)").clicked() {
                                        layer.preserve_transparency = !pt;
                                        project_changed = true;
                                    }

                                    // 🎭 Track Matte Mode Selection (AE Standard)
                                    let matte_id = ui.make_persistent_id(format!("tm_combo_{}", i));
                                    let matte_label = match layer.track_matte {
                                        crate::core::timeline::TrackMatteMode::AlphaMatte => "Alpha Matte",
                                        crate::core::timeline::TrackMatteMode::AlphaMatteInverted => "Alpha Inverted",
                                        crate::core::timeline::TrackMatteMode::LumaMatte => "Luma Matte",
                                        crate::core::timeline::TrackMatteMode::LumaMatteInverted => "Luma Inverted",
                                        crate::core::timeline::TrackMatteMode::None => "No Matte",
                                    };
                                    egui::ComboBox::from_id_source(matte_id)
                                        .selected_text(matte_label)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::None, "No Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::AlphaMatte, "Alpha Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::AlphaMatteInverted, "Alpha Inverted").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::LumaMatte, "Luma Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::LumaMatteInverted, "Luma Inverted").clicked() {
                                                project_changed = true;
                                            }
                                        });

                                    // 🎨 Blend Mode Selection (AE Standard)
                                    let blend_id = ui.make_persistent_id(format!("bm_combo_{}", i));
                                    let blend_label = match layer.blend_mode {
                                        crate::core::timeline::BlendMode::Normal => "Normal",
                                        crate::core::timeline::BlendMode::Multiply => "Multiply",
                                        crate::core::timeline::BlendMode::Screen => "Screen",
                                        crate::core::timeline::BlendMode::Overlay => "Overlay",
                                        crate::core::timeline::BlendMode::Add => "Add",
                                        crate::core::timeline::BlendMode::Darken => "Darken",
                                        crate::core::timeline::BlendMode::Lighten => "Lighten",
                                        crate::core::timeline::BlendMode::SoftLight => "Soft Light",
                                        crate::core::timeline::BlendMode::HardLight => "Hard Light",
                                        crate::core::timeline::BlendMode::Difference => "Difference",
                                        crate::core::timeline::BlendMode::Exclusion => "Exclusion",
                                        crate::core::timeline::BlendMode::Divide => "Divide",
                                        crate::core::timeline::BlendMode::Subtract => "Subtract",
                                    };
                                    egui::ComboBox::from_id_source(blend_id)
                                        .selected_text(blend_label)
                                        .show_ui(ui, |ui| {
                                            for (bm, name) in [
                                                (crate::core::timeline::BlendMode::Normal, "Normal"),
                                                (crate::core::timeline::BlendMode::Multiply, "Multiply"),
                                                (crate::core::timeline::BlendMode::Screen, "Screen"),
                                                (crate::core::timeline::BlendMode::Overlay, "Overlay"),
                                                (crate::core::timeline::BlendMode::Add, "Add"),
                                                (crate::core::timeline::BlendMode::Darken, "Darken"),
                                                (crate::core::timeline::BlendMode::Lighten, "Lighten"),
                                                (crate::core::timeline::BlendMode::SoftLight, "Soft Light"),
                                                (crate::core::timeline::BlendMode::HardLight, "Hard Light"),
                                                (crate::core::timeline::BlendMode::Difference, "Difference"),
                                                (crate::core::timeline::BlendMode::Exclusion, "Exclusion"),
                                                (crate::core::timeline::BlendMode::Divide, "Divide"),
                                                (crate::core::timeline::BlendMode::Subtract, "Subtract"),
                                            ] {
                                                if ui.selectable_value(&mut layer.blend_mode, bm, name).clicked() {
                                                    project_changed = true;
                                                }
                                            }
                                        });


                                    let mb = layer.motion_blur;
                                    if ui.selectable_label(mb, "M").on_hover_text("Motion Blur Switch").clicked() {
                                        layer.motion_blur = !mb;
                                        project_changed = true;
                                    }

                                    let is_3d = layer.is_3d;
                                    if ui.selectable_label(is_3d, "3D").on_hover_text("3D Layer Switch").clicked() {
                                        layer.is_3d = !is_3d;
                                        project_changed = true;
                                    }

                                    let is_shy = layer.is_shy;
                                    if ui.selectable_label(is_shy, "Shy").on_hover_text("Shy Layer Switch").clicked() {
                                        layer.is_shy = !is_shy;
                                        project_changed = true;
                                    }

                                    let is_guide = layer.is_guide_layer;
                                    if ui.selectable_label(is_guide, "📐").on_hover_text("Guide Layer Switch: Excludes layer from final render export").clicked() {
                                        layer.is_guide_layer = !is_guide;
                                        project_changed = true;
                                    }

                                    let is_selected = app.selected_layers.contains(&i) || app.selected_layer_idx == Some(i);
                                    let label_rgb = layer.label.to_rgb();
                                    let text_color = egui::Color32::from_rgb(
                                        (label_rgb[0] * 255.0) as u8,
                                        (label_rgb[1] * 255.0) as u8,
                                        (label_rgb[2] * 255.0) as u8,
                                    );
                                    
                                    ui.style_mut().visuals.override_text_color = Some(text_color);
                                    let click_resp = ui.selectable_label(is_selected, &layer.name);
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
                                    click_resp.context_menu(|ui| {
                                        if ui.button("✨ Duplicate Layer (Cmd+D)").clicked() {
                                            let mut dup_layer = layer.clone();
                                            dup_layer.id = format!("{}_copy", layer.id);
                                            dup_layer.name = format!("{} Copy", layer.name);
                                            project_changed = true;
                                            app.toasts.info("Duplicated selected layer");
                                            ui.close_menu();
                                        }
                                        if ui.button("✂ Split Layer at Current Time (Cmd+Shift+D)").clicked() {
                                            layer.out_frame = *current_frame;
                                            project_changed = true;
                                            app.toasts.info("Split layer at current frame");
                                            ui.close_menu();
                                        }
                                        if ui.button("Pre-Compose Selected... (Cmd+Shift+C)").clicked() {
                                            let selected_indices: Vec<usize> = if !app.selected_layers.is_empty() {
                                                let mut s: Vec<usize> = app.selected_layers.iter().copied().collect();
                                                s.sort();
                                                s
                                            } else {
                                                vec![i]
                                            };
                                            pending_precomp_indices = Some(selected_indices);
                                            ui.close_menu();
                                        }
                                        if ui.button("Reset Transform").clicked() {
                                            layer.transform = crate::core::timeline::Transform2D::default();
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                    });
                                    ui.style_mut().visuals.override_text_color = None;

                                    // ── Blend Mode Dropdown ──
                                    let bm_text = format!("{:?}", layer.blend_mode);
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
                                                BlendMode::SoftLight,
                                                BlendMode::HardLight,
                                                BlendMode::Difference,
                                                BlendMode::Exclusion,
                                                BlendMode::Divide,
                                                BlendMode::Subtract,
                                            ] {
                                                if ui.selectable_label(layer.blend_mode == bm, format!("{:?}", bm)).clicked() {
                                                    layer.blend_mode = bm;
                                                    project_changed = true;
                                                }
                                            }
                                        });

                                    // ── Track Matte Dropdown ──
                                    let tm_text = match layer.track_matte {
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
                                                if ui.selectable_label(layer.track_matte == mode, label).clicked() {
                                                    layer.track_matte = mode;
                                                    project_changed = true;
                                                }
                                            }
                                        });

                                    // ── Parenting Pick Whip @ & Dropdown ──
                                    let pw_btn = ui.selectable_label(false, "@").on_hover_text("Parenting Pick Whip: Click or Drag to link layer parent");
                                    if pw_btn.clicked() {
                                        app.toasts.info(format!("🌀 Drag Pickwhip from '{}' to target parent layer", layer.name));
                                    }
                                    let parent_text = layer.parent_id.as_deref().unwrap_or("None");
                                    egui::ComboBox::from_id_source(format!("tl_parent_{}", i))
                                        .selected_text(format!("Parent: {}", parent_text))
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(layer.parent_id.is_none(), "None").clicked() {
                                                layer.parent_id = None;
                                                project_changed = true;
                                            }
                                            for (p_idx, (p_id, p_name)) in parent_choices_ref.iter().enumerate() {
                                                if p_idx != i {
                                                    let is_p = layer.parent_id.as_deref() == Some(p_id);
                                                    if ui.selectable_label(is_p, p_name).clicked() {
                                                        // Cycle prevention check using parent_choices_ref
                                                        let is_cycle = parent_choices_ref.iter().any(|(p_id_check, _)| p_id_check == p_id && layer.parent_id.as_deref() == Some(&layer.id));

                                                        if is_cycle {
                                                            app.toasts.warning(format!("🚫 Cycle prevented! Cannot parent '{}' to '{}'", layer.name, p_name));
                                                        } else {
                                                            layer.parent_id = Some(p_id.clone());
                                                            project_changed = true;
                                                            app.toasts.info(format!("🌀 Parented '{}' ➔ '{}'", layer.name, p_name));
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                });
                            });

                            // Render Layer Bar Span & Waveform
                            let avail_w = ui.available_width();
                            let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, 24.0), egui::Sense::hover());
                            
                            let norm_in = (layer.in_frame.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                            let norm_out = (layer.out_frame.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                            
                            let layer_rect = egui::Rect::from_min_max(
                                egui::pos2(bar_rect.left() + norm_in * bar_rect.width(), bar_rect.top() + 3.0),
                                egui::pos2(bar_rect.left() + norm_out * bar_rect.width(), bar_rect.bottom() - 3.0),
                            );

                            let fill_c = if app.selected_layer_idx == Some(i) {
                                egui::Color32::from_rgb(0, 140, 240)
                            } else {
                                egui::Color32::from_rgb(50, 70, 100)
                            };

                            ui.painter().rect_filled(layer_rect, 2.0, fill_c);
                            ui.painter().rect_stroke(layer_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(160)));

                            if let LayerType::Audio { .. } = &layer.layer_type {
                                let samples = 12;
                                let step_x = layer_rect.width() / samples as f32;
                                for s in 0..samples {
                                    let h = ((s as f32 * 1.5).sin().abs() * 8.0).max(2.0);
                                    let sx = layer_rect.left() + s as f32 * step_x;
                                    let sy = layer_rect.center().y;
                                    ui.painter().line_segment(
                                        [egui::pos2(sx, sy - h), egui::pos2(sx, sy + h)],
                                        egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 220, 255)),
                                    );
                                }
                            }
                        });

                        // If expanded, render transform properties & effects
                        if app.expanded_layers.contains(&i) {
                            let pos_kfs = get_kfs(&layer.transform.position);
                            let scale_kfs = get_kfs(&layer.transform.scale);
                            let rot_kfs = get_kfs(&layer.transform.rotation);
                            let op_kfs = get_kfs(&layer.transform.opacity);

                            draw_prop_row(ui, "  ⏱ Position", &pos_kfs, current_frame, start_frame, zoom_span, left_pane_w);
                            draw_prop_row(ui, "  ⏱ Scale", &scale_kfs, current_frame, start_frame, zoom_span, left_pane_w);
                            draw_prop_row(ui, "  ⏱ Rotation", &rot_kfs, current_frame, start_frame, zoom_span, left_pane_w);
                            draw_prop_row(ui, "  ⏱ Opacity", &op_kfs, current_frame, start_frame, zoom_span, left_pane_w);

                            for effect in &layer.effects {
                                match &effect.effect_type {
                                    crate::core::timeline::EffectType::GaussianBlur { blur_radius, .. } => {
                                        draw_prop_row(ui, &format!("  [{}] Blur Radius", effect.name), &get_kfs(blur_radius), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::ColorTint { intensity, .. } => {
                                        draw_prop_row(ui, &format!("  [{}] Tint Intensity", effect.name), &get_kfs(intensity), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::DropShadow { distance, softness, .. } => {
                                        draw_prop_row(ui, &format!("  [{}] Distance", effect.name), &get_kfs(distance), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Softness", effect.name), &get_kfs(softness), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::HueSaturation { hue_shift, saturation, lightness } => {
                                        draw_prop_row(ui, &format!("  [{}] Hue Shift", effect.name), &get_kfs(hue_shift), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Saturation", effect.name), &get_kfs(saturation), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Lightness", effect.name), &get_kfs(lightness), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::Glow { threshold, radius, intensity, color } => {
                                        draw_prop_row(ui, &format!("  [{}] Threshold", effect.name), &get_kfs(threshold), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Radius", effect.name), &get_kfs(radius), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Intensity", effect.name), &get_kfs(intensity), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Color", effect.name), &get_kfs(color), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::MotionBlur { shutter_angle, .. } => {
                                        draw_prop_row(ui, &format!("  [{}] Shutter Angle", effect.name), &get_kfs(shutter_angle), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
                                        draw_prop_row(ui, &format!("  [{}] Top Left", effect.name), &get_kfs(top_left), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Top Right", effect.name), &get_kfs(top_right), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Bottom Left", effect.name), &get_kfs(bottom_left), current_frame, start_frame, zoom_span, left_pane_w);
                                        draw_prop_row(ui, &format!("  [{}] Bottom Right", effect.name), &get_kfs(bottom_right), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::ColorGradeLUT { intensity, .. } => {
                                        draw_prop_row(ui, &format!("  [{}] LUT Intensity", effect.name), &get_kfs(intensity), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    crate::core::timeline::EffectType::FilmGrain { intensity, .. } => {
                                        draw_prop_row(ui, &format!("  [{}] Grain Intensity", effect.name), &get_kfs(intensity), current_frame, start_frame, zoom_span, left_pane_w);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            });

            // ── AE Timeline Bottom Controls Bar (Toggle Switches / Modes F4) ──
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.selectable_label(app.show_switches_pane, "[◧] Switches").on_hover_text("Expand / Collapse Layer Switches Pane").clicked() {
                    app.show_switches_pane = true;
                }
                if ui.selectable_label(!app.show_switches_pane, "[⇆] Modes").on_hover_text("Expand / Collapse Transfer Controls Pane (Blend Modes & Track Mattes)").clicked() {
                    app.show_switches_pane = false;
                }
                if ui.button("Toggle Switches / Modes (F4)").on_hover_text("Toggle between Layer Switches and Transfer Modes (Shortcut: F4)").clicked() ||
                   ui.input(|i| i.key_pressed(egui::Key::F4)) {
                    app.show_switches_pane = !app.show_switches_pane;
                }
                ui.separator();
                ui.small(egui::RichText::new("AE Standard Timeline 1:1 Parity Mode").color(egui::Color32::from_gray(140)));
            });

            if let Some((a, b)) = swap_request {
                if a < temp_project.active_composition().layers.len() && b < temp_project.active_composition().layers.len() {
                    temp_project.active_composition_mut().layers.swap(a, b);
                    project_changed = true;
                }
            }

            if let Some(selected_indices) = pending_precomp_indices {
                let comp_len = temp_project.compositions.len();
                let (c_w, c_h, c_fps, c_dur) = {
                    let active = temp_project.active_composition();
                    (active.width, active.height, active.fps, active.duration_frames)
                };
                let precomp_id = format!("precomp_{}", comp_len);
                let precomp_name = format!("Pre-comp {}", comp_len + 1);
                let mut new_comp = crate::core::timeline::Composition::new(
                    precomp_id.clone(),
                    precomp_name.clone(),
                    c_w, c_h, c_fps, c_dur,
                );

                let active_comp = temp_project.active_composition_mut();
                let mut extracted_layers = Vec::new();
                for &idx in selected_indices.iter().rev() {
                    if idx < active_comp.layers.len() {
                        extracted_layers.push(active_comp.layers.remove(idx));
                    }
                }
                extracted_layers.reverse();
                new_comp.layers = extracted_layers;

                let precomp_layer = crate::core::timeline::Layer::new(
                    format!("layer_{}", precomp_id),
                    precomp_name,
                    crate::core::timeline::LayerType::PreComp { comp_id: precomp_id },
                    c_dur,
                );
                let insert_pos = selected_indices.first().copied().unwrap_or(0).min(active_comp.layers.len());
                active_comp.layers.insert(insert_pos, precomp_layer);
                temp_project.compositions.push(new_comp);

                app.selected_layers.clear();
                app.selected_layers.insert(insert_pos);
                app.selected_layer_idx = Some(insert_pos);
                project_changed = true;
            }

            // ── Cmd+D: Duplicate Selected Layer ──
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D)) {
                if let Some(sel_idx) = app.selected_layer_idx {
                    let layers_len = temp_project.active_composition().layers.len();
                    if sel_idx < layers_len {
                        let mut cloned = temp_project.active_composition().layers[sel_idx].clone();
                        cloned.id = format!("{}_copy", cloned.id);
                        cloned.name = format!("{} copy", cloned.name);
                        let insert_idx = sel_idx + 1;
                        temp_project.active_composition_mut().layers.insert(insert_idx, cloned);
                        app.selected_layer_idx = Some(insert_idx);
                        app.selected_layers.clear();
                        app.selected_layers.insert(insert_idx);
                        project_changed = true;
                        app.toasts.info("Duplicated layer (Cmd+D)");
                    }
                }
            }

            // ── Alt+[ : Trim Layer In Point to Current Frame ──
            if ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::OpenBracket)) {
                if let Some(sel_idx) = app.selected_layer_idx {
                    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(sel_idx) {
                        layer.in_frame = (*current_frame).min(layer.out_frame.saturating_sub(1));
                        project_changed = true;
                        app.toasts.info(format!("Trimmed In Point to frame {}", current_frame));
                    }
                }
            }

            // ── Alt+] : Trim Layer Out Point to Current Frame ──
            if ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::CloseBracket)) {
                if let Some(sel_idx) = app.selected_layer_idx {
                    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(sel_idx) {
                        layer.out_frame = (*current_frame).max(layer.in_frame + 1);
                        project_changed = true;
                        app.toasts.info(format!("Trimmed Out Point to frame {}", current_frame));
                    }
                }
            }

            if project_changed {
                // Transactional undo commit: snapshot on pointer-down, single entry on release
                let is_pointer_down = ui.input(|i| i.pointer.any_down());
                if is_pointer_down {
                    app.begin_drag("Timeline Edit");
                } else {
                    app.commit_drag();
                }
                crate::core::frame_cache::bump_version();
            }
        });
}
