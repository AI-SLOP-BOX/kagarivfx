use eframe::egui;
use crate::core::property::Animatable;

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

/// Interaction result of a single keyframe tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeTickResult {
    /// No interaction this frame.
    None,
    /// Tick was clicked. Carries modifier state so the caller can implement
    /// multi-select toggling (shift/cmd-click).
    Clicked { shift: bool, cmd: bool },
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
) -> KeyframeTickResult {
    let size = if is_sub_prop { 5.0 } else { 7.0 };
    let rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 2.0 + 4.0, size * 2.0 + 4.0));
    let color = if is_selected {
        egui::Color32::from_rgb(255, 150, 40)
    } else if *current_frame == kf_frame {
        egui::Color32::from_rgb(255, 200, 50)
    } else {
        egui::Color32::from_rgb(180, 180, 180)
    };

    let painter = ui.painter();
    let pts = vec![
        egui::pos2(x, y - size),
        egui::pos2(x + size, y),
        egui::pos2(x, y + size),
        egui::pos2(x - size, y),
    ];
    painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));

    if is_selected {
        let sel_rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size * 2.0, size * 2.0));
        painter.rect_stroke(sel_rect, 1.5, egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 120, 20)));
    }

    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    if response.clicked() {
        *current_frame = kf_frame;
        let mods = ui.input(|i| i.modifiers);
        return KeyframeTickResult::Clicked { shift: mods.shift, cmd: mods.command || mods.ctrl };
    }
    if response.dragged() {
        let delta = response.drag_delta().x;
        let step = (delta / 8.0).round() as i32;
        if step != 0 {
            let new_f = (kf_frame as i64 + step as i64).clamp(0, u32::MAX as i64) as u32;
            return KeyframeTickResult::Dragged { new_frame: new_f };
        }
    }
    KeyframeTickResult::None
}
