//! Versionable cross-domain document joining the VFX project and audio clock.

use crate::core::audio_types::MixerChannel;
use crate::core::automation_binding::{AutomationBinding, ProductionClock};
use crate::core::timeline::{LayerType, Project, ProjectItemType};
use crate::core::unified_time::TempoMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    #[serde(default)]
    pub audio: AudioDocumentSettings,
    #[serde(default)]
    pub tempo: TempoMap,
    #[serde(default)]
    pub bindings: Vec<AutomationBinding>,
}

impl ProductionDocument {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;
    pub const MAX_AUDIO_CHANNELS: usize = 4096;
    pub const MAX_BINDINGS: usize = 8192;
    pub const MAX_COMPOSITION_FRAMES: u32 = 10_000_000;

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
        if self.project.compositions.is_empty() {
            return Err("production document must contain at least one composition".into());
        }
        if self.project.active_composition_idx >= self.project.compositions.len() {
            return Err("production document active composition index is out of range".into());
        }
        let mut composition_ids = HashSet::new();
        for composition in &self.project.compositions {
            validate_composition(composition, 0, &mut composition_ids)?;
        }
        for composition in &self.project.compositions {
            validate_precomp_references(composition, &composition_ids)?;
        }
        let mut asset_ids = HashSet::new();
        let folder_ids = self
            .project
            .assets
            .iter()
            .filter_map(|asset| match &asset.item_type {
                ProjectItemType::Folder { .. } => Some(asset.id.as_str()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        for asset in &self.project.assets {
            if asset.id.trim().is_empty() || !asset_ids.insert(asset.id.clone()) {
                return Err("asset ids must be non-empty and unique".into());
            }
            if let Some(parent_folder) = &asset.parent_folder {
                if !folder_ids.contains(parent_folder.as_str()) {
                    return Err("asset parent folder reference is invalid".into());
                }
            }
            match &asset.item_type {
                ProjectItemType::Composition { comp_idx }
                    if *comp_idx >= self.project.compositions.len() =>
                {
                    return Err("asset composition index is out of range".into());
                }
                ProjectItemType::Image { width, height } if *width == 0 || *height == 0 => {
                    return Err("asset image dimensions must be non-zero".into());
                }
                ProjectItemType::Image { path, .. }
                | ProjectItemType::Video { source: path, .. }
                | ProjectItemType::Audio { path, .. }
                    if path.trim().is_empty() =>
                {
                    return Err("media asset path must not be empty".into());
                }
                ProjectItemType::Solid { color }
                    if color.iter().any(|channel| !channel.is_finite()) =>
                {
                    return Err("solid asset color must be finite".into());
                }
                ProjectItemType::Video { duration_sec, .. }
                | ProjectItemType::Audio { duration_sec, .. }
                    if !duration_sec.is_finite() || *duration_sec < 0.0 =>
                {
                    return Err("asset duration must be finite and non-negative".into());
                }
                _ => {}
            }
        }
        let folder_parents = self
            .project
            .assets
            .iter()
            .filter_map(|asset| match &asset.item_type {
                ProjectItemType::Folder { .. } => {
                    Some((asset.id.as_str(), asset.parent_folder.as_deref()))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        for folder_id in folder_parents.keys() {
            let mut current = Some(*folder_id);
            let mut visited = HashSet::new();
            while let Some(id) = current {
                if !visited.insert(id) {
                    return Err("asset folder hierarchy contains a cycle".into());
                }
                current = folder_parents.get(id).copied().flatten();
            }
        }
        if !(1..=384_000).contains(&self.audio.sample_rate) {
            return Err("audio sample rate is outside the supported range".into());
        }
        if !self.audio.master_gain.is_finite() || self.audio.master_gain < 0.0 {
            return Err("audio master gain must be finite and non-negative".into());
        }
        if self.audio.channels.len() > Self::MAX_AUDIO_CHANNELS {
            return Err("production document contains too many audio channels".into());
        }
        for channel in &self.audio.channels {
            channel.validate().map_err(str::to_owned)?;
        }
        self.tempo.validate().map_err(str::to_owned)?;
        if self.bindings.len() > Self::MAX_BINDINGS {
            return Err("production document contains too many automation bindings".into());
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
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = target.with_extension(format!(
            "production.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let json = self.to_json()?;
        let result = (|| {
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
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let json = std::fs::read_to_string(path.as_ref())
            .map_err(|error| format!("failed to read production document: {error}"))?;
        Self::from_json(&json)
    }

    /// Upgrade a legacy Project JSON into the unified production document.
    /// Audio, tempo, and binding data receive safe defaults until the caller
    /// supplies domain-specific values.
    pub fn from_legacy_project_json(json: &str) -> Result<Self, String> {
        let project = crate::core::project_migration::load_project_migrated(json)?;
        let document = Self::new(project);
        document.validate()?;
        Ok(document)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.project
    }
}

fn validate_composition(
    composition: &crate::core::timeline::Composition,
    depth: usize,
    composition_ids: &mut HashSet<String>,
) -> Result<(), String> {
    if depth > 1024 {
        return Err("production document composition nesting is too deep".into());
    }
    if composition.id.trim().is_empty() {
        return Err("composition id must not be empty".into());
    }
    if composition.name.trim().is_empty() {
        return Err("composition name must not be empty".into());
    }
    if !composition_ids.insert(composition.id.clone()) {
        return Err(format!("duplicate composition id: {}", composition.id));
    }
    if !(1..=65_535).contains(&composition.width)
        || !(1..=65_535).contains(&composition.height)
    {
        return Err("composition dimensions are outside the supported range".into());
    }
    if !(1..=240).contains(&composition.fps)
        || !(1..=ProductionDocument::MAX_COMPOSITION_FRAMES)
            .contains(&composition.duration_frames)
    {
        return Err("composition frame rate or duration is invalid".into());
    }
    if !composition.motion_blur_shutter_angle.is_finite()
        || !composition.motion_blur_shutter_phase.is_finite()
    {
        return Err("composition motion blur settings must be finite".into());
    }
    if composition.background_color.iter().any(|channel| !channel.is_finite()) {
        return Err("composition background color must be finite".into());
    }
    for layer in &composition.layers {
        if layer.in_frame >= layer.out_frame {
            return Err("layer frame range must have a positive duration".into());
        }
    }
    for nested in &composition.sub_compositions {
        validate_composition(nested, depth + 1, composition_ids)?;
    }
    Ok(())
}

fn validate_precomp_references(
    composition: &crate::core::timeline::Composition,
    composition_ids: &HashSet<String>,
) -> Result<(), String> {
    for layer in &composition.layers {
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            if !composition_ids.contains(comp_id) {
                return Err(format!("precomp references missing composition: {comp_id}"));
            }
        }
    }
    for nested in &composition.sub_compositions {
        validate_precomp_references(nested, composition_ids)?;
    }
    Ok(())
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

        let mut invalid_project = Project::default();
        invalid_project.active_composition_idx = invalid_project.compositions.len();
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].width = 0;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].duration_frames = 0;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].background_color[0] = f32::NAN;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].duration_frames =
            ProductionDocument::MAX_COMPOSITION_FRAMES + 1;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].id.clear();
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].name = "  ".into();
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let duplicate = invalid_project.compositions[0].clone();
        invalid_project.compositions.push(duplicate);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.assets.push(crate::core::timeline::ProjectItem::new(
            "bad-comp",
            "Bad Comp",
            ProjectItemType::Composition { comp_idx: 99 },
        ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.assets.push(crate::core::timeline::ProjectItem::new(
            "bad-image",
            "Bad Image",
            ProjectItemType::Image {
                path: "image.png".into(),
                width: 0,
                height: 1080,
            },
        ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.assets.push(crate::core::timeline::ProjectItem::new(
            "empty-media",
            "Empty Media",
            ProjectItemType::Audio {
                path: "  ".into(),
                duration_sec: 1.0,
            },
        ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.assets.push(crate::core::timeline::ProjectItem::new(
            "bad-solid",
            "Bad Solid",
            ProjectItemType::Solid {
                color: [f32::INFINITY, 0.0, 0.0, 1.0],
            },
        ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.assets.push(crate::core::timeline::ProjectItem::new(
            "item_comp1",
            "Duplicate",
            ProjectItemType::Folder { name: "x".into() },
        ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut asset = crate::core::timeline::ProjectItem::new(
            "asset",
            "Asset",
            ProjectItemType::Folder { name: "x".into() },
        );
        asset.parent_folder = Some("missing".into());
        invalid_project.assets.push(asset);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut folder_a = crate::core::timeline::ProjectItem::new(
            "folder-a",
            "A",
            ProjectItemType::Folder { name: "A".into() },
        );
        folder_a.parent_folder = Some("folder-b".into());
        let mut folder_b = crate::core::timeline::ProjectItem::new(
            "folder-b",
            "B",
            ProjectItemType::Folder { name: "B".into() },
        );
        folder_b.parent_folder = Some("folder-a".into());
        invalid_project.assets.extend([folder_a, folder_b]);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers.push(crate::core::timeline::Layer::new(
            "missing-precomp",
            "Missing Precomp",
            LayerType::PreComp {
                comp_id: "does-not-exist".into(),
            },
            300,
        ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0].out_frame =
            invalid_project.compositions[0].layers[0].in_frame;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());
    }

    #[test]
    fn rejects_unbounded_audio_and_binding_collections() {
        let mut document = ProductionDocument::new(Project::default());
        document
            .audio
            .channels
            .resize(ProductionDocument::MAX_AUDIO_CHANNELS + 1, MixerChannel::default());
        assert!(document.validate().is_err());

        let mut document = ProductionDocument::new(Project::default());
        document.bindings.resize(
            ProductionDocument::MAX_BINDINGS + 1,
            AutomationBinding {
                source: "audio.x".into(),
                target: "vfx.y".into(),
                curve: crate::core::automation_binding::AutomationCurve {
                    points: vec![crate::core::automation_binding::AutomationPoint {
                        time: crate::core::unified_time::Time::ZERO,
                        value: 0.0,
                    }],
                },
                input_min: 0.0,
                input_max: 1.0,
                output_min: 0.0,
                output_max: 1.0,
            },
        );
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

    #[test]
    fn failed_atomic_replace_removes_temporary_document() {
        let directory = std::env::temp_dir().join(format!(
            "aevfx_production_document_failure_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let target = directory.join("session.aura");
        std::fs::create_dir(&target).unwrap();

        let document = ProductionDocument::new(Project::default());
        assert!(document.save_atomic(&target).is_err());

        let temporary_files = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().starts_with("session.production."))
            .collect::<Vec<_>>();
        assert!(temporary_files.is_empty(), "temporary files: {temporary_files:?}");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn upgrades_legacy_project_json_with_safe_defaults() {
        let legacy =
            crate::core::project_migration::save_project_versioned(&Project::default()).unwrap();
        let document = ProductionDocument::from_legacy_project_json(&legacy).unwrap();

        assert_eq!(document.project().compositions.len(), 1);
        assert_eq!(document.audio.sample_rate, 48_000);
        assert!(document.bindings.is_empty());
    }

    #[test]
    fn legacy_migration_rejects_unusable_project() {
        let mut project = Project::default();
        project.active_composition_idx = project.compositions.len();
        let legacy = crate::core::project_migration::save_project_versioned(&project).unwrap();
        assert!(ProductionDocument::from_legacy_project_json(&legacy).is_err());
    }

    #[test]
    fn partial_production_documents_receive_domain_defaults() {
        let mut value: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&ProductionDocument::new(Project::default())).unwrap(),
        )
        .unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("audio");
        object.remove("tempo");
        object.remove("bindings");
        let document =
            ProductionDocument::from_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(document.audio.sample_rate, 48_000);
        assert_eq!(document.tempo.beat_at(crate::core::unified_time::Time::new(1, 1)), 2.0);
        assert!(document.bindings.is_empty());
    }
}
