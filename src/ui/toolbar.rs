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

                let tool_id = egui::Id::new("active_tool_selection");
                let mut current_tool = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(tool_id, || app.active_tool));

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
                    let is_selected = current_tool == tool;
                    if ui.selectable_label(is_selected, label).clicked() {
                        current_tool = tool;
                        app.active_tool = tool;
                        ctx.data_mut(|d| d.insert_temp(tool_id, tool));
                    }
                }
                app.active_tool = current_tool;

                ui.separator();
                ui.add_space(8.0);

                // Workspace Layout Selectors (AE Workspaces)
                ui.label(egui::RichText::new("Workspace:").small().color(egui::Color32::from_gray(180)));
                if ui.selectable_label(true, "Default").clicked() {}
                if ui.selectable_label(false, "Color").clicked() {}
                if ui.selectable_label(false, "Effects").clicked() {}
                if ui.selectable_label(false, "Motion Graphics").clicked() {}
            });
        });
}
