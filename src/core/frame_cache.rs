/// SQLite MVCC-inspired versioned frame render cache.
///
/// Every time the project changes (a new history commit is made), the cache
/// version increments. Reads always see a consistent snapshot; writes for the
/// new version happen concurrently without invalidating in-progress reads.
///
/// Includes strict RAM byte-memory limit guards (default 512 MB) & LRU eviction
/// policy to prevent OS Out-Of-Memory (OOM) crashes under 4K/8K video preview.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// A monotonically increasing version counter.
static GLOBAL_CACHE_VERSION: AtomicU64 = AtomicU64::new(1);

/// Bump the global cache version — call this after every `history.commit`.
pub fn bump_version() -> u64 {
    GLOBAL_CACHE_VERSION.fetch_add(1, Ordering::SeqCst) + 1
}

/// Read the current version without bumping.
pub fn current_version() -> u64 {
    GLOBAL_CACHE_VERSION.load(Ordering::SeqCst)
}

/// A single cached frame entry: raw RGBA pixel bytes for one frame at one version.
#[derive(Clone)]
pub struct CacheEntry {
    pub version: u64,
    pub width: u32,
    pub height: u32,
    /// Raw RGBA8 bytes. Length = width * height * 4.
    pub pixels: Arc<Vec<u8>>,
    /// LRU timestamp for eviction priority.
    pub last_accessed_at: Instant,
}

/// The frame cache. Key is `(frame_index, cache_version)`.
pub struct FrameCache {
    entries: HashMap<(u32, u64), CacheEntry>,
    /// Maximum number of entries before GC triggers automatically.
    max_entries: usize,
    /// Maximum memory limit in bytes for cached RGBA pixel buffers (e.g. 512MB).
    pub max_memory_bytes: usize,
    /// Currently allocated pixel bytes in memory.
    pub current_memory_bytes: usize,
}

impl FrameCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB default
            current_memory_bytes: 0,
        }
    }

    /// Try to retrieve a cached frame for the current global version (updates LRU timestamp).
    pub fn get(&mut self, frame: u32) -> Option<&CacheEntry> {
        let ver = current_version();
        let key = (frame, ver);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_accessed_at = Instant::now();
            return self.entries.get(&key);
        }
        None
    }

    /// Immutable check if frame is cached without updating LRU timestamp.
    pub fn is_cached(&self, frame: u32) -> bool {
        let ver = current_version();
        self.entries.contains_key(&(frame, ver))
    }

    /// Store a rendered frame for the current global version.
    pub fn insert(&mut self, frame: u32, width: u32, height: u32, pixels: Vec<u8>) {
        let ver = current_version();
        let bytes_size = pixels.len();

        // If replacing existing entry, subtract old size first
        if let Some(old) = self.entries.remove(&(frame, ver)) {
            self.current_memory_bytes = self.current_memory_bytes.saturating_sub(old.pixels.len());
        }

        self.current_memory_bytes += bytes_size;
        self.entries.insert(
            (frame, ver),
            CacheEntry {
                version: ver,
                width,
                height,
                pixels: Arc::new(pixels),
                last_accessed_at: Instant::now(),
            },
        );

        // Trigger LRU garbage collection if budget exceeded
        if self.entries.len() > self.max_entries || self.current_memory_bytes > self.max_memory_bytes {
            self.collect_garbage();
        }
    }

    /// Discard stale version entries and LRU evict unaccessed frames when memory budget is exceeded.
    pub fn collect_garbage(&mut self) {
        let current = current_version();
        self.collect_garbage_below(current);

        // Hysteresis LRU Eviction: Purge least-recently used entries down to 75% max_memory_bytes
        if self.current_memory_bytes > self.max_memory_bytes {
            let target_memory = (self.max_memory_bytes as f64 * 0.75) as usize;
            let mut keys_by_access: Vec<((u32, u64), Instant)> = self
                .entries
                .iter()
                .map(|(k, v)| (*k, v.last_accessed_at))
                .collect();

            // Sort by last_accessed_at ascending (oldest first)
            keys_by_access.sort_by_key(|(_k, accessed)| *accessed);

            for (key, _accessed) in keys_by_access {
                if self.current_memory_bytes <= target_memory {
                    break;
                }
                if let Some(removed) = self.entries.remove(&key) {
                    self.current_memory_bytes = self.current_memory_bytes.saturating_sub(removed.pixels.len());
                }
            }
        }
    }

    /// Discard all cache entries whose version is strictly older than `target_version`.
    pub fn collect_garbage_below(&mut self, target_version: u64) {
        let mut freed_bytes = 0usize;
        self.entries.retain(|(_frame, ver), entry| {
            let keep = *ver >= target_version;
            if !keep {
                freed_bytes += entry.pixels.len();
            }
            keep
        });
        self.current_memory_bytes = self.current_memory_bytes.saturating_sub(freed_bytes);
    }

    /// Discard the entire cache.
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.current_memory_bytes = 0;
    }

    /// How many entries are currently held (all versions combined).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// How many entries exist for the current version.
    pub fn current_version_len(&self) -> usize {
        let ver = current_version();
        self.entries.keys().filter(|(_, v)| *v == ver).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_miss_on_new_version() {
        let mut cache = FrameCache::new(256);
        let pixels = vec![0u8; 4];
        const FRAME: u32 = 2_000_001;

        let insert_ver = current_version();
        cache.insert(FRAME, 1, 1, pixels.clone());

        assert!(
            cache.entries.contains_key(&(FRAME, insert_ver)),
            "entry should exist at the version it was inserted",
        );

        bump_version();
        assert_ne!(current_version(), insert_ver, "version should have changed after bump");
        assert!(cache.get(FRAME).is_none(), "get() should miss when current version != insert version");
    }

    #[test]
    fn test_gc_removes_old_versions() {
        let mut cache = FrameCache::new(1024);
        let pixels = Arc::new(vec![0u8; 4]);
        const FRAME: u32 = 1_000_002;

        let old_ver = 1;
        let cur_ver = 2;

        cache.entries.insert((FRAME, old_ver), CacheEntry {
            version: old_ver,
            width: 1,
            height: 1,
            pixels: pixels.clone(),
            last_accessed_at: Instant::now(),
        });
        cache.entries.insert((FRAME, cur_ver), CacheEntry {
            version: cur_ver,
            width: 1,
            height: 1,
            pixels: pixels.clone(),
            last_accessed_at: Instant::now(),
        });

        assert_eq!(cache.len(), 2, "should have 2 versioned entries before GC");
        cache.collect_garbage_below(cur_ver);

        assert!(cache.entries.values().all(|e| e.version >= cur_ver), "all remaining entries must be >= cur_ver");
        assert!(cache.entries.contains_key(&(FRAME, cur_ver)), "current version entry must survive GC");
        assert!(!cache.entries.contains_key(&(FRAME, old_ver)), "old version entry must be removed by GC");
    }

    #[test]
    fn test_lru_memory_limit_purging() {
        let mut cache = FrameCache::new(1024);
        cache.max_memory_bytes = 100; // Small limit for testing

        let pixels = vec![255u8; 40]; // 40 bytes
        cache.insert(1, 1, 1, pixels.clone());
        cache.insert(2, 1, 1, pixels.clone());
        cache.insert(3, 1, 1, pixels.clone()); // Total 120 bytes > 100 max

        assert!(cache.current_memory_bytes <= 100, "LRU memory limit should automatically purge old entries");
    }
}
