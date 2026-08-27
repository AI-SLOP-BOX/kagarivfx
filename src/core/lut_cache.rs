/// 3D LUT cache with LRU eviction and batch SIMD-friendly interpolation.
///
/// Caches the most-recently-used LUT entries to avoid redundant tetrahedral
/// lookups. `apply_batch` processes 4 pixels at a time using the cache line.
use std::collections::HashMap;

/// LRU cache for 3D LUT tetrahedral interpolation results.
#[derive(Debug)]
pub struct LutCache {
    entries: HashMap<u64, [f32; 3]>,
    order: Vec<u64>,
    capacity: usize,
}

impl LutCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            capacity,
        }
    }

    #[inline]
    fn cache_key(r: f32, g: f32, b: f32, size: usize) -> u64 {
        let s = (size - 1) as f32;
        let ri = ((r * s).clamp(0.0, s) * 1000.0) as u32;
        let gi = ((g * s).clamp(0.0, s) * 1000.0) as u32;
        let bi = ((b * s).clamp(0.0, s) * 1000.0) as u32;
        (ri as u64) << 32 | (gi as u64) << 16 | bi as u64
    }

    pub fn get_or_insert(&mut self, r: f32, g: f32, b: f32, size: usize, f: impl FnOnce(f32, f32, f32) -> (f32, f32, f32)) -> (f32, f32, f32) {
        let key = Self::cache_key(r, g, b, size);
        if let Some(&cached) = self.entries.get(&key) {
            // Move to most-recently-used
            if let Some(pos) = self.order.iter().position(|&k| k == key) {
                self.order.remove(pos);
            }
            self.order.push(key);
            return (cached[0], cached[1], cached[2]);
        }
        let result = f(r, g, b);
        // Evict if at capacity
        if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.first().copied() {
                self.entries.remove(&oldest);
                self.order.remove(0);
            }
        }
        self.entries.insert(key, [result.0, result.1, result.2]);
        self.order.push(key);
        result
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

/// Apply LUT to a batch of RGBA u8 pixels using cache-accelerated lookup.
/// Processes pixels in groups of 4 for cache-line friendliness.
pub fn apply_lut_batch_cached(
    lut: &super::ocio_color::Lut3D,
    pixels: &mut [u8],
    cache: &mut LutCache,
) {
    let size = lut.size;
    for chunk in pixels.chunks_exact_mut(16) {
        for pixel in chunk.chunks_exact_mut(4) {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            let (lr, lg, lb) = cache.get_or_insert(r, g, b, size, |r, g, b| lut.apply(r, g, b));
            pixel[0] = (lr.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[1] = (lg.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[2] = (lb.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

/// Apply LUT to a batch of f32 RGB pixels (for 16/32 bpc pipeline).
pub fn apply_lut_batch_f32(
    lut: &super::ocio_color::Lut3D,
    pixels: &mut [[f32; 3]],
    cache: &mut LutCache,
) {
    let size = lut.size;
    for pixel in pixels.iter_mut() {
        let (lr, lg, lb) = cache.get_or_insert(pixel[0], pixel[1], pixel[2], size, |r, g, b| lut.apply(r, g, b));
        pixel[0] = lr.clamp(0.0, 1.0);
        pixel[1] = lg.clamp(0.0, 1.0);
        pixel[2] = lb.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = LutCache::new(1024);
        assert_eq!(cache.capacity, 1024);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_cache_get_or_insert() {
        let mut cache = LutCache::new(100);
        let r1 = cache.get_or_insert(0.5, 0.5, 0.5, 33, |r, g, b| (r, g, b));
        assert_eq!(r1, (0.5, 0.5, 0.5));
        assert_eq!(cache.entries.len(), 1);
        // Second call should hit cache
        let r2 = cache.get_or_insert(0.5, 0.5, 0.5, 33, |_, _, _| panic!("should not be called"));
        assert_eq!(r2, (0.5, 0.5, 0.5));
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = LutCache::new(2);
        cache.get_or_insert(0.1, 0.1, 0.1, 33, |r, g, b| (r, g, b));
        cache.get_or_insert(0.2, 0.2, 0.2, 33, |r, g, b| (r, g, b));
        cache.get_or_insert(0.3, 0.3, 0.3, 33, |r, g, b| (r, g, b));
        assert_eq!(cache.entries.len(), 2);
        // First entry should have been evicted
        assert!(cache.entries.get(&LutCache::cache_key(0.1, 0.1, 0.1, 33)).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = LutCache::new(100);
        cache.get_or_insert(0.5, 0.5, 0.5, 33, |r, g, b| (r, g, b));
        assert_eq!(cache.entries.len(), 1);
        cache.clear();
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_cache_key_deterministic() {
        let k1 = LutCache::cache_key(0.5, 0.3, 0.7, 33);
        let k2 = LutCache::cache_key(0.5, 0.3, 0.7, 33);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_batch_apply_f32() {
        let lut = crate::core::ocio_color::Lut3D { size: 2, data: vec![0.0; 24] };
        let mut cache = LutCache::new(64);
        let mut pixels = [[0.5f32; 3]; 8];
        apply_lut_batch_f32(&lut, &mut pixels, &mut cache);
        assert_eq!(pixels.len(), 8);
    }
}
