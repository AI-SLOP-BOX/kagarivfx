#![allow(clippy::too_many_arguments)]

use crate::KagariApp;
use eframe::egui;

pub fn draw_inline_numeric_editor(
    app: &mut KagariApp,
    ctx: &egui::Context,
    current_frame: u32,
    origin_x: f32,
    origin_y: f32,
    draw_w: f32,
    draw_h: f32,
    comp_w: f32,
    comp_h: f32,
) {
    if !app.show_inline_numeric_editor {
        return;
    }

    let sel_idx = match app.selection.selected_layer_idx {
        Some(idx) => idx,
        None => return,
    };

    let mut project_changed = false;
    let mut should_close = false;

    let temp_proj = app.history.current_mut();
    let comp = temp_proj.active_composition_mut();

    if sel_idx >= comp.layers.len() {
        return;
    }

    let layer = &mut comp.layers[sel_idx];
    let pos_now = layer.transform.position.evaluate(current_frame);

    // Calculate canvas screen coordinates for layer center
    let screen_x = origin_x + (pos_now[0] / comp_w) * draw_w;
    let screen_y = origin_y + (pos_now[1] / comp_h) * draw_h;

    let window_pos = egui::pos2(
        (screen_x - 110.0).max(origin_x + 10.0),
        (screen_y - 120.0).max(origin_y + 10.0),
    );

    let mut is_open = true;

    egui::Window::new(format!("✏ Quick Edit: {}", layer.name))
        .fixed_pos(window_pos)
        .fixed_size(egui::vec2(220.0, 140.0))
        .resizable(false)
        .collapsible(false)
        .title_bar(true)
        .open(&mut is_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Position X:");
                let mut px = pos_now[0];
                if ui.add(egui::DragValue::new(&mut px).speed(1.0)).changed() {
                    let new_p = [px, pos_now[1]];
                    layer
                        .transform
                        .position
                        .set_value_at_frame(current_frame, new_p);
                    project_changed = true;
                }
                ui.label("Y:");
                let mut py = pos_now[1];
                if ui.add(egui::DragValue::new(&mut py).speed(1.0)).changed() {
                    let new_p = [pos_now[0], py];
                    layer
                        .transform
                        .position
                        .set_value_at_frame(current_frame, new_p);
                    project_changed = true;
                }
            });

            let scale_now = layer.transform.scale.evaluate(current_frame);
            ui.horizontal(|ui| {
                ui.label("Scale X:");
                let mut sx = scale_now[0];
                if ui
                    .add(egui::DragValue::new(&mut sx).speed(0.5).suffix("%"))
                    .changed()
                {
                    let new_s = [sx, scale_now[1]];
                    layer
                        .transform
                        .scale
                        .set_value_at_frame(current_frame, new_s);
                    project_changed = true;
                }
                ui.label("Y:");
                let mut sy = scale_now[1];
                if ui
                    .add(egui::DragValue::new(&mut sy).speed(0.5).suffix("%"))
                    .changed()
                {
                    let new_s = [scale_now[0], sy];
                    layer
                        .transform
                        .scale
                        .set_value_at_frame(current_frame, new_s);
                    project_changed = true;
                }
            });

            let rot_now = layer.transform.rotation.evaluate(current_frame);
            ui.horizontal(|ui| {
                ui.label("Rotation:");
                let mut rot = rot_now;
                if ui
                    .add(egui::DragValue::new(&mut rot).speed(0.5).suffix("°"))
                    .changed()
                {
                    layer
                        .transform
                        .rotation
                        .set_value_at_frame(current_frame, rot);
                    project_changed = true;
                }
            });

            let op_now = layer.transform.opacity.evaluate(current_frame);
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                let mut op = op_now;
                if ui
                    .add(egui::Slider::new(&mut op, 0.0..=100.0).suffix("%"))
                    .changed()
                {
                    layer
                        .transform
                        .opacity
                        .set_value_at_frame(current_frame, op);
                    project_changed = true;
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Done (Enter)").clicked()
                    || ui.input(|i| {
                        i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)
                    })
                {
                    should_close = true;
                }
            });
        });

    if project_changed {
        crate::core::frame_cache::bump_version();
    }
    if should_close || !is_open {
        app.show_inline_numeric_editor = false;
    }
}
