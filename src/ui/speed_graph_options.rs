use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_speed_graph_options(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Graph Editor Options & Keyframe Velocity");
    ui.separator();

    ui.label("Graph Type:");
    let graph_mode_id = egui::Id::new("ae_graph_mode_select");
    let mut graph_mode = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(graph_mode_id, || 0));

    ui.horizontal(|ui| {
        if ui.selectable_value(&mut graph_mode, 0, "Edit Speed Graph").clicked() { ui.ctx().data_mut(|d| d.insert_temp(graph_mode_id, graph_mode)); }
        if ui.selectable_value(&mut graph_mode, 1, "Edit Value Graph").clicked() { ui.ctx().data_mut(|d| d.insert_temp(graph_mode_id, graph_mode)); }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label("Keyframe Velocity & Influence (F9 / Keyframe Assistant):");

    let mut incoming_speed: f32 = 0.0;
    ui.horizontal(|ui| {
        ui.label("Incoming Speed:");
        ui.add(egui::Slider::new(&mut incoming_speed, 0.0..=2000.0).suffix(" px/s"));
    });

    let mut incoming_influence: f32 = 33.3;
    ui.horizontal(|ui| {
        ui.label("Incoming Influence:");
        ui.add(egui::Slider::new(&mut incoming_influence, 0.0..=100.0).suffix("%"));
    });

    ui.add_space(4.0);
    let mut outgoing_speed: f32 = 0.0;
    ui.horizontal(|ui| {
        ui.label("Outgoing Speed:");
        ui.add(egui::Slider::new(&mut outgoing_speed, 0.0..=2000.0).suffix(" px/s"));
    });

    let mut outgoing_influence: f32 = 33.3;
    ui.horizontal(|ui| {
        ui.label("Outgoing Influence:");
        ui.add(egui::Slider::new(&mut outgoing_influence, 0.0..=100.0).suffix("%"));
    });

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("⚡ Easy Ease (F9)").clicked() { log::info!("Applied Easy Ease (33.3% influence)"); }
        if ui.button("⚡ Easy Ease In (Shift+F9)").clicked() { log::info!("Applied Easy Ease In"); }
        if ui.button("⚡ Easy Ease Out (Cmd+Shift+F9)").clicked() { log::info!("Applied Easy Ease Out"); }
    });
}
