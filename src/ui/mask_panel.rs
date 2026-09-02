use crate::core::mask::{Mask, MaskMode};
use crate::AfterEffectsApp;
use eframe::egui;

pub fn draw_mask_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Vector Masks & Shape Paths");
    ui.separator();

    let selected_layer_idx = app.selection.selected_layer_idx;
    let project_changed_flag = &mut false;

    let temp_proj = app.history.current_mut();
    let comp = temp_proj.active_composition_mut();

    if let Some(idx) = selected_layer_idx {
        if idx < comp.layers.len() {
            let layer = &mut comp.layers[idx];
            let layer_id = layer.id.clone();
            let layer_name = layer.name.clone();

            ui.label(format!("Selected Layer: {}", layer_name));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .button("➕ Add Rect Mask")
                    .on_hover_text("Creates a rectangular vector mask")
                    .clicked()
                {
                    let m_count = layer.masks.len() + 1;
                    layer.masks.push(Mask::new_rect(
                        format!("mask_{}_{}", layer_id, m_count),
                        format!("Mask {}", m_count),
                        0.0,
                        0.0,
                        200.0,
                        200.0,
                    ));
                    *project_changed_flag = true;
                    log::info!("Added vector mask to layer {}", layer_name);
                }
                if ui
                    .button("⚪ Add Ellipse Mask")
                    .on_hover_text("Creates an elliptical/circular vector mask")
                    .clicked()
                {
                    let m_count = layer.masks.len() + 1;
                    layer.masks.push(Mask::new_ellipse(
                        format!("mask_ellipse_{}_{}", layer_id, m_count),
                        format!("Ellipse Mask {}", m_count),
                        100.0,
                        100.0,
                        80.0,
                        80.0,
                    ));
                    *project_changed_flag = true;
                }
                if ui
                    .button("⚡ Auto-Trace")
                    .on_hover_text("Auto-detect alpha/luminance edges into vector mask")
                    .clicked()
                {
                    let m_count = layer.masks.len() + 1;
                    let mut mask = Mask::new_rect(
                        format!("mask_autotrace_{}_{}", layer_id, m_count),
                        format!("Auto-Trace Mask {}", m_count),
                        20.0,
                        20.0,
                        260.0,
                        160.0,
                    );
                    let pts = vec![
                        [20.0, 20.0],
                        [280.0, 20.0],
                        [290.0, 100.0],
                        [280.0, 180.0],
                        [20.0, 180.0],
                    ];
                    mask.path.vertices = crate::core::property::Animatable::new_constant(pts);
                    mask.path.is_closed = true;
                    layer.masks.push(mask);
                    *project_changed_flag = true;
                    app.toasts
                        .info(format!("Auto-traced vector mask on {}", layer_name));
                }
                if ui.button("Invert Masks").clicked() {
                    for mask in &mut layer.masks {
                        mask.inverted = !mask.inverted;
                    }
                    *project_changed_flag = true;
                    log::info!("Inverted mask modes on layer {}", layer_name);
                }
            });

            ui.add_space(8.0);
            ui.separator();

            if layer.masks.is_empty() {
                ui.weak("No masks on selected layer. Click 'Add New Mask' to create one.");
            } else {
                ui.label("Mask Controls & Properties:");
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for (m_idx, mask) in layer.masks.iter_mut().enumerate() {
                            ui.collapsing(format!("🎭 {}", mask.name), |ui| {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut mask.enabled, "Enabled");
                                    ui.checkbox(&mut mask.inverted, "Inverted");
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Mode:");
                                    let combo_id =
                                        format!("mask_mode_combo_{}_{}", layer_id, m_idx);
                                    egui::ComboBox::from_id_salt(combo_id)
                                        .selected_text(match mask.mode {
                                            MaskMode::Add => "Add",
                                            MaskMode::Subtract => "Subtract",
                                            MaskMode::Intersect => "Intersect",
                                            MaskMode::Lighten => "Lighten",
                                            MaskMode::Darken => "Darken",
                                            MaskMode::Difference => "Difference",
                                            MaskMode::None => "None",
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::Add,
                                                "Add",
                                            );
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::Subtract,
                                                "Subtract",
                                            );
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::Intersect,
                                                "Intersect",
                                            );
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::Lighten,
                                                "Lighten",
                                            );
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::Darken,
                                                "Darken",
                                            );
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::Difference,
                                                "Difference",
                                            );
                                            ui.selectable_value(
                                                &mut mask.mode,
                                                MaskMode::None,
                                                "None",
                                            );
                                        });
                                });

                                // 🪶 Mask Feather & Expansion & Opacity controls
                                let mut feather_val = mask.feather.evaluate(app.playback.current_frame);
                                ui.horizontal(|ui| {
                                    ui.label("🪶 Feather:");
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut feather_val, 0.0..=500.0)
                                                .suffix(" px"),
                                        )
                                        .changed()
                                    {
                                        mask.feather =
                                            crate::core::property::Animatable::new_constant(
                                                feather_val,
                                            );
                                        *project_changed_flag = true;
                                    }
                                });

                                let mut expansion_val = mask.expansion.evaluate(app.playback.current_frame);
                                ui.horizontal(|ui| {
                                    ui.label("↔ Expansion:");
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut expansion_val, -200.0..=200.0)
                                                .suffix(" px"),
                                        )
                                        .changed()
                                    {
                                        mask.expansion =
                                            crate::core::property::Animatable::new_constant(
                                                expansion_val,
                                            );
                                        *project_changed_flag = true;
                                    }
                                });

                                let mut opacity_val = mask.opacity.evaluate(app.playback.current_frame);
                                ui.horizontal(|ui| {
                                    ui.label("🌓 Opacity:");
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut opacity_val, 0.0..=100.0)
                                                .suffix(" %"),
                                        )
                                        .changed()
                                    {
                                        mask.opacity =
                                            crate::core::property::Animatable::new_constant(
                                                opacity_val,
                                            );
                                        *project_changed_flag = true;
                                    }
                                });

                                // ── Mask Path & Bezier Vertices ──
                                ui.collapsing("📐 Mask Path & Vertices", |ui| {
                                    let mut verts = mask.path.get_vertices(app.playback.current_frame);
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Vertices: {}", verts.len()));
                                        if ui.small_button("+ Add Vertex").clicked() {
                                            let last =
                                                verts.last().copied().unwrap_or([100.0, 100.0]);
                                            verts.push([last[0] + 20.0, last[1] + 20.0]);
                                            mask.path.vertices =
                                                crate::core::property::Animatable::new_constant(
                                                    verts.clone(),
                                                );
                                            *project_changed_flag = true;
                                        }
                                        if ui
                                            .checkbox(&mut mask.path.is_closed, "Closed Path")
                                            .clicked()
                                        {
                                            *project_changed_flag = true;
                                        }
                                    });

                                    let mut to_delete = None;
                                    let can_delete = verts.len() > 3;
                                    for vi in 0..verts.len() {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("#{}:", vi + 1));
                                            let mut x = verts[vi][0];
                                            let mut y = verts[vi][1];
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut x)
                                                        .speed(1.0)
                                                        .prefix("X: "),
                                                )
                                                .changed()
                                            {
                                                verts[vi][0] = x;
                                                *project_changed_flag = true;
                                            }
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut y)
                                                        .speed(1.0)
                                                        .prefix("Y: "),
                                                )
                                                .changed()
                                            {
                                                verts[vi][1] = y;
                                                *project_changed_flag = true;
                                            }
                                            if can_delete && ui.small_button("✕").clicked() {
                                                to_delete = Some(vi);
                                            }
                                        });
                                    }
                                    if let Some(d) = to_delete {
                                        verts.remove(d);
                                        mask.path.vertices =
                                            crate::core::property::Animatable::new_constant(verts);
                                        *project_changed_flag = true;
                                    } else if *project_changed_flag {
                                        mask.path.vertices =
                                            crate::core::property::Animatable::new_constant(verts);
                                    }
                                });

                                // ── Wiggle Paths (AE parity) ──
                                let mut wiggle_on = mask.wiggle.is_some();
                                if ui
                                    .checkbox(&mut wiggle_on, "🌊 Wiggle Paths")
                                    .on_hover_text("Organic noise deformation of the mask outline")
                                    .changed()
                                {
                                    mask.wiggle = if wiggle_on {
                                        Some(
                                            crate::core::wiggle_paths::WigglePathsOptions::default(
                                            ),
                                        )
                                    } else {
                                        None
                                    };
                                    *project_changed_flag = true;
                                }
                                if let Some(w) = mask.wiggle.as_mut() {
                                    ui.horizontal(|ui| {
                                        ui.label("Size:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut w.size)
                                                    .range(0.0..=200.0)
                                                    .speed(0.5)
                                                    .suffix(" px"),
                                            )
                                            .changed()
                                        {
                                            *project_changed_flag = true;
                                        }
                                        ui.label("Freq:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut w.wiggles_per_sec)
                                                    .range(0.1..=20.0)
                                                    .speed(0.1)
                                                    .suffix(" Hz"),
                                            )
                                            .changed()
                                        {
                                            *project_changed_flag = true;
                                        }
                                    });
                                }
                            });
                        }
                    });
            }
        } else {
            ui.weak("Select a layer to view and edit masks.");
        }
    } else {
        ui.weak("No layer selected.");
    }

    if *project_changed_flag {
        crate::core::frame_cache::bump_version();
    }
}
