use crate::core::timeline::{Composition, LayerType, TrackMatteMode};
use crate::ui::theme::colors;
use eframe::egui;

fn compute_parent_depth(
    comp: &Composition,
    layer_idx: usize,
    visited: &mut std::collections::HashSet<usize>,
) -> usize {
    if visited.contains(&layer_idx) || layer_idx >= comp.layers.len() {
        return 0;
    }
    visited.insert(layer_idx);

    if let Some(ref pid) = comp.layers[layer_idx].parent_id {
        if let Some(parent_idx) = comp.layers.iter().position(|l| &l.id == pid) {
            return 1 + compute_parent_depth(comp, parent_idx, visited);
        }
    }
    0
}

pub fn draw_node_graph_panel(
    ui: &mut egui::Ui,
    comp: &Composition,
    selected_layer_idx: &mut Option<usize>,
    selected_layers: &mut std::collections::HashSet<usize>,
    show_graph_editor: &mut bool,
) {
    ui.group(|ui: &mut egui::Ui| {
        ui.horizontal(|ui: &mut egui::Ui| {
            ui.label(
                egui::RichText::new("🕸 Hybrid Node Graph View")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.weak("— Visual Pipeline & Layer Dependencies");
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui: &mut egui::Ui| {
                    if ui.button("❌ Close Node View (Tab)").clicked() {
                        *show_graph_editor = false;
                    }
                },
            );
        });
        ui.separator();

        let layers_count = comp.layers.len();

        if layers_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.weak("No layers in active composition");
            });
            return;
        }

        // Calculate node positions in canvas space
        let node_w = 140.0;
        let node_h = 36.0;
        let col_spacing = 170.0;
        let row_spacing = 52.0;

        let total_h = (layers_count as f32 * row_spacing + 60.0).max(220.0);
        let max_depth = comp
            .layers
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let mut visited = std::collections::HashSet::new();
                compute_parent_depth(comp, idx, &mut visited)
            })
            .max()
            .unwrap_or(0);

        let total_w = (max_depth as f32 * col_spacing + 300.0).max(ui.available_width());

        egui::ScrollArea::both()
            .max_height(260.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (rect, _response) = ui.allocate_exact_size(
                    egui::vec2(total_w, total_h),
                    egui::Sense::click_and_drag(),
                );

                // Draw Dark Canvas Grid
                ui.painter().rect_filled(rect, 4.0, colors::BG_DEEPEST);
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
                );

                let grid_size = 24.0;
                let mut gx = rect.left();
                while gx < rect.right() {
                    ui.painter().line_segment(
                        [egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())],
                        egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                    );
                    gx += grid_size;
                }
                let mut gy = rect.top();
                while gy < rect.bottom() {
                    ui.painter().line_segment(
                        [egui::pos2(rect.left(), gy), egui::pos2(rect.right(), gy)],
                        egui::Stroke::new(0.5_f32, colors::GRID_LINE),
                    );
                    gy += grid_size;
                }

                let mut node_positions = Vec::with_capacity(layers_count);

                for (idx, _) in comp.layers.iter().enumerate() {
                    let mut visited = std::collections::HashSet::new();
                    let depth = compute_parent_depth(comp, idx, &mut visited);
                    let nx = rect.left() + 30.0 + depth as f32 * col_spacing;
                    let ny = rect.top() + 24.0 + idx as f32 * row_spacing;
                    node_positions.push(egui::pos2(nx, ny));
                }

                // 1. Draw Connecting Wires (Parent-Child Blue & Track Matte Amber)
                for (idx, layer) in comp.layers.iter().enumerate() {
                    let child_pos = node_positions[idx];
                    let child_input = egui::pos2(child_pos.x, child_pos.y + node_h * 0.5);

                    // Parent Connection Wire (Blue Curve)
                    if let Some(ref pid) = layer.parent_id {
                        if let Some(parent_idx) = comp.layers.iter().position(|l| &l.id == pid) {
                            let parent_pos = node_positions[parent_idx];
                            let parent_output =
                                egui::pos2(parent_pos.x + node_w, parent_pos.y + node_h * 0.5);

                            let ctrl1 = egui::pos2(parent_output.x + 40.0, parent_output.y);
                            let ctrl2 = egui::pos2(child_input.x - 40.0, child_input.y);

                            let mut wire_pts = Vec::with_capacity(16);
                            for step in 0..=15 {
                                let t = step as f32 / 15.0;
                                let inv_t = 1.0 - t;
                                let wx = inv_t.powi(3) * parent_output.x
                                    + 3.0 * inv_t.powi(2) * t * ctrl1.x
                                    + 3.0 * inv_t * t.powi(2) * ctrl2.x
                                    + t.powi(3) * child_input.x;
                                let wy = inv_t.powi(3) * parent_output.y
                                    + 3.0 * inv_t.powi(2) * t * ctrl1.y
                                    + 3.0 * inv_t * t.powi(2) * ctrl2.y
                                    + t.powi(3) * child_input.y;
                                wire_pts.push(egui::pos2(wx, wy));
                            }

                            for win in wire_pts.windows(2) {
                                ui.painter().line_segment(
                                    [win[0], win[1]],
                                    egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE),
                                );
                            }
                        }
                    }

                    // Track Matte Connection Wire (Amber/Orange Curve)
                    if layer.track_matte != TrackMatteMode::None && idx > 0 {
                        let matte_idx = idx - 1;
                        let matte_pos = node_positions[matte_idx];
                        let matte_output =
                            egui::pos2(matte_pos.x + node_w, matte_pos.y + node_h * 0.5);

                        let ctrl1 = egui::pos2(matte_output.x + 30.0, matte_output.y);
                        let ctrl2 = egui::pos2(child_input.x - 30.0, child_input.y);

                        let mut wire_pts = Vec::with_capacity(16);
                        for step in 0..=15 {
                            let t = step as f32 / 15.0;
                            let inv_t = 1.0 - t;
                            let wx = inv_t.powi(3) * matte_output.x
                                + 3.0 * inv_t.powi(2) * t * ctrl1.x
                                + 3.0 * inv_t * t.powi(2) * ctrl2.x
                                + t.powi(3) * child_input.x;
                            let wy = inv_t.powi(3) * matte_output.y
                                + 3.0 * inv_t.powi(2) * t * ctrl1.y
                                + 3.0 * inv_t * t.powi(2) * ctrl2.y
                                + t.powi(3) * child_input.y;
                            wire_pts.push(egui::pos2(wx, wy));
                        }

                        for win in wire_pts.windows(2) {
                            ui.painter().line_segment(
                                [win[0], win[1]],
                                egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(245, 158, 11)),
                            );
                        }
                    }
                }

                // 2. Render Layer Card Nodes
                for (idx, layer) in comp.layers.iter().enumerate() {
                    let pos = node_positions[idx];
                    let is_selected =
                        *selected_layer_idx == Some(idx) || selected_layers.contains(&idx);

                    let node_rect = egui::Rect::from_min_size(pos, egui::vec2(node_w, node_h));

                    let bg_c = if is_selected {
                        colors::BG_ACTIVE
                    } else {
                        colors::BG_DARK
                    };

                    let stroke_c = if is_selected {
                        colors::ACCENT_CYAN
                    } else {
                        colors::BORDER_STRONG
                    };

                    ui.painter().rect_filled(node_rect, 4.0, bg_c);
                    ui.painter()
                        .rect_stroke(node_rect, 4.0, egui::Stroke::new(1.5_f32, stroke_c));

                    // Layer Label Color Square
                    let rgb = layer.label.to_rgb();
                    let label_c = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8,
                        (rgb[1] * 255.0) as u8,
                        (rgb[2] * 255.0) as u8,
                    );
                    let indicator_rect = egui::Rect::from_min_size(
                        egui::pos2(pos.x + 4.0, pos.y + 6.0),
                        egui::vec2(6.0, node_h - 12.0),
                    );
                    ui.painter().rect_filled(indicator_rect, 1.0, label_c);

                    // Icon prefix
                    let icon = match &layer.layer_type {
                        crate::core::timeline::LayerType::Video { .. } => "Video",
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
                        if is_selected {
                            colors::HANDLE_NORMAL
                        } else {
                            colors::TEXT_PRIMARY
                        },
                    );

                    // Handle Node Click Selection
                    let node_response = ui.interact(
                        node_rect,
                        ui.id().with(format!("node_click_{}", idx)),
                        egui::Sense::click(),
                    );
                    if node_response.clicked() {
                        selected_layers.clear();
                        selected_layers.insert(idx);
                        *selected_layer_idx = Some(idx);
                    }
                }
            });
    });
}
