pub mod utils;
pub mod header;
pub mod layers;
pub mod ruler_bar;
pub mod breadcrumb;
pub mod precomp_children;
pub mod pending_actions;
pub mod keyframe_rows;

use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{BlendMode, LabelColor, LayerType, TrackMatteMode};
use crate::ui::theme::colors;
use utils::maybe_snap_frame;
use header::draw_timeline_header;

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
            let _compact_mode = ui.ctx().data_mut(|d| {
                *d.get_temp_mut_or_insert_with(egui::Id::new("ae_compact_timeline"), || false)
            });
            let mut pending_precomp_indices: Option<Vec<usize>> = None;
            let mut swap_request: Option<(usize, usize)> = None;
            // Context-menu actions deferred to after the layer loop (borrow-safe)
            let mut pending_dup_layer: Option<usize> = None;
            let mut pending_split_layer: Option<usize> = None;
            let mut pending_layer_marker: Option<usize> = None;
            let mut pending_clear_markers: Option<usize> = None;
            let mut pending_select_all_kfs: Option<usize> = None;
            // Shift-trim ripple: (layer_idx, old_out, shift)
            let mut pending_ripple: Option<(usize, u32, i64)> = None;
            // Double-clicked PreComp layer: comp_id to open
            let mut pending_open_comp: Option<String> = None;
            // Ruler menu: new comp duration
            let mut pending_duration: Option<u32> = None;
            // Trim comp to work area: (w_in, w_out)
            let mut pending_trim_work_area: Option<(u32, u32)> = None;

            crate::ui::timeline::breadcrumb::draw_comp_breadcrumb(app, ui);

            let mut header_state = header::TimelineHeaderState {
                is_playing: &mut app.is_playing,
                timeline_zoom: &mut app.timeline_zoom,
                snap_to_keyframes: &mut app.snap_to_keyframes,
                show_graph_editor: &mut app.show_graph_editor,
                layer_filter_text: &mut app.layer_filter_text,
                timeline_view_start: &mut app.timeline_view_start,
                work_area_in: &mut app.work_area_in,
                work_area_out: &mut app.work_area_out,
                expanded_layers: &mut app.expanded_layers,
                fit_to_selection: &mut app.timeline_fit_to_selection,
                fit_all: &mut app.timeline_fit_all,
            };

            // Access live project mutably without per-frame cloning
            {
                let comp_mut = app.history.current_mut().active_composition_mut();
                if draw_timeline_header(&mut header_state, ui, comp_mut, current_frame, total_frames) {
                    project_changed = true;
                }

                // ── Fit to Selection: zoom timeline to selected layers' time range ──
                if app.timeline_fit_to_selection {
                    app.timeline_fit_to_selection = false;
                    let comp = app.history.current().active_composition();
                    let layers = if app.timeline_fit_all {
                        comp.layers.iter().enumerate().collect::<Vec<_>>()
                    } else {
                        let sel = &app.selected_layers;
                        comp.layers.iter().enumerate().filter(|(i, _)| sel.contains(i)).collect::<Vec<_>>()
                    };
                    if !layers.is_empty() {
                        let mut min_f = u32::MAX;
                        let mut max_f = 0u32;
                        for &(_, layer) in &layers {
                            min_f = min_f.min(layer.in_frame);
                            max_f = max_f.max(layer.out_frame);
                        }
                        if max_f > min_f {
                            let span = (max_f - min_f).max(10);
                            app.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                            app.timeline_view_start = min_f;
                        }
                    }
                    app.timeline_fit_all = false;
                }
            }


            ui.add_space(4.0);

            crate::ui::timeline::ruler_bar::draw_ram_ruler(app, ui, current_frame, total_frames);

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


            // ── Pre-collect sub-comp layer data (before mutable borrow) ──
            let precomp_children = crate::ui::timeline::precomp_children::collect(temp_project, &app.expanded_layers);

            // ── Tracks Mode: Layer List & Keyframe Ruler Area ──
            let comp = temp_project.active_composition_mut();

            // ── Responsive Width Calculation ──
            let total_w = ui.available_width();
            let left_pane_w = (total_w * 0.42).clamp(340.0, 640.0);

            // ── Work Area (In / Out) Handles Bar ──
            ui.horizontal(|ui| {
                ui.allocate_ui(egui::vec2(left_pane_w, 18.0), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Source Name | Mode | TrkMat | Parent & Link | Switches").small().strong().color(colors::TEXT_SECONDARY));

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
                    egui::vec2(avail_w, 26.0),
                    egui::Sense::click_and_drag(),
                );

                // Ruler scrub cursor feedback
                if ruler_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }

                ui.painter().rect_filled(ruler_rect, 0.0, colors::BG_DARK);

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
                let mut wa_drag_active = false;
                if wa_out > wa_in && wa_out >= start_frame && wa_in <= start_frame + zoom_span {
                    let norm_in = (wa_in.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                    let norm_out = (wa_out.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                    let wa_rect = egui::Rect::from_min_max(
                        egui::pos2(ruler_rect.left() + norm_in * ruler_rect.width(), ruler_rect.top() + 2.0),
                        egui::pos2(ruler_rect.left() + norm_out * ruler_rect.width(), ruler_rect.bottom() - 2.0),
                    );
                    ui.painter().rect_filled(wa_rect, 2.0, colors::TIMELINE_SELECTION);
                    ui.painter().rect_stroke(wa_rect, 2.0, egui::Stroke::new(1.0, colors::BORDER_ACTIVE));

                    // ── Work-area edge handles: drag to set In/Out points ──
                    const WA_HW: f32 = 5.0;
                    let wa_in_handle = egui::Rect::from_min_size(
                        egui::pos2(wa_rect.left() - WA_HW * 0.5, ruler_rect.top()),
                        egui::vec2(WA_HW, ruler_rect.height()),
                    );
                    let wa_out_handle = egui::Rect::from_min_size(
                        egui::pos2(wa_rect.right() - WA_HW * 0.5, ruler_rect.top()),
                        egui::vec2(WA_HW, ruler_rect.height()),
                    );
                    let wa_in_resp = ui.interact(wa_in_handle, egui::Id::new("wa_in_handle"), egui::Sense::drag());
                    let wa_out_resp = ui.interact(wa_out_handle, egui::Id::new("wa_out_handle"), egui::Sense::drag());
                    let wa_frame_from_pointer = |resp: &egui::Response| -> Option<u32> {
                        resp.interact_pointer_pos().map(|p| {
                            let norm = ((p.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                            start_frame.saturating_add((norm * zoom_span as f32).round() as u32)
                        })
                    };
                    if wa_in_resp.dragged() {
                        wa_drag_active = true;
                        if let Some(f) = wa_frame_from_pointer(&wa_in_resp) {
                            app.work_area_in = Some(f.min(app.work_area_out.unwrap_or(total_frames).saturating_sub(1)));
                            crate::core::frame_cache::bump_version();
                        }
                    }
                    if wa_out_resp.dragged() {
                        wa_drag_active = true;
                        if let Some(f) = wa_frame_from_pointer(&wa_out_resp) {
                            app.work_area_out = Some(f.max(app.work_area_in.unwrap_or(0) + 1));
                            crate::core::frame_cache::bump_version();
                        }
                    }
                    if wa_in_resp.hovered() || wa_out_resp.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                    }
                }

                // Ruler Ticks
                let step = (zoom_span / 10).max(1);
                let fps = comp.fps.max(1);
                // Thin out timecode labels when ticks would overlap (11 mono chars ≈ 66px)
                let tick_px = ruler_rect.width() * (step as f32 / zoom_span as f32);
                let label_every = ((70.0 / tick_px.max(1.0)).ceil() as usize).max(1);
                for (tick_i, f) in (start_frame..=(start_frame + zoom_span)).step_by(step as usize).enumerate() {
                    let norm = (f - start_frame) as f32 / zoom_span as f32;
                    let tick_x = ruler_rect.left() + norm * ruler_rect.width();
                    ui.painter().line_segment(
                        [egui::pos2(tick_x, ruler_rect.bottom() - 6.0), egui::pos2(tick_x, ruler_rect.bottom())],
                        egui::Stroke::new(1.0, colors::BORDER_STRONG),
                    );
                    if tick_px < 70.0 && tick_i % label_every != 0 {
                        continue;
                    }
                    let tc_s = f / fps;
                    let tc_sub = f % fps;
                    let tc_m = tc_s / 60;
                    let tc_h = tc_m / 60;
                    let tc_str = format!("{:02}:{:02}:{:02}:{:02}", tc_h, tc_m % 60, tc_s % 60, tc_sub);
                    ui.painter().text(
                        egui::pos2(tick_x, ruler_rect.top() + 2.0),
                        egui::Align2::CENTER_TOP,
                        &tc_str,
                        egui::FontId::monospace(9.0),
                        colors::TEXT_SECONDARY,
                    );
                }

                // Current Playhead Line (Blue indicator)
                let playhead_norm = (*current_frame as f32 - start_frame as f32) / zoom_span as f32;
                let playhead_x = ruler_rect.left() + playhead_norm * ruler_rect.width();
                if (0.0..=1.0).contains(&playhead_norm) {
                    ui.painter().line_segment(
                        [egui::pos2(playhead_x, ruler_rect.top() + 4.0), egui::pos2(playhead_x, ruler_rect.bottom() + 300.0)],
                        egui::Stroke::new(1.5, colors::TIMELINE_PLAYHEAD),
                    );
                    // Inverted triangle handle (AE-style grab point)
                    let tri = vec![
                        egui::pos2(playhead_x - 5.0, ruler_rect.top()),
                        egui::pos2(playhead_x + 5.0, ruler_rect.top()),
                        egui::pos2(playhead_x, ruler_rect.top() + 4.0),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(tri, colors::TIMELINE_PLAYHEAD, egui::Stroke::NONE));
                }

                // ── Ctrl/Cmd+drag on ruler: rubber-band Work Area definition ──
                let cmd_mod = ui.input(|i| i.modifiers.command);
                if !wa_drag_active && cmd_mod && ruler_response.dragged() {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        let drag_id = egui::Id::new("wa_rubber_start");
                        let anchor_f = ui.ctx().data_mut(|d| {
                            let cur = d.get_temp::<u32>(drag_id);
                            if cur.is_none() { d.insert_temp(drag_id, raw_f); }
                            cur
                        });
                        if let Some(s) = anchor_f {
                            app.work_area_in = Some(s.min(raw_f));
                            app.work_area_out = Some(s.max(raw_f).saturating_add(1));
                        }
                    }
                }
                if ruler_response.drag_stopped() {
                    ui.ctx().data_mut(|d| d.remove_temp::<u32>(egui::Id::new("wa_rubber_start")));
                }

                if ruler_response.double_clicked() && !cmd_mod {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        comp.markers.push(crate::core::timeline::TimelineMarker {
                            frame: raw_f,
                            label: format!("Marker {}", comp.markers.len() + 1),
                            color: [0.35, 0.75, 1.0],
                        });
                        project_changed = true;
                        app.toasts.info(format!("Comp marker added at frame {}", raw_f));
                    }
                }

                if !wa_drag_active && !cmd_mod && (ruler_response.clicked() || ruler_response.dragged()) {
                    if let Some(pos) = ruler_response.interact_pointer_pos() {
                        let norm = ((pos.x - ruler_rect.left()) / ruler_rect.width()).clamp(0.0, 1.0);
                        let raw_f = start_frame + (norm * zoom_span as f32).round() as u32;
                        *current_frame = maybe_snap_frame(raw_f, app.snap_to_keyframes, comp);
                    }
                }

                // ── Ruler context menu (AE parity) ──
                ruler_response.context_menu(|ui| {
                    if ui.button("📍 Add Composition Marker at Playhead").clicked() {
                        comp.markers.push(crate::core::timeline::TimelineMarker {
                            frame: *current_frame,
                            label: format!("Marker {}", comp.markers.len() + 1),
                            color: [0.35, 0.75, 1.0],
                        });
                        project_changed = true;
                        app.toasts.info(format!("Comp marker at frame {}", current_frame));
                        ui.close_menu();
                    }
                    if !comp.markers.is_empty() && ui.button("🧹 Clear All Composition Markers").clicked() {
                        comp.markers.clear();
                        project_changed = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🔍 Zoom to Work Area").clicked() {
                        let w_in = app.work_area_in.unwrap_or(0);
                        let w_out = app.work_area_out.unwrap_or(total_frames).max(w_in + 1);
                        let span = (w_out - w_in).max(10);
                        app.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                        app.timeline_view_start = w_in;
                        ui.close_menu();
                    }
                    if ui.button("⏱ Set Comp Duration to Work Area End").clicked() {
                        let w_out = app.work_area_out.unwrap_or(total_frames);
                        if w_out > 0 && w_out <= total_frames {
                            pending_duration = Some(w_out);
                            app.toasts.info(format!("Comp duration set to {} frames", w_out));
                        }
                        ui.close_menu();
                    }
                    if ui.button("✂ Trim Comp to Work Area").on_hover_text("Set comp duration to work area and trim layers beyond it").clicked() {
                        let w_in = app.work_area_in.unwrap_or(0);
                        let w_out = app.work_area_out.unwrap_or(total_frames);
                        if w_out > w_in {
                            pending_duration = Some(w_out);
                            pending_trim_work_area = Some((w_in, w_out));
                            app.toasts.info(format!("Trimmed to work area: {}f–{}f", w_in, w_out));
                        }
                        ui.close_menu();
                    }
                    if ui.button("↺ Reset Work Area to Full Comp").clicked() {
                        app.work_area_in = None;
                        app.work_area_out = None;
                        ui.close_menu();
                    }
                });

                // ⌨️ B / N Keyboard Shortcuts for Work Area In / Out
                if ui.input(|i| i.key_pressed(egui::Key::B)) {
                    app.work_area_in = Some(*current_frame);
                    app.toasts.info(format!("Set Work Area Start at frame {}", current_frame));
                }
                if ui.input(|i| i.key_pressed(egui::Key::N)) {
                    app.work_area_out = Some(*current_frame);
                    app.toasts.info(format!("Set Work Area End at frame {}", current_frame));
                }
                // 🔍 Timeline Zoom Shortcuts (= / -), anchored on the playhead so the
                // frame under it stays visually in place while zooming.
                {
                    // Returns the new window start that keeps the playhead stationary.
                    let view_start = app.timeline_view_start;
                    let anchored_start = |old_zoom: f32, new_zoom: f32| -> u32 {
                        let old_span = (total_frames as f32 / old_zoom.max(0.01)).max(10.0) as u32;
                        let new_span = (total_frames as f32 / new_zoom.max(0.01)).max(10.0) as u32;
                        // The playhead's offset from the window's left edge scales with
                        // the span ratio, keeping the playhead stationary on screen.
                        let head_offset = current_frame.saturating_sub(view_start);
                        let new_head_offset = ((head_offset as f32) * (new_span as f32 / old_span.max(1) as f32)) as u32;
                        current_frame
                            .saturating_sub(new_head_offset)
                            .min(total_frames.saturating_sub(new_span))
                    };
                    if ui.input(|i| i.key_pressed(egui::Key::Equals)) && app.timeline_zoom < 20.0 {
                        let new_zoom = (app.timeline_zoom * 1.25).min(20.0);
                        app.timeline_view_start = anchored_start(app.timeline_zoom, new_zoom);
                        app.timeline_zoom = new_zoom;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Minus)) && app.timeline_zoom > 0.1 {
                        let new_zoom = (app.timeline_zoom / 1.25).max(0.1);
                        app.timeline_view_start = anchored_start(app.timeline_zoom, new_zoom);
                        app.timeline_zoom = new_zoom;
                    }
                    // Scroll wheel over ruler: zoom anchored at pointer (NLE standard)
                    if ruler_response.hovered() && ui.input(|i| i.raw_scroll_delta.y != 0.0) && !ui.input(|i| i.modifiers.command) {
                        let scroll = ui.input(|i| i.raw_scroll_delta.y);
                        let factor = if scroll > 0.0 { 1.0 / 1.15 } else { 1.15 };
                        let new_zoom = (app.timeline_zoom * factor).clamp(0.1, 20.0);
                        if new_zoom != app.timeline_zoom {
                            app.timeline_view_start = anchored_start(app.timeline_zoom, new_zoom);
                            app.timeline_zoom = new_zoom;
                        }
                    }
                    // Cmd/Ctrl + scroll: horizontal pan
                    else if ruler_response.hovered() && ui.input(|i| i.modifiers.command) {
                        let scroll = ui.input(|i| i.raw_scroll_delta.y);
                        if scroll != 0.0 {
                            let new_zoom = (app.timeline_zoom * (if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 })).clamp(0.1, 20.0);
                            if new_zoom != app.timeline_zoom {
                                app.timeline_view_start = anchored_start(app.timeline_zoom, new_zoom);
                                app.timeline_zoom = new_zoom;
                            }
                        }
                    }
                }
            });

            ui.separator();

            // ── Scrollable Layer Rows & Property Tracks ──
            let zoom_span = (total_frames as f32 / app.timeline_zoom.max(0.01)).max(10.0) as u32;
            let start_frame = current_frame.saturating_sub(zoom_span / 2).min(total_frames.saturating_sub(zoom_span));
            // Capture effect drag info before closure to avoid borrow conflicts
            let drag_info = app.dragging_effect.clone();
            // Collect pending effect drops to apply after the closure
            let mut pending_effect_drops: Vec<(usize, String, usize)> = Vec::new();

            let _scroll_resp = egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                let layers_len = comp.layers.len();
                let comp_w_f = comp.width as f32;
                let parent_choices: Vec<(String, String)> = comp.layers.iter().map(|l| (l.id.clone(), l.name.clone())).collect();
                let parent_choices_ref = &parent_choices;
                let layer_edges: Vec<(u32, u32)> = comp.layers.iter().map(|l| (l.in_frame, l.out_frame)).collect();
                let all_kf_frames: Vec<u32> = crate::ui::timeline::utils::collect_all_kf_frames(comp);
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
                                    ui.label(egui::RichText::new(format!("{:02}", i + 1)).small().strong().color(colors::TEXT_SECONDARY));
                                    ui.add_space(2.0);

                                    // ── Layer type icon ──
                                    {
let type_icon = crate::ui::icons::layer_icon(&layer.layer_type);
                                        ui.label(egui::RichText::new(type_icon).small().color(colors::TEXT_SECONDARY));
                                    }
                                    ui.add_space(1.0);

                                    // ── Label color chip: click cycles through AE label colors ──
                                    {
                                        let rgb = layer.label.to_rgb();
                                        let chip_color = egui::Color32::from_rgb(
                                            (rgb[0] * 255.0) as u8,
                                            (rgb[1] * 255.0) as u8,
                                            (rgb[2] * 255.0) as u8,
                                        );
                                        let (chip_rect, chip_resp) = ui.allocate_exact_size(
                                            egui::vec2(10.0, 14.0),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(chip_rect, 2.0, chip_color);
                                        let row_selected = app.selected_layers.contains(&i) || app.selected_layer_idx == Some(i);
                                        if row_selected {
                                            ui.painter().rect_stroke(chip_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
                                        }
                                        if chip_resp.clicked() {
                                            let next = match layer.label {
                                                crate::core::timeline::LabelColor::None => crate::core::timeline::LabelColor::Red,
                                                crate::core::timeline::LabelColor::Red => crate::core::timeline::LabelColor::Yellow,
                                                crate::core::timeline::LabelColor::Yellow => crate::core::timeline::LabelColor::Aqua,
                                                crate::core::timeline::LabelColor::Aqua => crate::core::timeline::LabelColor::Pink,
                                                crate::core::timeline::LabelColor::Pink => crate::core::timeline::LabelColor::Lavender,
                                                crate::core::timeline::LabelColor::Lavender => crate::core::timeline::LabelColor::Peach,
                                                crate::core::timeline::LabelColor::Peach => crate::core::timeline::LabelColor::Sea,
                                                crate::core::timeline::LabelColor::Sea => crate::core::timeline::LabelColor::Blue,
                                                crate::core::timeline::LabelColor::Blue => crate::core::timeline::LabelColor::Purple,
                                                crate::core::timeline::LabelColor::Purple => crate::core::timeline::LabelColor::None,
                                            };
                                            // Direct mutation: layer is already &mut from the row loop
                                            layer.label = next;
                                            project_changed = true;
                                        }
                                        if chip_resp.hovered() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                    }
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
                // ── Click empty area below layers → deselect all ──
                let remaining = ui.available_rect_before_wrap();
                let bg_resp = ui.interact(remaining, egui::Id::new("timeline_bg_deselect"), egui::Sense::click());
                if bg_resp.clicked() {
                    app.selected_layers.clear();
                    app.selected_layer_idx = None;
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
                                    egui::ComboBox::from_id_salt(matte_id)
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
                                    egui::ComboBox::from_id_salt(blend_id)
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
                                    if click_resp.double_clicked() {
                                        // Double-click a PreComp layer opens its nested composition (AE parity)
                                        // (deferred: resolved after the row loop)
                                        if let crate::core::timeline::LayerType::PreComp { comp_id } = &layer.layer_type {
                                            pending_open_comp = Some(comp_id.clone());
                                        } else {
                                            app.renaming_layer = Some(i);
                                        }
                                    }
                                    if app.renaming_layer == Some(i) {
                                        let mut name_buf = layer.name.clone();
                                        let rename_resp = ui.text_edit_singleline(&mut name_buf);
                                        if rename_resp.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                            if !name_buf.is_empty() && name_buf != layer.name {
                                                layer.name = name_buf;
                                                project_changed = true;
                                            }
                                            app.renaming_layer = None;
                                        }
                                    }

                                    // ── Drag & drop reordering (live, AE-style) ──
                                    if click_resp.dragged() && app.dragging_layer.is_none() {
                                        app.dragging_layer = Some(i);
                                    }
                                    if let Some(src) = app.dragging_layer {
                                        if click_resp.hovered() && src != i {
                                            // Live swap; the dragged row follows the pointer
                                            swap_request = Some((src, i));
                                            app.dragging_layer = Some(i);
                                        }
                                    }
                                    if click_resp.drag_stopped() || (app.dragging_layer.is_some() && !click_resp.dragged()) {
                                        // Drag ended — clear state (row index moved with the pointer)
                                        app.dragging_layer = None;
                                    }

                                    // ── Effect drop zone: drag from Effects & Presets library ──
                                    if let Some((ref effect_name, preset_idx)) = drag_info {
                                        if click_resp.hovered() {
                                            // Visual drop indicator: blue glow on the layer row
                                            let row_rect = click_resp.rect;
                                             ui.painter().rect_stroke(
                                                 row_rect, 4.0,
                                                 egui::Stroke::new(2.0, colors::ACCENT_BLUE)
                                             );
                                             ui.painter().rect_filled(
                                                 row_rect, 4.0,
                                                 colors::TIMELINE_SELECTION
                                             );
                                        }
                                        // Collect effect on drop (apply after loop to avoid borrow conflicts)
                                        if click_resp.drag_stopped() && click_resp.hovered() {
                                            pending_effect_drops.push((i, effect_name.clone(), preset_idx));
                                            app.dragging_effect = None;
                                        }
                                    }

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
                                            pending_dup_layer = Some(i);
                                            app.toasts.info("Duplicated selected layer");
                                            ui.close_menu();
                                        }
                                        if ui.button("✂ Split Layer at Current Time (Cmd+Shift+D)").clicked() {
                                            pending_split_layer = Some(i);
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Into ← Set In-Point to Current Time (I)").clicked() {
                                            layer.in_frame = *current_frame;
                                            if layer.in_frame >= layer.out_frame {
                                                layer.out_frame = layer.in_frame + 1;
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("→ Set Out-Point to Current Time (O)").clicked() {
                                            layer.out_frame = *current_frame;
                                            if layer.out_frame <= layer.in_frame {
                                                layer.in_frame = layer.out_frame.saturating_sub(1);
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("📍 Add Layer Marker at Playhead").clicked() {
                                            pending_layer_marker = Some(i);
                                            ui.close_menu();
                                        }
                                        if !layer.markers.is_empty()
                                            && ui.button("🧹 Clear Layer Markers").clicked() {
                                            pending_clear_markers = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("⬚ Select All Keyframes").on_hover_text("Select every keyframe on this layer (transform, effects, pins)").clicked() {
                                            pending_select_all_kfs = Some(i);
                                            ui.close_menu();
                                        }
                                        // ── ✨ Animation Presets ──
                                        ui.menu_button("✨ Animation Presets", |ui| {
                                            let cf = *current_frame;
                                            for name in crate::core::presets::NAMES {
                                                if ui.small_button(*name).clicked() {
                                                    let ok = crate::core::presets::apply_by_name(name, layer, cf, comp_w_f, 0.0);
                                                    if ok {
                                                        project_changed = true;
                                                        app.toasts.info(format!("{} applied", name));
                                                    } else {
                                                        app.toasts.error(format!("{}: not applicable here", name));
                                                    }
                                                    ui.close_menu();
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                        if !layer.paint_strokes.is_empty()
                                            && ui.button("🧹 Clear All Paint Strokes").clicked() {
                                            layer.paint_strokes.clear();
                                            project_changed = true;
                                            app.toasts.info("All paint strokes cleared");
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
                                        // ── Audio fades (Audio layers only) ──
                                        if matches!(layer.layer_type, crate::core::timeline::LayerType::Audio { .. }) {
                                            ui.separator();
                                            const FADE_F: u32 = 10;
                                            if ui.button("🔉 Add 10-frame Audio Fade In").clicked() {
                                                if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                                                    let base = volume.evaluate(*current_frame);
                                                    let start_f = layer.in_frame;
                                                    let end_f = (start_f + FADE_F).min(layer.out_frame);
                                                    let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                                                        crate::core::keyframe::Keyframe::new(start_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                                        crate::core::keyframe::Keyframe::new(end_f.max(start_f + 1), base, crate::core::keyframe::InterpolationType::Linear),
                                                    ];
                                                    if let Some(old) = volume.keyframes() {
                                                        for k in old { if k.frame > end_f { kfs.push(k.clone()); } }
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *volume = crate::core::property::Animatable::Animated(kfs);
                                                }
                                                project_changed = true;
                                                app.toasts.info("Audio fade-in added");
                                                ui.close_menu();
                                            }
                                            if ui.button("🔉 Add 10-frame Audio Fade Out").clicked() {
                                                if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                                                    let base = volume.evaluate(*current_frame);
                                                    let end_f = layer.out_frame.saturating_sub(1);
                                                    let start_f = end_f.saturating_sub(FADE_F).max(layer.in_frame);
                                                    let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                                                        crate::core::keyframe::Keyframe::new(end_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                                        crate::core::keyframe::Keyframe::new(start_f, base, crate::core::keyframe::InterpolationType::Linear),
                                                    ];
                                                    if let Some(old) = volume.keyframes() {
                                                        for k in old { if k.frame < start_f { kfs.push(k.clone()); } }
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *volume = crate::core::property::Animatable::Animated(kfs);
                                                }
                                                project_changed = true;
                                                app.toasts.info("Audio fade-out added");
                                                ui.close_menu();
                                            }
                                        }

                                        ui.separator();
                                        let any_disabled = layer.effects.iter().any(|e| !e.enabled);
                                        let fx_label = if any_disabled { "✅ Enable All Effects" } else { "🚫 Disable All Effects" };
                                        if ui.button(fx_label).clicked() {
                                            let target = any_disabled;
                                            for fx in layer.effects.iter_mut() {
                                                fx.enabled = target;
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                    });
                                    ui.style_mut().visuals.override_text_color = None;

                                    // ── Blend Mode Dropdown ──
                                    let bm_text = format!("{:?}", layer.blend_mode);
                                    egui::ComboBox::from_id_salt(format!("tl_blend_{}", i))
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
                                    egui::ComboBox::from_id_salt(format!("tl_matte_{}", i))
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
                                    egui::ComboBox::from_id_salt(format!("tl_parent_{}", i))
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

                            // Render Layer Bar Span & Waveform + parent link line
                            let avail_w = ui.available_width();
                            let (bar_rect, _bar_sense) = ui.allocate_exact_size(
                                egui::vec2(avail_w, 24.0),
                                egui::Sense::hover().union(egui::Sense::drag()),
                            );
                            // Draw parent connection if this layer has a parent
                            if let Some(pid) = &layer.parent_id {
                                if parent_choices_ref.iter().any(|(lid, _)| lid == pid) {
                                    // Small chain icon on the left of the bar
                                     ui.painter().text(
                                         egui::pos2(bar_rect.left() + 8.0, bar_rect.top() + 4.0),
                                         egui::Align2::LEFT_TOP,
                                         "🔗",
                                         egui::FontId::proportional(10.0),
                                         colors::ACCENT_PURPLE,
                                     );
                                }
                            }
                            
                            let norm_in = (layer.in_frame.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                            let norm_out = (layer.out_frame.saturating_sub(start_frame)) as f32 / zoom_span as f32;
                            
                            let layer_rect = egui::Rect::from_min_max(
                                egui::pos2(bar_rect.left() + norm_in * bar_rect.width(), bar_rect.top() + 3.0),
                                egui::pos2(bar_rect.left() + norm_out * bar_rect.width(), bar_rect.bottom() - 3.0),
                            );

                            let fill_c = if app.selected_layer_idx == Some(i) {
                                let [r, g, b] = layer.label.to_rgb();
                                egui::Color32::from_rgb(
                                    (r * 255.0) as u8,
                                    (g * 255.0) as u8,
                                    (b * 255.0) as u8,
                                )
                            } else {
                                let [r, g, b] = layer.label.to_rgb();
                                egui::Color32::from_rgb(
                                    (r * 120.0) as u8,
                                    (g * 120.0) as u8,
                                    (b * 120.0) as u8,
                                )
                            };

                            ui.painter().rect_filled(layer_rect, 2.0, fill_c);
                            ui.painter().rect_stroke(layer_rect, 2.0, egui::Stroke::new(1.0, colors::TEXT_SECONDARY));

                            // ── Audio waveform overlay inside the bar ──
                            let audio_path: Option<String> = match &layer.layer_type {
                                crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
                                crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } => Some(w.clone()),
                                _ => None,
                            };
                            if let Some(apath) = audio_path {
                                let key = egui::Id::new(("tl_waveform", apath.as_str()));
                                let cached: std::sync::Arc<(Vec<f32>, f32)> = ui.ctx().data_mut(|d| {
                                    d.get_temp::<std::sync::Arc<(Vec<f32>, f32)>>(key).unwrap_or_else(|| {
                                        let built = crate::core::audio_engine::AudioBuffer::load_wav(std::path::Path::new(&apath))
                                            .map(|b| {
                                                let dur = b.samples.len() as f32
                                                    / (b.sample_rate.max(1) as f32 * b.channels.max(1) as f32);
                                                (b.waveform_peaks(600), dur)
                                            })
                                            .unwrap_or((Vec::new(), 0.0));
                                        let arc = std::sync::Arc::new(built);
                                        d.insert_temp(key, arc.clone());
                                        arc
                                    })
                                });
                                let (peaks, dur_sec) = (&cached.0, cached.1);
                                if peaks.len() > 1 && dur_sec > 0.0 && layer_rect.width() > 8.0 {
                                    let fps_c = comp.fps.max(1) as f32;
                                    let mid_y = layer_rect.center().y;
                                    let amp = layer_rect.height() * 0.40;
                                    let wf_color = colors::TIMELINE_WAVEFORM.linear_multiply(0.9);
                                    let mut x = layer_rect.left() + 2.0;
                                    while x <= layer_rect.right() - 2.0 {
                                        let t = (x - layer_rect.left()) / layer_rect.width();
                                        let frame_f = layer.in_frame as f32 + t * (layer.out_frame - layer.in_frame) as f32;
                                        let sec = frame_f / fps_c;
                                        if sec < dur_sec {
                                            let bin = ((sec / dur_sec) * peaks.len() as f32) as usize;
                                            let pk = peaks.get(bin.min(peaks.len() - 1)).copied().unwrap_or(0.0);
                                            let h = (pk * amp).clamp(1.0, amp);
                                            ui.painter().line_segment(
                                                [egui::pos2(x, mid_y - h), egui::pos2(x, mid_y + h)],
                                                egui::Stroke::new(1.0, wf_color),
                                            );
                                        }
                                        x += 2.0;
                                    }
                                }
                            }

                            // ── Trim handles: drag bar edges to set in/out points ──
                            fn handle_rect_of(is_in: bool, lr: &egui::Rect) -> egui::Rect {
                                const HW: f32 = 6.0;
                                if is_in {
                                    egui::Rect::from_min_size(egui::pos2(lr.left() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
                                } else {
                                    egui::Rect::from_min_size(egui::pos2(lr.right() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
                                }
                            }
                            const HANDLE_W: f32 = 6.0;
                            let in_handle = egui::Rect::from_min_size(
                                egui::pos2(layer_rect.left() - HANDLE_W * 0.5, layer_rect.top()),
                                egui::vec2(HANDLE_W, layer_rect.height()),
                            );
                            let out_handle = egui::Rect::from_min_size(
                                egui::pos2(layer_rect.right() - HANDLE_W * 0.5, layer_rect.top()),
                                egui::vec2(HANDLE_W, layer_rect.height()),
                            );
                            let in_resp = ui.interact(in_handle, egui::Id::new(("trim_in", i)), egui::Sense::click_and_drag());
                            let out_resp = ui.interact(out_handle, egui::Id::new(("trim_out", i)), egui::Sense::click_and_drag());

                            // ── Body drag: slide the whole layer (in/out preserved) ──
                            {
                                const HEADER_W: f32 = 6.0;
                                let body_rect = egui::Rect::from_min_max(
                                    egui::pos2(layer_rect.left() + HEADER_W, layer_rect.top()),
                                    egui::pos2(layer_rect.right() - HEADER_W, layer_rect.bottom()),
                                );
                                let body_resp = ui.interact(
                                    body_rect,
                                    egui::Id::new(("layer_slide", i, layer.id.as_str())),
                                    egui::Sense::drag(),
                                );
                                if body_resp.dragged() {
                                    let delta_frames =
                                        (body_resp.drag_delta().x / bar_rect.width() * zoom_span as f32).round() as i32;
                                    // ── Vertical drag → reorder layers in stack ──
                                    let row_h = 26.0f32;
                                    let y_delta_key = egui::Id::new(("body_y_accum", i, layer.id.as_str()));
                                    let y_accum: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(y_delta_key, || 0.0f32));
                                    let new_accum = y_accum + body_resp.drag_delta().y;
                                    let row_offset = (new_accum / row_h).trunc() as i32;
                                    if row_offset != 0 {
                                        let target = (i as i32 - row_offset).clamp(0, layers_len as i32 - 1) as usize;
                                        if target != i {
                                            swap_request = Some((i, target));
                                        }
                                        ui.ctx().data_mut(|d| d.insert_temp(y_delta_key, new_accum - row_offset as f32 * row_h));
                                    } else {
                                        ui.ctx().data_mut(|d| d.insert_temp(y_delta_key, new_accum));
                                    }
                                    if delta_frames != 0 {
                                        let alt_held = ui.input(|inp| inp.modifiers.alt);
                                        if alt_held {
                                            // ── Slip edit: bar stays, content timing shifts ──
                                            // Keyframes & markers move opposite the drag so a
                                            // different moment of the source plays at the same time.
                                            let shift = -delta_frames;
                                            for kf in layer.transform.position.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.position.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.scale.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.scale.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.rotation.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.rotation.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.opacity.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.opacity.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for m in layer.markers.iter_mut() {
                                                m.frame = (m.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            // Slip effect keyframes too
                                            use crate::core::effect_params::ParamRef;
                                            for effect in layer.effects.iter_mut() {
                                                for (_, param) in effect.effect_type.animatable_params() {
                                                    match param {
                                                        ParamRef::Scalar(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec2(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec4Color(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            // Slide: move the whole bar
                                            let span = layer.out_frame - layer.in_frame;
                                            let raw_in = (layer.in_frame as i32 + delta_frames)
                                                .clamp(0, (total_frames - span) as i32)
                                                as u32;
                                            let new_in = if app.snap_to_keyframes {
                                                let threshold = 5i32;
                                                let mut best = raw_in;
                                                let mut best_dist = threshold + 1;
                                                for (j, &(e_in, e_out)) in layer_edges.iter().enumerate() {
                                                    if j == i { continue; }
                                                    for edge in [e_in, e_out] {
                                                        let dist = (raw_in as i32 - edge as i32).abs();
                                                        if dist < best_dist {
                                                            best_dist = dist;
                                                            best = edge;
                                                        }
                                                    }
                                                }
                                                best
                                            } else {
                                                raw_in
                                            };
                                            layer.in_frame = new_in;
                                            layer.out_frame = new_in + span;
                                        }
                                        project_changed = true;
                                    }
                                }
                                if body_resp.hovered() && !in_resp.hovered() && !out_resp.hovered() {
                                    let alt_held = ui.input(|inp| inp.modifiers.alt);
                                    ui.ctx().set_cursor_icon(if alt_held { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::Grab });
                                    body_resp.on_hover_text(format!(
                                        "{} — In: {} Out: {} ({:.2}s)",
                                        layer.name,
                                        layer.in_frame,
                                        layer.out_frame,
                                        layer.duration_frames() as f32 / comp.fps.max(1) as f32
                                    ));
                                }
                            }

                            for (resp, is_in) in [(&in_resp, true), (&out_resp, false)] {
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                    ui.painter().rect_filled(handle_rect_of(is_in, &layer_rect), 2.0, colors::HANDLE_HOVER_FILL.linear_multiply(180.0 / 255.0));
                                }
                                if resp.dragged() {
                                    if let Some(pos) = resp.interact_pointer_pos() {
                                        let norm = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                                        let frame = (start_frame as f32 + norm * zoom_span as f32).round() as u32;
                                        let frame = frame.min(total_frames);
                                        if is_in {
                                            layer.in_frame = frame.min(layer.out_frame.saturating_sub(1));
                                        } else {
                                            let old_out = layer.out_frame;
                                            layer.out_frame = frame.max(layer.in_frame + 1);
                                            // ── Shift+drag Out = ripple edit: close gaps for layers below ──
                                            if ui.input(|inp| inp.modifiers.shift) && layer.out_frame != old_out {
                                                let shift = layer.out_frame as i64 - old_out as i64;
                                                pending_ripple = Some((i, old_out, shift));
                                            }
                                        }
                                        project_changed = true;
                                    }
                                }
                            }

                            // ── Layer markers: small triangles on the bar (AE parity) ──
                            for marker in &layer.markers {
                                if marker.frame < start_frame || marker.frame > start_frame + zoom_span { continue; }
                                let norm = (marker.frame - start_frame) as f32 / zoom_span as f32;
                                let mx = layer_rect.left() + norm * layer_rect.width();
                                let my = layer_rect.top() + 1.0;
                                let [r, g, b] = marker.color;
                                let mcol = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                                let pts = vec![
                                    egui::pos2(mx, my + 5.0),
                                    egui::pos2(mx - 4.0, my),
                                    egui::pos2(mx + 4.0, my),
                                ];
                                ui.painter().add(egui::Shape::convex_polygon(pts, mcol, egui::Stroke::NONE));
                            }

                            // ── Real waveform for video layers with audio ──
                            if let LayerType::Video { audio_wav: Some(wav_path), .. } = &layer.layer_type {
                                let wav_id = egui::Id::new(("wav_peaks", layer.id.as_str()));
                                let peaks: std::sync::Arc<Vec<f32>> = ui.ctx().data_mut(|d| {
                                    d.get_temp::<std::sync::Arc<Vec<f32>>>(wav_id)
                                        .unwrap_or_else(|| {
                                            let peaks = std::cell::RefCell::new(Vec::new());
                                            if let Ok(buf) = crate::core::audio_engine::AudioBuffer::load_wav(std::path::Path::new(wav_path)) {
                                                *peaks.borrow_mut() = buf.waveform_peaks(200);
                                            }
                                            let peaks = std::sync::Arc::new(peaks.into_inner());
                                            d.insert_temp(wav_id, peaks.clone());
                                            peaks
                                        })
                                });
                                if !peaks.is_empty() {
                                    let bin_span = (total_frames.max(1) as f32) / peaks.len() as f32;
                                    for (bin, &amp) in peaks.iter().enumerate() {
                                        let frame_at = bin as f32 * bin_span;
                                        if frame_at < start_frame as f32 || frame_at > (start_frame + zoom_span) as f32 {
                                            continue;
                                        }
                                        let norm = (frame_at - start_frame as f32) / zoom_span as f32;
                                        let sx = bar_rect.left() + norm * bar_rect.width();
                                        let h = (amp * layer_rect.height() * 0.45).max(1.0);
                                        let sy = layer_rect.center().y;
                                        ui.painter().line_segment(
                                            [egui::pos2(sx, sy - h), egui::pos2(sx, sy + h)],
                                            egui::Stroke::new(1.0, colors::ACCENT_CYAN),
                                        );
                                    }
                                }
                            }

                            if let LayerType::Audio { path, .. } = &layer.layer_type {
                                // Real waveform from the audio file (cached per layer)
                                let wav_id = egui::Id::new(("audio_peaks", layer.id.as_str()));
                                let peaks: std::sync::Arc<Vec<f32>> = ui.ctx().data_mut(|d| {
                                    d.get_temp::<std::sync::Arc<Vec<f32>>>(wav_id)
                                        .unwrap_or_else(|| {
                                            let peaks = std::cell::RefCell::new(Vec::new());
                                            if let Ok(buf) = crate::core::audio_engine::AudioBuffer::load_wav(std::path::Path::new(path)) {
                                                *peaks.borrow_mut() = buf.waveform_peaks(200);
                                            }
                                            let peaks = std::sync::Arc::new(peaks.into_inner());
                                            d.insert_temp(wav_id, peaks.clone());
                                            peaks
                                        })
                                });
                                if peaks.is_empty() {
                                    // No decodable audio: flat placeholder line
                                    ui.painter().line_segment(
                                        [egui::pos2(layer_rect.left() + 2.0, layer_rect.center().y),
                                         egui::pos2(layer_rect.right() - 2.0, layer_rect.center().y)],
                                        egui::Stroke::new(1.0, colors::ACCENT_CYAN.linear_multiply(0.5)),
                                    );
                                } else {
                                    let bin_span = (total_frames.max(1) as f32) / peaks.len() as f32;
                                    for (bin, &amp) in peaks.iter().enumerate() {
                                        let frame_at = bin as f32 * bin_span;
                                        if frame_at < start_frame as f32 || frame_at > (start_frame + zoom_span) as f32 {
                                            continue;
                                        }
                                        let norm = (frame_at - start_frame as f32) / zoom_span as f32;
                                        let sx = bar_rect.left() + norm * bar_rect.width();
                                        let h = (amp * layer_rect.height() * 0.45).max(1.0);
                                        let sy = layer_rect.center().y;
                                        ui.painter().line_segment(
                                            [egui::pos2(sx, sy - h), egui::pos2(sx, sy + h)],
                                            egui::Stroke::new(1.0, colors::ACCENT_CYAN),
                                        );
                                    }
                                }
                            }
                        });

                        // If expanded, render transform properties & effects
                        // If expanded, render transform properties & effects
                        if app.expanded_layers.contains(&i) {
                            crate::ui::timeline::keyframe_rows::draw_expanded_rows(
                                ui, layer, i,
                                &mut app.selected_keyframes,
                                &mut app.selected_property,
                                current_frame, start_frame, zoom_span, left_pane_w,
                                total_frames, &all_kf_frames, &precomp_children,
                                &mut pending_open_comp, &mut project_changed,
                            );
                        }
                    }
                }
            });

            // ── Select every keyframe on a layer (context menu) ──
            if let Some(idx) = pending_select_all_kfs {
                let mut collected: Vec<(String, u32)> = Vec::new();
                if let Some(layer) = comp.layers.get(idx) {
                    macro_rules! track {
                        ($key:expr, $anim:expr) => {
                            if let Some(kfs) = $anim.keyframes() {
                                for k in kfs { collected.push(($key.to_string(), k.frame)); }
                            }
                        };
                    }
                    track!("position", layer.transform.position);
                    track!("scale", layer.transform.scale);
                    track!("rotation", layer.transform.rotation);
                    track!("opacity", layer.transform.opacity);
                    for eff in &layer.effects {
                        for (lbl, pref) in eff.effect_type.animatable_params_ref() {
                            let key = format!("fx_{}_{}", eff.name, lbl);
                            match pref {
                                crate::core::effect_params::ParamRefRef::Scalar(a) => track!(key.as_str(), a),
                                crate::core::effect_params::ParamRefRef::Vec2(a) => track!(key.as_str(), a),
                                crate::core::effect_params::ParamRefRef::Vec4Color(a) => track!(key.as_str(), a),
                            }
                        }
                    }
                    for pin in &layer.puppet_pins {
                        track!(format!("pin_{}", pin.id), pin.position);
                    }
                }
                if collected.is_empty() {
                    app.toasts.info("Layer has no keyframes");
                } else {
                    app.selected_keyframes.clear();
                    for (pk, f) in collected {
                        app.selected_keyframes.insert((idx, pk, f));
                    }


            crate::ui::timeline::pending_actions::apply_effect_drops(app, pending_effect_drops, &mut project_changed);

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
                ui.small(egui::RichText::new("AE Standard Timeline 1:1 Parity Mode").color(colors::TEXT_SECONDARY));
            });

            crate::ui::timeline::pending_actions::apply(
                app,
                ui,
                *current_frame,
                &mut project_changed,
                swap_request,
                pending_duration,
                pending_trim_work_area,
                pending_open_comp,
                pending_ripple,
                pending_layer_marker,
                pending_clear_markers,
                pending_dup_layer,
                pending_split_layer,
                pending_precomp_indices,
            );

                    app.toasts.info("Selected all keyframes on layer");
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
