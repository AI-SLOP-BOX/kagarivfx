#![allow(clippy::too_many_arguments)]

use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::LayerType;

pub fn draw_camera_3d_viewport(
    ui: &mut egui::Ui,
    app: &mut AfterEffectsApp,
    ctx: &egui::Context,
    current_frame: u32,
    viewport_response: &egui::Response,
    rect: egui::Rect,
    comp_w: f32,
    comp_h: f32,
    draw_w: f32,
    draw_h: f32,
) {
    if viewport_response.dragged_by(egui::PointerButton::Secondary) {
        let d = viewport_response.drag_delta();
        app.camera_orbit.0 += d.x * 0.5;   // yaw
        app.camera_orbit.1 += d.y * 0.5;   // pitch
        app.camera_orbit.1 = app.camera_orbit.1.clamp(-89.0, 89.0);
    }
    // Scroll to zoom
    let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() > 0.5 {
        app.camera_orbit.2 = (app.camera_orbit.2 - scroll * 12.0).clamp(100.0, 4000.0);
    }

    let (yaw_deg, pitch_deg, zoom) = app.camera_orbit;
    let yaw = yaw_deg.to_radians();
    let pitch = pitch_deg.to_radians();
    let cx_world = comp_w * 0.5;
    let cy_world = comp_h * 0.5;

    // Simple perspective projection helper
    let project_3d = |px: f32, py: f32, pz: f32| -> egui::Pos2 {
        // Translate to camera-centered coords
        let dx = px - cx_world;
        let dy = py - cy_world;
        let dz = pz;
        // Apply yaw (around Y axis)
        let rx = dx * yaw.cos() + dz * yaw.sin();
        let ry_tmp = dy;
        let rz = -dx * yaw.sin() + dz * yaw.cos();
        // Apply pitch (around X axis)
        let ry = ry_tmp * pitch.cos() - rz * pitch.sin();
        let rz2 = ry_tmp * pitch.sin() + rz * pitch.cos();
        // Perspective divide
        let z_cam = rz2 + zoom;
        let fov_scale = zoom / z_cam.max(1.0);
        let sx = rect.center().x + rx * fov_scale * (draw_w / comp_w);
        let sy = rect.center().y + ry * fov_scale * (draw_h / comp_h);
        egui::pos2(sx, sy)
    };

    // Draw a wireframe floor grid in 3D
    let grid_n = 6;
    let grid_step = comp_w / grid_n as f32;
    let grid_color = egui::Color32::from_rgba_unmultiplied(60, 80, 120, 80);
    for gx in 0..=grid_n {
        let x = gx as f32 * grid_step - comp_w * 0.5 + cx_world;
        let p0 = project_3d(x, comp_h, 0.0);
        let p1 = project_3d(x, 0.0, 0.0);
        ui.painter().line_segment([p0, p1], egui::Stroke::new(0.8, grid_color));
    }
    for gy in 0..=grid_n {
        let y = gy as f32 * (comp_h / grid_n as f32);
        let p0 = project_3d(0.0, y, 0.0);
        let p1 = project_3d(comp_w, y, 0.0);
        ui.painter().line_segment([p0, p1], egui::Stroke::new(0.8, grid_color));
    }

    // Draw comp canvas border in 3D
    let corners = [
        project_3d(0.0, 0.0, 0.0),
        project_3d(comp_w, 0.0, 0.0),
        project_3d(comp_w, comp_h, 0.0),
        project_3d(0.0, comp_h, 0.0),
    ];
    for i in 0..4 {
        ui.painter().line_segment(
            [corners[i], corners[(i + 1) % 4]],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 130, 240)),
        );
    }

    // Draw each 3D layer as a projected billboard
    let comp = app.history.current().active_composition();
    for (li, layer) in comp.layers.iter().enumerate() {
        if !layer.is_active(current_frame) { continue; }
        let (pos, scale, _rot, opacity) = comp.resolve_world_transform(layer, current_frame);
        let z_depth = if layer.is_3d {
            layer.transform_3d.position.evaluate(current_frame)[2]
        } else {
            0.0
        };
        let op = (opacity / 100.0).clamp(0.0, 1.0);
        let color = match &layer.layer_type {
            LayerType::Solid { color } | LayerType::Text { color, .. } => {
                egui::Color32::from_rgba_premultiplied(
                    (color[0] * 255.0) as u8, (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8, (op * 200.0) as u8)
            }
            LayerType::Shape { color, .. } => {
                egui::Color32::from_rgba_premultiplied(
                    (color[0] * 255.0) as u8, (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8, (op * 200.0) as u8)
            }
            _ => egui::Color32::from_rgba_premultiplied(100, 180, 255, (op * 160.0) as u8),
        };
        let center = project_3d(pos[0], pos[1], z_depth);
        let w = scale[0].abs() * 0.5 * (draw_w / comp_w);
        let h = scale[1].abs() * 0.5 * (draw_h / comp_h);
        let bbox = egui::Rect::from_center_size(center, egui::vec2(w, h));
        ui.painter().rect_filled(bbox, 3.0, color);
        ui.painter().rect_stroke(bbox, 3.0, egui::Stroke::new(1.0,
            if Some(li) == app.selected_layer_idx {
                egui::Color32::from_rgb(100, 220, 255)
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60)
            }
        ));
        ui.painter().text(egui::pos2(center.x, bbox.top() - 10.0),
            egui::Align2::CENTER_CENTER, &layer.name,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgba_unmultiplied(200, 220, 255, 180));
    }

    // 3D Camera HUD overlay
    let hud = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 10.0, rect.bottom() - 38.0),
        egui::vec2(250.0, 28.0),
    );
    ui.painter().rect_filled(hud, 4.0, egui::Color32::from_rgba_unmultiplied(15, 20, 35, 220));
    ui.painter().rect_stroke(hud, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 100, 200)));
    ui.painter().text(hud.center(), egui::Align2::CENTER_CENTER,
        format!("[3D CAMERA] Yaw: {:.1}°  Pitch: {:.1}°  Z: {:.0}", yaw_deg, pitch_deg, zoom),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(140, 200, 255));
}
