use eframe::egui;
use super::utils::draw_keyframe_tick;

pub fn draw_prop_row(
    ui: &mut egui::Ui,
    label: &str,
    kfs: &[(u32, crate::core::keyframe::InterpolationType)],
    current_frame: &mut u32,
    start_frame: u32,
    zoom_span: u32,
) {
    ui.horizontal(|ui| {
        ui.allocate_ui(egui::vec2(500.0, 18.0), |ui| {
            ui.label(egui::RichText::new(label).small().color(egui::Color32::from_gray(170)));
        });

        let avail_w = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, 18.0), egui::Sense::click_and_drag());
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            egui::Stroke::new(0.5, egui::Color32::from_gray(40)),
        );

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let norm = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                *current_frame = start_frame + (norm * zoom_span as f32).round() as u32;
            }
        }

        for &(kf_frame, interpolation) in kfs {
            if kf_frame >= start_frame && kf_frame <= start_frame + zoom_span {
                let norm = (kf_frame - start_frame) as f32 / zoom_span as f32;
                let kf_x = rect.left() + norm * rect.width();
                let kf_y = rect.center().y;
                draw_keyframe_tick(ui, kf_x, kf_y, true, current_frame, kf_frame, Some(interpolation));
            }
        }
    });
}
