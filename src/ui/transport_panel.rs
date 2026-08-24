use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

pub fn draw_transport_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui, current_frame: &mut u32, total_frames: u32) {
    ui.heading("Preview / Time Controls");
    ui.separator();

    // ── Current Timecode + Frame Number ──
    {
        let fps = app.history.current().active_composition().fps.max(1);
        let secs = *current_frame / fps;
        let sub_f = *current_frame % fps;
        let mins = secs / 60;
        let hours = mins / 60;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{:02}:{:02}:{:02}:{:02}", hours, mins % 60, secs % 60, sub_f))
                .strong().color(colors::ACCENT_YELLOW));
            ui.separator();
            ui.label(egui::RichText::new(format!("Frame {} / {}", current_frame, total_frames))
                .small().color(colors::TEXT_SECONDARY));
        });
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        // Vector transport icons
        use crate::ui::icons::{render_svg_bytes, SVG_PLAY, SVG_PAUSE};
        const SKIP_START: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><polygon points="19 20 9 12 19 4 19 20"/><rect x="5" y="4" width="2" height="16"/></svg>"#;
        const STEP_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><polygon points="17 3 7 12 17 21 17 3"/></svg>"#;
        const STEP_FWD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><polygon points="7 3 17 12 7 21 7 3"/></svg>"#;
        const SKIP_END: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><polygon points="5 4 15 12 5 20 5 4"/><rect x="17" y="4" width="2" height="16"/></svg>"#;
        let size = egui::vec2(16.0, 16.0);

        if render_svg_bytes(ui, "t_first", SKIP_START, size, colors::TEXT_PRIMARY).clicked() {
            *current_frame = 0;
        }
        if render_svg_bytes(ui, "t_prev", STEP_BACK, size, colors::TEXT_PRIMARY)
            .on_hover_text("Previous Frame (PageUp)")
            .clicked()
        {
            *current_frame = current_frame.saturating_sub(1);
        }

        // Play / Pause: prominent accent button
        let (_icon, label) = if app.is_playing {
            (SVG_PAUSE, "Pause")
        } else {
            (SVG_PLAY, "Play (Space)")
        };
        let play_label = if app.is_playing { "⏸ Pause" } else { "▶ Play (Space)" };
        if custom_widgets::ae_button_accent(ui, play_label).on_hover_text(label).clicked() {
            app.is_playing = !app.is_playing;
        }

        if render_svg_bytes(ui, "t_next", STEP_FWD, size, colors::TEXT_PRIMARY)
            .on_hover_text("Next Frame (PageDown)")
            .clicked()
        {
            *current_frame = (*current_frame + 1).min(total_frames.saturating_sub(1));
        }
        if render_svg_bytes(ui, "t_last", SKIP_END, size, colors::TEXT_PRIMARY).clicked() {
            *current_frame = total_frames.saturating_sub(1);
        }
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.checkbox(&mut app.loop_playback, "Loop Playback").changed() {
            // Applied live by the playback loop in app_state
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
    // Bound directly to the viewport render-resolution setting used by the GPU path
    egui::ComboBox::from_id_salt("preview_res_combo")
        .selected_text(match app.viewport_render_resolution {
            0 => "Full (1:1 Resolution)",
            1 => "Half (1/2 Resolution)",
            2 => "Third (1/3 Resolution)",
            _ => "Quarter (1/4 Resolution)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.viewport_render_resolution, 0, "Full (1:1 Resolution)");
            ui.selectable_value(&mut app.viewport_render_resolution, 1, "Half (1/2 Resolution)");
            ui.selectable_value(&mut app.viewport_render_resolution, 2, "Third (1/3 Resolution)");
            ui.selectable_value(&mut app.viewport_render_resolution, 3, "Quarter (1/4 Resolution)");
        });
}
