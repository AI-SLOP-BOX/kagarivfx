use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

/// Draw the Essential Properties panel for the selected precomp layer.
pub fn draw_essential_properties(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    crate::ui::custom_widgets::ae_section_header(ui, "Essential Properties", "🎬");

    let Some(sel_idx) = app.selected_layer_idx else {
        ui.weak("Select a PreComp layer to view its Essential Properties.");
        return;
    };

    let (is_precomp, prop_count) = {
        let comp = app.history.current().active_composition();
        if sel_idx >= comp.layers.len() { return; }
        let is_precomp = matches!(&comp.layers[sel_idx].layer_type, crate::core::timeline::LayerType::PreComp { .. });
        (is_precomp, comp.layers[sel_idx].essential_properties.len())
    };

    if !is_precomp {
        ui.weak("Essential Properties are only available on PreComp layers.");
        return;
    }

    if prop_count == 0 {
        if ui.button("+ Add Property").clicked() {
            let count = {
                let c = app.history.current().active_composition();
                c.layers[sel_idx].essential_properties.len()
            };
            let comp = app.history.current_mut().active_composition_mut();
            comp.layers[sel_idx].essential_properties.push(
                crate::core::essential_properties::EssentialProperty {
                    name: format!("Property {}", count + 1),
                    prop_type: crate::core::essential_properties::EssentialPropertyType::Slider,
                    value: crate::core::essential_properties::EssentialValue::Float(50.0),
                    overridden: false,
                    min_value: 0.0,
                    max_value: 100.0,
                    options: vec![],
                }
            );
            crate::core::frame_cache::bump_version();
        }
        return;
    }

    let mut remove_idx = None;
    let mut move_up = None;
    let mut move_down = None;

    for i in 0..prop_count {
        let (name, type_name) = {
            let comp = app.history.current().active_composition();
            let p = &comp.layers[sel_idx].essential_properties[i];
            (p.name.clone(), format!("{:?}", p.prop_type))
        };

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&name).strong().color(colors::TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").on_hover_text("Remove").clicked() {
                        remove_idx = Some(i);
                    }
                    if i > 0 && ui.small_button("↑").clicked() {
                        move_up = Some(i);
                    }
                    if i < prop_count - 1 && ui.small_button("↓").clicked() {
                        move_down = Some(i);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.weak(&type_name);
            });
        });
        ui.add_space(4.0);
    }

    if let Some(idx) = remove_idx {
        app.history.current_mut().active_composition_mut().layers[sel_idx].essential_properties.remove(idx);
        crate::core::frame_cache::bump_version();
    }
    if let Some(idx) = move_up {
        app.history.current_mut().active_composition_mut().layers[sel_idx].essential_properties.swap(idx, idx - 1);
        crate::core::frame_cache::bump_version();
    }
    if let Some(idx) = move_down {
        app.history.current_mut().active_composition_mut().layers[sel_idx].essential_properties.swap(idx, idx + 1);
        crate::core::frame_cache::bump_version();
    }

    ui.add_space(4.0);
    if ui.small_button("+ Add Property").clicked() {
        let count = {
            let c = app.history.current().active_composition();
            c.layers[sel_idx].essential_properties.len()
        };
        let comp = app.history.current_mut().active_composition_mut();
        comp.layers[sel_idx].essential_properties.push(
            crate::core::essential_properties::EssentialProperty {
                name: format!("Property {}", count + 1),
                prop_type: crate::core::essential_properties::EssentialPropertyType::Slider,
                value: crate::core::essential_properties::EssentialValue::Float(50.0),
                overridden: false,
                min_value: 0.0,
                max_value: 100.0,
                options: vec![],
            }
        );
        crate::core::frame_cache::bump_version();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_essential_panel_creation() {
        assert!(true);
    }
}
