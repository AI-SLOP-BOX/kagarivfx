use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_scripting_console(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("ExtendScript / Scripting Console");
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("▶ Run Script File (.jsx)").clicked() {
            log::info!("Executing ExtendScript file...");
        }
        if ui.button("🗑 Clear Console Output").clicked() {
            log::info!("Console cleared");
        }
    });

    ui.add_space(6.0);
    ui.label("Console Output:");
    egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
        ui.monospace("[INFO] ExtendScript Engine v4.2.1 initialized.");
        ui.monospace("[INFO] Active Comp: Comp 1 (1920x1080 @ 30fps)");
        ui.monospace("[SUCCESS] Script execution finished in 0.012s");
    });

    ui.add_space(6.0);
    ui.separator();
    ui.label("Interactive Script Command:");
    let cmd_id = egui::Id::new("ae_script_cmd_input");
    let mut cmd_str: String = ui.ctx().data_mut(|d| d.get_temp_mut_or_insert_with(cmd_id, || "".to_string()).clone());
    ui.horizontal(|ui| {
        if ui.add(egui::TextEdit::singleline(&mut cmd_str).hint_text("app.project.activeItem.layers.addSolid(...)")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(cmd_id, cmd_str.clone()));
        }
        if ui.button("Execute").clicked() {
            log::info!("Executed command: {}", cmd_str);
        }
    });
}
