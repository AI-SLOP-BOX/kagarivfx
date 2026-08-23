use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTool {
    #[default]
    Selection,
    Hand,
    Zoom,
    Camera3D,
    Rotation,
    AnchorPoint,
    Rectangle,
    Pen,
    Text,
    Brush,
    CloneStamp,
    Eraser,
    RotoBrush,
    PuppetPin,
}

#[allow(dead_code)]
pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    use crate::ui::theme::colors;
    let frame = egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .stroke(egui::Stroke::new(1.0, colors::BORDER_SUBTLE));

    egui::TopBottomPanel::top("ae_toolbar")
        .frame(frame)
        .default_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 3.0;

                // App logo mark
                crate::ui::icons::draw_logo(ui, 20.0);
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);

                // Vector tool icons with hover tooltips
                let tools: [(ActiveTool, &'static str, &'static str); 14] = [
                    (ActiveTool::Selection, SVG_TOOL_SELECT, "Select (V)"),
                    (ActiveTool::Hand, SVG_TOOL_HAND, "Hand (H)"),
                    (ActiveTool::Zoom, SVG_TOOL_ZOOM, "Zoom (Z)"),
                    (ActiveTool::Camera3D, SVG_TOOL_CAMERA, "Camera (C)"),
                    (ActiveTool::Rotation, SVG_TOOL_ROTATE, "Rotate (W)"),
                    (ActiveTool::AnchorPoint, SVG_TOOL_ANCHOR, "Anchor Point (Y)"),
                    (ActiveTool::Rectangle, SVG_TOOL_SHAPE, "Shape (Q)"),
                    (ActiveTool::Pen, SVG_TOOL_PEN, "Pen (G)"),
                    (ActiveTool::Text, SVG_TOOL_TEXT, "Text (Cmd+T)"),
                    (ActiveTool::Brush, SVG_TOOL_BRUSH, "Brush"),
                    (ActiveTool::CloneStamp, SVG_TOOL_STAMP, "Clone Stamp"),
                    (ActiveTool::Eraser, SVG_TOOL_ERASER, "Eraser"),
                    (ActiveTool::RotoBrush, SVG_TOOL_ROTO, "Roto Brush"),
                    (ActiveTool::PuppetPin, SVG_TOOL_PUPPET, "Puppet Pin"),
                ];
                use crate::ui::icons::*;

                for (tool, svg, tooltip) in tools {
                    let is_selected = app.active_tool == tool;
                    let accent = colors::ACCENT_BLUE;
                    let tint = if is_selected { accent } else { colors::TEXT_SECONDARY };

                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                    let fill = if is_selected { colors::BG_HOVER } else { egui::Color32::TRANSPARENT };
                    ui.painter().rect_filled(rect, 4.0, fill);
                    if is_selected {
                        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, accent));
                    }
                    // Draw icon centered inside the button rect
                    let icon_rect = rect.shrink(4.0);
                    let icon_resp = crate::ui::icons::render_svg_at(ui, format!("tool_{:?}", tool), svg, icon_rect.size(), tint, icon_rect.min);
                    let _ = icon_resp;
                    let _ = rect; // rect used above
                    if resp.clicked() {
                        app.active_tool = tool;
                    }
                    resp.on_hover_text(tooltip);
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // AE Snapping Toggle (icon + label)
                let snap_id = egui::Id::new("ae_snapping_enabled");
                let mut snapping = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(snap_id, || true));
                let snap_changed = {
                    ui.toggle_value(&mut snapping, "🧲 Snap").changed()
                };
                if snap_changed {
                    ctx.data_mut(|d| d.insert_temp(snap_id, snapping));
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // AE Workspace Layout Switcher Pill Buttons
                ui.label(egui::RichText::new("Workspace:").small().color(colors::TEXT_SECONDARY));
                for (name, l_idx, r_idx) in [
                    ("Default", 0, 0),
                    ("Learn", 0, 4),
                    ("Assembly", 0, 2),
                    ("Editing", 0, 1),
                    ("Color", 1, 19),
                    ("Effects", 1, 0),
                    ("Audio", 0, 7),
                    ("Libraries", 0, 20),
                ] {
                    let is_active = app.left_tab_idx == l_idx && app.right_tab_idx == r_idx;
                    if ui.selectable_label(is_active, name).clicked() {
                        app.left_tab_idx = l_idx;
                        app.right_tab_idx = r_idx;
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                crate::ui::align_hud::draw_alignment_hud(app, ui);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("🚀 Render Queue (Cmd+M)").strong().color(colors::ACCENT_YELLOW))
                        .clicked()
                    {
                        app.show_export_dialog = true;
                    }
                    ui.add_space(8.0);
                    let search_id = egui::Id::new("ae_toolbar_search_query");
                    let mut search_query = ctx.data_mut(|d| d.get_temp_mut_or_insert_with(search_id, || "".to_string()).clone());
                    ui.add_sized([120.0, 18.0], egui::TextEdit::singleline(&mut search_query).hint_text("Search Effects..."));
                    ctx.data_mut(|d| d.insert_temp(search_id, search_query));
                });
            });
        });
}
