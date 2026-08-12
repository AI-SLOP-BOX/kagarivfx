use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_camera_views(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("3D Viewports & Multi-Camera Layout");
    ui.separator();

    ui.label("Viewport Layout Split:");
    let layout_id = egui::Id::new("ae_3d_viewport_layout");
    let mut layout_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(layout_id, || 0));

    egui::ComboBox::from_id_source("viewport_layout_combo")
        .selected_text(match layout_idx {
            0 => "1 View (Active Camera / Front)",
            1 => "2 Views - Horizontal Split",
            2 => "2 Views - Vertical Split",
            _ => "4 Views (Top, Left, Front, Active Camera)",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut layout_idx, 0, "1 View (Active Camera / Front)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(layout_id, layout_idx)); }
            if ui.selectable_value(&mut layout_idx, 1, "2 Views - Horizontal Split").clicked() { ui.ctx().data_mut(|d| d.insert_temp(layout_id, layout_idx)); }
            if ui.selectable_value(&mut layout_idx, 2, "2 Views - Vertical Split").clicked() { ui.ctx().data_mut(|d| d.insert_temp(layout_id, layout_idx)); }
            if ui.selectable_value(&mut layout_idx, 3, "4 Views (Top, Left, Front, Active Camera)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(layout_id, layout_idx)); }
        });

    ui.add_space(8.0);
    ui.separator();
    ui.label("3D View Angle Selectors:");

    let views = ["Active Camera", "Front", "Left", "Top", "Right", "Back", "Bottom", "Custom View 1", "Custom View 2", "Custom View 3"];
    let cam_id = egui::Id::new("ae_active_3d_cam_view");
    let mut cam_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(cam_id, || 0));

    egui::Grid::new("cam_views_grid").striped(true).show(ui, |ui| {
        for (i, v_name) in views.iter().enumerate() {
            if ui.selectable_label(cam_idx == i, *v_name).clicked() {
                cam_idx = i;
                ui.ctx().data_mut(|d| d.insert_temp(cam_id, cam_idx));
                log::info!("Selected 3D View: {}", v_name);
            }
            if (i + 1) % 2 == 0 {
                ui.end_row();
            }
        }
    });
}
