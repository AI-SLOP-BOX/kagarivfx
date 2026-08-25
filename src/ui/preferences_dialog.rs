//! Preferences dialog: performance / history / autosave / audio settings
//! with JSON persistence in $HOME/.aevfx_prefs.json.
use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Prefs {
    pub cache_mb: usize,
    pub undo_steps: usize,
    pub autosave_secs: u64,
    pub audio_preview: bool,
    pub adaptive_preview: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { cache_mb: 512, undo_steps: 50, autosave_secs: 30, audio_preview: true, adaptive_preview: true }
    }
}

fn prefs_path() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join(".aevfx_prefs.json")
}

fn load() -> Prefs {
    std::fs::read_to_string(prefs_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(p: &Prefs) {
    if let Ok(json) = serde_json::to_string_pretty(p) {
        let _ = std::fs::write(prefs_path(), json);
    }
}

/// Apply stored prefs to live app state. Called once at startup.
pub fn apply_loaded(app: &mut AfterEffectsApp) {
    let p = load();
    apply(app, &p);
}

fn apply(app: &mut AfterEffectsApp, p: &Prefs) {
    app.frame_cache.max_memory_bytes = p.cache_mb * 1024 * 1024;
    app.history.set_max_history_entries(p.undo_steps);
    app.autosave.set_interval_secs(p.autosave_secs);
    app.audio_preview_enabled = p.audio_preview;
    if !p.adaptive_preview {
        app.adaptive_preview_factor = 1.0;
    }
}

pub fn draw_preferences_dialog(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    if !app.show_preferences {
        return;
    }

    let mut open = true;
    let mut keep_open = true;
    egui::Window::new("⚙ Preferences")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let id = egui::Id::new("ae_prefs_draft");
            let mut p = ctx.data_mut(|d| d.get_temp::<Prefs>(id).unwrap_or_else(load));

            ui.add_space(2.0);

            // ── Performance ──
            ui.label(egui::RichText::new("PERFORMANCE").small().strong().color(colors::ACCENT_CYAN));
            ui.horizontal(|ui| {
                ui.label("Frame cache budget:");
                ui.add(egui::Slider::new(&mut p.cache_mb, 128..=2048).step_by(64.0).suffix(" MB"));
            });
            ui.checkbox(&mut p.adaptive_preview, "Adaptive preview quality (auto-reduce while playing)")
                .on_hover_text("When off, preview always renders at full resolution");

            ui.add_space(6.0);

            // ── History ──
            ui.label(egui::RichText::new("HISTORY").small().strong().color(colors::ACCENT_CYAN));
            ui.horizontal(|ui| {
                ui.label("Undo steps:");
                ui.add(egui::Slider::new(&mut p.undo_steps, 10..=500).logarithmic(true));
            });
            ui.label(egui::RichText::new(format!("Approx RAM ceiling: {} MB", app.history.approx_bytes() / 1024 / 1024)).small().color(colors::TEXT_MUTED));

            ui.add_space(6.0);

            // ── Autosave ──
            ui.label(egui::RichText::new("AUTOSAVE").small().strong().color(colors::ACCENT_CYAN));
            ui.horizontal(|ui| {
                ui.label("Interval:");
                ui.add(egui::Slider::new(&mut p.autosave_secs, 5..=600).suffix(" s"));
            });

            ui.add_space(6.0);

            // ── Audio ──
            ui.label(egui::RichText::new("AUDIO").small().strong().color(colors::ACCENT_CYAN));
            ui.checkbox(&mut p.audio_preview, "Preview audio during playback");

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    apply(app, &p);
                    save(&p);
                    app.toasts.info("Preferences saved");
                    keep_open = false;
                }
                if ui.button("Cancel").clicked() {
                    keep_open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(prefs_path().display().to_string()).small().color(colors::TEXT_MUTED));
                });
            });

            ctx.data_mut(|d| d.insert_temp(id, p));
        });

    app.show_preferences = open && keep_open;
}
