use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_expression_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Expression Engine & Editor");
    ui.separator();

    let comp = app.history.current().active_composition();
    if let Some(idx) = app.selected_layer_idx {
        if idx < comp.layers.len() {
            let layer_name = &comp.layers[idx].name;
            ui.label(format!("Selected Layer: {}", layer_name));

            ui.add_space(4.0);
            let prop_name = app.selected_property.as_deref().unwrap_or("Position");
            ui.label(format!("Target Property: {}", prop_name));

            ui.add_space(6.0);
            let expr_id = egui::Id::new(format!("ae_expr_script_{}_{}", idx, prop_name));
            let mut script: String = ui.ctx().data_mut(|d| {
                d.get_temp_mut_or_insert_with(expr_id, || "wiggle(5, 20)".to_string()).clone()
            });

            ui.label("Expression Script:");
            if ui.add(egui::TextEdit::multiline(&mut script).code_editor().desired_rows(6)).changed() {
                ui.ctx().data_mut(|d| d.insert_temp(expr_id, script.clone()));
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("▶ Test Expression").on_hover_text("Execute JS Expression Sandbox").clicked() {
                    log::info!("Tested expression on layer {}: {}", layer_name, script);
                }
                if ui.button("Preset: wiggle(f, a)").clicked() {
                    script = "wiggle(4, 30)".to_string();
                    ui.ctx().data_mut(|d| d.insert_temp(expr_id, script.clone()));
                }
                if ui.button("Preset: loopOut()").clicked() {
                    script = "loopOut(\"cycle\")".to_string();
                    ui.ctx().data_mut(|d| d.insert_temp(expr_id, script.clone()));
                }
            });
        } else {
            ui.weak("Select a layer and property to write JavaScript expressions.");
        }
    } else {
        ui.weak("No layer selected.");
    }
}
