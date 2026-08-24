//! Composition breadcrumb bar: shows the trail of nested-composition
//! navigation (double-click PreComp) and lets the user jump back.
use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

/// Draw the breadcrumb strip. No-op when the nav stack is empty
/// (i.e. user is at a top-level composition).
pub fn draw_comp_breadcrumb(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    if app.comp_nav_stack.is_empty() {
        return;
    }

    // Snapshot labels first so the project borrow ends before mutations.
    let (stack_labels, current_name) = {
        let proj = app.history.current();
        let labels: Vec<String> = app
            .comp_nav_stack
            .iter()
            .map(|&idx| {
                proj.compositions
                    .get(idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| format!("Comp #{}", idx))
            })
            .collect();
        (labels, proj.active_composition().name.clone())
    };

    let mut jump_to: Option<(usize, usize)> = None; // (stack_pos, comp_idx)
    let mut back_requested = false;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🧭").small());

        for (pos, label) in stack_labels.iter().enumerate() {
            if ui.small_button(label).clicked() {
                jump_to = Some((pos, app.comp_nav_stack[pos]));
            }
            ui.label(egui::RichText::new("›").color(colors::TEXT_MUTED));
        }

        // Current location crumb (non-clickable, highlighted).
        ui.label(
            egui::RichText::new(format!("📦 {}", current_name))
                .small()
                .strong()
                .color(colors::ACCENT_BLUE),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("⬅ Back").on_hover_text("Return to previous composition (pop breadcrumb)").clicked() {
                back_requested = true;
            }
        });
    });

    if let Some((pos, comp_idx)) = jump_to {
        // Truncate stack so clicked crumb becomes current, then navigate.
        app.comp_nav_stack.truncate(pos);
        app.history.current_mut().active_composition_idx = comp_idx;
        crate::core::frame_cache::bump_version();
    } else if back_requested {
        if let Some(prev) = app.comp_nav_stack.pop() {
            app.history.current_mut().active_composition_idx = prev;
            crate::core::frame_cache::bump_version();
        }
    }
}
