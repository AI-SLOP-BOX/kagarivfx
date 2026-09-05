use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_automation_panel {
        return;
    }
    let mut open = app.show_automation_panel;
    let mut remove_index = None;
    let mut selected_binding = app.selected_automation_binding;
    let mut move_key: Option<(usize, crate::core::unified_time::Time, i64)> = None;
    let mut undo_requested = false;
    let mut redo_requested = false;
    let before_bindings = app
        .production_document
        .as_ref()
        .map(|document| document.bindings.clone());
    let frame_rate = crate::core::unified_time::FrameRate::new(
        app.history.current().active_composition().fps.max(1),
        1,
    );
    egui::Window::new("🎚 Automation Bindings")
        .open(&mut open)
        .default_width(430.0)
        .default_height(300.0)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 500.0))
        .show(ctx, |ui| {
            let Some(document) = app.production_document.as_mut() else {
                ui.label(
                    egui::RichText::new("No unified production document loaded.")
                        .color(colors::TEXT_SECONDARY),
                );
                return;
            };
            let rate = frame_rate;
            let time = rate.map(|rate| {
                crate::core::unified_time::Time::from_frame(app.playback.current_frame as i64, rate)
            });
            ui.label(
                egui::RichText::new(format!(
                    "{} binding(s) · frame {}",
                    document.bindings.len(),
                    app.playback.current_frame
                ))
                .color(colors::TEXT_SECONDARY),
            );
            ui.horizontal(|ui| {
                if ui
                    .button("📈 Open Graph Editor")
                    .on_hover_text("Open the keyframe Graph Editor for the selected layer")
                    .clicked()
                {
                    app.show_graph_editor = true;
                }
                ui.label(
                    egui::RichText::new("Shared timeline automation")
                        .small()
                        .color(colors::TEXT_MUTED),
                );
            });
            if ui.button("＋ Add Binding").clicked() {
                document
                    .bindings
                    .push(crate::core::automation_binding::AutomationBinding {
                        source: "audio.source".into(),
                        target: "vfx.parameter".into(),
                        curve: crate::core::automation_binding::AutomationCurve {
                            points: vec![crate::core::automation_binding::AutomationPoint {
                                time: crate::core::unified_time::Time::ZERO,
                                value: 0.0,
                            }],
                        },
                        input_min: 0.0,
                        input_max: 1.0,
                        output_min: 0.0,
                        output_max: 1.0,
                    });
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!app.automation_undo.is_empty(), egui::Button::new("Undo"))
                    .clicked()
                {
                    undo_requested = true;
                }
                if ui
                    .add_enabled(!app.automation_redo.is_empty(), egui::Button::new("Redo"))
                    .clicked()
                {
                    redo_requested = true;
                }
            });
            if document.bindings.is_empty() {
                ui.label("No bindings configured.");
            }
            egui::ScrollArea::vertical().show_rows(
                ui,
                150.0,
                document.bindings.len(),
                |ui, range| {
                    for index in range {
                        let binding = &mut document.bindings[index];
                        let value = time.and_then(|time| binding.evaluate(time));
                        let row_response = ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Source");
                                ui.add_sized(
                                    [120.0, 20.0],
                                    egui::TextEdit::singleline(&mut binding.source),
                                );
                                ui.label("→");
                                ui.label("Target");
                                ui.add_sized(
                                    [140.0, 20.0],
                                    egui::TextEdit::singleline(&mut binding.target),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Input");
                                ui.add(egui::DragValue::new(&mut binding.input_min).speed(0.01));
                                ui.add(egui::DragValue::new(&mut binding.input_max).speed(0.01));
                                ui.label("Output");
                                ui.add(egui::DragValue::new(&mut binding.output_min).speed(0.1));
                                ui.add(egui::DragValue::new(&mut binding.output_max).speed(0.1));
                            });
                            if let Some(value) = value {
                                ui.label(
                                    egui::RichText::new(format!("Current: {value:.3}"))
                                        .color(colors::ACCENT_GREEN),
                                );
                            }
                            if let Some(time) = time {
                                let key_value =
                                    binding.curve.sample(time).unwrap_or(binding.input_min);
                                if ui
                                    .small_button("◆ Key")
                                    .on_hover_text(
                                        "Add or update an automation key at the current frame",
                                    )
                                    .clicked()
                                {
                                    let _ = binding.curve.upsert_point(time, key_value);
                                }
                                if ui
                                    .small_button("⌫ Key")
                                    .on_hover_text("Remove the automation key at the current frame")
                                    .clicked()
                                {
                                    let _ = binding.curve.remove_point_at(time);
                                }
                            }
                            if let Err(error) = binding.validate() {
                                ui.label(
                                    egui::RichText::new(error).small().color(colors::ACCENT_RED),
                                );
                            }
                            ui.label(format!(
                                "Keys ({}) — first 4 shown",
                                binding.curve.points.len()
                            ));
                            for (key_index, point) in
                                binding.curve.points.iter_mut().take(4).enumerate()
                            {
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "#{key_index} t={}/{}",
                                        point.time.numerator, point.time.denominator
                                    ));
                                    ui.add(egui::DragValue::new(&mut point.value).speed(0.01));
                                });
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text("Remove this automation binding")
                                .clicked()
                            {
                                remove_index = Some(index);
                            }
                        });
                        if row_response.response.clicked() {
                            selected_binding = Some(index);
                        }
                    }
                },
            );
            if let Some(index) = selected_binding.filter(|index| *index < document.bindings.len()) {
                ui.separator();
                ui.label(format!("Selected binding: {index} — all keys"));
                let binding = &mut document.bindings[index];
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for (key_index, point) in binding.curve.points.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{key_index}"));
                                if ui.small_button("◀").clicked() {
                                    move_key = Some((index, point.time, -1));
                                }
                                ui.add(egui::DragValue::new(&mut point.time.numerator).speed(1));
                                ui.label("/");
                                ui.add(
                                    egui::DragValue::new(&mut point.time.denominator)
                                        .speed(1)
                                        .range(1..=u32::MAX),
                                );
                                ui.add(egui::DragValue::new(&mut point.value).speed(0.01));
                                if ui.small_button("▶").clicked() {
                                    move_key = Some((index, point.time, 1));
                                }
                            });
                        }
                    });
                ui.label("Curve preview");
                let (plot_rect, plot_response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 110.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(plot_rect);
                painter.rect_filled(plot_rect, 3.0, colors::BG_PANEL);
                painter.rect_stroke(
                    plot_rect,
                    3.0,
                    egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM),
                );
                if !binding.curve.points.is_empty() {
                    let min_time = binding
                        .curve
                        .points
                        .first()
                        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
                        .unwrap_or(0.0);
                    let max_time = binding
                        .curve
                        .points
                        .last()
                        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
                        .unwrap_or(min_time + 1.0)
                        .max(min_time + f32::EPSILON);
                    let min_value = binding
                        .curve
                        .points
                        .iter()
                        .map(|point| point.value as f32)
                        .fold(f32::INFINITY, f32::min);
                    let max_value = binding
                        .curve
                        .points
                        .iter()
                        .map(|point| point.value as f32)
                        .fold(f32::NEG_INFINITY, f32::max)
                        .max(min_value + f32::EPSILON);
                    let to_screen = |time: f32, value: f32| {
                        egui::pos2(
                            plot_rect.left()
                                + ((time - min_time) / (max_time - min_time)).clamp(0.0, 1.0)
                                    * plot_rect.width(),
                            plot_rect.bottom()
                                - ((value - min_value) / (max_value - min_value)).clamp(0.0, 1.0)
                                    * plot_rect.height(),
                        )
                    };
                    let sampled: Vec<_> = (0..=64)
                        .filter_map(|sample| {
                            let normalized = sample as f32 / 64.0;
                            let time = min_time + normalized * (max_time - min_time);
                            let rational_time = crate::core::unified_time::Time::new(
                                (time * 1_000_000.0).round() as i64,
                                1_000_000,
                            );
                            binding
                                .curve
                                .sample(rational_time)
                                .map(|value| to_screen(time, value as f32))
                        })
                        .collect();
                    for segment in sampled.windows(2) {
                        painter.line_segment(
                            [segment[0], segment[1]],
                            egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE),
                        );
                    }
                    let key_points: Vec<_> = binding
                        .curve
                        .points
                        .iter()
                        .map(|point| {
                            to_screen(
                                point.time.numerator as f32 / point.time.denominator as f32,
                                point.value as f32,
                            )
                        })
                        .collect();
                    for point in key_points {
                        painter.circle_filled(point, 3.5, colors::ACCENT_ORANGE);
                    }

                    if plot_response.dragged() {
                        if let Some(pointer) = plot_response.interact_pointer_pos() {
                            let normalized_time = ((pointer.x - plot_rect.left())
                                / plot_rect.width().max(1.0))
                            .clamp(0.0, 1.0);
                            let normalized_value = ((plot_rect.bottom() - pointer.y)
                                / plot_rect.height().max(1.0))
                            .clamp(0.0, 1.0);
                            let target_time = min_time + normalized_time * (max_time - min_time);
                            let nearest = binding
                                .curve
                                .points
                                .iter()
                                .enumerate()
                                .min_by(|(_, left), (_, right)| {
                                    let left_time =
                                        left.time.numerator as f32 / left.time.denominator as f32;
                                    let right_time =
                                        right.time.numerator as f32 / right.time.denominator as f32;
                                    (left_time - target_time)
                                        .abs()
                                        .total_cmp(&(right_time - target_time).abs())
                                })
                                .map(|(index, point)| (index, point.time));
                            if let Some((index, from)) = nearest {
                                if ui.input(|input| input.modifiers.shift) {
                                    let new_time = crate::core::unified_time::Time::new(
                                        (target_time * 1_000_000.0).round() as i64,
                                        1_000_000,
                                    );
                                    if new_time != from {
                                        let _ = binding.curve.move_point(from, new_time);
                                    }
                                } else if let Some(point) = binding.curve.points.get_mut(index) {
                                    point.value = (min_value
                                        + normalized_value * (max_value - min_value))
                                        as f64;
                                }
                            }
                        }
                    }
                    if plot_response.double_clicked() {
                        if let Some(pointer) = plot_response.interact_pointer_pos() {
                            let normalized_time = ((pointer.x - plot_rect.left())
                                / plot_rect.width().max(1.0))
                            .clamp(0.0, 1.0);
                            let normalized_value = ((plot_rect.bottom() - pointer.y)
                                / plot_rect.height().max(1.0))
                            .clamp(0.0, 1.0);
                            let time = min_time + normalized_time * (max_time - min_time);
                            let value = min_value + normalized_value * (max_value - min_value);
                            let _ = binding.curve.upsert_point(
                                crate::core::unified_time::Time::new(
                                    (time * 1_000_000.0).round() as i64,
                                    1_000_000,
                                ),
                                value as f64,
                            );
                        }
                    }
                }
            }
        });
    if let Some(index) = remove_index {
        if let Some(document) = app.production_document.as_mut() {
            if index < document.bindings.len() {
                document.bindings.remove(index);
                if selected_binding == Some(index) {
                    selected_binding = None;
                }
                app.toasts.info("Automation binding removed");
            }
        }
    }
    if let Some(before) = before_bindings {
        app.record_automation_edit(before);
    }
    if let Some((index, from, frames)) = move_key {
        if let (Some(document), Some(rate)) = (app.production_document.as_mut(), frame_rate) {
            if let Some(binding) = document.bindings.get_mut(index) {
                let _ = binding.curve.move_point_by_frames(from, frames, rate);
            }
        }
    }
    if undo_requested {
        app.undo_automation_edit();
    } else if redo_requested {
        app.redo_automation_edit();
    }
    app.show_automation_panel = open;
    app.selected_automation_binding = selected_binding.filter(|index| {
        app.production_document
            .as_ref()
            .is_some_and(|document| *index < document.bindings.len())
    });
}
