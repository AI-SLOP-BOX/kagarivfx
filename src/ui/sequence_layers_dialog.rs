use crate::KagariApp;
use eframe::egui;

/// AE Keyframe Assistant → Sequence Layers:
/// rearranges the selected layers so they play one after another,
/// optionally overlapping by a fixed number of frames.
pub fn draw_sequence_layers_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_sequence_layers {
        return;
    }

    let mut open = app.show_sequence_layers;
    egui::Window::new("🔗 Sequence Layers")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            let selected: Vec<usize> = {
                let mut s: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
                s.sort_unstable();
                s
            };

            if selected.len() < 2 {
                ui.label("Select at least 2 layers to sequence.");
                ui.separator();
                if ui.button("Close").clicked() {
                    app.show_sequence_layers = false;
                }
                return;
            }

            ui.label(format!("{} layers selected", selected.len()));
            ui.add_space(4.0);

            let overlap_id = egui::Id::new("ae_seq_layers_overlap");
            let mut overlap = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(overlap_id, || 0i32));
            ui.horizontal(|ui| {
                ui.label("Overlap:");
                ui.add(
                    egui::DragValue::new(&mut overlap)
                        .range(-300..=300)
                        .suffix(" frames"),
                )
                .on_hover_text(
                    "Negative = gap between layers, 0 = butt-jointed, positive = crossfade zone",
                );
                if overlap != 0 {
                    ui.ctx().data_mut(|d| d.insert_temp(overlap_id, overlap));
                }
            });

            // Preview of resulting schedule
            ui.add_space(6.0);
            {
                let comp = app.history.current().active_composition();
                let mut cursor: Option<u32> = None;
                for &idx in &selected {
                    if idx >= comp.layers.len() {
                        continue;
                    }
                    let l = &comp.layers[idx];
                    let start = cursor.unwrap_or(l.in_frame);
                    ui.monospace(format!(
                        "  {:<18} {} → {}",
                        truncate(&l.name, 16),
                        start,
                        start + (l.out_frame - l.in_frame)
                    ));
                    let span = (l.out_frame - l.in_frame) as i32;
                    cursor = Some((start as i32 + span - overlap).max(0) as u32);
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Apply Sequence").clicked() {
                    let overlap_f = overlap;
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    // Anchor at earliest in-point among selected layers
                    let anchor = selected
                        .iter()
                        .filter_map(|&i| comp_mut.layers.get(i).map(|l| l.in_frame))
                        .min()
                        .unwrap_or(0);
                    let mut cursor = anchor;
                    for &idx in &selected {
                        if idx >= comp_mut.layers.len() {
                            continue;
                        }
                        let l = &mut comp_mut.layers[idx];
                        let span = l.out_frame - l.in_frame;
                        l.in_frame = cursor;
                        l.out_frame = cursor + span.max(1);
                        cursor =
                            (cursor as i64 + span.max(1) as i64 - overlap_f as i64).max(0) as u32;
                    }
                    app.history.commit(temp_proj);
                    app.toasts.info(format!(
                        "Sequenced {} layers (overlap {}f)",
                        selected.len(),
                        overlap
                    ));
                    app.show_sequence_layers = false;
                }
                if ui.button("Cancel").clicked() {
                    app.show_sequence_layers = false;
                }
            });
        });

    app.show_sequence_layers = open;
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}
