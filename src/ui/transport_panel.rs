use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_transport_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: &mut u32, total_frames: u32) {
    ui.heading("Preview / Time Controls");
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("|◀").on_hover_text("First Frame (Home)").clicked() {
            *current_frame = 0;
        }
        if ui.button("◀").on_hover_text("Previous Frame (PageUp / ←)").clicked() {
            *current_frame = current_frame.saturating_sub(1);
        }

        let play_btn_text = if app.is_playing { "⏸ Pause" } else { "▶ Play (Space)" };
        if ui.button(play_btn_text).clicked() {
            app.is_playing = !app.is_playing;
        }

        if ui.button("▶").on_hover_text("Next Frame (PageDown / →)").clicked() {
            *current_frame = (*current_frame + 1).min(total_frames.saturating_sub(1));
        }
        if ui.button("▶|").on_hover_text("Last Frame (End)").clicked() {
            *current_frame = total_frames.saturating_sub(1);
        }
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let is_loop = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_loop_playback"), || true));
        let mut loop_val = is_loop;
        if ui.checkbox(&mut loop_val, "Loop Playback").changed() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_loop_playback"), loop_val));
        }

        let is_audio = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new("ae_audio_preview"), || true));
        let mut audio_val = is_audio;
        if ui.checkbox(&mut audio_val, "Audio Preview").changed() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_audio_preview"), audio_val));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    ui.label("Preview Quality & Downsampling:");
    let res_id = egui::Id::new("ae_preview_resolution");
    let mut res_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(res_id, || 0));

    egui::ComboBox::from_id_source("preview_res_combo")
        .selected_text(match res_idx {
            0 => "Full (1:1 Resolution)",
            1 => "Half (1/2 Resolution)",
            2 => "Third (1/3 Resolution)",
            _ => "Quarter (1/4 Resolution)",
        })
        .show_ui(ui, |ui| {
            if ui.selectable_value(&mut res_idx, 0, "Full (1:1 Resolution)").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(res_id, res_idx));
            }
            if ui.selectable_value(&mut res_idx, 1, "Half (1/2 Resolution)").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(res_id, res_idx));
            }
            if ui.selectable_value(&mut res_idx, 2, "Third (1/3 Resolution)").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(res_id, res_idx));
            }
            if ui.selectable_value(&mut res_idx, 3, "Quarter (1/4 Resolution)").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(res_id, res_idx));
            }
        });
}
