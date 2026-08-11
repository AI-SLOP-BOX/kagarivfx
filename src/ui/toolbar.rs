use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTool {
    #[default]
    Selection,
    Hand,
    Zoom,
    Rotation,
    AnchorPoint,
    Rectangle,
    Pen,
    Text,
}

#[allow(dead_code)]
pub fn draw(app: &mut crate::AfterEffectsApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("ae_toolbar")
        .default_height(28.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 2.0;

                let tools = [
                    (ActiveTool::Selection, "Select (V)"),
                    (ActiveTool::Hand, "Hand (H)"),
                    (ActiveTool::Zoom, "Zoom (Z)"),
                    (ActiveTool::Rotation, "Rotate (W)"),
                    (ActiveTool::AnchorPoint, "Anchor (Y)"),
                    (ActiveTool::Rectangle, "Shape (Q)"),
                    (ActiveTool::Pen, "Pen (G)"),
                    (ActiveTool::Text, "Text (⌘T)"),
                ];

                for (tool, label) in tools {
                    let is_selected = app.active_tool == tool;
                    if ui.selectable_label(is_selected, label).clicked() {
                        app.active_tool = tool;
                    }
                }

                ui.separator();
                ui.add_space(8.0);

                // Workspace Layout Selectors (AE Workspaces)
                ui.label(egui::RichText::new("Workspace:").small().color(egui::Color32::from_gray(180)));
                let ws_default = app.left_tab_idx == 0 && app.right_tab_idx == 0;
                if ui.selectable_label(ws_default, "Default").clicked() {
                    app.left_tab_idx = 0;
                    app.right_tab_idx = 0;
                }
                let ws_color = app.left_tab_idx == 1;
                if ui.selectable_label(ws_color, "Color").clicked() {
                    app.left_tab_idx = 1;
                    app.right_tab_idx = 0;
                }
                let ws_fx = app.right_tab_idx == 0;
                if ui.selectable_label(ws_fx, "Effects").clicked() {
                    app.right_tab_idx = 0;
                }
                let ws_mg = app.right_tab_idx == 1;
                if ui.selectable_label(ws_mg, "Motion Graphics").clicked() {
                    app.right_tab_idx = 1;
                }
            });
        });
}
