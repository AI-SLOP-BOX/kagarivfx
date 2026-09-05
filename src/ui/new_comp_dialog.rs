//! New Composition dialog: name + resolution/fps/duration presets +
//! editable fields. Replaces the old instant-create Cmd+N behavior.
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

const PRESETS: &[(&str, u32, u32, u32, u32)] = &[
    ("HD 1920×1080 · 30fps · 10s", 1920, 1080, 30, 300),
    ("Cinematic 1920×1080 · 24fps · 10s", 1920, 1080, 24, 240),
    ("4K UHD 3840×2160 · 30fps · 10s", 3840, 2160, 30, 300),
    ("Square 1080×1080 · 30fps · 10s", 1080, 1080, 30, 300),
    ("Vertical 1080×1920 · 30fps · 15s", 1080, 1920, 30, 450),
    ("Custom…", 1920, 1080, 30, 300),
];

pub fn draw_new_comp_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_new_comp_dialog {
        return;
    }

    let mut open = true;
    let mut keep_open = true;
    egui::Window::new("🆕 New Composition")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let id = egui::Id::new("ae_newcomp_draft");
            // Draft state: (name, preset_idx, w, h, fps, dur)
            let count = app.history.current().compositions.len();
            let mut draft = ctx.data_mut(|d| {
                d.get_temp::<(String, usize, u32, u32, u32, u32)>(id)
                    .unwrap_or_else(|| {
                        (
                            format!("Composition {}", count + 1),
                            0usize,
                            1920,
                            1080,
                            30,
                            300,
                        )
                    })
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut draft.0)
                        .desired_width(220.0)
                        .hint_text("My Composition"),
                );
                if !resp.has_focus() && draft.0.trim().is_empty() {
                    draft.0 = format!("Composition {}", count + 1);
                }
            });

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Preset")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            let mut picked = None;
            egui::ComboBox::from_id_salt("newcomp_preset")
                .selected_text(PRESETS[draft.1].0)
                .width(260.0)
                .show_ui(ui, |ui| {
                    for (i, (label, w, h, f, d)) in PRESETS.iter().enumerate() {
                        if ui.selectable_label(draft.1 == i, *label).clicked() {
                            picked = Some((i, *w, *h, *f, *d));
                        }
                    }
                });
            if let Some((i, w, h, f, d)) = picked {
                draft.1 = i;
                if i + 1 < PRESETS.len() {
                    draft.2 = w;
                    draft.3 = h;
                    draft.4 = f;
                    draft.5 = d;
                }
            }

            ui.add_space(6.0);
            egui::Grid::new("newcomp_fields")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Width:");
                    ui.add(
                        egui::DragValue::new(&mut draft.2)
                            .range(16..=7680)
                            .suffix(" px"),
                    );
                    ui.label("Height:");
                    ui.add(
                        egui::DragValue::new(&mut draft.3)
                            .range(16..=4320)
                            .suffix(" px"),
                    );
                    ui.end_row();
                    ui.label("Frame Rate:");
                    ui.add(
                        egui::DragValue::new(&mut draft.4)
                            .range(1..=120)
                            .suffix(" fps"),
                    );
                    ui.label("Duration:");
                    ui.add(
                        egui::DragValue::new(&mut draft.5)
                            .range(1..=180_000)
                            .suffix(" fr"),
                    );
                    ui.end_row();
                });
            if draft.1 + 1 < PRESETS.len() {
                ui.label(
                    egui::RichText::new("Editing fields switches to Custom")
                        .small()
                        .color(colors::TEXT_MUTED),
                );
            }

            ui.add_space(8.0);
            let depth_id = egui::Id::new("ae_newcomp_depth");
            let mut depth_idx =
                ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(depth_id, || 0usize));
            ui.horizontal(|ui| {
                ui.label("Color Depth:");
                let label = match depth_idx {
                    0 => "8 bpc (Integer)",
                    1 => "16 bpc (Half Float)",
                    _ => "32 bpc (Float / HDR)",
                };
                egui::ComboBox::from_id_salt("newcomp_depth_combo")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut depth_idx, 0, "8 bpc (Integer)");
                        ui.selectable_value(&mut depth_idx, 1, "16 bpc (Half Float)");
                        ui.selectable_value(&mut depth_idx, 2, "32 bpc (Float / HDR)");
                    });
            });
            ctx.data_mut(|d| d.insert_temp(depth_id, depth_idx));

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("✓ Create").min_size(egui::vec2(90.0, 26.0)))
                    .clicked()
                {
                    let name = if draft.0.trim().is_empty() {
                        format!("Composition {}", count + 1)
                    } else {
                        draft.0.trim().to_string()
                    };
                    let mut comp = crate::core::timeline::Composition::new(
                        format!("comp_{}", app.history.current().compositions.len()),
                        name.clone(),
                        draft.2,
                        draft.3,
                        draft.4,
                        draft.5,
                    );
                    comp.dither_output = true;
                    comp.bit_depth = match depth_idx {
                        1 => crate::core::color_science::BitDepth::SixteenBit,
                        2 => crate::core::color_science::BitDepth::ThirtyTwoBitFloat,
                        _ => crate::core::color_science::BitDepth::EightBit,
                    };
                    let proj = app.history.current_mut();
                    proj.compositions.push(comp);
                    proj.active_composition_idx = proj.compositions.len() - 1;
                    crate::core::frame_cache::bump_version();
                    app.toasts.info(format!(
                        "Created '{}' — {}×{} @{}fps ({})",
                        name,
                        draft.2,
                        draft.3,
                        draft.4,
                        match depth_idx {
                            1 => "16bpc",
                            2 => "32bpc Float",
                            _ => "8bpc",
                        }
                    ));
                    keep_open = false;
                }
                if ui.button("Cancel").clicked() {
                    keep_open = false;
                }
            });

            ctx.data_mut(|d| d.insert_temp(id, draft));
        });

    app.show_new_comp_dialog = open && keep_open;
}
