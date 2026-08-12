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
    let frame = egui::Frame::none()
        .fill(egui::Color32::from_rgb(26, 26, 26))
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)));

    egui::TopBottomPanel::top("ae_toolbar")
        .frame(frame)
        .default_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 3.0;

                let tools = [
                    (ActiveTool::Selection, "↖ Select (V)"),
                    (ActiveTool::Hand, "✋ Hand (H)"),
                    (ActiveTool::Zoom, "🔍 Zoom (Z)"),
                    (ActiveTool::Camera3D, "📷 Camera (C)"),
                    (ActiveTool::Rotation, "🔄 Rotate (W)"),
                    (ActiveTool::AnchorPoint, "🎯 Anchor (Y)"),
                    (ActiveTool::Rectangle, "▭ Shape (Q)"),
                    (ActiveTool::Pen, "✒ Pen (G)"),
                    (ActiveTool::Text, "T Text (⌘T)"),
                    (ActiveTool::Brush, "🖌 Brush"),
                    (ActiveTool::CloneStamp, "🔀 Stamp"),
                    (ActiveTool::Eraser, "🧹 Eraser"),
                    (ActiveTool::RotoBrush, "✂ Roto"),
                    (ActiveTool::PuppetPin, "📌 Puppet"),
                ];

                for (tool, label) in tools {
                    let is_selected = app.active_tool == tool;
                    let text = if is_selected {
                        egui::RichText::new(label).strong().color(egui::Color32::from_rgb(0, 180, 255))
                    } else {
                        egui::RichText::new(label).color(egui::Color32::from_rgb(200, 200, 200))
                    };

                    let btn = egui::Button::new(text)
                        .fill(if is_selected { egui::Color32::from_rgb(40, 50, 70) } else { egui::Color32::TRANSPARENT })
                        .stroke(if is_selected { egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 160, 240)) } else { egui::Stroke::NONE });

                    if ui.add(btn).clicked() {
                        app.active_tool = tool;
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Workspace Layout Selectors (AE Workspaces)
                ui.label(egui::RichText::new("Workspace:").small().color(egui::Color32::from_gray(160)));
                let ws_default = app.left_tab_idx == 0 && app.right_tab_idx == 0;
                if ui.selectable_label(ws_default, "Default").clicked() {
                    app.left_tab_idx = 0;
                    app.right_tab_idx = 0;
                }
                let ws_color = app.left_tab_idx == 1;
                if ui.selectable_label(ws_color, "Color & FX").clicked() {
                    app.left_tab_idx = 1;
                    app.right_tab_idx = 0;
                }
                let ws_mg = app.right_tab_idx == 1;
                if ui.selectable_label(ws_mg, "Motion Graphics").clicked() {
                    app.right_tab_idx = 1;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("⚡ Render Queue (Cmd+M)").strong().color(egui::Color32::from_rgb(255, 200, 80)))
                        .clicked()
                    {
                        app.show_export_dialog = true;
                    }
                });
            });
        });
}
