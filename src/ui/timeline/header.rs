use eframe::egui;
use crate::core::timeline::{Composition, Layer, LayerType, ShapeType};
use crate::core::property::Animatable;

pub struct TimelineHeaderState<'a> {
    pub is_playing: &'a mut bool,
    pub timeline_zoom: &'a mut f32,
    pub snap_to_keyframes: &'a mut bool,
    pub show_graph_editor: &'a mut bool,
    pub layer_filter_text: &'a mut String,
}

pub fn draw_timeline_header(
    state: &mut TimelineHeaderState,
    ui: &mut egui::Ui,
    comp: &mut Composition,
    current_frame: &mut u32,
    total_frames: u32,
) -> bool {
    let mut project_changed = false;

    ui.horizontal(|ui| {
        let fps = comp.fps.max(1);
        let secs = *current_frame / fps;
        let sub_f = *current_frame % fps;
        let mins = secs / 60;
        let hours = mins / 60;
        let tc_str = format!("{:02}:{:02}:{:02}:{:02}", hours, mins % 60, secs % 60, sub_f);

        ui.label(egui::RichText::new(format!("TC: {}", tc_str)).strong().color(egui::Color32::from_rgb(255, 234, 0)));
        ui.add_space(4.0);
        ui.add(egui::DragValue::new(current_frame).clamp_range(0..=total_frames).prefix("Frame: ").suffix(format!(" / {}", total_frames)))
            .on_hover_text("Click or Drag to set current frame timecode");
        ui.add_space(8.0);
        if ui.button("|< First").clicked() { *current_frame = 0; }
        if ui.button("< Prev").clicked() { *current_frame = current_frame.saturating_sub(1); }
        if ui.button(if *state.is_playing { "|| Pause" } else { "> Play" }).clicked() {
            *state.is_playing = !*state.is_playing;
        }
        if ui.button("Next >").clicked() { *current_frame = (*current_frame + 1).min(total_frames); }
        if ui.button("Last >|").clicked() { *current_frame = total_frames; }

        ui.separator();
        ui.label("Zoom:");
        ui.add(egui::Slider::new(state.timeline_zoom, 0.1..=10.0));
        ui.checkbox(state.snap_to_keyframes, "Snap");
        let mode_btn_text = if *state.show_graph_editor { "Graph Mode" } else { "Tracks Mode" };
        if ui.selectable_label(*state.show_graph_editor, mode_btn_text).clicked() {
            *state.show_graph_editor = !*state.show_graph_editor;
        }

        // ── AE Timeline Layer Filter ──
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Filter:").small().color(crate::ui::theme::colors::TEXT_SECONDARY));
        ui.add(egui::TextEdit::singleline(state.layer_filter_text).hint_text("Search layers...").desired_width(110.0));
        
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
            let mut layer = Layer::new(id, name, LayerType::new_text("New Text", 48, [1.0, 1.0, 1.0, 1.0]), total_frames);

            layer.transform.position = Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 / 2.0]);
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
        if ui.button("+ Shape").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Shape {}", comp.layers.len());
            let mut layer = Layer::new(id, name, LayerType::Shape {
                shape_type: ShapeType::Star {
                    points: Animatable::new_constant(5.0),
                    inner_radius: Animatable::new_constant(40.0),
                    outer_radius: Animatable::new_constant(100.0),
                },
                color: [0.9, 0.4, 0.2, 1.0],
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
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
        if ui.button("+ Particles").clicked() {
            let id = format!("layer_{}", comp.layers.len());
            let name = format!("Particles {}", comp.layers.len());
            let mut layer = Layer::new(id, name, LayerType::Particle {
                emitter: crate::core::particle_system::ParticleEmitter::default(),
            }, total_frames);
            layer.transform.position = Animatable::new_constant([comp.width as f32 / 2.0, comp.height as f32 * 0.75]);
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

    project_changed
}
