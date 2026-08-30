//! Audio-domain value types shared by the mixer, renderer, and UI adapters.

use serde::{Deserialize, Serialize};

/// Per-channel mixer controls. This type intentionally lives in Core so
/// headless rendering does not depend on application state or egui.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MixerChannel {
    pub gain_db: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}
