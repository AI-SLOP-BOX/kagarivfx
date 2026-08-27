use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_workspace_manager(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    crate::ui::custom_widgets::ae_section_header(ui, "Workspaces", "📐");

    // Built-in workspace presets
    let workspaces = [
        ("Standard", "Default AE 3-Column Layout", 0, 0),
        ("Small Screen", "Compact mode for laptops", 0, 1),
        ("Motion Tracking", "Tracker & viewport controls", 0, 2),
        ("Paint & Roto", "Paint brushes & vector masks", 0, 4),
        ("Text & Type", "Character & paragraph panels", 0, 7),
        ("Color & Grading", "Color management & VU meters", 1, 19),
        ("Minimal", "Maximizes viewport canvas", 1, 0),
        ("Audio", "Audio mixer & waveform", 1, 23),
        ("3D Layout", "Camera views & 3D options", 0, 25),
        ("Animation", "Keyframes, graph editor, presets", 0, 5),
        ("Effects", "Effect controls & library focus", 0, 10),
        ("Essential Graphics", "Master properties panel", 0, 12),
    ];

    let mut new_ws: Option<(usize, usize)> = None;

    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        for (name, desc, l_idx, r_idx) in workspaces.iter() {
            let is_active = app.left_tab_idx == *l_idx && app.right_tab_idx == *r_idx;
            let response = ui.selectable_label(is_active,
                egui::RichText::new(*name).small().color(
                    if is_active { colors::ACCENT_CYAN } else { colors::TEXT_PRIMARY }
                )
            );
            if response.clicked() {
                new_ws = Some((*l_idx, *r_idx));
            }
            ui.label(egui::RichText::new(*desc).small().color(colors::TEXT_MUTED));
            ui.add_space(2.0);
        }
    });

    if let Some((l, r)) = new_ws {
        app.left_tab_idx = l;
        app.right_tab_idx = r;
    }

    ui.add_space(4.0);
    ui.separator();

    // Custom workspace actions
    ui.label(egui::RichText::new("Custom Workspaces").small().strong().color(colors::TEXT_PRIMARY));
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        if crate::ui::custom_widgets::ae_button(ui, "💾 Save Current").clicked() {
            // Save current layout as a custom workspace
            let ws = crate::ui::workspace_manager::SavedWorkspace {
                name: format!("Custom {}", app.custom_workspaces.len() + 1),
                left_tab: app.left_tab_idx,
                right_tab: app.right_tab_idx,
            };
            app.custom_workspaces.push(ws);
            app.toasts.info("Workspace saved".to_string());
        }
        if crate::ui::custom_widgets::ae_button(ui, "🔄 Reset").clicked() {
            app.left_tab_idx = 0;
            app.right_tab_idx = 0;
            app.toasts.info("Workspace reset to default".to_string());
        }
    });

    // Show saved custom workspaces
    if !app.custom_workspaces.is_empty() {
        ui.add_space(4.0);
        let mut delete_idx = None;
        for (i, ws) in app.custom_workspaces.iter().enumerate() {
            let is_active = app.left_tab_idx == ws.left_tab && app.right_tab_idx == ws.right_tab;
            ui.horizontal(|ui| {
                if ui.selectable_label(is_active,
                    egui::RichText::new(&ws.name).small().color(
                        if is_active { colors::ACCENT_CYAN } else { colors::TEXT_PRIMARY }
                    )
                ).clicked() {
                    app.left_tab_idx = ws.left_tab;
                    app.right_tab_idx = ws.right_tab;
                }
                if ui.small_button("✕").on_hover_text("Delete").clicked() {
                    delete_idx = Some(i);
                }
            });
        }
        if let Some(idx) = delete_idx {
            app.custom_workspaces.remove(idx);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SavedWorkspace {
    pub name: String,
    pub left_tab: usize,
    pub right_tab: usize,
}
