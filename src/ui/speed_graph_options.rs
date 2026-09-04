use crate::core::keyframe::InterpolationType;
use crate::core::property::Animatable;
use crate::KagariApp;
use eframe::egui;

/// Applies a custom bezier derived from AE velocity/influence sliders to every
/// keyframe of the selected track. Influence maps to the x control points
/// (33.3% -> 0.25, matching Easy Ease); speed biases the y handles so higher
/// incoming speed sharpens the approach.
fn apply_velocity_bezier<T: Clone + crate::core::property::Interpolate>(
    anim: &mut Animatable<T>,
    in_influence: f32,
    in_speed: f32,
    out_influence: f32,
    out_speed: f32,
) {
    if let Some(kfs) = anim.keyframes_mut() {
        let x1 = (out_influence / 100.0 * 0.75).clamp(0.0, 0.9);
        let x2 = 1.0 - (in_influence / 100.0 * 0.75).clamp(0.0, 0.9);
        // Map speed (0..2000 px/s) to a normalized y bias
        let y1 = 0.1 + (out_speed / 2000.0).clamp(0.0, 1.0) * 0.8;
        let y2 = 0.9 - (in_speed / 2000.0).clamp(0.0, 1.0) * 0.8;
        for kf in kfs.iter_mut() {
            kf.interpolation = InterpolationType::Bezier {
                outgoing: crate::core::keyframe::BezierControlPoint {
                    influence: x1,
                    speed: out_speed,
                },
                incoming: crate::core::keyframe::BezierControlPoint {
                    influence: x2,
                    speed: in_speed,
                },
                custom_bezier: Some([x1, y1, x2, y2]),
            };
        }
    }
}

pub fn draw_speed_graph_options(app: &mut KagariApp, ui: &mut egui::Ui, current_frame: u32) {
    ui.heading("Graph Editor Options & Keyframe Velocity");
    ui.separator();

    ui.label("Graph Type:");
    let graph_mode_id = egui::Id::new("ae_graph_mode_select");
    let mut graph_mode = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(graph_mode_id, || 0));

    ui.horizontal(|ui| {
        if ui
            .selectable_value(&mut graph_mode, 0, "Edit Speed Graph")
            .clicked()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(graph_mode_id, graph_mode));
        }
        if ui
            .selectable_value(&mut graph_mode, 1, "Edit Value Graph")
            .clicked()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(graph_mode_id, graph_mode));
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("Keyframe Velocity & Influence (applies to the selected layer's graph property):");

    let vel_id = egui::Id::new("ae_velocity_sliders");
    let (mut in_influence, mut in_speed, mut out_influence, mut out_speed) =
        ui.ctx().data_mut(|d| {
            *d.get_temp_mut_or_insert_with(vel_id, || (33.3f32, 0.0f32, 33.3f32, 0.0f32))
        });

    ui.horizontal(|ui| {
        ui.label("Incoming Speed:");
        ui.add(egui::Slider::new(&mut in_speed, 0.0..=2000.0).suffix(" px/s"));
        ui.label("Influence:");
        ui.add(egui::Slider::new(&mut in_influence, 0.0..=100.0).suffix("%"));
    });
    ui.horizontal(|ui| {
        ui.label("Outgoing Speed:");
        ui.add(egui::Slider::new(&mut out_speed, 0.0..=2000.0).suffix(" px/s"));
        ui.label("Influence:");
        ui.add(egui::Slider::new(&mut out_influence, 0.0..=100.0).suffix("%"));
    });
    ui.ctx()
        .data_mut(|d| d.insert_temp(vel_id, (in_influence, in_speed, out_influence, out_speed)));

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        if ui
            .button("Apply Velocity")
            .on_hover_text(
                "Write the velocity/influence values into the selected track's keyframes",
            )
            .clicked()
        {
            let Some(layer_idx) = app.selection.selected_layer_idx else {
                app.toasts.error("Select a layer first");
                return;
            };
            let prop = app
                .selection.selected_property
                .clone()
                .unwrap_or_else(|| "Position X".into());
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let Some(layer) = comp.layers.get_mut(layer_idx) else {
                    return;
                };
                let t = &mut layer.transform;
                match prop.as_str() {
                    "Position X" | "Position Y" => apply_velocity_bezier(
                        &mut t.position,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    "Scale X" | "Scale Y" => apply_velocity_bezier(
                        &mut t.scale,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    "Rotation" => apply_velocity_bezier(
                        &mut t.rotation,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    "Opacity" => apply_velocity_bezier(
                        &mut t.opacity,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    _ => {}
                }
            });
            app.toasts.info(format!("Velocity applied to {}", prop));
        }
        if ui
            .button("Easy Ease (F9)")
            .on_hover_text("Symmetrical smooth ease on all keyframes")
            .clicked()
        {
            apply_preset_ease(app, 33.3, 0.0, 33.3, 0.0, |x1, _y1, x2, _y2| {
                [x1, 0.1, x2, 1.0]
            });
        }
        if ui
            .button("Easy Ease In")
            .on_hover_text("Ease into the next keyframe")
            .clicked()
        {
            apply_preset_ease(app, 33.3, 0.0, 33.3, 0.0, |_x1, _y1, x2, _y2| {
                [0.0, 0.0, x2, 1.0]
            });
        }
        if ui
            .button("Easy Ease Out")
            .on_hover_text("Ease out of the previous keyframe")
            .clicked()
        {
            apply_preset_ease(app, 33.3, 0.0, 33.3, 0.0, |x1, _y1, _x2, _y2| {
                [x1, 0.1, 1.0, 1.0]
            });
        }
    });
    let _ = current_frame;
}

/// Applies an ease preset to the selected layer's graph property keyframes.
fn apply_preset_ease(
    app: &mut KagariApp,
    in_inf: f32,
    _in_spd: f32,
    out_inf: f32,
    _out_spd: f32,
    coords: impl Fn(f32, f32, f32, f32) -> [f32; 4],
) {
    let Some(layer_idx) = app.selection.selected_layer_idx else {
        app.toasts.error("Select a layer first");
        return;
    };
    let prop = app
        .selection.selected_property
        .clone()
        .unwrap_or_else(|| "Position X".into());
    let x1 = out_inf / 100.0 * 0.75;
    let x2 = 1.0 - in_inf / 100.0 * 0.75;
    app.modify_project(|p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(layer_idx) else {
            return;
        };
        let t = &mut layer.transform;
        fn apply_to<T: Clone + crate::core::property::Interpolate>(
            anim: &mut Animatable<T>,
            x1: f32,
            x2: f32,
            coords: &dyn Fn(f32, f32, f32, f32) -> [f32; 4],
        ) {
            if let Some(kfs) = anim.keyframes_mut() {
                for kf in kfs.iter_mut() {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: crate::core::keyframe::BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: crate::core::keyframe::BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(coords(x1, 0.0, x2, 1.0)),
                    };
                }
            }
        }
        match prop.as_str() {
            "Position X" | "Position Y" => apply_to(&mut t.position, x1, x2, &coords),
            "Scale X" | "Scale Y" => apply_to(&mut t.scale, x1, x2, &coords),
            "Rotation" => apply_to(&mut t.rotation, x1, x2, &coords),
            "Opacity" => apply_to(&mut t.opacity, x1, x2, &coords),
            _ => {}
        }
    });
    app.toasts.info(format!("Ease applied to {}", prop));
}
