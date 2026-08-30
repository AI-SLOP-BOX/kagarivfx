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

    pub fn save_atomic(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let target = path.as_ref();
        let temporary = target.with_extension("production.tmp");
        let json = self.to_json()?;
        let mut file = std::fs::File::create(&temporary)
            .map_err(|error| format!("failed to create production document: {error}"))?;
        use std::io::Write;
        file.write_all(json.as_bytes())
            .map_err(|error| format!("failed to write production document: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync production document: {error}"))?;
        drop(file);
        std::fs::rename(&temporary, target)
            .map_err(|error| format!("failed to replace production document: {error}"))?;
        Ok(())
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let json = std::fs::read_to_string(path.as_ref())
            .map_err(|error| format!("failed to read production document: {error}"))?;
        Self::from_json(&json)
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

    #[test]
    fn atomic_save_and_load_preserve_contract() {
        let directory =
            std::env::temp_dir().join(format!("aevfx_production_document_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("session.aura");
        let document = ProductionDocument::new(Project::default());

        document.save_atomic(&path).unwrap();
        let loaded = ProductionDocument::load(&path).unwrap();
        assert_eq!(loaded.schema_version, document.schema_version);
        assert!(path.exists());
        assert!(!directory.join("session.production.tmp").exists());

        let _ = std::fs::remove_dir_all(directory);
    }
}
