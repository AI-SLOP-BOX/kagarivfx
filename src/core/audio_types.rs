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

impl MixerChannel {
    pub fn validate(self) -> Result<(), &'static str> {
        if !self.gain_db.is_finite() || self.gain_db < -144.0 || self.gain_db > 24.0 {
            return Err("mixer gain must be finite and within -144..=24 dB");
        }
        if !self.pan.is_finite() || !(-1.0..=1.0).contains(&self.pan) {
            return Err("mixer pan must be finite and within -1..=1");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_channel_accepts_normal_values() {
        assert!(MixerChannel::default().validate().is_ok());
        assert!(MixerChannel {
            gain_db: 6.0,
            pan: -1.0,
            ..Default::default()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn mixer_channel_rejects_non_finite_and_extreme_values() {
        for gain_db in [f32::NAN, f32::INFINITY, -145.0, 25.0] {
            assert!(MixerChannel {
                gain_db,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
        for pan in [f32::NAN, f32::NEG_INFINITY, -1.1, 1.1] {
            assert!(MixerChannel {
                pan,
                ..Default::default()
            }
            .validate()
            .is_err());
        }
    }
}
