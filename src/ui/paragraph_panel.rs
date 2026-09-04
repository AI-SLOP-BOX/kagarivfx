use crate::core::text_layout::TextAlign;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_paragraph_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    let comp = app.history.current_mut().active_composition_mut();

    let layer_idx = match app.selection.selected_layer_idx {
        Some(idx) if idx < comp.layers.len() => idx,
        _ => {
            ui.label(
                egui::RichText::new("No layer selected.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            return;
        }
    };

    let layer = &mut comp.layers[layer_idx];

    if !matches!(
        layer.layer_type,
        crate::core::timeline::LayerType::Text { .. }
    ) {
        ui.label(
            egui::RichText::new("Selected layer is not a text layer.")
                .small()
                .color(colors::TEXT_MUTED),
        );
        return;
    }

    // Ensure text_formatting exists, then borrow it for the panel.
    let Some(fmt) = (match layer.text_formatting {
        Some(_) => layer.text_formatting.as_mut(),
        None => {
            layer.text_formatting = Some(crate::core::timeline::TextFormatting::default());
            layer.text_formatting.as_mut()
        }
    }) else {
        return;
    };
    let mut changed = false;

    // ── Alignment ──
    crate::ui::custom_widgets::ae_section_header(ui, "Paragraph", "📝");
    ui.label(
        egui::RichText::new("Alignment")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
    ui.horizontal(|ui| {
        let aligns = [
            (TextAlign::Left, "Left"),
            (TextAlign::Center, "Center"),
            (TextAlign::Right, "Right"),
            (TextAlign::Justify, "Justify"),
        ];
        for (mode, label) in aligns {
            let is_selected = fmt.alignment == mode as u32;
            if ui
                .selectable_label(
                    is_selected,
                    egui::RichText::new(label).small().color(if is_selected {
                        colors::ACCENT_CYAN
                    } else {
                        colors::TEXT_PRIMARY
                    }),
                )
                .clicked()
            {
                fmt.alignment = mode as u32;
                changed = true;
            }
        }
    });

    ui.add_space(4.0);

    // ── Text Box Dimensions ──
    crate::ui::custom_widgets::ae_section_header(ui, "Text Box", "📦");
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Width")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
            if ui
                .add(
                    egui::DragValue::new(&mut fmt.box_width)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed()
            {
                changed = true;
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Height")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
            if ui
                .add(
                    egui::DragValue::new(&mut fmt.box_height)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.add_space(4.0);

    // ── Spacing ──
    crate::ui::custom_widgets::ae_section_header(ui, "Spacing", "↔");
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Tracking")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
            if ui
                .add(
                    egui::DragValue::new(&mut fmt.tracking)
                        .speed(0.1)
                        .range(-100.0..=1000.0),
                )
                .changed()
            {
                changed = true;
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Leading")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::DragValue::new(&mut fmt.leading)
                        .speed(0.01)
                        .range(0.1..=10.0),
                )
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.add_space(4.0);

    // ── Stroke ──
    crate::ui::custom_widgets::ae_section_header(ui, "Stroke", "✏");
    let mut stroke_enabled = fmt.stroke_color.is_some();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Enabled")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        if ui.checkbox(&mut stroke_enabled, "").clicked() {
            if stroke_enabled {
                fmt.stroke_color = Some([0.0, 0.0, 0.0, 1.0]);
            } else {
                fmt.stroke_color = None;
            }
            changed = true;
        }
    });

    if let Some(ref mut stroke_color) = fmt.stroke_color {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Width")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("px").small().color(colors::TEXT_MUTED));
                if ui
                    .add(
                        egui::DragValue::new(&mut fmt.stroke_width)
                            .speed(0.1)
                            .range(0.0..=100.0),
                    )
                    .changed()
                {
                    changed = true;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Color")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let r = (stroke_color[0] * 255.0) as u8;
            let g = (stroke_color[1] * 255.0) as u8;
            let b = (stroke_color[2] * 255.0) as u8;
            let a = (stroke_color[3] * 255.0) as u8;

            // Show color swatch and individual channel drag values
            let swatch_rect = ui.allocate_space(egui::vec2(16.0, 16.0)).1;
            ui.painter().rect_filled(
                swatch_rect,
                2.0,
                egui::Color32::from_rgba_premultiplied(r, g, b, a),
            );
            ui.painter().rect_stroke(
                swatch_rect,
                2.0,
                egui::Stroke::new(1.0, colors::BORDER_MEDIUM),
            );

            if ui
                .add(
                    egui::DragValue::new(&mut stroke_color[0])
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("R "),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::DragValue::new(&mut stroke_color[1])
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("G "),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::DragValue::new(&mut stroke_color[2])
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("B "),
                )
                .changed()
            {
                changed = true;
            }
        });
    }

    if changed {
        crate::core::frame_cache::bump_version();
    }
}
