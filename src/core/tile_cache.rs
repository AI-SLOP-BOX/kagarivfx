use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static TILE_VERSION: AtomicU64 = AtomicU64::new(0);

pub fn bump_tile_version() {
    TILE_VERSION.fetch_add(1, Ordering::Relaxed);
}

pub fn current_tile_version() -> u64 {
    TILE_VERSION.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub tx: u32,
    pub ty: u32,
}

#[derive(Debug, Clone)]
pub struct TileEntry {
    pub pixels: Vec<u8>,
    pub version: u64,
    pub lru_stamp: u64,
}

pub struct TileCache {
    tiles: HashMap<(u32, u32, TileCoord), TileEntry>,
    tile_size: u32,
    lru_clock: u64,
    current_memory: usize,
    max_memory: usize,
    version_override: Option<u64>,
}

impl Default for TileCache {
    fn default() -> Self {
        Self {
            tiles: HashMap::new(),
            tile_size: 256,
            lru_clock: 0,
            current_memory: 0,
            max_memory: 256 * 1024 * 1024, // 256 MB
            version_override: None,
        }
    }
}

impl TileCache {
    pub fn new(tile_size: u32, max_memory_bytes: usize) -> Self {
        Self {
            tile_size: tile_size.max(1),
            max_memory: max_memory_bytes,
            ..Default::default()
        }
    }

    pub fn with_version(tile_size: u32, max_memory_bytes: usize, version: u64) -> Self {
        Self {
            tile_size: tile_size.max(1),
            max_memory: max_memory_bytes,
            version_override: Some(version),
            ..Default::default()
        }
    }

    fn effective_version(&self) -> u64 {
        self.version_override.unwrap_or_else(current_tile_version)
    }

    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    pub fn tiles_for_frame(&self, _frame: u32, comp_w: u32, comp_h: u32) -> Vec<TileCoord> {
        let cols = comp_w.div_ceil(self.tile_size);
        let rows = comp_h.div_ceil(self.tile_size);
        let Some(count) = (cols as usize).checked_mul(rows as usize) else {
            return Vec::new();
        };
        if count > isize::MAX as usize {
            return Vec::new();
        }
        let mut coords = Vec::with_capacity(count);
        for ty in 0..rows {
            for tx in 0..cols {
                coords.push(TileCoord { tx, ty });
            }
        }
        coords
    }

    pub fn tile_rect(&self, coord: TileCoord, comp_w: u32, comp_h: u32) -> (u32, u32, u32, u32) {
        let x = coord.tx.saturating_mul(self.tile_size);
        let y = coord.ty.saturating_mul(self.tile_size);
        let w = self.tile_size.min(comp_w.saturating_sub(x));
        let h = self.tile_size.min(comp_h.saturating_sub(y));
        (x, y, w, h)
    }

    pub fn get(&mut self, frame: u32, coord: TileCoord) -> Option<&[u8]> {
        let version = self.effective_version();
        let key = (frame, 0, coord);
        if self
            .tiles
            .get(&key)
            .is_some_and(|entry| entry.version != version)
        {
            if let Some(entry) = self.tiles.remove(&key) {
                self.current_memory = self.current_memory.saturating_sub(entry.pixels.len());
            }
            return None;
        }
        let entry = self.tiles.get_mut(&key)?;
        self.lru_clock += 1;
        entry.lru_stamp = self.lru_clock;
        Some(&entry.pixels)
    }

    pub fn insert(&mut self, frame: u32, coord: TileCoord, pixels: Vec<u8>) {
        let version = self.effective_version();
        let tile_bytes = pixels.len();
        if tile_bytes > self.max_memory {
            return;
        }
        self.lru_clock += 1;

        let key = (frame, 0, coord);
        if let Some(previous) = self.tiles.remove(&key) {
            self.current_memory = self.current_memory.saturating_sub(previous.pixels.len());
        }

        // Evict if over budget
        while self
            .current_memory
            .checked_add(tile_bytes)
            .is_none_or(|total| total > self.max_memory)
            && !self.tiles.is_empty()
        {
            if let Some((worst_key, worst_entry)) = self
                .tiles
                .iter()
                .min_by_key(|(_, e)| e.lru_stamp)
                .map(|(k, e)| (*k, e.clone()))
            {
                self.current_memory -= worst_entry.pixels.len();
                self.tiles.remove(&worst_key);
            } else {
                break;
            }
        }

        let Some(new_memory) = self.current_memory.checked_add(tile_bytes) else {
            return;
        };
        self.current_memory = new_memory;
        self.tiles.insert(
            key,
            TileEntry {
                pixels,
                version,
                lru_stamp: self.lru_clock,
            },
        );
    }

    pub fn invalidate_all(&mut self) {
        self.tiles.clear();
        self.current_memory = 0;
        bump_tile_version();
    }

    pub fn invalidate_frame(&mut self, frame: u32) {
        let version = current_tile_version();
        let keys: Vec<_> = self
            .tiles
            .keys()
            .filter(|(f, _, _)| *f == frame)
            .copied()
            .collect();
        for key in keys {
            if let Some(entry) = self.tiles.remove(&key) {
                self.current_memory -= entry.pixels.len();
            }
        }
        let _ = version;
    }

    pub fn memory_usage(&self) -> usize {
        self.current_memory
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiles_for_frame() {
        let cache = TileCache::new(256, 1024 * 1024);
        let tiles = cache.tiles_for_frame(0, 1920, 1080);
        // 1920/256 = 8 cols, 1080/256 = 5 rows (rounded up)
        assert_eq!(tiles.len(), 8 * 5);
        assert_eq!(tiles[0], TileCoord { tx: 0, ty: 0 });
    }

    #[test]
    fn test_tile_rect() {
        let cache = TileCache::new(256, 1024 * 1024);
        let (x, y, w, h) = cache.tile_rect(TileCoord { tx: 7, ty: 4 }, 1920, 1080);
        assert_eq!(x, 7 * 256);
        assert_eq!(y, 4 * 256);
        assert!(w <= 256);
        assert!(h <= 256);
    }

    #[test]
    fn test_insert_and_get() {
        let mut cache = TileCache::new(256, 1024 * 1024);
        let pixels = vec![0u8; 256 * 256 * 4];
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, pixels.clone());
        let got = cache.get(0, TileCoord { tx: 0, ty: 0 });
        assert!(got.is_some());
        assert_eq!(got.unwrap().len(), pixels.len());
    }

    #[test]
    fn test_invalidate_all() {
        let mut cache = TileCache::new(256, 1024 * 1024);
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, vec![0u8; 100]);
        assert_eq!(cache.tile_count(), 1);
        cache.invalidate_all();
        assert_eq!(cache.tile_count(), 0);
    }

    #[test]
    fn test_eviction() {
        let mut cache = TileCache::new(256, 1024); // Very small cache
        for i in 0..10 {
            cache.insert(i, TileCoord { tx: 0, ty: 0 }, vec![i as u8; 256]);
        }
        // Should have evicted some tiles
        assert!(cache.tile_count() < 10);
        assert!(cache.memory_usage() <= 1024);
    }

    #[test]
    fn get_refreshes_lru_before_eviction() {
        let mut cache = TileCache::new(16, 32);
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, vec![0; 16]);
        cache.insert(1, TileCoord { tx: 0, ty: 0 }, vec![1; 16]);
        assert!(cache.get(0, TileCoord { tx: 0, ty: 0 }).is_some());
        cache.insert(2, TileCoord { tx: 0, ty: 0 }, vec![2; 16]);
        assert!(cache.get(0, TileCoord { tx: 0, ty: 0 }).is_some());
        assert!(cache.get(1, TileCoord { tx: 0, ty: 0 }).is_none());
    }

    #[test]
    fn stale_tile_is_removed_when_read_after_invalidation() {
        let mut cache = TileCache::new(16, 32);
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, vec![0; 16]);
        bump_tile_version();
        assert!(cache.get(0, TileCoord { tx: 0, ty: 0 }).is_none());
        assert_eq!(cache.tile_count(), 0);
        assert_eq!(cache.memory_usage(), 0);
    }

    #[test]
    fn reinserting_a_tile_replaces_its_memory_accounting() {
        let mut cache = TileCache::new(16, 32);
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, vec![1; 16]);
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, vec![2; 8]);
        assert_eq!(cache.tile_count(), 1);
        assert_eq!(cache.memory_usage(), 8);
        assert_eq!(cache.get(0, TileCoord { tx: 0, ty: 0 }), Some(&[2; 8][..]));
    }

    #[test]
    fn oversized_tiles_are_rejected_without_exceeding_budget() {
        let mut cache = TileCache::new(16, 8);
        cache.insert(0, TileCoord { tx: 0, ty: 0 }, vec![0; 9]);
        assert_eq!(cache.tile_count(), 0);
        assert_eq!(cache.memory_usage(), 0);
    }

    #[test]
    fn zero_tile_size_is_clamped_to_a_safe_value() {
        let cache = TileCache::new(0, 64);
        assert_eq!(cache.tile_size(), 1);
        assert_eq!(cache.tiles_for_frame(0, 2, 1).len(), 2);
    }

    #[test]
    fn huge_tile_grid_is_rejected_without_capacity_overflow() {
        let cache = TileCache::new(1, 64);
        assert!(cache.tiles_for_frame(0, u32::MAX, u32::MAX).is_empty());
    }

    #[test]
    fn overflowing_tile_coordinates_saturate_outside_frame() {
        let cache = TileCache::new(u32::MAX, 64);
        assert_eq!(
            cache.tile_rect(TileCoord { tx: 2, ty: 2 }, 100, 100),
            (u32::MAX, u32::MAX, 0, 0)
        );
    }
}
