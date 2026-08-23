//! Crash-recovery autosave system.
//!
//! Periodically snapshots the project to a rotating set of recovery files so that
//! a crash (or power loss) loses at most one autosave interval of work. Recovery
//! files are validated on load: a truncated or corrupt snapshot is skipped rather
//! than propagated to the user.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::core::timeline::Project;

/// Number of rotating recovery files kept on disk.
pub const MAX_AUTOSAVE_SLOTS: usize = 5;

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

    /// Call whenever the project is modified.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns the path of the recovery file for a slot.
    fn slot_path(&self, slot: usize) -> PathBuf {
        self.recovery_dir.join(format!("{}_{}.json", self.stem, slot))
    }

    /// Writes a recovery snapshot if the dirty flag is set and the interval elapsed.
    /// Returns the path written, if any.
    pub fn tick(&mut self, project: &Project) -> Option<PathBuf> {
        if !self.dirty || self.last_save.elapsed() < self.interval {
            return None;
        }
        let path = self.write_snapshot(project);
        self.dirty = false;
        self.last_save = Instant::now();
        path.ok()
    }

    /// Forces an immediate snapshot (e.g. on app exit or before risky operations).
    pub fn save_now(&mut self, project: &Project) -> std::io::Result<PathBuf> {
        let path = self.write_snapshot(project)?;
        self.dirty = false;
        self.last_save = Instant::now();
        Ok(path)
    }

    /// Writes to the next rotating slot, tolerating individual write failures.
    fn write_snapshot(&mut self, project: &Project) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.recovery_dir)?;
        let path = self.slot_path(self.next_slot % MAX_AUTOSAVE_SLOTS);
        self.next_slot = (self.next_slot + 1) % MAX_AUTOSAVE_SLOTS;

        // Atomic write: temp file + rename so a crash mid-write cannot truncate the slot.
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string(project)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
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
                if let Ok(project) = serde_json::from_str::<Project>(&json) {
                    return Some(project);
                }
                log::warn!("[Autosave] Skipping corrupt recovery file {:?}", path);
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

/// Returns true if the path looks like a valid project JSON (cheap sanity check
/// used by the recovery picker UI).
pub fn is_valid_project_file(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Project>(&s).ok())
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn sample_project() -> Project {
        let mut comp = Composition::new("c1".into(), "AutoSaveComp".into(), 64, 64, 30, 30);
        let layer = Layer::new("l1".into(), "Solid".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        comp.layers.push(layer);
        Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
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
        let path = mgr.tick(&sample_project()).expect("should write when dirty");
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
        assert!(count <= MAX_AUTOSAVE_SLOTS, "must not exceed slot limit, got {}", count);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_corrupt_recovery_files_are_skipped() {
        let dir = std::env::temp_dir().join(format!("aevfx_autosave_corrupt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Slot 0: truncated garbage; slot 1: valid project
        std::fs::write(dir.join("recovery_0.json"), "{\"compositions\": [").unwrap();
        let good = dir.join("recovery_1.json");
        serde_json::to_writer_pretty(
            std::fs::File::create(&good).unwrap(),
            &sample_project(),
        )
        .unwrap();

        let mgr = AutosaveManager::new(&dir);
        let recovered = mgr.load_latest_recovery().expect("valid slot must be found");
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
            .filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "tmp"))
            .count();
        assert_eq!(tmp_count, 0, "no .tmp files may remain after successful save");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
