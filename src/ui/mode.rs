//! Skill-level UI mode: Beginner hides advanced panels/menus so the first
//! session stays approachable; Advanced exposes everything (previous default).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Beginner,
    Advanced,
}

impl UiMode {
    pub fn is_beginner(self) -> bool {
        matches!(self, UiMode::Beginner)
    }

    pub fn is_advanced(self) -> bool {
        matches!(self, UiMode::Advanced)
    }

    pub fn label(self) -> &'static str {
        match self {
            UiMode::Beginner => "初心者 Beginner",
            UiMode::Advanced => "上級者 Advanced",
        }
    }
}

/// Panels/menus hidden in Beginner mode. Central list keeps gating consistent.
pub const BEGINNER_HIDDEN_MENUS: &[&str] = &["OpenFX Plugins", "VFX & Color"];

/// Whether a given menu title should render under the current mode.
pub fn menu_visible(mode: UiMode, title: &str) -> bool {
    if mode.is_advanced() {
        return true;
    }
    !BEGINNER_HIDDEN_MENUS.contains(&title)
}

/// Settings file path (~/.kagari/ui_settings.json).
fn settings_path() -> Option<std::path::PathBuf> {
    dirs_home().map(|h| h.join(".kagari").join("ui_settings.json"))
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(std::path::PathBuf::from))
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct UiSettings {
    #[serde(default)]
    ui_mode_advanced: bool,
}

/// Persist the current mode; best-effort (never panics, silently ignores IO errors).
pub fn save_mode(mode: UiMode) {
    let Some(path) = settings_path() else { return };
    let s = UiSettings {
        ui_mode_advanced: mode.is_advanced(),
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&s) {
        let _ = std::fs::write(path, json);
    }
}

/// Load the persisted mode at startup. Defaults to Beginner for first run.
pub fn load_mode() -> UiMode {
    let Some(path) = settings_path() else {
        return UiMode::Beginner;
    };
    match std::fs::read_to_string(path) {
        Ok(json) => {
            let s: UiSettings = serde_json::from_str(&json).unwrap_or_default();
            if s.ui_mode_advanced {
                UiMode::Advanced
            } else {
                UiMode::Beginner
            }
        }
        Err(_) => UiMode::Beginner,
    }
}
