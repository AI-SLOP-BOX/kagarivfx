use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

/// Startup crash-recovery prompt: offers to restore the latest autosave snapshot.
pub fn draw_recovery_dialog(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_recovery_dialog {
        return;
    }

    let mut open = app.show_recovery_dialog;
    egui::Window::new("💥 クラッシュリカバリ")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚠").size(28.0).color(colors::ACCENT_ORANGE));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("前回のセッションが異常終了した可能性があります").strong());
                    ui.label("リカバリースナップショットが見つかりました。");
                });
            });

            ui.add_space(8.0);
            if let Some(ref recovered_at) = app.recovery_snapshot_time.clone() {
                ui.label(format!("スナップショット時刻: {}", recovered_at));
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if custom_widgets::ae_button_accent(ui, "✅ 復元する").clicked() {
                    if let Some(project) = app.autosave.load_latest_recovery() {
                        app.history = crate::core::history::ProjectHistory::new(project);
                        crate::core::frame_cache::bump_version();
                        app.frame_cache.collect_garbage();
                    }
                    app.show_recovery_dialog = false;
                    app.autosave.clear_recovery();
                }
                if custom_widgets::ae_button(ui, "🗑 破棄して新規開始").clicked() {
                    app.autosave.clear_recovery();
                    app.show_recovery_dialog = false;
                }
            });
        });

    app.show_recovery_dialog = open;
}
