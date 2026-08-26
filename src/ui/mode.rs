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
pub const BEGINNER_HIDDEN_MENUS: &[&str] = &[
    "OpenFX Plugins",
    "VFX & Color",
];

/// Whether a given menu title should render under the current mode.
pub fn menu_visible(mode: UiMode, title: &str) -> bool {
    if mode.is_advanced() {
        return true;
    }
    !BEGINNER_HIDDEN_MENUS.contains(&title)
}
