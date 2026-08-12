use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_flowchart_inspector(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Composition Flowchart & Hierarchy");
    ui.separator();

    let comp = app.history.current().active_composition();
    ui.label(format!("Root Composition: {}", comp.name));
    ui.label(format!("Total Layers: {}", comp.layers.len()));

    ui.add_space(8.0);
    ui.separator();

    ui.label("Composition Dependency Graph (DAG):");
    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        ui.monospace("📽 [Comp 1] (Active Root)");
        ui.indent("comp_root_indent", |ui| {
            for (i, layer) in comp.layers.iter().enumerate() {
                let icon = match layer.layer_type {
                    crate::core::timeline::LayerType::Text { .. } => "Text",
                    crate::core::timeline::LayerType::Solid { .. } => "Solid",
                    crate::core::timeline::LayerType::Image { .. } => "Image",
                    crate::core::timeline::LayerType::Shape { .. } => "Shape",
                    crate::core::timeline::LayerType::Null => "Null",
                    crate::core::timeline::LayerType::PreComp { .. } => "PreComp",
                    crate::core::timeline::LayerType::Audio { .. } => "Audio",
                };
                let parent_info = if let Some(ref p_id) = layer.parent_id {
                    format!(" 🔗 Parent: {}", p_id)
                } else {
                    "".to_string()
                };
                ui.label(format!("└─ {:02}. {} {}{}", i + 1, icon, layer.name, parent_info));
            }
        });
    });
}
