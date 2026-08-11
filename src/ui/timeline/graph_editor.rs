use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::Composition;
use crate::core::property::Animatable;

fn get_kfs<T: Clone>(prop: &Animatable<T>) -> Vec<(u32, crate::core::keyframe::InterpolationType)> {
    prop.keyframes()
        .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.interpolation)).collect())
        .unwrap_or_default()
}

pub fn draw_graph_editor(
    app: &mut AfterEffectsApp,
    ui: &mut egui::Ui,
    comp: &Composition,
    current_frame: &mut u32,
    total_frames: u32,
) {
    let avail_size = ui.available_size();
    let (graph_rect, graph_response) = ui.allocate_exact_size(
        egui::vec2(avail_size.x, (avail_size.y - 10.0).max(180.0)),
        egui::Sense::click_and_drag(),
    );

    let painter = ui.painter();
    painter.rect_filled(graph_rect, 4.0, egui::Color32::from_rgb(22, 22, 28));
    painter.rect_stroke(graph_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)));

    let prop_name = app.selected_property.as_deref().unwrap_or("Position X");
    let layer_name = app
        .selected_layer_idx
        .and_then(|idx| comp.layers.get(idx))
        .map(|l| l.name.as_str())
        .unwrap_or("Layer 0");

    painter.text(
        egui::pos2(graph_rect.left() + 10.0, graph_rect.top() + 10.0),
        egui::Align2::LEFT_TOP,
        format!("📈 Graph Editor — {} :: {}", layer_name, prop_name),
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(100, 220, 255),
    );

    // Draw Grid lines
    let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20);
    for f in (0..=total_frames).step_by(10.max((total_frames / 10) as usize)) {
        let gx = graph_rect.left() + (f as f32 / total_frames as f32) * graph_rect.width();
        painter.line_segment(
            [egui::pos2(gx, graph_rect.top()), egui::pos2(gx, graph_rect.bottom())],
            egui::Stroke::new(1.0, grid_color),
        );
    }

    // Sample property values over total_frames
    if let Some(idx) = app.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer = &comp.layers[idx];
            let mut samples: Vec<(f32, f32)> = Vec::new();
            let steps = 60;
            let mut min_val = f32::MAX;
            let mut max_val = f32::MIN;

            for step in 0..=steps {
                let frame = (step as f32 / steps as f32 * total_frames as f32) as u32;
                let val = match prop_name {
                    "Position X" => layer.transform.eval_position(frame, comp.fps)[0],
                    "Position Y" => layer.transform.eval_position(frame, comp.fps)[1],
                    "Scale X" => layer.transform.eval_scale(frame, comp.fps)[0],
                    "Scale Y" => layer.transform.eval_scale(frame, comp.fps)[1],
                    "Rotation" => layer.transform.eval_rotation(frame, comp.fps),
                    "Opacity" => layer.transform.eval_opacity(frame, comp.fps),
                    _ => layer.transform.eval_position(frame, comp.fps)[0],
                };
                min_val = min_val.min(val);
                max_val = max_val.max(val);
                samples.push((frame as f32, val));
            }

            let val_range = (max_val - min_val).max(1.0);
            let pad_top = 35.0;
            let pad_bottom = 15.0;
            let draw_h = graph_rect.height() - pad_top - pad_bottom;

            let to_screen = |f: f32, v: f32| -> egui::Pos2 {
                let x = graph_rect.left() + (f / total_frames as f32) * graph_rect.width();
                let norm_y = (v - min_val) / val_range;
                let y = graph_rect.bottom() - pad_bottom - norm_y * draw_h;
                egui::pos2(x, y)
            };

            // Draw Curve
            let mut pts: Vec<egui::Pos2> = Vec::new();
            for (f, v) in &samples {
                pts.push(to_screen(*f, *v));
            }
            if pts.len() >= 2 {
                painter.add(egui::Shape::line(
                    pts,
                    egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 180, 50)),
                ));
            }

            // Draw Keyframe Nodes
            let kfs: Vec<u32> = match prop_name {
                "Position X" | "Position Y" => get_kfs(&layer.transform.position).into_iter().map(|(f, _)| f).collect(),
                "Scale X" | "Scale Y" => get_kfs(&layer.transform.scale).into_iter().map(|(f, _)| f).collect(),
                "Rotation" => get_kfs(&layer.transform.rotation).into_iter().map(|(f, _)| f).collect(),
                "Opacity" => get_kfs(&layer.transform.opacity).into_iter().map(|(f, _)| f).collect(),
                _ => vec![],
            };

            for kf in kfs {
                let val = match prop_name {
                    "Position X" => layer.transform.eval_position(kf, comp.fps)[0],
                    "Position Y" => layer.transform.eval_position(kf, comp.fps)[1],
                    "Scale X" => layer.transform.eval_scale(kf, comp.fps)[0],
                    "Scale Y" => layer.transform.eval_scale(kf, comp.fps)[1],
                    "Rotation" => layer.transform.eval_rotation(kf, comp.fps),
                    "Opacity" => layer.transform.eval_opacity(kf, comp.fps),
                    _ => 0.0,
                };
                let pos = to_screen(kf as f32, val);
                let diamond = vec![
                    pos + egui::vec2(0.0, -6.0),
                    pos + egui::vec2(6.0, 0.0),
                    pos + egui::vec2(0.0, 6.0),
                    pos + egui::vec2(-6.0, 0.0),
                ];
                painter.add(egui::Shape::convex_polygon(
                    diamond,
                    egui::Color32::from_rgb(255, 240, 100),
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                ));
            }
        }
    }

    // Playhead red line on graph
    let playhead_x = graph_rect.left() + (*current_frame as f32 / total_frames as f32) * graph_rect.width();
    painter.line_segment(
        [egui::pos2(playhead_x, graph_rect.top()), egui::pos2(playhead_x, graph_rect.bottom())],
        egui::Stroke::new(1.5, egui::Color32::RED),
    );

    if graph_response.dragged() || graph_response.clicked() {
        if let Some(ptr) = graph_response.interact_pointer_pos() {
            let norm = ((ptr.x - graph_rect.left()) / graph_rect.width()).clamp(0.0, 1.0);
            *current_frame = (norm * total_frames as f32) as u32;
        }
    }
}
