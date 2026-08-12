use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Composition, Layer, LayerType, ShapeType};
use crate::core::property::Animatable;

pub fn draw_timeline_header(
    app: &mut AfterEffectsApp,
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

        // ── AE Timeline Layer Filter ──
        let filter_id = ui.make_persistent_id("ae_timeline_filter");
        let mut filter_text = ui.ctx().data_mut(|d| d.get_temp_mut_or_insert_with(filter_id, String::new).clone());
        ui.add_space(8.0);
        ui.label("Filter:");
        if ui.add(egui::TextEdit::singleline(&mut filter_text).hint_text("Search layers...").desired_width(110.0)).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(filter_id, filter_text.clone()));
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

    project_changed
}
