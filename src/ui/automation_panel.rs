use crate::ui::theme::colors;
use crate::AfterEffectsApp;
use eframe::egui;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_automation_panel {
        return;
    }
    let mut open = app.show_automation_panel;
    let mut remove_index = None;
    let mut selected_binding = app.selected_automation_binding;
    let mut undo_requested = false;
    let mut redo_requested = false;
    let before_bindings = app
        .production_document
        .as_ref()
        .map(|document| document.bindings.clone());
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
            let comp = app.history.current().active_composition();
            let rate = crate::core::unified_time::FrameRate::new(comp.fps.max(1), 1);
            let time = rate.map(|rate| {
                crate::core::unified_time::Time::from_frame(app.current_frame as i64, rate)
            });
            ui.label(
                egui::RichText::new(format!(
                    "{} binding(s) · frame {}",
                    document.bindings.len(),
                    app.current_frame
                ))
                .color(colors::TEXT_SECONDARY),
            );
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
                                ui.add(egui::DragValue::new(&mut point.time.numerator).speed(1));
                                ui.label("/");
                                ui.add(
                                    egui::DragValue::new(&mut point.time.denominator)
                                        .speed(1)
                                        .clamp_range(1..=u32::MAX),
                                );
                                ui.add(egui::DragValue::new(&mut point.value).speed(0.01));
                            });
                        }
                    });
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
