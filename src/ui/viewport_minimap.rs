use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_minimap(
    app: &mut AfterEffectsApp,
    ui: &mut egui::Ui,
    current_frame: u32,
    comp_w: f32,
    comp_h: f32,
) {
    if comp_w <= 0.0 || comp_h <= 0.0 {
        return;
    }

    let map_w = 140.0;
    let map_h = map_w * (comp_h / comp_w);

    let (rect, response) = ui.allocate_exact_size(egui::vec2(map_w, map_h), egui::Sense::click_and_drag());

    // Dark Map Background & Border
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgba_unmultiplied(14, 18, 24, 230));
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 160, 255)));

    let scale_x = map_w / comp_w;
    let scale_y = map_h / comp_h;

    // Render Layer Rectangles on Minimap
    let comp = app.history.current().active_composition();
    for layer in &comp.layers {
        if !layer.is_active(current_frame) || !layer.visible {
            continue;
        }
        let pos = layer.transform.position.evaluate(current_frame);
        let scale = layer.transform.scale.evaluate(current_frame);

        let lx = rect.left() + (pos[0] / comp_w) * map_w;
        let ly = rect.top() + (pos[1] / comp_h) * map_h;
        let lw = (scale[0].abs() * 0.8 * scale_x).clamp(4.0, map_w);
        let lh = (scale[1].abs() * 0.8 * scale_y).clamp(4.0, map_h);

        let l_rect = egui::Rect::from_center_size(egui::pos2(lx, ly), egui::vec2(lw, lh));
        let rgb = layer.label.to_rgb();
        let fill_c = egui::Color32::from_rgba_unmultiplied((rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[2] * 255.0) as u8, 180);
        ui.painter().rect_filled(l_rect, 1.0, fill_c);
    }

    // Viewport Frame Indicator (Current Camera Focus Area)
    let focus_center = rect.center();
    let view_w = (map_w * 0.75).clamp(20.0, map_w);
    let view_h = (map_h * 0.75).clamp(15.0, map_h);
    let view_rect = egui::Rect::from_center_size(focus_center, egui::vec2(view_w, view_h));

    ui.painter().rect_stroke(view_rect, 2.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 220, 255)));

    // Handle Minimap Click / Layer Selection Interaction
    if response.clicked() {
        if let Some(ptr_pos) = response.interact_pointer_pos() {
            let norm_x = (ptr_pos.x - rect.left()) / map_w;
            let norm_y = (ptr_pos.y - rect.top()) / map_h;
            let target_cx = norm_x * comp_w;
            let target_cy = norm_y * comp_h;

            // Find closest layer on click
            let mut closest_idx = None;
            let mut min_dist = f32::MAX;
            for (idx, layer) in comp.layers.iter().enumerate() {
                let pos = layer.transform.position.evaluate(current_frame);
                let dist = (pos[0] - target_cx).hypot(pos[1] - target_cy);
                if dist < min_dist {
                    min_dist = dist;
                    closest_idx = Some(idx);
                }
            }
            if let Some(c_idx) = closest_idx {
                app.selected_layer_idx = Some(c_idx);
                app.selected_layers.clear();
                app.selected_layers.insert(c_idx);
            }
        }
    }
}
