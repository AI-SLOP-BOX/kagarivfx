use crate::core::keyframe::InterpolationType;
use crate::core::property::Animatable;
use crate::ui::theme::colors;
use eframe::egui;

pub fn get_kfs<T: Clone>(
    prop: &Animatable<T>,
) -> Vec<(u32, crate::core::keyframe::InterpolationType)> {
    prop.keyframes()
        .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.interpolation)).collect())
        .unwrap_or_default()
}

pub fn maybe_snap_frame(frame: u32, snap: bool, comp: &crate::core::timeline::Composition) -> u32 {
    if !snap {
        return frame;
    }
    let threshold = 3i32;
    for layer in &comp.layers {
        for (kf_f, _) in get_kfs(&layer.transform.position)
            .into_iter()
            .chain(get_kfs(&layer.transform.scale))
            .chain(get_kfs(&layer.transform.rotation))
            .chain(get_kfs(&layer.transform.opacity))
        {
            if (frame as i32 - kf_f as i32).abs() <= threshold {
                return kf_f;
            }
        }
    }
    frame
}

pub fn snap_to_layer_edges(
    frame: u32,
    exclude_idx: usize,
    comp: &crate::core::timeline::Composition,
) -> u32 {
    let threshold = 5i32;
    let mut best = frame;
    let mut best_dist = threshold + 1;
    for (i, layer) in comp.layers.iter().enumerate() {
        if i == exclude_idx {
            continue;
        }
        for edge in [layer.in_frame, layer.out_frame] {
            let dist = (frame as i32 - edge as i32).abs();
            if dist < best_dist {
                best_dist = dist;
                best = edge;
            }
        }
    }
    best
}

pub fn collect_all_kf_frames(comp: &crate::core::timeline::Composition) -> Vec<u32> {
    let mut frames = Vec::new();
    for layer in &comp.layers {
        if let Some(kfs) = layer.transform.position.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        if let Some(kfs) = layer.transform.scale.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        if let Some(kfs) = layer.transform.rotation.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        if let Some(kfs) = layer.transform.opacity.keyframes() {
            for kf in kfs {
                frames.push(kf.frame);
            }
        }
        for pin in &layer.puppet_pins {
            if let Some(kfs) = pin.position.keyframes() {
                for kf in kfs {
                    frames.push(kf.frame);
                }
            }
        }
    }
    frames.sort();
    frames.dedup();
    frames
}

/// Interaction result of a single keyframe tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeTickResult {
    /// No interaction this frame.
    None,
    /// Tick was clicked. Carries modifier state so the caller can implement
    /// multi-select toggling (shift/cmd-click).
    Clicked { shift: bool, cmd: bool },
    /// Tick was right-clicked (context menu trigger).
    RightClicked,
    /// Tick was dragged to a new frame.
    Dragged { new_frame: u32 },
}

#[allow(clippy::too_many_arguments)]
pub fn draw_keyframe_tick(
    ui: &mut egui::Ui,
    x: f32,
    y: f32,
    is_sub_prop: bool,
    current_frame: &mut u32,
    kf_frame: u32,
    _interpolation: Option<crate::core::keyframe::InterpolationType>,
    is_selected: bool,
) -> (KeyframeTickResult, egui::Response) {
    let size = if is_sub_prop { 5.0 } else { 7.0 };
    let rect = egui::Rect::from_center_size(
        egui::pos2(x, y),
        egui::vec2(size * 2.0 + 4.0, size * 2.0 + 4.0),
    );
    let color = if is_selected {
        colors::ACCENT_ORANGE
    } else if *current_frame == kf_frame {
        colors::TIMELINE_KEYFRAME
    } else {
        colors::TEXT_SECONDARY
    };

    // Shape by interpolation type: diamond=bezier/ease, circle=linear, square=hold
    let painter = ui.painter();
    let is_linear = matches!(_interpolation, Some(InterpolationType::Linear));
    let is_hold = matches!(_interpolation, Some(InterpolationType::Hold));

    if is_selected {
        // Selection glow ring behind keyframe
        painter.circle_filled(
            egui::pos2(x, y),
            size + 3.0,
            egui::Color32::from_rgba_premultiplied(255, 140, 0, 50),
        );
        painter.circle_stroke(
            egui::pos2(x, y),
            size + 2.0,
            egui::Stroke::new(1.5_f32, colors::ACCENT_ORANGE),
        );
    }

    if is_linear {
        // Linear: circle (AE convention)
        painter.circle_filled(egui::pos2(x, y), size * 0.8, color);
    } else if is_hold {
        // Hold: square (AE convention)
        painter.rect_filled(
            egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 1.4, size * 1.4)),
            1.0,
            color,
        );
    } else {
        // Bezier/Default: diamond (AE convention)
        #[allow(clippy::useless_vec)]
        let pts = vec![
            egui::pos2(x, y - size),
            egui::pos2(x + size, y),
            egui::pos2(x, y + size),
            egui::pos2(x - size, y),
        ];
        painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
    }

    if is_selected {
        let sel_rect =
            egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 2.0, size * 2.0));
        painter.rect_stroke(
            sel_rect,
            1.5,
            egui::Stroke::new(1.5_f32, colors::ACCENT_ORANGE),
        );
    }

    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

    // Hover tooltip: frame + interpolation shape (AE shows similar hints)
    {
        let interp_name = match _interpolation {
            Some(InterpolationType::Linear) => "Linear",
            Some(InterpolationType::Hold) => "Hold",
            Some(InterpolationType::Bezier {
                custom_bezier: Some(_),
                ..
            }) => "Bezier (custom)",
            Some(InterpolationType::Bezier { .. }) => "Bezier / Ease",
            None => "Keyframe",
        };
        response
            .clone()
            .on_hover_text(format!("Frame {} · {}", kf_frame, interp_name));
    }
    if response.secondary_clicked() {
        return (KeyframeTickResult::RightClicked, response);
    }
    if response.clicked() {
        *current_frame = kf_frame;
        let mods = ui.input(|i| i.modifiers);
        return (
            KeyframeTickResult::Clicked {
                shift: mods.shift,
                cmd: mods.command || mods.ctrl,
            },
            response,
        );
    }
    if response.dragged() {
        let delta = response.drag_delta().x;
        let step = (delta / 8.0).round() as i32;
        if step != 0 {
            let new_f = (kf_frame as i64 + step as i64).clamp(0, u32::MAX as i64) as u32;
            return (KeyframeTickResult::Dragged { new_frame: new_f }, response);
        }
    }
    (KeyframeTickResult::None, response)
}

/// Authoritative visible timeline range. Used by ruler, layer bars, keyframes,
/// playhead, markers, snapping, and hit-testing. Every call site must use this
/// instead of deriving its own `start_frame` / `zoom_span`.
pub fn visible_range(total_frames: u32, zoom: f32, view_start: u32) -> (u32, u32) {
    let zoom_span = (total_frames as f32 / zoom.max(0.01)).max(10.0) as u32;
    (view_start, zoom_span)
}

/// Convert a frame number to pixel x-position within `rect`.
pub fn frame_to_x(frame: u32, start_frame: u32, zoom_span: u32, rect: egui::Rect) -> f32 {
    let norm = (frame.saturating_sub(start_frame)) as f32 / zoom_span.max(1) as f32;
    rect.left() + norm * rect.width()
}

/// Convert a pixel x-position within `rect` to a frame number.
pub fn x_to_frame(px: f32, start_frame: u32, zoom_span: u32, rect: egui::Rect) -> u32 {
    let norm = ((px - rect.left()) / rect.width()).clamp(0.0, 1.0);
    start_frame + (norm * zoom_span as f32).round() as u32
}

/// Ensure `view_start` auto-recenters when the playhead leaves the visible range.
pub fn clamp_view_start(
    current_frame: u32,
    zoom_span: u32,
    total_frames: u32,
    view_start: u32,
) -> u32 {
    if current_frame < view_start || current_frame >= view_start.saturating_add(zoom_span) {
        current_frame
            .saturating_sub(zoom_span / 2)
            .min(total_frames.saturating_sub(zoom_span))
    } else {
        view_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_range_basic() {
        let (start, span) = visible_range(1000, 1.0, 0);
        assert_eq!(start, 0);
        assert_eq!(span, 1000);
    }

    #[test]
    fn visible_range_zoomed() {
        let (_, span) = visible_range(1000, 2.0, 0);
        assert_eq!(span, 500);
    }

    #[test]
    fn visible_range_min_span() {
        let (_, span) = visible_range(0, 1.0, 0);
        assert_eq!(span, 10);
    }

    #[test]
    fn frame_to_x_start() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        let x = frame_to_x(0, 0, 100, rect);
        assert_eq!(x, 100.0);
    }

    #[test]
    fn frame_to_x_end() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        let x = frame_to_x(100, 0, 100, rect);
        assert_eq!(x, 500.0);
    }

    #[test]
    fn frame_to_x_offset_view() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 20.0));
        let x = frame_to_x(50, 50, 100, rect);
        assert_eq!(x, 0.0);
    }

    #[test]
    fn x_to_frame_roundtrip() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(400.0, 20.0));
        let f = x_to_frame(300.0, 0, 100, rect);
        assert_eq!(f, 50);
    }

    #[test]
    fn clamp_view_start_no_change() {
        let v = clamp_view_start(50, 100, 1000, 0);
        assert_eq!(v, 0);
    }

    #[test]
    fn clamp_view_start_recenter() {
        let v = clamp_view_start(150, 100, 1000, 0);
        assert_eq!(v, 100);
    }

    #[test]
    fn clamp_view_start_at_end() {
        let v = clamp_view_start(999, 100, 1000, 0);
        assert!(v + 100 >= 999);
    }

    #[test]
    fn ruler_and_layers_share_same_start_frame() {
        let total_frames = 500u32;
        let zoom = 2.0f32;
        let view_start = 100u32;
        let current_frame = 200u32;

        let (zoom_span_1, _) = visible_range(total_frames, zoom, view_start);
        let effective_start_1 =
            clamp_view_start(current_frame, zoom_span_1, total_frames, view_start);

        let (zoom_span_2, _) = visible_range(total_frames, zoom, view_start);
        let effective_start_2 =
            clamp_view_start(current_frame, zoom_span_2, total_frames, view_start);

        assert_eq!(effective_start_1, effective_start_2);
        assert_eq!(zoom_span_1, zoom_span_2);
    }
}
