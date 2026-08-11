/// SQLite MVCC-inspired versioned frame render cache.
///
/// Every time the project changes (a new history commit is made), the cache
/// version increments. Reads always see a consistent snapshot; writes for the
/// new version happen concurrently without invalidating in-progress reads.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A monotonically increasing version counter.
/// Bump this whenever the project state changes (history commit).
static GLOBAL_CACHE_VERSION: AtomicU64 = AtomicU64::new(1);

/// Bump the global cache version — call this after every `history.commit`.
pub fn bump_version() -> u64 {
    GLOBAL_CACHE_VERSION.fetch_add(1, Ordering::SeqCst) + 1
}

/// Read the current version without bumping.
pub fn current_version() -> u64 {
    GLOBAL_CACHE_VERSION.load(Ordering::SeqCst)
}

/// A single cached frame entry: the raw RGBA pixel bytes for one frame
/// at one specific cache version.
#[derive(Clone)]
pub struct CacheEntry {
    pub version: u64,
    pub width: u32,
    pub height: u32,
    /// Raw RGBA8 bytes. Length = width * height * 4.
    pub pixels: Arc<Vec<u8>>,
}

/// The frame cache. Key is `(frame_index, cache_version)`.
/// Old versions are retained until an explicit `collect_garbage` call,
/// mirroring MVCC's multi-version concurrency pattern.
pub struct FrameCache {
    entries: HashMap<(u32, u64), CacheEntry>,
    /// Maximum number of entries before GC triggers automatically.
    max_entries: usize,
}

impl FrameCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    /// Try to retrieve a cached frame for the current global version.
    /// Returns `None` on a cache miss (stale or never rendered).
    pub fn get(&self, frame: u32) -> Option<&CacheEntry> {
        let ver = current_version();
        self.entries.get(&(frame, ver))
    }

    /// Store a rendered frame for the current global version.
    pub fn insert(&mut self, frame: u32, width: u32, height: u32, pixels: Vec<u8>) {
        let ver = current_version();
        self.entries.insert(
            (frame, ver),
            CacheEntry {
                version: ver,
                width,
                height,
                pixels: Arc::new(pixels),
            },
        );
        // Auto-GC when we exceed the budget
        if self.entries.len() > self.max_entries {
            self.collect_garbage();
        }
    }

    /// Discard all cache entries whose version is older than the current global version.
    /// Safe to call at any time — in-flight readers hold an `Arc` to pixel data.
    pub fn collect_garbage(&mut self) {
        let current = current_version();
        self.entries.retain(|(_frame, ver), _| *ver == current);
    }

    /// Discard the entire cache (e.g. on resolution change).
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
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

    /// Returns true if this frame is cached for the current version.
    pub fn is_cached(&self, frame: u32) -> bool {
        self.get(frame).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests must run single-threaded because they share the global AtomicU64.
    // We compensate by snapshotting the version at test start so relative
    // bump counts are deterministic regardless of insertion order.

    #[test]
    fn test_cache_miss_on_new_version() {
        let mut cache = FrameCache::new(256);
        let pixels = vec![0u8; 4];
        const FRAME: u32 = 2_000_001; // unique frame id to avoid cross-test interference

        // Record the version at the time of insertion.
        let insert_ver = current_version();
        cache.insert(FRAME, 1, 1, pixels.clone());

        // Verify the entry exists at the version we used.
        assert!(
            cache.entries.contains_key(&(FRAME, insert_ver)),
            "entry should exist at the version it was inserted",
        );

        // After bumping, current_version() != insert_ver, so get() returns None.
        bump_version();
        assert_ne!(current_version(), insert_ver, "version should have changed after bump");
        assert!(cache.get(FRAME).is_none(), "get() should miss when current version != insert version");
    }

    #[test]
    fn test_gc_removes_old_versions() {
        let mut cache = FrameCache::new(1024);
        let pixels = vec![0u8; 4];
        const FRAME: u32 = 1_000_002; // unique frame id to avoid cross-test contamination

        // Insert at current version
        cache.insert(FRAME, 1, 1, pixels.clone());
        assert_eq!(cache.current_version_len(), cache.entries.iter()
            .filter(|((f, _), _)| *f == FRAME).count());

        // Bump so the previous entry is now "old"
        bump_version();
        // Insert a second entry at the new version
        cache.insert(FRAME, 1, 1, pixels.clone());
        // We have two versioned entries for this frame
        let count_before = cache.entries.iter().filter(|((f, _), _)| *f == FRAME).count();
        assert_eq!(count_before, 2, "before GC we should have two versioned entries for the frame");

        cache.collect_garbage();

        // After GC: only the current-version entry survives
        let count_after = cache.entries.iter().filter(|((f, _), _)| *f == FRAME).count();
        assert_eq!(count_after, 1, "GC should leave only the current version entry");
        assert!(cache.is_cached(FRAME));
    }
}
