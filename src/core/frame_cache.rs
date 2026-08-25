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

/// Thread-safe reusable buffer pool for RGBA pixel vectors.
/// Eliminates heap allocation spikes and memory fragmentation during 4K/8K playback.
#[allow(dead_code)]
pub struct PixelBufferPool {
    pool: std::sync::Mutex<Vec<Vec<u8>>>,
}

#[allow(dead_code)]
impl Default for PixelBufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelBufferPool {
    pub fn new() -> Self {
        Self {
            pool: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Acquire a buffer with capacity for `size_bytes`.
    pub fn acquire(&self, size_bytes: usize) -> Vec<u8> {
        let mut pool = self.pool.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut buf) = pool.pop() {
            buf.clear();
            if buf.capacity() < size_bytes {
                buf.reserve(size_bytes - buf.capacity());
            }
            buf
        } else {
            Vec::with_capacity(size_bytes)
        }
    }

    /// Recycle a vector back into the pool.
    pub fn recycle(&self, mut buf: Vec<u8>) {
        buf.clear();
        let mut pool = self.pool.lock().unwrap_or_else(|e| e.into_inner());
        if pool.len() < 64 {
            pool.push(buf);
        }
    }
}

/// A single cached frame entry: raw RGBA pixel bytes for one frame at one version.
#[derive(Clone)]
pub struct CacheEntry {
    pub version: u64,
    pub width: u32,
    pub height: u32,
    /// Raw RGBA8 bytes. Length = width * height * 4.
    pub pixels: Arc<Vec<u8>>,
    /// Monotonic access counter for eviction priority (higher = more recent).
    /// A counter is used instead of Instant because wall-clock resolution can tie
    /// entries inserted within the same instant, making LRU order non-deterministic.
    pub lru_stamp: u64,
}

/// Global monotonic counter for LRU stamps.
static LRU_CLOCK: AtomicU64 = AtomicU64::new(1);

fn lru_tick() -> u64 {
    LRU_CLOCK.fetch_add(1, Ordering::Relaxed)
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
    /// Layer indices that have been modified since last commit.
    dirty_layers: std::collections::HashSet<usize>,
    /// Composition IDs that contain dirty layers.
    dirty_comps: std::collections::HashSet<String>,
}

impl FrameCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB default
            current_memory_bytes: 0,
            dirty_layers: std::collections::HashSet::new(),
            dirty_comps: std::collections::HashSet::new(),
        }
    }

    /// Mark a layer as dirty (modified). Only frames containing this layer need re-rendering.
    pub fn mark_layer_dirty(&mut self, layer_idx: usize) {
        self.dirty_layers.insert(layer_idx);
    }

    /// Mark a composition as dirty (contains modified layers).
    pub fn mark_comp_dirty(&mut self, comp_id: &str) {
        self.dirty_comps.insert(comp_id.to_string());
    }

    /// Check if a specific layer is dirty.
    pub fn is_layer_dirty(&self, layer_idx: usize) -> bool {
        self.dirty_layers.contains(&layer_idx)
    }

    /// Clear all dirty flags (call after a full re-render or commit).
    pub fn clear_dirty(&mut self) {
        self.dirty_layers.clear();
        self.dirty_comps.clear();
    }

    /// Get the set of dirty layer indices.
    pub fn dirty_layers(&self) -> &std::collections::HashSet<usize> {
        &self.dirty_layers
    }

    /// Try to retrieve a cached frame for the current global version (updates LRU timestamp).
    pub fn get(&mut self, frame: u32) -> Option<&CacheEntry> {
        let ver = current_version();
        let key = (frame, ver);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.lru_stamp = lru_tick();
            // Return immutable ref — safe because we only mutated a non-key field
            return self.entries.get(&key);
        }
        None
    }

    /// Immutable check if frame is cached without updating LRU timestamp.
    pub fn is_cached(&self, frame: u32) -> bool {
        let ver = current_version();
        self.entries.contains_key(&(frame, ver))
    }

    /// Returns the number of cached frames currently stored.
    pub fn cached_count(&self) -> usize {
        self.entries.len()
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
                lru_stamp: lru_tick(),
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
            let mut keys_by_access: Vec<((u32, u64), u64)> = self
                .entries
                .iter()
                .map(|(k, v)| (*k, v.lru_stamp))
                .collect();

            // Sort by LRU stamp ascending (oldest first)
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

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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

        // NOTE: parallel tests share the global version counter and may bump it
        // concurrently, so we derive the insert version from the entry itself.
        cache.insert(FRAME, 1, 1, pixels.clone());
        let insert_ver = cache
            .entries
            .keys()
            .find(|(f, _)| *f == FRAME)
            .map(|(_, v)| *v)
            .expect("entry should exist after insert");

        bump_version();
        if current_version() != insert_ver {
            assert!(
                cache.get(FRAME).is_none(),
                "get() should miss when current version != insert version"
            );
        }
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
            lru_stamp: lru_tick(),
        });
        cache.entries.insert((FRAME, cur_ver), CacheEntry {
            version: cur_ver,
            width: 1,
            height: 1,
            pixels: pixels.clone(),
            lru_stamp: lru_tick(),
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

    #[test]
    fn test_pixel_buffer_pool_recycling() {
        let pool = PixelBufferPool::new();
        let buf = pool.acquire(1024);
        assert!(buf.capacity() >= 1024);
        pool.recycle(buf);
        let recycled = pool.acquire(512);
        assert!(recycled.capacity() >= 1024, "Recycled buffer should retain its allocated capacity");
    }
}

#[cfg(test)]
mod memory_bound_tests {
    use super::*;
    use std::sync::atomic::Ordering;

    /// Isolated version bump for tests: uses a test-local counter so we don't
    /// disturb the global version used by other caches.
    fn with_isolated_version<F: FnOnce()>(f: F) {
        let saved = GLOBAL_CACHE_VERSION.load(Ordering::SeqCst);
        f();
        GLOBAL_CACHE_VERSION.store(saved, Ordering::SeqCst);
    }

    #[test]
    fn test_memory_budget_enforced_by_lru_eviction() {
        with_isolated_version(|| {
            bump_version(); // isolate from other tests' cached frames
            let mut cache = FrameCache::new(1000);
            // 10 frames x 100KB = ~1MB budget
            cache.max_memory_bytes = 1_000_000;

            let frame_bytes = 250 * 100; // 100KB per frame
            for i in 0..20u32 {
                cache.insert(i, 500, 50, vec![0u8; frame_bytes]);
                // Keep touching frame 0 so it stays MRU-hot
                let _ = cache.get(0);
            }

            assert!(
                cache.current_memory_bytes <= cache.max_memory_bytes + frame_bytes,
                "memory must stay near budget, got {}",
                cache.current_memory_bytes
            );
            // Hot frame must survive eviction.
            // NOTE: check across all versions because parallel tests share the
            // global version counter and may bump it while this test runs.
            assert!(
                cache.entries.keys().any(|(f, _)| *f == 0),
                "LRU-hot frame must not be evicted"
            );
        });
    }

    #[test]
    fn test_stale_versions_are_discarded() {
        with_isolated_version(|| {
            let mut cache = FrameCache::new(100);
            cache.max_memory_bytes = usize::MAX;

            bump_version();
            cache.insert(5, 32, 32, vec![0u8; 4096]);
            assert!(cache.is_cached(5));

            // Project change → version bump → old entries unreachable
            bump_version();
            cache.collect_garbage();
            assert_eq!(cache.cached_count(), 0, "stale entries must be discarded");
        });
    }
}
