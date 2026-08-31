use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType, ShapeType};
use crate::ui::theme::colors;
use eframe::egui;
use std::collections::HashSet;

pub struct TimelineHeaderState<'a> {
    pub is_playing: &'a mut bool,
    pub timeline_zoom: &'a mut f32,
    pub snap_to_keyframes: &'a mut bool,
    pub show_graph_editor: &'a mut bool,
    pub layer_filter_text: &'a mut String,
    pub timeline_view_start: &'a mut u32,
    pub work_area_in: &'a mut Option<u32>,
    pub work_area_out: &'a mut Option<u32>,
    pub expanded_layers: &'a mut HashSet<usize>,
    pub fit_to_selection: &'a mut bool,
    pub fit_all: &'a mut bool,
}

pub fn draw_timeline_header(
    state: &mut TimelineHeaderState,
    ui: &mut egui::Ui,
    comp: &mut Composition,
    current_frame: &mut u32,
    total_frames: u32,
) -> bool {
    let mut project_changed = false;

    // Timecode "Go to Frame" popup state (persisted via egui temp data)
    let tc_popup_id = ui.make_persistent_id("ae_tc_goto_popup");
    let mut show_tc_popup: bool = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(tc_popup_id, || false));
    let mut tc_input_buf: String = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_insert_with(ui.make_persistent_id("ae_tc_goto_buf"), || {
            current_frame.to_string()
        })
        .clone()
    });
    let mut goto_frame: Option<u32> = None;

    ui.horizontal(|ui| {
        let fps = comp.fps.max(1);
        let secs = *current_frame / fps;
        let sub_f = *current_frame % fps;
        let mins = secs / 60;
        let hours = mins / 60;
        let tc_str = format!(
            "{:02}:{:02}:{:02}:{:02}",
            hours,
            mins % 60,
            secs % 60,
            sub_f
        );

        // Clickable timecode → opens "Go to Frame" popup
        let tc_resp = ui.label(
            egui::RichText::new(format!("TC: {}", tc_str))
                .strong()
                .color(colors::ACCENT_YELLOW),
        );
        if tc_resp.interact(egui::Sense::click()).clicked() {
            show_tc_popup = !show_tc_popup;
            tc_input_buf = current_frame.to_string();
        }
        ui.add_space(4.0);
        ui.add(
            egui::DragValue::new(current_frame)
                .range(0..=total_frames)
                .prefix("Frame: ")
                .suffix(format!(" / {}", total_frames)),
        )
        .on_hover_text("Click or Drag to set current frame timecode");
        ui.add_space(8.0);
        use crate::ui::icons::*;

        if ui.small_button("⏮").on_hover_text("Go to First Frame (Home)").clicked() {
            *current_frame = 0;
        }
        if ui.small_button("◀").on_hover_text("Previous Frame (PageUp / Left)").clicked() {
            *current_frame = current_frame.saturating_sub(1);
        }
        let play_btn_text = if *state.is_playing { "⏸ Pause" } else { "▶ Play" };
        if ui
            .button(egui::RichText::new(play_btn_text).strong().color(if *state.is_playing { colors::ACCENT_YELLOW } else { colors::ACCENT_GREEN }))
            .on_hover_text("Play / Pause RAM Preview (Spacebar)")
            .clicked()
        {
            *state.is_playing = !*state.is_playing;
        }
        if ui.small_button("▶").on_hover_text("Next Frame (PageDown / Right)").clicked() {
            *current_frame = (*current_frame + 1).min(total_frames);
        }
        if ui.small_button("⏭").on_hover_text("Go to Last Frame (End)").clicked() {
            *current_frame = total_frames;
        }

        ui.separator();
        ui.label("Zoom:");
        // Logarithmic zoom control (0.1 ..= 20.0)
        let mut zoom_log = (*state.timeline_zoom).max(0.1).log10();
        ui.add(
            egui::DragValue::new(&mut zoom_log)
                .speed(0.02)
                .range(-1.0..=(20.0f32).log10())
                .prefix("x"),
        )
        .on_hover_text("Timeline zoom (logarithmic, 0.1x - 20.0x)");
        *state.timeline_zoom = 10f32.powf(zoom_log).clamp(0.1, 20.0);
        if ui
            .button("Fit")
            .on_hover_text("Fit Timeline to Work Area (or full duration)")
            .clicked()
        {
            let w_in = *state.work_area_in;
            let w_out = *state.work_area_out;
            let span = match (w_in, w_out) {
                (Some(wi), Some(wo)) => (wo - wi).max(10),
                (Some(wi), None) => (total_frames - wi).max(10),
                (None, Some(wo)) => wo.max(10),
                (None, None) => total_frames,
            };
            *state.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
            *state.timeline_view_start = w_in.unwrap_or(0);
        }
        if ui
            .button("Fit Sel")
            .on_hover_text("Fit Timeline to Selected Layers' time range")
            .clicked()
        {
            *state.fit_to_selection = true;
        }
        if ui
            .button("Fit All")
            .on_hover_text("Fit Timeline to show all layers' time range")
            .clicked()
        {
            *state.fit_to_selection = true;
            *state.fit_all = true;
        }
        if ui
            .button("Clear WA")
            .on_hover_text("Clear Work Area (In/Out)")
            .clicked()
        {
            *state.work_area_in = None;
            *state.work_area_out = None;
        }
        if ui
            .button("Expand All")
            .on_hover_text("Expand all layer properties")
            .clicked()
        {
            for i in 0..comp.layers.len() {
                state.expanded_layers.insert(i);
            }
        }
        if ui
            .button("Collapse All")
            .on_hover_text("Collapse all layer properties")
            .clicked()
        {
            state.expanded_layers.clear();
        }

        use crate::ui::custom_widgets::ae_svg_toggle;

        ae_svg_toggle(
            ui,
            state.snap_to_keyframes,
            SVG_SNAP,
            "snap_btn_header",
            egui::vec2(22.0, 22.0),
            colors::ACCENT_CYAN,
            "Toggle Keyframe & Marker Snapping (Shift+S)",
        );

        ae_svg_toggle(
            ui,
            state.show_graph_editor,
            SVG_GRAPH_EDITOR,
            "graph_btn_header",
            egui::vec2(22.0, 22.0),
            colors::ACCENT_BLUE,
            "Toggle Graph Editor / Speed Curves (Shift+F3)",
        );

        // ── 8bpc / 16bpc / 32bpc (Float) HDR Color Depth Quick Toggle ──
        ui.add_space(4.0);
        let depth_badge_color = match comp.bit_depth {
            crate::core::color_science::BitDepth::EightBit => egui::Color32::from_rgb(140, 140, 150),
            crate::core::color_science::BitDepth::SixteenBit => egui::Color32::from_rgb(100, 180, 255),
            crate::core::color_science::BitDepth::ThirtyTwoBitFloat => egui::Color32::from_rgb(255, 200, 60),
        };
        let depth_btn = ui.add(
            egui::Button::new(
                egui::RichText::new(format!("💎 {}", comp.bit_depth.label()))
                    .small()
                    .strong()
                    .color(depth_badge_color),
            )
            .fill(egui::Color32::from_rgb(30, 32, 40)),
        )
        .on_hover_text("Click to cycle Color Depth: 8bpc (Integer) ↔ 16bpc (Half Float) ↔ 32bpc (Float / HDR)");
        if depth_btn.clicked() {
            comp.bit_depth = comp.bit_depth.next();
            crate::core::frame_cache::bump_version();
            project_changed = true;
        }

        // ── AE Timeline Layer Filter ──
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Filter:")
                .small()
                .color(crate::ui::theme::colors::TEXT_SECONDARY),
        );
        ui.add(
            egui::TextEdit::singleline(state.layer_filter_text)
                .hint_text("Search layers...")
                .desired_width(110.0),
        );

        ui.add_space(15.0);
        if ui.button("+ Solid").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Solid {}", comp.layers.len());
            let mut layer = Layer::new(
                id,
                name,
                LayerType::Solid {
                    color: [0.3, 0.5, 0.7, 1.0],
                },
                total_frames,
            );
            layer.transform.position =
                Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
            comp.add_layer(layer);
            project_changed = true;
        }
        if ui.button("+ Text").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Text {}", comp.layers.len());
            let mut layer = Layer::new(
                id,
                name,
                LayerType::new_text("New Text", 48, [1.0, 1.0, 1.0, 1.0]),
                total_frames,
            );

            layer.transform.position =
                Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
            comp.add_layer(layer);
            project_changed = true;
        }
        if ui.button("+ Adj").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Adjustment Layer {}", comp.layers.len());
            let layer = Layer::new_adjustment(id, name, total_frames);
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
        if !comp.markers.is_empty()
            && ui
                .small_button("📋 YouTube Chapters")
                .on_hover_text("Copy YouTube timestamp chapters formatted from markers")
                .clicked()
        {
            let fps = (comp.fps as f32).max(1.0);
            let mut sorted = comp.markers.clone();
            sorted.sort_by_key(|m| m.frame);
            let mut lines = Vec::new();
            for m in sorted {
                let total_secs = (m.frame as f32 / fps).floor() as u32;
                let mins = total_secs / 60;
                let secs = total_secs % 60;
                lines.push(format!("{:02}:{:02} {}", mins, secs, m.label));
            }
            let output = lines.join("\n");
            ui.output_mut(|o| o.copied_text = output);
        }
        if ui
            .button("+ Camera")
            .on_hover_text("Add 3D Camera layer")
            .clicked()
        {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Camera {}", comp.layers.len() + 1);
            let mut layer = Layer::new(id, name, LayerType::Null, total_frames);
            layer.is_3d = true;
            comp.add_layer(layer);
            project_changed = true;
        }
        if ui
            .button("+ Light")
            .on_hover_text("Add Point Light layer")
            .clicked()
        {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Light {}", comp.layers.len() + 1);
            let mut layer = Layer::new(id, name, LayerType::Null, total_frames);
            layer.is_3d = true;
            comp.add_layer(layer);
            project_changed = true;
        }
        if ui.button("+ Shape").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Shape {}", comp.layers.len());
            let mut layer = Layer::new(
                id,
                name,
                LayerType::Shape {
                    shape_type: ShapeType::Star {
                        points: Animatable::new_constant(5.0),
                        inner_radius: Animatable::new_constant(40.0),
                        outer_radius: Animatable::new_constant(100.0),
                    },
                    color: [0.9, 0.4, 0.2, 1.0],
                    stroke_color: [0.0, 0.0, 0.0, 1.0],
                    stroke_width: 0.0,
                    fill_type: Default::default(),
                    extrusion_depth: 0.0,
                    bevel_depth: 0.0,
                },
                total_frames,
            );
            layer.transform.position =
                Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
            comp.add_layer(layer);
            project_changed = true;
        }
        if ui.button("+ Audio").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Audio {}", comp.layers.len());
            let mut layer = Layer::new(
                id,
                name,
                LayerType::Audio {
                    path: "audio_track_01.wav".to_string(),
                    volume: Animatable::new_constant(1.0),
                },
                total_frames,
            );
            layer.label = crate::core::timeline::LabelColor::Aqua;
            comp.add_layer(layer);
            project_changed = true;
        }
        if ui.button("+ Particles").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Particles {}", comp.layers.len());
            let mut layer = Layer::new(
                id,
                name,
                LayerType::Particle {
                    emitter: crate::core::particle_system::ParticleEmitter::default(),
                },
                total_frames,
            );
            layer.transform.position =
                Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 * 0.75]);
            layer.label = crate::core::timeline::LabelColor::Yellow;
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
        let add_marker_clicked =
            ui.button("+ Marker (M)").clicked() || ui.input(|i| i.key_pressed(egui::Key::M));
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

    // ── Go to Frame popup ──
    if show_tc_popup {
        egui::Area::new(egui::Id::new("ae_tc_goto"))
            .fixed_pos(ui.cursor().left_top())
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.group(|ui| {
                    ui.set_min_width(160.0);
                    ui.label(egui::RichText::new("Go to Frame").strong());
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut tc_input_buf)
                            .desired_width(100.0)
                            .hint_text("Frame #"),
                    );
                    if ui.button("Go").clicked()
                        || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        if let Ok(f) = tc_input_buf.trim().parse::<u32>() {
                            goto_frame = Some(f.min(total_frames));
                        }
                        show_tc_popup = false;
                    }
                    if ui.button("Cancel").clicked() {
                        show_tc_popup = false;
                    }
                });
            });
    }

    if let Some(f) = goto_frame {
        *current_frame = f;
    }

    // Persist popup state
    ui.ctx().data_mut(|d| {
        d.insert_temp(tc_popup_id, show_tc_popup);
    });
    ui.ctx().data_mut(|d| {
        d.insert_temp(ui.make_persistent_id("ae_tc_goto_buf"), tc_input_buf);
    });

    project_changed
}
