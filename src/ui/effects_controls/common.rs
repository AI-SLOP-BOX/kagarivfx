use crate::core::property::Animatable;
use crate::ui::inspector_property::draw_property_ui;
use eframe::egui;

pub fn draw_prop(
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
    label: &str,
    field: &mut Animatable<f32>,
    draw_value: impl FnOnce(&mut egui::Ui, &mut f32),
) {
    let before = field.clone();
    if let Some(nf) = draw_property_ui(current_frame, ui, label, field, draw_value) {
        *next_frame = Some(nf);
    }
    if before != *field {
        *project_changed = true;
    }
}
