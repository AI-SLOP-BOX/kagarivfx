use eframe::egui;
use super::utils::{draw_keyframe_tick, KeyframeTickResult};

/// Legacy signature kept for backward compatibility (no selection support).
#[allow(clippy::too_many_arguments)]
pub fn draw_prop_row(
    ui: &mut egui::Ui,
    label: &str,
    kfs: &[(u32, crate::core::keyframe::InterpolationType)],
    current_frame: &mut u32,
    start_frame: u32,
    zoom_span: u32,
    left_pane_w: f32,
    on_move: Option<&mut dyn FnMut(u32, u32)>,
) -> Option<u32> {
    draw_prop_row_ext(ui, label, kfs, current_frame, start_frame, zoom_span, left_pane_w,
        &std::collections::HashSet::new(), "", on_move, None)
}

/// Extended version with keyframe selection support.
#[allow(clippy::too_many_arguments)]
pub fn draw_prop_row_ext(
    ui: &mut egui::Ui,
    label: &str,
    kfs: &[(u32, crate::core::keyframe::InterpolationType)],
    current_frame: &mut u32,
    start_frame: u32,
    zoom_span: u32,
    left_pane_w: f32,
    // Selected keyframes for this property (prop_key, frame). Empty => no highlight.
    selected_kfs: &std::collections::HashSet<(String, u32)>,
    // Stable key identifying this property within the layer (e.g. "position").
    prop_key: &'static str,
    // Optional mutator invoked when a keyframe is dragged: (old_frame, new_frame).
    // None => read-only display (legacy behavior).
    mut on_move: Option<&mut dyn FnMut(u32, u32)>,
    // Optional callback invoked when a keyframe tick is clicked:
    // (prop_key, frame, shift_held, cmd_ctrl_held). Use this to toggle
    // selection in app state.
    mut on_select: Option<&mut dyn FnMut(&'static str, u32, bool, bool)>,
) -> Option<u32> {
    let mut requested_frame = None;
    let mut pending_move: Option<(u32, u32)> = None;
    let mut pending_select: Option<(&'static str, u32, bool, bool)> = None;

    ui.horizontal(|ui| {
        ui.allocate_ui(egui::vec2(left_pane_w, 18.0), |ui| {
            ui.label(egui::RichText::new(label).small().color(crate::ui::theme::colors::TEXT_SECONDARY));
        });

        let avail_w = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, 18.0), egui::Sense::click_and_drag());
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            egui::Stroke::new(0.5, crate::ui::theme::colors::BORDER_SUBTLE),
        );

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let norm = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let target_f = start_frame + (norm * zoom_span as f32).round() as u32;
                *current_frame = target_f;
                requested_frame = Some(target_f);
            }
        }

        for &(kf_frame, interpolation) in kfs {
            if kf_frame >= start_frame && kf_frame <= start_frame + zoom_span {
                let norm = (kf_frame - start_frame) as f32 / zoom_span as f32;
                let kf_x = rect.left() + norm * rect.width();
                let kf_y = rect.center().y;
                let is_selected = selected_kfs.contains(&(prop_key.to_string(), kf_frame));
                match draw_keyframe_tick(ui, kf_x, kf_y, true, current_frame, kf_frame, Some(interpolation), is_selected) {
                    KeyframeTickResult::Clicked { shift, cmd } => {
                        requested_frame = Some(kf_frame);
                        pending_select = Some((prop_key, kf_frame, shift, cmd));
                    }
                    KeyframeTickResult::Dragged { new_frame } => {
                        requested_frame = Some(new_frame);
                        if new_frame != kf_frame {
                            pending_move = Some((kf_frame, new_frame));
                        }
                    }
                    KeyframeTickResult::None => {}
                }
            }
        }

        // Apply callbacks outside the keyframe loop (they may touch shared state)
        if let Some((old_f, new_f)) = pending_move.take() {
            if let Some(ref mut cb) = on_move {
                cb(old_f, new_f);
            } else if new_f != old_f {
                // Legacy read-only behavior: follow the dragged frame.
                requested_frame.get_or_insert(new_f);
            }
        }
        if let Some((pk, f, sh, cm)) = pending_select.take() {
            if let Some(ref mut cb) = on_select {
                cb(pk, f, sh, cm);
            }
        }
    });

    requested_frame
}
