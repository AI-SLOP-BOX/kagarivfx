use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_workspace_manager(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Workspace Layout Presets");
    ui.separator();

    ui.label("Select After Effects Studio Workspace:");
    ui.add_space(4.0);

    let workspaces = [
        ("All Panels", "Shows all 25 panels & inspector tools"),
        ("Standard", "Default AE 3-Column Layout"),
        ("Small Screen", "Compact 2-Column mode for laptop screens"),
        ("Motion Tracking", "Highlights Tracker & Viewport controls"),
        ("Paint & Rotoscoping", "Expands Paint Brushes & Vector Masks"),
        ("Text & Typography", "Focuses on Character & Paragraph panels"),
        ("Color & Grading", "Prioritizes Color Management & VU Meters"),
        ("Minimal", "Maximizes Viewport canvas area"),
    ];

    let ws_id = egui::Id::new("ae_active_workspace");
    let mut active_ws = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(ws_id, || 0));

    egui::Grid::new("workspaces_grid").striped(true).show(ui, |ui| {
        for (i, (name, desc)) in workspaces.iter().enumerate() {
            if ui.selectable_label(active_ws == i, *name).clicked() {
                active_ws = i;
                ui.ctx().data_mut(|d| d.insert_temp(ws_id, active_ws));
                log::info!("Switched workspace layout to: {}", name);
            }
            ui.label(egui::RichText::new(*desc).weak());
            ui.end_row();
        }
    });

    ui.add_space(8.0);
    ui.separator();
    if ui.button("🔄 Reset Current Workspace to Default Layout").clicked() {
        app.right_tab_idx = 0;
        app.left_tab_idx = 0;
        log::info!("Reset workspace layout");
    }
}
