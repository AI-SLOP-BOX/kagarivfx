//! Versionable cross-domain document joining the VFX project and audio clock.

use crate::core::audio_types::MixerChannel;
use crate::core::automation_binding::{AutomationBinding, ProductionClock};
use crate::core::timeline::Project;
use crate::core::unified_time::TempoMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDocumentSettings {
    pub sample_rate: u32,
    pub master_gain: f32,
    pub channels: Vec<MixerChannel>,
}

impl Default for AudioDocumentSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            master_gain: 1.0,
            channels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionDocument {
    pub schema_version: u32,
    pub project: Project,
    pub audio: AudioDocumentSettings,
    pub tempo: TempoMap,
    pub bindings: Vec<AutomationBinding>,
}

impl ProductionDocument {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    pub fn new(project: Project) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            project,
            audio: AudioDocumentSettings::default(),
            tempo: TempoMap::new(120.0),
            bindings: Vec::new(),
        }
    }

    pub fn clock(&self) -> ProductionClock {
        ProductionClock {
            tempo: self.tempo.clone(),
            sample_rate: self.audio.sample_rate,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version > Self::CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "production document schema {} is newer than supported {}",
                self.schema_version,
                Self::CURRENT_SCHEMA_VERSION
            ));
        }
        if !(1..=384_000).contains(&self.audio.sample_rate) {
            return Err("audio sample rate is outside the supported range".into());
        }
        if !self.audio.master_gain.is_finite() || self.audio.master_gain < 0.0 {
            return Err("audio master gain must be finite and non-negative".into());
        }
        for binding in &self.bindings {
            binding.validate().map_err(str::to_owned)?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let document: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        document.validate()?;
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_audio_vfx_contract() {
        let mut document = ProductionDocument::new(Project::default());
        document.audio.sample_rate = 44_100;
        document.bindings.push(AutomationBinding {
            source: "audio.bass".into(),
            target: "vfx.glow.intensity".into(),
            curve: crate::core::automation_binding::AutomationCurve {
                points: vec![crate::core::automation_binding::AutomationPoint {
                    time: crate::core::unified_time::Time::ZERO,
                    value: 0.5,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 100.0,
        });

        let restored = ProductionDocument::from_json(&document.to_json().unwrap()).unwrap();
        assert_eq!(restored.audio.sample_rate, 44_100);
        assert_eq!(restored.bindings.len(), 1);
        assert_eq!(
            restored
                .clock()
                .sample_position(crate::core::unified_time::Time::new(1, 1)),
            44_100
        );
    }

    #[test]
    fn rejects_future_schema_and_invalid_audio() {
        let mut document = ProductionDocument::new(Project::default());
        document.schema_version += 1;
        assert!(document.validate().is_err());

        document.schema_version = ProductionDocument::CURRENT_SCHEMA_VERSION;
        document.audio.sample_rate = 0;
        assert!(document.validate().is_err());
    }
}
