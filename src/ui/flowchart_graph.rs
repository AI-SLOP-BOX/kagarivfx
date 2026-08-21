use eframe::egui;
use crate::core::timeline::{Composition, LayerType};



pub fn draw_node_graph_panel(
    ui: &mut egui::Ui,
    comp: &Composition,
    selected_layer_idx: &mut Option<usize>,
    selected_layers: &mut std::collections::HashSet<usize>,
    show_graph_editor: &mut bool,
) {
    ui.group(|ui: &mut egui::Ui| {
        ui.horizontal(|ui: &mut egui::Ui| {
            ui.label(egui::RichText::new("🕸 Hybrid Node Graph View").strong().color(egui::Color32::from_rgb(0, 200, 255)));
            ui.weak("— Visual Pipeline & Layer Dependencies");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                if ui.button("❌ Close Node View (Tab)").clicked() {
                    *show_graph_editor = false;
                }
            });
        });
        ui.separator();

        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 200.0),
            egui::Sense::click_and_drag(),
        );

        // Draw Dark Canvas Grid
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 24, 30));
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 70)));

        let grid_size = 20.0;
        let mut gx = rect.left();
        while gx < rect.right() {
            ui.painter().line_segment(
                [egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())],
                egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)),
            );
            gx += grid_size;
        }

        let layers_count = comp.layers.len();

        if layers_count == 0 {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No layers in active composition",
                egui::FontId::proportional(12.0),
                egui::Color32::from_gray(120),
            );
            return;
        }

        // Calculate node positions in canvas space
        let node_w = 120.0;
        let node_h = 36.0;
        let col_spacing = 160.0;
        let row_spacing = 50.0;

        let mut node_positions = Vec::with_capacity(layers_count);

        for (idx, layer) in comp.layers.iter().enumerate() {
            // Level index based on parenting depth
            let depth = if layer.parent_id.is_some() { 1.0 } else { 0.0 };
            let nx = rect.left() + 40.0 + depth * col_spacing;
            let ny = rect.top() + 30.0 + idx as f32 * row_spacing;
            node_positions.push(egui::pos2(nx, ny));
        }

        // 1. Draw Connecting Wires (Parent-Child & Track Matte Links)
        for (idx, layer) in comp.layers.iter().enumerate() {
            let child_pos = node_positions[idx];
            let child_input = egui::pos2(child_pos.x, child_pos.y + node_h * 0.5);

            // Parent Connection Wire (Blue Curve)
            if let Some(ref pid) = layer.parent_id {
                if let Some(parent_idx) = comp.layers.iter().position(|l| &l.id == pid) {
                    let parent_pos = node_positions[parent_idx];
                    let parent_output = egui::pos2(parent_pos.x + node_w, parent_pos.y + node_h * 0.5);

                    let ctrl1 = egui::pos2(parent_output.x + 40.0, parent_output.y);
                    let ctrl2 = egui::pos2(child_input.x - 40.0, child_input.y);

                    let mut wire_pts = Vec::with_capacity(16);
                    for step in 0..=15 {
                        let t = step as f32 / 15.0;
                        let inv_t = 1.0 - t;
                        let wx = inv_t.powi(3) * parent_output.x + 3.0 * inv_t.powi(2) * t * ctrl1.x + 3.0 * inv_t * t.powi(2) * ctrl2.x + t.powi(3) * child_input.x;
                        let wy = inv_t.powi(3) * parent_output.y + 3.0 * inv_t.powi(2) * t * ctrl1.y + 3.0 * inv_t * t.powi(2) * ctrl2.y + t.powi(3) * child_input.y;
                        wire_pts.push(egui::pos2(wx, wy));
                    }

                    for win in wire_pts.windows(2) {
                        ui.painter().line_segment([win[0], win[1]], egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 160, 255)));
                    }
                }
            }
        }

        // 2. Render Layer Card Nodes
        for (idx, layer) in comp.layers.iter().enumerate() {
            let pos = node_positions[idx];
            let is_selected = *selected_layer_idx == Some(idx) || selected_layers.contains(&idx);


            let node_rect = egui::Rect::from_min_size(pos, egui::vec2(node_w, node_h));

            let bg_c = if is_selected {
                egui::Color32::from_rgb(0, 110, 200)
            } else {
                egui::Color32::from_rgb(32, 38, 48)
            };

            let stroke_c = if is_selected {
                egui::Color32::from_rgb(100, 200, 255)
            } else {
                egui::Color32::from_rgb(60, 75, 95)
            };

            ui.painter().rect_filled(node_rect, 4.0, bg_c);
            ui.painter().rect_stroke(node_rect, 4.0, egui::Stroke::new(1.5, stroke_c));

            // Layer Label Color Square
            let rgb = layer.label.to_rgb();
            let label_c = egui::Color32::from_rgb((rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[2] * 255.0) as u8);
            let indicator_rect = egui::Rect::from_min_size(egui::pos2(pos.x + 4.0, pos.y + 6.0), egui::vec2(6.0, node_h - 12.0));
            ui.painter().rect_filled(indicator_rect, 1.0, label_c);

            // Icon prefix
            let icon = match &layer.layer_type {
                LayerType::Solid { .. } => "█",
                LayerType::Image { .. } => "🖼",
                LayerType::Text { .. } => "T",
                LayerType::Shape { .. } => "⬡",
                LayerType::Null => "⌖",
                LayerType::PreComp { .. } => "🎞",
                LayerType::AdjustmentLayer => "◐",
                LayerType::Audio { .. } => "🎵",
                LayerType::Particle { .. } => "✦",
            };


            let title_text = format!("{} {}", icon, layer.name);
            ui.painter().text(
                egui::pos2(pos.x + 16.0, pos.y + node_h * 0.5),
                egui::Align2::LEFT_CENTER,
                title_text,
                egui::FontId::proportional(11.0),
                if is_selected { egui::Color32::WHITE } else { egui::Color32::from_gray(210) },
            );

            // Handle Node Click Selection
            let node_response = ui.interact(node_rect, ui.id().with(format!("node_click_{}", idx)), egui::Sense::click());
            if node_response.clicked() {
                selected_layers.clear();
                selected_layers.insert(idx);
                *selected_layer_idx = Some(idx);
            }
        }
    });
}
