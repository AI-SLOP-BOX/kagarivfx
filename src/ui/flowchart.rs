use crate::core::timeline::{Composition, LayerType, TrackMatteMode};
use crate::core::vfx_graph_compiler::{LayerOpType, VfxGraphCompiler};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

/// Maps layer_idx → 1-based execution order from a compiled schedule
/// (first RenderLayer step wins when a layer appears in several ops).
fn execution_order_map(
    steps: &[crate::core::vfx_graph_compiler::ExecutionStep],
) -> std::collections::HashMap<usize, usize> {
    let mut out = std::collections::HashMap::new();
    for (i, step) in steps.iter().enumerate() {
        if let LayerOpType::RenderLayer { layer_idx } = &step.op {
            out.entry(*layer_idx).or_insert(i + 1);
        }
    }
    out
}

#[allow(dead_code)]
pub fn draw_flowchart_view(app: &mut KagariApp, ui: &mut egui::Ui, comp: &Composition) {
    // Compile the dependency graph once per frame: gives topological
    // execution order + parenting/matte/cycle analysis for the overlays.
    let mut graph = VfxGraphCompiler::new();
    graph.compile(comp, app.playback.current_frame);
    let order_map = execution_order_map(&graph.steps);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Comp & Layer Flowchart View (Shift+F3)")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.weak("— Visual Graph of Compositions, Layer Hierarchies & Mattes");
        });
        ui.separator();

        if graph.cycle_detected {
            ui.colored_label(
                egui::Color32::from_rgb(230, 90, 90),
                "⚠ Dependency cycle detected between layers — evaluation order may be incorrect",
            );
            ui.separator();
        }

        let avail_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(avail_size.x, avail_size.y.max(220.0)),
            egui::Sense::drag(),
        );

        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, colors::BG_DEEPEST);
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, colors::BORDER_MEDIUM));

        // Grid Background
        let grid_color = colors::GRID_LINE;
        let step = 30.0;
        let mut x = rect.left();
        while x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, grid_color),
            );
            x += step;
        }
        let mut y = rect.top();
        while y < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, grid_color),
            );
            y += step;
        }

        // Draw Composition Root Node
        let root_pos = egui::pos2(rect.left() + 90.0, rect.center().y);
        let root_rect = egui::Rect::from_center_size(root_pos, egui::vec2(130.0, 44.0));
        painter.rect_filled(root_rect, 6.0, colors::BG_ACTIVE);
        painter.rect_stroke(root_rect, 6.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
        painter.text(
            root_pos,
            egui::Align2::CENTER_CENTER,
            format!("[COMP]\n{}", comp.name),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );

        // Draw Layer Nodes
        let layer_count = comp.layers.len();
        if layer_count == 0 {
            painter.text(
                egui::pos2(rect.center().x + 80.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                "No layers in composition",
                egui::FontId::proportional(14.0),
                colors::TEXT_MUTED,
            );
            return;
        }

        let start_x = rect.left() + 280.0;
        let spacing_y = (rect.height() - 40.0) / (layer_count as f32 + 1.0).max(1.0);
        let mut node_positions: Vec<egui::Pos2> = Vec::new();

        for (i, layer) in comp.layers.iter().enumerate() {
            let ny = rect.top() + 20.0 + spacing_y * (i as f32 + 1.0);
            let npos = egui::pos2(start_x, ny);
            node_positions.push(npos);

            let is_selected = app.selection.selected_layers.contains(&i) || app.selection.selected_layer_idx == Some(i);
            let node_rect = egui::Rect::from_center_size(npos, egui::vec2(140.0, 36.0));

            let base_color = match layer.layer_type {
                LayerType::Null => egui::Color32::from_rgb(140, 80, 80),
                LayerType::Audio { .. } => egui::Color32::from_rgb(30, 120, 90),
                LayerType::Text { .. } => egui::Color32::from_rgb(160, 110, 40),
                LayerType::Shape { .. } => egui::Color32::from_rgb(130, 70, 150),
                _ => egui::Color32::from_rgb(50, 70, 100),
            };

            let stroke_color = if is_selected {
                colors::ACCENT_CYAN
            } else {
                colors::BORDER_STRONG
            };

            painter.rect_filled(node_rect, 4.0, base_color);
            painter.rect_stroke(
                node_rect,
                4.0,
                egui::Stroke::new(if is_selected { 2.5 } else { 1.0 }, stroke_color),
            );

            let tag = match layer.layer_type {
                LayerType::Text { .. } => "[TXT]",
                LayerType::Shape { .. } => "[SHP]",
                LayerType::Audio { .. } => "[AUD]",
                LayerType::Null => "[NUL]",
                _ => "[VID]",
            };

            painter.text(
                npos,
                egui::Align2::CENTER_CENTER,
                format!("{} {}\n#{}", tag, layer.name, i + 1),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            // Topological execution-order badge (from VfxGraphCompiler)
            if let Some(order) = order_map.get(&i) {
                painter.text(
                    egui::pos2(npos.x + 62.0, ny - 14.0),
                    egui::Align2::CENTER_CENTER,
                    format!("⏱ {}", order),
                    egui::FontId::proportional(9.0),
                    colors::ACCENT_CYAN,
                );
            }

            // Curve line from Root Comp to Layer
            let control1 = egui::pos2(root_pos.x + 80.0, root_pos.y);
            let control2 = egui::pos2(npos.x - 80.0, npos.y);
            let points = [root_pos, control1, control2, npos];
            let curve_pts: Vec<egui::Pos2> = (0..=20)
                .map(|t| {
                    let u = t as f32 / 20.0;
                    let u1 = 1.0 - u;
                    let x = u1 * u1 * u1 * points[0].x
                        + 3.0 * u1 * u1 * u * points[1].x
                        + 3.0 * u1 * u * u * points[2].x
                        + u * u * u * points[3].x;
                    let y = u1 * u1 * u1 * points[0].y
                        + 3.0 * u1 * u1 * u * points[1].y
                        + 3.0 * u1 * u * u * points[2].y
                        + u * u * u * points[3].y;
                    egui::pos2(x, y)
                })
                .collect();

            painter.add(egui::Shape::line(
                curve_pts,
                egui::Stroke::new(1.2, colors::MOTION_PATH.linear_multiply(140.0 / 255.0)),
            ));

            // Draw Parent Connection Lines
            if let Some(ref pid) = layer.parent_id {
                if let Some(p_idx) = comp.layers.iter().position(|l| l.id == *pid) {
                    if p_idx < node_positions.len() {
                        let parent_pos = node_positions[p_idx];
                        painter.line_segment(
                            [npos, parent_pos],
                            egui::Stroke::new(1.8, colors::TIMELINE_KEYFRAME),
                        );
                    }
                }
            }
        }

        // Track-matte dependency arrows: matte layer → consumer layer.
        // Dashed cyan so they read differently from parenting lines.
        for (i, layer) in comp.layers.iter().enumerate() {
            if layer.track_matte == TrackMatteMode::None || i == 0 {
                continue;
            }
            if let (Some(&matte_pos), Some(&consumer_pos)) =
                (node_positions.get(i - 1), node_positions.get(i))
            {
                let stroke = egui::Stroke::new(1.6, colors::ACCENT_CYAN);
                for seg in egui::Shape::dashed_line(&[matte_pos, consumer_pos], stroke, 6.0, 5.0) {
                    painter.add(seg);
                }
                painter.text(
                    egui::pos2(
                        (matte_pos.x + consumer_pos.x) * 0.5,
                        (matte_pos.y + consumer_pos.y) * 0.5 - 8.0,
                    ),
                    egui::Align2::CENTER_CENTER,
                    "matte",
                    egui::FontId::proportional(9.0),
                    colors::ACCENT_CYAN,
                );
            }
        }

        if response.clicked() {
            if let Some(ptr) = response.interact_pointer_pos() {
                for (i, npos) in node_positions.iter().enumerate() {
                    let nrect = egui::Rect::from_center_size(*npos, egui::vec2(140.0, 36.0));
                    if nrect.contains(ptr) {
                        app.selection.selected_layers.clear();
                        app.selection.selected_layers.insert(i);
                        app.selection.selected_layer_idx = Some(i);
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vfx_graph_compiler::ExecutionStep;

    #[test]
    fn order_map_uses_first_render_step_and_is_one_based() {
        let steps = vec![
            ExecutionStep {
                step_id: 0,
                op: LayerOpType::EvaluateParentTransform { layer_idx: 2 },
                dependencies: vec![],
            },
            ExecutionStep {
                step_id: 1,
                op: LayerOpType::RenderLayer { layer_idx: 1 },
                dependencies: vec![],
            },
            ExecutionStep {
                step_id: 2,
                op: LayerOpType::RenderLayer { layer_idx: 2 },
                dependencies: vec![],
            },
            ExecutionStep {
                step_id: 3,
                op: LayerOpType::CompositeTrackMatte {
                    layer_idx: 2,
                    matte_layer_idx: 1,
                },
                dependencies: vec![],
            },
            ExecutionStep {
                step_id: 4,
                op: LayerOpType::RenderLayer { layer_idx: 2 },
                dependencies: vec![],
            },
            ExecutionStep {
                step_id: 5,
                op: LayerOpType::EvaluatePreComp {
                    layer_idx: 3,
                    comp_id: "c".into(),
                },
                dependencies: vec![],
            },
        ];
        let m = execution_order_map(&steps);
        assert_eq!(m[&1], 2, "first RenderLayer step sets the order");
        assert_eq!(m[&2], 3, "first occurrence wins over later re-renders");
        assert!(
            !m.contains_key(&3),
            "EvaluatePreComp ops must not create badges"
        );
    }
}
