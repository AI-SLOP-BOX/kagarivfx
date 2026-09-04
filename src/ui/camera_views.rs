use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_camera_views(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::custom_widgets::ae_section_header(ui, "3D Viewports", "📷");

    // Viewport layout selector
    ui.label(
        egui::RichText::new("Layout")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
    let layouts = [
        (0, "1 View"),
        (1, "2H Split"),
        (2, "2V Split"),
        (3, "4 Views"),
    ];
    ui.horizontal(|ui| {
        for (idx, label) in layouts {
            let is_active = app.camera_view_layout == idx;
            if ui
                .selectable_label(
                    is_active,
                    egui::RichText::new(label).small().color(if is_active {
                        colors::ACCENT_CYAN
                    } else {
                        colors::TEXT_PRIMARY
                    }),
                )
                .clicked()
            {
                app.camera_view_layout = idx;
            }
        }
    });

    // Viewport preview (visual indicator of split layout)
    let preview_rect = ui
        .allocate_space(egui::vec2(ui.available_width().min(200.0), 120.0))
        .1;
    ui.painter()
        .rect_filled(preview_rect, 4.0, colors::BG_DEEPEST);
    ui.painter().rect_stroke(
        preview_rect,
        4.0,
        egui::Stroke::new(1.0, colors::BORDER_MEDIUM),
    );

    let view_names = ["Active", "Front", "Left", "Top", "Right"];
    match app.camera_view_layout {
        0 => {
            // Single view
            ui.painter().text(
                preview_rect.center(),
                egui::Align2::CENTER_CENTER,
                view_names[app.camera_view_angle.min(4)],
                egui::FontId::proportional(11.0),
                colors::TEXT_SECONDARY,
            );
        }
        1 => {
            // Horizontal split
            let mid = preview_rect.center().x;
            ui.painter().line_segment(
                [
                    egui::pos2(mid, preview_rect.top()),
                    egui::pos2(mid, preview_rect.bottom()),
                ],
                egui::Stroke::new(1.0, colors::BORDER_STRONG),
            );
            let left_center = egui::pos2(
                preview_rect.left() + preview_rect.width() * 0.25,
                preview_rect.center().y,
            );
            let right_center = egui::pos2(
                preview_rect.left() + preview_rect.width() * 0.75,
                preview_rect.center().y,
            );
            ui.painter().text(
                left_center,
                egui::Align2::CENTER_CENTER,
                view_names[0],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                right_center,
                egui::Align2::CENTER_CENTER,
                view_names[app.camera_view_angle.min(4)],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
        }
        2 => {
            // Vertical split
            let mid = preview_rect.center().y;
            ui.painter().line_segment(
                [
                    egui::pos2(preview_rect.left(), mid),
                    egui::pos2(preview_rect.right(), mid),
                ],
                egui::Stroke::new(1.0, colors::BORDER_STRONG),
            );
            let top_center = egui::pos2(
                preview_rect.center().x,
                preview_rect.top() + preview_rect.height() * 0.25,
            );
            let bot_center = egui::pos2(
                preview_rect.center().x,
                preview_rect.top() + preview_rect.height() * 0.75,
            );
            ui.painter().text(
                top_center,
                egui::Align2::CENTER_CENTER,
                view_names[0],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                bot_center,
                egui::Align2::CENTER_CENTER,
                view_names[app.camera_view_angle.min(4)],
                egui::FontId::proportional(9.0),
                colors::TEXT_MUTED,
            );
        }
        3 => {
            // 4 views
            let cx = preview_rect.center().x;
            let cy = preview_rect.center().y;
            ui.painter().line_segment(
                [
                    egui::pos2(cx, preview_rect.top()),
                    egui::pos2(cx, preview_rect.bottom()),
                ],
                egui::Stroke::new(1.0, colors::BORDER_STRONG),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(preview_rect.left(), cy),
                    egui::pos2(preview_rect.right(), cy),
                ],
                egui::Stroke::new(1.0, colors::BORDER_STRONG),
            );
            let positions = [
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.25,
                    preview_rect.top() + preview_rect.height() * 0.25,
                ),
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.75,
                    preview_rect.top() + preview_rect.height() * 0.25,
                ),
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.25,
                    preview_rect.top() + preview_rect.height() * 0.75,
                ),
                egui::pos2(
                    preview_rect.left() + preview_rect.width() * 0.75,
                    preview_rect.top() + preview_rect.height() * 0.75,
                ),
            ];
            let quad_names = ["Top", "Front", "Left", "Active"];
            for (pos, name) in positions.iter().zip(quad_names.iter()) {
                ui.painter().text(
                    *pos,
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(8.0),
                    colors::TEXT_MUTED,
                );
            }
        }
        _ => {}
    }

    ui.add_space(6.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Active View", "👁");

    let view_labels = [
        "Active Camera",
        "Front",
        "Left",
        "Top",
        "Right",
        "Back",
        "Bottom",
        "Custom View 1",
        "Custom View 2",
        "Custom View 3",
    ];

    egui::Grid::new("cam_views_grid")
        .striped(true)
        .show(ui, |ui| {
            for (i, v_name) in view_labels.iter().enumerate() {
                let is_active = app.camera_view_angle == i;
                if ui
                    .selectable_label(
                        is_active,
                        egui::RichText::new(*v_name).small().color(if is_active {
                            colors::ACCENT_CYAN
                        } else {
                            colors::TEXT_PRIMARY
                        }),
                    )
                    .clicked()
                {
                    app.camera_view_angle = i;
                }
                if (i + 1) % 2 == 0 {
                    ui.end_row();
                }
            }
        });

    // ── 3D Rendering Engine & Advanced Space ──
    ui.add_space(8.0);
    crate::ui::custom_widgets::ae_section_header(ui, "3D Engine", "⚙");
    ui.group(|ui| {
        let mut engine_idx = 0;
        ui.horizontal(|ui| {
            ui.label("Renderer:");
            egui::ComboBox::from_id_salt("3d_engine_mode")
                .selected_text(match engine_idx {
                    0 => "Classic 3D (Fast GPU)",
                    1 => "Ray-traced 3D (Physical)",
                    2 => "Cinema 4D (Extruded)",
                    _ => "Classic 3D",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut engine_idx, 0, "Classic 3D (Fast GPU)");
                    ui.selectable_value(&mut engine_idx, 1, "Ray-traced 3D (Physical)");
                    ui.selectable_value(&mut engine_idx, 2, "Cinema 4D (Extruded)");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Shadow Map Res:");
            let mut shadow_res = 2048u32;
            egui::ComboBox::from_id_salt("shadow_map_res")
                .selected_text(format!("{shadow_res} px"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut shadow_res, 1024, "1024 px (Low)");
                    ui.selectable_value(&mut shadow_res, 2048, "2048 px (Standard)");
                    ui.selectable_value(&mut shadow_res, 4096, "4096 px (High Detail)");
                });
        });

        let mut z_sort = true;
        ui.checkbox(&mut z_sort, "Z-Depth Layer Intersection")
            .on_hover_text("Enable physical 3D intersection between overlapping 3D layers");
    });
}
