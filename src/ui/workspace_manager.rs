use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_workspace_manager(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Workspace Layout Presets");
    ui.separator();

    ui.label("Select After Effects Studio Workspace:");
    ui.add_space(4.0);

    // (name, description, left_tab_idx, right_tab_idx) — tab targets match the
    // toolbar workspace switcher so both UIs stay consistent.
    let workspaces = [
        ("All Panels", "Shows all 25 panels & inspector tools", 0, 0),
        ("Standard", "Default AE 3-Column Layout", 0, 0),
        ("Small Screen", "Compact 2-Column mode for laptop screens", 0, 1),
        ("Motion Tracking", "Highlights Tracker & Viewport controls", 0, 2),
        ("Paint & Rotoscoping", "Expands Paint Brushes & Vector Masks", 0, 4),
        ("Text & Typography", "Focuses on Character & Paragraph panels", 0, 7),
        ("Color & Grading", "Prioritizes Color Management & VU Meters", 1, 19),
        ("Minimal", "Maximizes Viewport canvas area", 1, 0),
    ];

    let ws_id = egui::Id::new("ae_active_workspace");
    let mut active_ws = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(ws_id, || 0));

    egui::Grid::new("workspaces_grid").striped(true).show(ui, |ui| {
        for (i, (name, desc, l_idx, r_idx)) in workspaces.iter().enumerate() {
            if ui.selectable_label(active_ws == i, *name).clicked() {
                active_ws = i;
                ui.ctx().data_mut(|d| d.insert_temp(ws_id, active_ws));
                // Actually switch the panel layout
                app.left_tab_idx = *l_idx;
                app.right_tab_idx = *r_idx;
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
