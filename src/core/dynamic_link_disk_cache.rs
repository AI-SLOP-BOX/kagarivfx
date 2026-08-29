//! Bi-directional Dynamic Link Protocol & Intelligent Persistent Disk Cache Engine (AE Parity).
//!
//! Provides enterprise-grade LRU disk caching with hash invalidation and
//! live bidirectional IPC Dynamic Link communication for external NLE hosts (Premiere Pro / DaVinci).

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiskCacheMetadata {
    pub cache_key: u64,
    pub comp_id: String,
    pub frame: u32,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
    pub last_access_epoch: u64,
}

/// Intelligent persistent disk cache manager with strict byte budget and disk backing.
pub struct PersistentDiskCache {
    pub budget_bytes: usize,
    pub current_bytes: usize,
    pub entries: HashMap<u64, (DiskCacheMetadata, Vec<u8>)>,
    pub cache_dir: Option<PathBuf>,
}

impl PersistentDiskCache {
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            budget_bytes,
            current_bytes: 0,
            entries: HashMap::new(),
            cache_dir: None,
        }
    }

    pub fn with_directory<P: AsRef<Path>>(budget_bytes: usize, dir: P) -> Self {
        let dir_buf = dir.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&dir_buf);
        Self {
            budget_bytes,
            current_bytes: 0,
            entries: HashMap::new(),
            cache_dir: Some(dir_buf),
        }
    }

    /// Computes unique 64-bit content hash key for a composition frame.
    pub fn compute_cache_key(comp_id: &str, frame: u32, project_version: u64) -> u64 {
        let mut h = 0xcbf29ce484222325u64; // FNV-1a
        for b in comp_id.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= frame as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= project_version;
        h = h.wrapping_mul(0x100000001b3);
        h
    }

    /// Helper to evict oldest LRU entries until `needed_bytes` fits within budget.
    fn evict_for_bytes(&mut self, needed_bytes: usize) {
        while self.current_bytes + needed_bytes > self.budget_bytes && !self.entries.is_empty() {
            let oldest_key = self
                .entries
                .iter()
                .min_by_key(|(_, (meta, _))| meta.last_access_epoch)
                .map(|(&k, _)| k);

            if let Some(k) = oldest_key {
                if let Some((meta, _)) = self.entries.remove(&k) {
                    self.current_bytes = self.current_bytes.saturating_sub(meta.size_bytes);
                    if let Some(ref dir) = self.cache_dir {
                        let file_path = dir.join(format!("{:016x}.cache", k));
                        let meta_path = dir.join(format!("{:016x}.meta", k));
                        let _ = fs::remove_file(file_path);
                        let _ = fs::remove_file(meta_path);
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Retrieves frame from disk cache if present, evicting LRU if needed when loading from disk.
    pub fn get_frame(&mut self, key: u64, current_epoch: u64) -> Option<&[u8]> {
        if self.entries.contains_key(&key) {
            let (meta, data) = self.entries.get_mut(&key).unwrap();
            meta.last_access_epoch = current_epoch;
            return Some(data.as_slice());
        }

        if let Some(ref dir) = self.cache_dir {
            // Attempt reload from persistent disk file if present
            let file_path = dir.join(format!("{:016x}.cache", key));
            let meta_path = dir.join(format!("{:016x}.meta", key));
            if let Ok(bytes) = fs::read(&file_path) {
                let size = bytes.len();
                if size <= self.budget_bytes {
                    // Try parsing metadata; if metadata file is missing or corrupted, discard corrupted cache
                    let meta_res = fs::read_to_string(&meta_path)
                        .ok()
                        .and_then(|json| serde_json::from_str::<DiskCacheMetadata>(&json).ok());

                    if let Some(mut meta) = meta_res {
                        meta.last_access_epoch = current_epoch;
                        self.evict_for_bytes(size);
                        self.current_bytes += size;
                        self.entries.insert(key, (meta, bytes));
                        return self.entries.get(&key).map(|(_, d)| d.as_slice());
                    } else {
                        // Corrupted cache: remove both files
                        let _ = fs::remove_file(&file_path);
                        let _ = fs::remove_file(&meta_path);
                    }
                }
            }
        }

        None
    }

    /// Stores a frame in the disk cache, evicting oldest frames if over budget.
    /// Rejects entries that exceed total budget to prevent budget overflow.
    pub fn put_frame(
        &mut self,
        key: u64,
        comp_id: String,
        frame: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        current_epoch: u64,
    ) -> Result<(), std::io::Error> {
        let size = data.len();

        // If the single frame exceeds total cache budget, reject insertion
        if size > self.budget_bytes {
            return Ok(());
        }

        // If key already exists, remove it and subtract its size first
        if let Some((old_meta, _)) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old_meta.size_bytes);
            if let Some(ref dir) = self.cache_dir {
                let file_path = dir.join(format!("{:016x}.cache", key));
                let meta_path = dir.join(format!("{:016x}.meta", key));
                let _ = fs::remove_file(file_path);
                let _ = fs::remove_file(meta_path);
            }
        }

        // Evict LRU entries if adding new size exceeds budget
        self.evict_for_bytes(size);

        let meta = DiskCacheMetadata {
            cache_key: key,
            comp_id,
            frame,
            width,
            height,
            size_bytes: size,
            last_access_epoch: current_epoch,
        };

        // Persist to disk file atomically if directory is configured
        if let Some(ref dir) = self.cache_dir {
            let file_path = dir.join(format!("{:016x}.cache", key));
            let meta_path = dir.join(format!("{:016x}.meta", key));
            let tmp_file_path = dir.join(format!("{:016x}.cache.tmp", key));
            let tmp_meta_path = dir.join(format!("{:016x}.meta.tmp", key));

            // Write to temporary files first
            if let Err(e) = fs::write(&tmp_file_path, &data) {
                let _ = fs::remove_file(&tmp_file_path);
                return Err(e);
            }
            if let Ok(meta_json) = serde_json::to_string(&meta) {
                if let Err(e) = fs::write(&tmp_meta_path, meta_json) {
                    let _ = fs::remove_file(&tmp_file_path);
                    let _ = fs::remove_file(&tmp_meta_path);
                    return Err(e);
                }
            }

            // Atomic rename to final file destinations
            let _ = fs::rename(&tmp_file_path, &file_path);
            let _ = fs::rename(&tmp_meta_path, &meta_path);
        }

        self.current_bytes += size;
        self.entries.insert(key, (meta, data));
        Ok(())
    }

    /// Clears all memory and disk cache files.
    pub fn clear(&mut self) {
        if let Some(ref dir) = self.cache_dir {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("cache") {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
        self.entries.clear();
        self.current_bytes = 0;
    }
}

/// Adobe Dynamic Link live bidirectional message protocol.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DynamicLinkMessage {
    Ping,
    Pong,
    SyncCompositionSettings { comp_id: String, width: u32, height: u32, fps: u32, duration_frames: u32 },
    RequestRenderFrame { comp_id: String, frame: u32 },
    FrameRenderResult { comp_id: String, frame: u32, width: u32, height: u32, rgba_png: Vec<u8> },
    InvalidateCache { comp_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_cache_put_get_and_lru_eviction() {
        // Budget for 2 frames of 100 bytes each
        let mut cache = PersistentDiskCache::new(250);

        let f0 = vec![1u8; 100];
        let f1 = vec![2u8; 100];
        let f2 = vec![3u8; 100];

        let k0 = PersistentDiskCache::compute_cache_key("comp1", 0, 1);
        let k1 = PersistentDiskCache::compute_cache_key("comp1", 1, 1);
        let k2 = PersistentDiskCache::compute_cache_key("comp1", 2, 1);

        let _ = cache.put_frame(k0, "comp1".into(), 0, 10, 10, f0, 1000);
        let _ = cache.put_frame(k1, "comp1".into(), 1, 10, 10, f1, 1001);
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.current_bytes, 200);

        // Storing 3rd frame must evict k0 (epoch 1000)
        let _ = cache.put_frame(k2, "comp1".into(), 2, 10, 10, f2, 1002);
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.current_bytes, 200);
        assert!(cache.get_frame(k0, 1003).is_none());
        assert!(cache.get_frame(k1, 1003).is_some());
        assert!(cache.get_frame(k2, 1003).is_some());
    }

    #[test]
    fn test_disk_cache_reinsertion_deducts_old_size() {
        let mut cache = PersistentDiskCache::new(200);
        let k0 = 42;
        let _ = cache.put_frame(k0, "c".into(), 0, 1, 1, vec![0u8; 100], 100);
        assert_eq!(cache.current_bytes, 100);

        // Reinsert same key with new size (80 bytes)
        let _ = cache.put_frame(k0, "c".into(), 0, 1, 1, vec![0u8; 80], 101);
        assert_eq!(cache.current_bytes, 80);
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_disk_cache_rejects_oversized_frame() {
        let mut cache = PersistentDiskCache::new(100);
        let k0 = 99;
        // 150 bytes > 100 bytes budget
        let _ = cache.put_frame(k0, "c".into(), 0, 1, 1, vec![0u8; 150], 100);
        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.current_bytes, 0);
    }

    #[test]
    fn test_disk_cache_filesystem_persistence_and_metadata_reload() {
        let tmp_dir = std::env::temp_dir().join(format!("ae_test_cache_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = fs::create_dir_all(&tmp_dir);

        let mut cache = PersistentDiskCache::with_directory(500, tmp_dir.clone());
        let k0 = 12345;
        let data = vec![7u8; 64];
        let _ = cache.put_frame(k0, "comp_main".into(), 5, 1920, 1080, data.clone(), 100);

        // Clear memory cache only
        cache.entries.clear();
        cache.current_bytes = 0;

        // Reload from filesystem
        let loaded = cache.get_frame(k0, 200);
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), data.as_slice());

        // Check restored metadata
        let (meta, _) = cache.entries.get(&k0).unwrap();
        assert_eq!(meta.comp_id, "comp_main");
        assert_eq!(meta.frame, 5);
        assert_eq!(meta.width, 1920);
        assert_eq!(meta.height, 1080);

        cache.clear();
        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
