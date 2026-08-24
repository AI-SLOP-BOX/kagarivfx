//! Undo History panel: lists named history steps newest-first, highlights
//! the active entry, and jumps to any step on click.
use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_history_panel(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_history_panel {
        return;
    }

    let mut open = app.show_history_panel;
    egui::Window::new("🕘 Undo History")
        .open(&mut open)
        .default_width(260.0)
        .default_height(320.0)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -46.0))
        .show(ctx, |ui| {
            // Snapshot metadata so project borrow ends before jump mutation.
            let (names, current_idx) = {
                let names: Vec<String> = (0..app.history.len())
                    .filter_map(|i| app.history.action_name_at(i).map(|s| s.to_string()))
                    .collect();
                (names, app.history.current_index())
            };

            if names.is_empty() {
                ui.label("No history yet.");
                return;
            }

            ui.label(
                egui::RichText::new(format!("{} steps — click to jump", names.len()))
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut jump_target: Option<usize> = None;
                // Newest first for AE-like reading order.
                for (i, name) in names.iter().enumerate().rev() {
                    let is_current = i == current_idx;
                    let is_future = i > current_idx;
                    let label = if is_future {
                        egui::RichText::new(name).color(colors::TEXT_MUTED)
                    } else if is_current {
                        egui::RichText::new(format!("▶ {}", name)).strong().color(colors::ACCENT_BLUE)
                    } else {
                        egui::RichText::new(name).color(colors::TEXT_PRIMARY)
                    };
                    if ui.selectable_label(is_current, label).clicked() {
                        jump_target = Some(i);
                    }
                }
                if let Some(target) = jump_target {
                    if app.history.jump_to(target) {
                        app.toasts.info(format!("History → step {} ({})", target + 1, names[target]));
                    }
                }
            });
        });

    app.show_history_panel = open;
}
