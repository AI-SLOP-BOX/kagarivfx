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
                if ui.button("▶ Test Expression").on_hover_text("Execute the expression at the current frame and show the result").clicked() {
                    use crate::core::expression_engine::{build_comp_snapshot, eval_f32_with_comp};
                    let current_frame = app.current_frame;
                    let project = app.history.current();
                    let comp = project.active_composition();
                    let snap = build_comp_snapshot(comp, current_frame);
                    let this_layer = comp.layers.iter().find(|l| &l.name == layer_name)
                        .and_then(|l| snap.layers.get(&l.name).cloned());
                    let base = crate::core::timeline::Expression::evaluate_v2(
                        &crate::core::timeline::Expression::Raw(script.clone()),
                        [0.0, 0.0], current_frame, comp.fps.max(1),
                    );
                    let result = eval_f32_with_comp(&script, base[0], current_frame, comp.fps.max(1), &snap, this_layer.as_ref());
                    let msg = if result != base[0] || script.contains("value") || script.contains("this") {
                        format!("Result @f{}: {:.3}", current_frame, result)
                    } else {
                        format!("Result @f{}: {:.3} (base)", current_frame, result)
                    };
                    app.toasts.info(msg);
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
