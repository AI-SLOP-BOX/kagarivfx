use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_camera_views(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("3D Viewports & Multi-Camera Layout");
    ui.separator();

    ui.label("Viewport Layout Split:");
    egui::ComboBox::from_id_source("viewport_layout_combo")
        .selected_text(match app.camera_view_layout {
            0 => "1 View (Active Camera / Front)",
            1 => "2 Views - Horizontal Split",
            2 => "2 Views - Vertical Split",
            _ => "4 Views (Top, Left, Front, Active Camera)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.camera_view_layout, 0, "1 View (Active Camera / Front)");
            ui.selectable_value(&mut app.camera_view_layout, 1, "2 Views - Horizontal Split");
            ui.selectable_value(&mut app.camera_view_layout, 2, "2 Views - Vertical Split");
            ui.selectable_value(&mut app.camera_view_layout, 3, "4 Views (Top, Left, Front, Active Camera)");
        });

    ui.add_space(8.0);
    ui.separator();
    ui.label("3D View Angle Selectors:");

    let views = [
        "Active Camera", "Front", "Left", "Top",
        "Right", "Back", "Bottom",
        "Custom View 1", "Custom View 2", "Custom View 3",
    ];

    egui::Grid::new("cam_views_grid").striped(true).show(ui, |ui| {
        for (i, v_name) in views.iter().enumerate() {
            if ui.selectable_label(app.camera_view_angle == i, *v_name).clicked() {
                app.camera_view_angle = i;
                log::info!("Selected 3D View: {}", v_name);
            }
            if (i + 1) % 2 == 0 {
                ui.end_row();
            }
        }
    });
}
