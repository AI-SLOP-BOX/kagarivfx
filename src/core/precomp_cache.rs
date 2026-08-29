/// Precomp render cache: avoids re-rendering the same sub-composition
/// at the same frame when it's referenced by multiple layers.
///
/// Uses a thread-local LRU cache keyed by composition content, frame, and resolution.
use std::collections::HashMap;

type CacheKey = (String, u64, u32, u32, u32);

struct CacheEntry {
    pixels: Vec<u8>,
}

pub struct PrecompCache {
    entries: HashMap<CacheKey, CacheEntry>,
    order: Vec<CacheKey>,
    capacity: usize,
}

impl PrecompCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn get(
        &mut self,
        comp_id: &str,
        content_revision: u64,
        frame: u32,
        w: u32,
        h: u32,
    ) -> Option<Vec<u8>> {
        let key = (comp_id.to_string(), content_revision, frame, w, h);
        if let Some(entry) = self.entries.get(&key) {
            self.order.retain(|k| *k != key);
            self.order.push(key);
            return Some(entry.pixels.clone());
        }
        None
    }

    pub fn insert(
        &mut self,
        comp_id: &str,
        content_revision: u64,
        frame: u32,
        w: u32,
        h: u32,
        pixels: Vec<u8>,
    ) {
        let key = (comp_id.to_string(), content_revision, frame, w, h);
        if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.first().cloned() {
                self.entries.remove(&oldest);
                self.order.retain(|k| *k != oldest);
            }
        }
        self.entries.insert(key.clone(), CacheEntry { pixels });
        self.order.push(key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for PrecompCache {
    fn default() -> Self {
        Self::new(64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precomp_cache_basic() {
        let mut cache = PrecompCache::new(4);
        assert!(cache.get("comp1", 1, 0, 1920, 1080).is_none());
        let data = vec![0u8; 1920 * 1080 * 4];
        cache.insert("comp1", 1, 0, 1920, 1080, data.clone());
        let cached = cache.get("comp1", 1, 0, 1920, 1080);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), data);
    }

    #[test]
    fn test_precomp_cache_eviction() {
        let mut cache = PrecompCache::new(2);
        cache.insert("a", 1, 0, 100, 100, vec![0u8; 400]);
        cache.insert("b", 1, 0, 100, 100, vec![0u8; 400]);
        cache.insert("c", 1, 0, 100, 100, vec![0u8; 400]);
        assert_eq!(cache.len(), 2);
        assert!(cache.get("a", 1, 0, 100, 100).is_none());
    }

    #[test]
    fn test_precomp_cache_different_frames() {
        let mut cache = PrecompCache::new(4);
        cache.insert("comp1", 1, 0, 100, 100, vec![1u8; 400]);
        cache.insert("comp1", 1, 1, 100, 100, vec![2u8; 400]);
        assert_eq!(cache.get("comp1", 1, 0, 100, 100).unwrap()[0], 1);
        assert_eq!(cache.get("comp1", 1, 1, 100, 100).unwrap()[0], 2);
    }

    #[test]
    fn test_content_revision_prevents_stale_reuse() {
        let mut cache = PrecompCache::new(4);
        cache.insert("comp1", 10, 0, 100, 100, vec![1u8; 400]);

        assert!(cache.get("comp1", 11, 0, 100, 100).is_none());
        assert_eq!(cache.get("comp1", 10, 0, 100, 100).unwrap()[0], 1);
    }
}
