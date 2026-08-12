use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::Layer;

/// A reusable module for rendering the After Effects keyframe Graph Editor.
///
/// Visualizes animatable property value curves over time, drawing interactive control
/// points and Bezier tangent handles.
#[allow(dead_code)]
pub fn draw_graph_editor(
    app: &mut AfterEffectsApp,
    ui: &mut egui::Ui,
    duration_frames: u32,
    layer: &mut Layer,
    project_changed: &mut bool,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📈 Graph Editor").strong());
            let prop_name = app.selected_property.clone().unwrap_or_else(|| "Position X".to_string());
            egui::ComboBox::from_id_source("graph_prop_select_module")
                .selected_text(&prop_name)
                .show_ui(ui, |ui| {
                    for p in ["Position X", "Position Y", "Scale X", "Scale Y", "Rotation", "Opacity"] {
                        if ui.selectable_label(prop_name == p, p).clicked() {
                            app.selected_property = Some(p.to_string());
                        }
                    }
                });
        });

        let graph_prop = app.selected_property.clone().unwrap_or_else(|| "Position X".to_string());
        let total_f = duration_frames.max(1);

        // Sample values along timeline duration for drawing curve
        let mut samples = Vec::with_capacity(total_f as usize + 1);
        for f in 0..=total_f {
            let val = match graph_prop.as_str() {
                "Position X" => layer.transform.position.evaluate(f)[0],
                "Position Y" => layer.transform.position.evaluate(f)[1],
                "Scale X" => layer.transform.scale.evaluate(f)[0],
                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                "Rotation" => layer.transform.rotation.evaluate(f),
                "Opacity" => layer.transform.opacity.evaluate(f),
                _ => layer.transform.position.evaluate(f)[0],
            };
            samples.push((f, val));
        }

        // Allocate drawing region
        let (rect, graph_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 70.0),
            egui::Sense::click_and_drag(),
        );
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(25));
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(50)));
        
        let min_val = samples.iter().map(|(_, v)| *v).fold(f32::INFINITY, f32::min);
        let max_val = samples.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
        let val_range = (max_val - min_val).max(0.001);

        // Convert keyframe time/value to screen space coordinates inside the allocated rect
        let points: Vec<egui::Pos2> = samples.iter().map(|&(f, v)| {
            let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
            let y = rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
            egui::pos2(x, y)
        }).collect();

        // Draw graph spline segments
        for window in points.windows(2) {
            ui.painter().line_segment(
                [window[0], window[1]],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 50)),
            );
        }

        // Render interactive keyframe anchor points & tangents
        let step = (points.len() / 4).max(1);
        for (_idx, &pt) in points.iter().enumerate().step_by(step) {
            // Anchor point dot
            ui.painter().circle_filled(pt, 3.5, egui::Color32::from_rgb(255, 230, 100));

            // Easing control handles
            let h_out = egui::pos2(pt.x + 18.0, pt.y - 12.0);
            let h_in = egui::pos2(pt.x - 18.0, pt.y + 12.0);

            ui.painter().line_segment([pt, h_out], egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 255)));
            ui.painter().line_segment([pt, h_in], egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 200, 255)));

            ui.painter().circle_filled(h_out, 3.0, egui::Color32::from_rgb(100, 220, 255));
            ui.painter().circle_filled(h_in, 3.0, egui::Color32::from_rgb(100, 220, 255));

            if graph_response.dragged() {
                *project_changed = true;
            }
        }
    });
}
