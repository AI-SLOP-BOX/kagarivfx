use crate::core::keyframe::InterpolationType;
use eframe::egui;
use crate::core::property::Animatable;
use crate::ui::theme::colors;

pub fn get_kfs<T: Clone>(prop: &Animatable<T>) -> Vec<(u32, crate::core::keyframe::InterpolationType)> {
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

pub fn snap_to_layer_edges(frame: u32, exclude_idx: usize, comp: &crate::core::timeline::Composition) -> u32 {
    let threshold = 5i32;
    let mut best = frame;
    let mut best_dist = threshold + 1;
    for (i, layer) in comp.layers.iter().enumerate() {
        if i == exclude_idx { continue; }
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
            for kf in kfs { frames.push(kf.frame); }
        }
        if let Some(kfs) = layer.transform.scale.keyframes() {
            for kf in kfs { frames.push(kf.frame); }
        }
        if let Some(kfs) = layer.transform.rotation.keyframes() {
            for kf in kfs { frames.push(kf.frame); }
        }
        if let Some(kfs) = layer.transform.opacity.keyframes() {
            for kf in kfs { frames.push(kf.frame); }
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
    let rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 2.0 + 4.0, size * 2.0 + 4.0));
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
        painter.circle_filled(egui::pos2(x, y), size + 3.0,
            egui::Color32::from_rgba_premultiplied(255, 140, 0, 50));
        painter.circle_stroke(egui::pos2(x, y), size + 2.0,
            egui::Stroke::new(1.5, colors::ACCENT_ORANGE));
    }

    if is_linear {
        // Linear: circle (AE convention)
        painter.circle_filled(egui::pos2(x, y), size * 0.8, color);
    } else if is_hold {
        // Hold: square (AE convention)
        painter.rect_filled(
            egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 1.4, size * 1.4)),
            1.0, color);
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
        let sel_rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 2.0, size * 2.0));
        painter.rect_stroke(sel_rect, 1.5, egui::Stroke::new(1.5, colors::ACCENT_ORANGE));
    }

    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    if response.secondary_clicked() {
        return (KeyframeTickResult::RightClicked, response);
    }
    if response.clicked() {
        *current_frame = kf_frame;
        let mods = ui.input(|i| i.modifiers);
        return (KeyframeTickResult::Clicked { shift: mods.shift, cmd: mods.command || mods.ctrl }, response);
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
