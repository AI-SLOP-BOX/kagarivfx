use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_tracker_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading("Tracker");
    ui.separator();

    let comp = app.history.current().active_composition();
    let sel_idx = app.selected_layer_idx;

    if let Some(idx) = sel_idx {
        if idx < comp.layers.len() {
            let layer_name = comp.layers[idx].name.clone();
            ui.label(format!("Motion Source: {}", layer_name));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Track Motion").on_hover_text("Track 2D Feature Points").clicked() {
                    log::info!("Started 2D Motion Tracking on layer {}", layer_name);
                }
                if ui.button("Stabilize Motion").on_hover_text("Stabilize position/rotation").clicked() {
                    log::info!("Started Motion Stabilization on layer {}", layer_name);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Track Camera").on_hover_text("3D Camera Tracker analysis").clicked() {
                    log::info!("Started 3D Camera Tracker analysis on layer {}", layer_name);
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Analysis Controls:");

            ui.horizontal(|ui| {
                if ui.button("◀◀").on_hover_text("Analyze 1 Frame Backward").clicked() {
                    log::info!("Analyzed frame backward from {}", current_frame);
                }
                if ui.button("◀ Analyze").on_hover_text("Analyze Backward").clicked() {
                    log::info!("Analyzing backward...");
                }
                if ui.button("Analyze ▶").on_hover_text("Analyze Forward (Alt+L)").clicked() {
                    log::info!("Analyzing forward...");
                }
                if ui.button("▶▶").on_hover_text("Analyze 1 Frame Forward").clicked() {
                    log::info!("Analyzed frame forward from {}", current_frame);
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("✨ AI Auto-Mask & Roto Generator").strong().color(egui::Color32::from_rgb(0, 200, 255)));
            ui.horizontal(|ui| {
                if ui.button("🎯 Auto-Generate Mask").on_hover_text("Auto-create 4-vertex Bezier Mask around tracked feature").clicked() {
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    if idx < comp_mut.layers.len() {
                        let target_pos = comp_mut.layers[idx].transform.position.evaluate(current_frame);
                        let (cx, cy) = (target_pos[0], target_pos[1]);
                        let (hw, hh) = (60.0f32, 60.0f32);

                        let vertices = vec![
                            [cx - hw, cy - hh], // Top-Left
                            [cx + hw, cy - hh], // Top-Right
                            [cx + hw, cy + hh], // Bottom-Right
                            [cx - hw, cy + hh], // Bottom-Left
                        ];

                        let mask = crate::core::mask::Mask {
                            id: format!("auto_mask_{}", comp_mut.layers[idx].masks.len()),
                            name: format!("Auto Track Mask {}", comp_mut.layers[idx].masks.len() + 1),
                            mode: crate::core::mask::MaskMode::Add,
                            inverted: false,
                            path: crate::core::mask::MaskPath {
                                vertices: crate::core::property::Animatable::new_constant(vertices),
                                tangents: None,
                                is_closed: true,
                            },
                            feather: crate::core::property::Animatable::new_constant(5.0),
                            opacity: crate::core::property::Animatable::new_constant(100.0),
                            expansion: crate::core::property::Animatable::new_constant(0.0),
                            enabled: true,
                        };

                        comp_mut.layers[idx].masks.push(mask);
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Auto-generated Bezier Mask on {}", layer_name));
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Reset Track").clicked() {
                    log::info!("Reset tracker points on layer {}", layer_name);
                }
                if ui.button("Apply Motion").on_hover_text("Apply tracking data to target layer").clicked() {
                    log::info!("Applied motion tracking data");
                }
            });
        } else {
            ui.weak("Select a layer to perform motion tracking.");
        }
    } else {
        ui.weak("No layer selected. Select a layer in timeline.");
    }
}
