//! First-run welcome overlay: kills the "empty void" first impression by
//! offering a demo scene, a new composition, and a shortcut cheat-sheet.
use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    let project_empty = app.history.current().compositions.is_empty();
    if !project_empty && !app.show_welcome {
        return;
    }

    let mut open = app.show_welcome || project_empty;
    egui::Window::new("✨ Welcome")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("After Effects OSS Alternative");
            ui.label(egui::RichText::new("GPU-accelerated motion graphics in Rust").color(colors::TEXT_SECONDARY));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("🎬 Load Demo Scene").min_size(egui::vec2(150.0, 34.0)))
                    .on_hover_text("Instant animated composition — press Space to play").clicked() {
                    crate::ui::demo_scene::build(app);
                    app.show_welcome = false;
                }
                if ui.add(egui::Button::new("＋ New Comp (Cmd+N)").min_size(egui::vec2(160.0, 34.0))).clicked() {
                    app.show_welcome = false;
                    app.show_new_comp_dialog = true;
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("🡇 Drop PNG / JPG / MP4 / WAV files anywhere to import").color(colors::TEXT_ACCENT));
            ui.add_space(6.0);
            egui::Grid::new("welcome_keys").num_columns(2).spacing([12.0, 3.0]).show(ui, |ui| {
                for (k, d) in [
                    ("Space", "Play / Pause"),
                    ("Cmd+Y", "New Solid"),
                    ("Cmd+T", "Text Tool"),
                    ("V H Z Q", "Select / Hand / Zoom / Shape tools"),
                    ("J K L", "Prev KF / Next KF / Play faster"),
                    ("Alt+[ ]", "Trim layer In / Out"),
                    ("Cmd+K", "Command Palette"),
                    ("?", "All shortcuts"),
                ] {
                    ui.label(egui::RichText::new(k).strong().monospace().color(colors::ACCENT_CYAN));
                    ui.label(egui::RichText::new(d).small().color(colors::TEXT_SECONDARY));
                    ui.end_row();
                }
            });
        });
    app.show_welcome = open && !project_empty; // empty project keeps it pinned
}
