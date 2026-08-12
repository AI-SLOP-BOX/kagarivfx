use eframe::egui;
use crate::AfterEffectsApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
    JustifyLeft,
    JustifyAll,
}

pub fn draw_paragraph_panel(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Paragraph");
    ui.separator();

    let alignment_id = egui::Id::new("ae_paragraph_alignment");
    let mut align: TextAlignment = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(alignment_id, || TextAlignment::Left));

    ui.label("Alignment:");
    ui.horizontal(|ui| {
        if ui.selectable_label(align == TextAlignment::Left, " Left").clicked() {
            align = TextAlignment::Left;
            ui.ctx().data_mut(|d| d.insert_temp(alignment_id, align));
        }
        if ui.selectable_label(align == TextAlignment::Center, " Center").clicked() {
            align = TextAlignment::Center;
            ui.ctx().data_mut(|d| d.insert_temp(alignment_id, align));
        }
        if ui.selectable_label(align == TextAlignment::Right, " Right").clicked() {
            align = TextAlignment::Right;
            ui.ctx().data_mut(|d| d.insert_temp(alignment_id, align));
        }
    });

    ui.horizontal(|ui| {
        if ui.selectable_label(align == TextAlignment::JustifyLeft, " Justify Left").clicked() {
            align = TextAlignment::JustifyLeft;
            ui.ctx().data_mut(|d| d.insert_temp(alignment_id, align));
        }
        if ui.selectable_label(align == TextAlignment::JustifyAll, " Justify All").clicked() {
            align = TextAlignment::JustifyAll;
            ui.ctx().data_mut(|d| d.insert_temp(alignment_id, align));
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    ui.label("Indentation & Spacing:");

    let indent_left_id = egui::Id::new("ae_para_indent_left");
    let mut indent_left: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(indent_left_id, || 0.0));
    ui.horizontal(|ui| {
        ui.label("Indent Left:");
        if ui.add(egui::DragValue::new(&mut indent_left).speed(1.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(indent_left_id, indent_left));
        }
    });

    let indent_right_id = egui::Id::new("ae_para_indent_right");
    let mut indent_right: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(indent_right_id, || 0.0));
    ui.horizontal(|ui| {
        ui.label("Indent Right:");
        if ui.add(egui::DragValue::new(&mut indent_right).speed(1.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(indent_right_id, indent_right));
        }
    });

    let first_line_id = egui::Id::new("ae_para_first_line");
    let mut first_line: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(first_line_id, || 0.0));
    ui.horizontal(|ui| {
        ui.label("First Line Indent:");
        if ui.add(egui::DragValue::new(&mut first_line).speed(1.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(first_line_id, first_line));
        }
    });

    let space_before_id = egui::Id::new("ae_para_space_before");
    let mut space_before: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(space_before_id, || 0.0));
    ui.horizontal(|ui| {
        ui.label("Space Before:");
        if ui.add(egui::DragValue::new(&mut space_before).speed(1.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(space_before_id, space_before));
        }
    });

    let space_after_id = egui::Id::new("ae_para_space_after");
    let mut space_after: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(space_after_id, || 0.0));
    ui.horizontal(|ui| {
        ui.label("Space After:");
        if ui.add(egui::DragValue::new(&mut space_after).speed(1.0).suffix(" px")).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(space_after_id, space_after));
        }
    });
}
