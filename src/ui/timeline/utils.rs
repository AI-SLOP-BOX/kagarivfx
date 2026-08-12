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

pub fn draw_keyframe_tick(
    ui: &mut egui::Ui,
    x: f32,
    y: f32,
    is_sub_prop: bool,
    current_frame: &mut u32,
    kf_frame: u32,
    _interpolation: Option<crate::core::keyframe::InterpolationType>,
) -> Option<u32> {
    let size = if is_sub_prop { 5.0 } else { 7.0 };
    let rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(size, size));
    let color = if *current_frame == kf_frame {
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

    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    if response.clicked() {
        *current_frame = kf_frame;
    }
    let mut new_f = None;
    if response.dragged() {
        let delta = response.drag_delta().x;
        let step = (delta / 8.0).round() as i32;
        if step != 0 {
            new_f = Some((kf_frame as i32 + step).max(0) as u32);
        }
    }
    new_f
}
