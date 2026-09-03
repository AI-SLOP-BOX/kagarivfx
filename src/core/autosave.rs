//! Crash-recovery autosave system.
//!
//! Periodically snapshots the project to a rotating set of recovery files so that
//! a crash (or power loss) loses at most one autosave interval of work. Recovery
//! files are validated on load: a truncated or corrupt snapshot is skipped rather
//! than propagated to the user.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::core::production_document::ProductionDocument;
use crate::core::timeline::Project;

#[derive(serde::Serialize, serde::Deserialize)]
struct AutosaveSnapshot {
    project: Project,
    #[serde(default)]
    production_document: Option<ProductionDocument>,
}

/// Number of rotating recovery files kept on disk.
pub const MAX_AUTOSAVE_SLOTS: usize = 5;

static AUTOSAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Autosave manager: tracks dirty state and writes rotating recovery snapshots.
pub struct AutosaveManager {
    /// Directory where recovery files are written.
    recovery_dir: PathBuf,
    /// Base file stem, e.g. "recovery" -> recovery_0.json .. recovery_4.json
    stem: String,
    /// How often to save while there are unsaved changes.
    interval: Duration,
    /// Last time an autosave was written.
    last_save: Instant,
    /// Round-robin slot for the next write.
    next_slot: usize,
    /// Set true by `mark_dirty` when the project changes.
    dirty: bool,
}

impl AutosaveManager {
    pub fn new(recovery_dir: impl Into<PathBuf>) -> Self {
        Self {
            recovery_dir: recovery_dir.into(),
            stem: "recovery".to_string(),
            interval: Duration::from_secs(30),
            last_save: Instant::now(),
            next_slot: 0,
            dirty: false,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Runtime interval adjustment (seconds), used by the Preferences dialog.
    pub fn set_interval_secs(&mut self, secs: u64) {
        self.interval = Duration::from_secs(secs.clamp(5, 3600));
    }

    /// Current autosave interval in seconds.
    pub fn interval_secs(&self) -> u64 {
        self.interval.as_secs()
    }

    /// Call whenever the project is modified.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns the path of the recovery file for a slot.
    fn slot_path(&self, slot: usize) -> PathBuf {
        self.recovery_dir
            .join(format!("{}_{}.json", self.stem, slot))
    }

    /// Writes a recovery snapshot if the dirty flag is set and the interval elapsed.
    /// Returns the path written, if any.
    pub fn tick(&mut self, project: &Project) -> Option<PathBuf> {
        if !self.dirty || self.last_save.elapsed() < self.interval {
            return None;
        }
        let path = self.write_snapshot(project, None);
        if path.is_ok() {
            self.dirty = false;
            self.last_save = Instant::now();
        }
        path.ok()
    }

    /// Forces an immediate snapshot (e.g. on app exit or before risky operations).
    pub fn save_now(&mut self, project: &Project) -> std::io::Result<PathBuf> {
        let path = self.write_snapshot(project, None)?;
        self.dirty = false;
        self.last_save = Instant::now();
        Ok(path)
    }

    pub fn tick_production(
        &mut self,
        project: &Project,
        document: &ProductionDocument,
    ) -> Option<PathBuf> {
        if !self.dirty || self.last_save.elapsed() < self.interval {
            return None;
        }
        let mut current_document = document.clone();
        current_document.project = project.clone();
        let path = self.write_snapshot(project, Some(&current_document));
        if path.is_ok() {
            self.dirty = false;
            self.last_save = Instant::now();
        }
        path.ok()
    }

    /// Writes to the next rotating slot, tolerating individual write failures.
    fn write_snapshot(
        &mut self,
        project: &Project,
        production_document: Option<&ProductionDocument>,
    ) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.recovery_dir)?;
        let path = self.slot_path(self.next_slot % MAX_AUTOSAVE_SLOTS);
        self.next_slot = (self.next_slot + 1) % MAX_AUTOSAVE_SLOTS;

        // Atomic write: a unique temp file prevents concurrent managers from
        // overwriting each other's in-flight snapshot.
        let sequence = AUTOSAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("json.{}.{}.tmp", std::process::id(), sequence));
        let json = serde_json::to_string(&AutosaveSnapshot {
            project: project.clone(),
            production_document: production_document.cloned(),
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let result = (|| {
            std::fs::write(&tmp, json)?;
            std::fs::rename(&tmp, &path)?;
            Ok(path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    /// Loads the newest valid recovery snapshot, if one exists.
    /// Corrupt/truncated slots are skipped automatically.
    pub fn load_latest_recovery(&self) -> Option<Project> {
        let mut slots: Vec<(std::time::SystemTime, PathBuf)> = (0..MAX_AUTOSAVE_SLOTS)
            .map(|i| self.slot_path(i))
            .filter_map(|p| {
                let meta = std::fs::metadata(&p).ok()?;
                let modified = meta.modified().ok()?;
                Some((modified, p))
            })
            .collect();
        slots.sort_by(|a, b| b.0.cmp(&a.0)); // newest first

        for (_, path) in slots {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<AutosaveSnapshot>(&json) {
                    if is_recoverable_project(&snapshot.project) {
                        return Some(snapshot.project);
                    }
                    log::warn!(
                        "[Autosave] Skipping invalid project recovery file {:?}",
                        path
                    );
                }
                if let Ok(project) = serde_json::from_str::<Project>(&json) {
                    if is_recoverable_project(&project) {
                        return Some(project);
                    }
                    log::warn!(
                        "[Autosave] Skipping invalid project recovery file {:?}",
                        path
                    );
                }
                log::warn!("[Autosave] Skipping corrupt recovery file {:?}", path);
            }
        }
        None
    }

    pub fn load_latest_production(&self) -> Option<ProductionDocument> {
        let mut slots: Vec<(std::time::SystemTime, PathBuf)> = (0..MAX_AUTOSAVE_SLOTS)
            .map(|i| self.slot_path(i))
            .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
            .collect();
        slots.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, path) in slots {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<AutosaveSnapshot>(&json) {
                    if let Some(document) = snapshot.production_document {
                        if document.validate().is_ok() && is_recoverable_project(&document.project)
                        {
                            return Some(document);
                        }
                        log::warn!("[Autosave] Skipping invalid production document {:?}", path);
                    }
                }
            }
        }
        None
    }

    /// True if at least one recovery snapshot exists on disk.
    pub fn has_recovery(&self) -> bool {
        (0..MAX_AUTOSAVE_SLOTS).any(|i| self.slot_path(i).exists())
    }

    /// Removes all recovery files (e.g. after a clean, successful save).
    pub fn clear_recovery(&self) {
        for i in 0..MAX_AUTOSAVE_SLOTS {
            let _ = std::fs::remove_file(self.slot_path(i));
        }
    }
}

fn is_recoverable_project(project: &Project) -> bool {
    !project.compositions.is_empty() && project.active_composition_idx < project.compositions.len()
}

/// Returns true if the path looks like a valid project JSON (cheap sanity check
/// used by the recovery picker UI).
pub fn is_valid_project_file(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| {
            ProductionDocument::from_json(&s)
                .map(|_| ())
                .or_else(|_| serde_json::from_str::<Project>(&s).map(|_| ()))
                .ok()
        })
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn sample_project() -> Project {
        let mut comp = Composition::new("c1".into(), "AutoSaveComp".into(), 64, 64, 30, 30);
        let layer = Layer::new(
            "l1".into(),
            "Solid".into(),
            LayerType::Solid { color: [1.0; 4] },
            30,
        );
        comp.layers.push(layer);
        Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        }
    }

    #[test]
    fn test_tick_writes_only_when_dirty_and_due() {
        let dir = std::env::temp_dir().join(format!("aevfx_autosave_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut mgr = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));

        // Not dirty → no write
        assert!(mgr.tick(&sample_project()).is_none());
        // Dirty → writes
        mgr.mark_dirty();
        let path = mgr
            .tick(&sample_project())
            .expect("should write when dirty");
        assert!(path.exists());

        // Recovery loads back the same project
        assert!(mgr.has_recovery());
        let recovered = mgr.load_latest_recovery().expect("recovery should load");
        assert_eq!(recovered.compositions[0].name, "AutoSaveComp");

        // Clear removes everything
        mgr.clear_recovery();
        assert!(!mgr.has_recovery());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn production_autosave_uses_latest_project_with_metadata() {
        let dir =
            std::env::temp_dir().join(format!("aevfx_autosave_production_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut manager = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));
        let mut document = ProductionDocument::new(sample_project());
        document.audio.sample_rate = 44_100;
        let mut latest = sample_project();
        latest.active_composition_mut().name = "Latest".into();
        manager.mark_dirty();
        manager
            .tick_production(&latest, &document)
            .expect("writes production snapshot");

        let recovered = manager.load_latest_recovery().expect("recovers project");
        assert_eq!(recovered.active_composition().name, "Latest");
        assert_eq!(
            manager.load_latest_production().unwrap().audio.sample_rate,
            44_100
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_project_snapshot_remains_recoverable() {
        let dir =
            std::env::temp_dir().join(format!("aevfx_autosave_legacy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let json = serde_json::to_string(&sample_project()).unwrap();
        std::fs::write(dir.join("recovery_0.json"), json).unwrap();
        let manager = AutosaveManager::new(&dir);
        assert_eq!(
            manager
                .load_latest_recovery()
                .unwrap()
                .active_composition()
                .name,
            "AutoSaveComp"
        );
        assert!(manager.load_latest_production().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_production_snapshot_is_not_returned_as_recovery() {
        let dir = std::env::temp_dir().join(format!(
            "aevfx_autosave_invalid_production_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut document = ProductionDocument::new(sample_project());
        document.audio.sample_rate = 0;
        let snapshot = AutosaveSnapshot {
            project: sample_project(),
            production_document: Some(document),
        };
        std::fs::write(
            dir.join("recovery_0.json"),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();

        let manager = AutosaveManager::new(&dir);
        assert!(manager.load_latest_production().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn project_file_validation_accepts_legacy_and_production_documents() {
        let dir = std::env::temp_dir().join(format!(
            "aevfx_project_file_validation_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let legacy_path = dir.join("legacy.json");
        let production_path = dir.join("production.aura");
        let invalid_path = dir.join("invalid.json");
        std::fs::write(
            &legacy_path,
            serde_json::to_string(&sample_project()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            &production_path,
            ProductionDocument::new(sample_project()).to_json().unwrap(),
        )
        .unwrap();
        std::fs::write(&invalid_path, "{\"project\": null}").unwrap();

        assert!(is_valid_project_file(&legacy_path));
        assert!(is_valid_project_file(&production_path));
        assert!(!is_valid_project_file(&invalid_path));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recovery_skips_project_with_invalid_active_composition() {
        let dir = std::env::temp_dir().join(format!(
            "aevfx_autosave_invalid_project_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut project = sample_project();
        project.active_composition_idx = project.compositions.len();
        std::fs::write(
            dir.join("recovery_0.json"),
            serde_json::to_string(&project).unwrap(),
        )
        .unwrap();

        assert!(AutosaveManager::new(&dir).load_latest_recovery().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn production_recovery_skips_document_with_invalid_project() {
        let dir = std::env::temp_dir().join(format!(
            "aevfx_autosave_invalid_production_project_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut project = sample_project();
        project.active_composition_idx = project.compositions.len();
        let document = ProductionDocument::new(project.clone());
        let snapshot = AutosaveSnapshot {
            project,
            production_document: Some(document),
        };
        std::fs::write(
            dir.join("recovery_0.json"),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();

        assert!(AutosaveManager::new(&dir)
            .load_latest_production()
            .is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn failed_tick_keeps_dirty_state_for_retry() {
        let path =
            std::env::temp_dir().join(format!("aevfx_autosave_blocked_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::write(&path, "not a directory").unwrap();
        let mut manager = AutosaveManager::new(&path).with_interval(Duration::from_secs(0));
        manager.mark_dirty();
        assert!(manager.tick(&sample_project()).is_none());
        std::fs::remove_file(&path).unwrap();
        assert!(manager.tick(&sample_project()).is_some());
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn failed_snapshot_replace_removes_unique_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "aevfx_autosave_replace_failure_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir(dir.join("recovery_0.json")).unwrap();

        let mut manager = AutosaveManager::new(&dir).with_interval(Duration::ZERO);
        manager.mark_dirty();
        assert!(manager.tick(&sample_project()).is_none());

        let temporary_files = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().contains(".json."))
            .collect::<Vec<_>>();
        assert!(
            temporary_files.is_empty(),
            "temporary files: {temporary_files:?}"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_rotating_slots_never_exceed_limit() {
        let dir = std::env::temp_dir().join(format!("aevfx_autosave_rot_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut mgr = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));

        for i in 0..(MAX_AUTOSAVE_SLOTS * 3) {
            mgr.mark_dirty();
            mgr.tick(&sample_project()).expect("each tick should write");
            assert_eq!(mgr.next_slot, (i + 1) % MAX_AUTOSAVE_SLOTS);
        }

        // Count recovery files on disk
        let count = std::fs::read_dir(&dir).unwrap().count();
        assert!(
            count <= MAX_AUTOSAVE_SLOTS,
            "must not exceed slot limit, got {}",
            count
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_corrupt_recovery_files_are_skipped() {
        let dir =
            std::env::temp_dir().join(format!("aevfx_autosave_corrupt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Slot 0: truncated garbage; slot 1: valid project
        std::fs::write(dir.join("recovery_0.json"), "{\"compositions\": [").unwrap();
        let good = dir.join("recovery_1.json");
        serde_json::to_writer_pretty(std::fs::File::create(&good).unwrap(), &sample_project())
            .unwrap();

        let mgr = AutosaveManager::new(&dir);
        let recovered = mgr
            .load_latest_recovery()
            .expect("valid slot must be found");
        assert_eq!(recovered.compositions[0].name, "AutoSaveComp");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_atomic_write_leaves_no_tmp_files() {
        let dir = std::env::temp_dir().join(format!("aevfx_autosave_tmp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut mgr = AutosaveManager::new(&dir).with_interval(Duration::from_secs(0));
        mgr.mark_dirty();
        mgr.save_now(&sample_project()).expect("save_now works");

        let tmp_count = std::fs::read_dir(&dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == "tmp")
            })
            .count();
        assert_eq!(
            tmp_count, 0,
            "no .tmp files may remain after successful save"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
