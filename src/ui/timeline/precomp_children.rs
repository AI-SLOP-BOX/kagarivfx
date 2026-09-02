//! Pre-comp nested timeline: snapshot child layers of expanded PreComp layers
//! and render them as indented rows beneath the parent.
use crate::core::timeline::{LayerType, Project};
use crate::ui::theme::colors;
use eframe::egui;

#[derive(Clone)]
pub struct PreCompChild {
    pub name: String,
    pub type_icon: String,
    pub label_rgb: [f32; 3],
    pub in_frame: u32,
    pub out_frame: u32,
}

fn layer_icon(lt: &LayerType) -> &'static str {
    match lt {
        LayerType::Video { .. } => "🎬",
        LayerType::Image { .. } => "🖼",
        LayerType::Audio { .. } => "🔊",
        LayerType::Text { .. } => "T",
        LayerType::Shape { .. } => "◆",
        LayerType::Solid { .. } => "■",
        LayerType::Null => "∅",
        LayerType::PreComp { .. } => "📦",
        LayerType::AdjustmentLayer => "◐",
        LayerType::Particle { .. } => "✦",
    }
}

/// Snapshot child layers for every expanded PreComp layer.
/// Runs BEFORE the mutable composition borrow to avoid aliasing.
pub fn collect(
    project: &Project,
    expanded_layers: &std::collections::HashSet<usize>,
) -> Vec<(usize, Vec<PreCompChild>, String)> {
    let comp_ro = project.active_composition();
    expanded_layers
        .iter()
        .filter_map(|&parent_i| {
            let layer = comp_ro.layers.get(parent_i)?;
            if let LayerType::PreComp { comp_id } = &layer.layer_type {
                let sub = comp_ro.find_sub_comp(comp_id)?;
                let children: Vec<PreCompChild> = sub
                    .layers
                    .iter()
                    .map(|cl| {
                        let rgb = cl.label.to_rgb();
                        PreCompChild {
                            name: cl.name.clone(),
                            type_icon: layer_icon(&cl.layer_type).to_string(),
                            label_rgb: rgb,
                            in_frame: cl.in_frame,
                            out_frame: cl.out_frame,
                        }
                    })
                    .collect();
                Some((parent_i, children, sub.id.clone()))
            } else {
                None
            }
        })
        .collect()
}

/// Draw the indented child rows under an expanded PreComp layer.
/// Returns true when the user clicks "Open Pre-comp".
pub fn draw_children_rows(
    ui: &mut egui::Ui,
    sub_id: &str,
    children: &[PreCompChild],
    total_frames: u32,
) -> bool {
    ui.add_space(2.0);
    for child in children {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new(&child.type_icon)
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let cr = (child.label_rgb[0] * 255.0) as u8;
            let cg = (child.label_rgb[1] * 255.0) as u8;
            let cb = (child.label_rgb[2] * 255.0) as u8;
            let chip_rect = ui.allocate_space(egui::vec2(6.0, 12.0)).1;
            ui.painter()
                .rect_filled(chip_rect, 1.0, egui::Color32::from_rgb(cr, cg, cb));
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(&child.name)
                    .small()
                    .color(colors::TEXT_PRIMARY),
            );
            ui.add_space(4.0);
            // Mini timeline bar for the child layer
            let bar_avail = ui.available_width();
            let bar_resp = ui
                .allocate_ui_with_layout(
                    egui::vec2(bar_avail.min(200.0), 12.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let total_f = total_frames.max(1);
                        let bar_rect = ui.max_rect();
                        ui.painter().rect_filled(bar_rect, 0.0, colors::BG_DARK);
                        let in_x = bar_rect.left()
                            + (child.in_frame as f32 / total_f as f32) * bar_rect.width();
                        let out_x = bar_rect.left()
                            + (child.out_frame as f32 / total_f as f32) * bar_rect.width();
                        let layer_rect = egui::Rect::from_min_size(
                            egui::pos2(in_x, bar_rect.top()),
                            egui::vec2((out_x - in_x).max(2.0), bar_rect.height()),
                        );
                        ui.painter()
                            .rect_filled(layer_rect, 2.0, colors::ACCENT_BLUE);
                    },
                )
                .response;
            bar_resp.on_hover_text(format!(
                "{}: frames {}–{}",
                child.name, child.in_frame, child.out_frame
            ));
        });
    }
    // "Open Pre-comp" link
    let mut open_requested = false;
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        if ui
            .small(egui::RichText::new("📂 Open Pre-comp...").color(colors::ACCENT_BLUE))
            .clicked()
        {
            open_requested = true;
        }
    });
    let _ = sub_id;
    open_requested
}
