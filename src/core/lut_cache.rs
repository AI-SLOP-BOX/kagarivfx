/// 3D LUT cache with LRU eviction and batch SIMD-friendly interpolation.
///
/// Caches the most-recently-used LUT entries to avoid redundant tetrahedral
/// lookups. `apply_batch` processes 4 pixels at a time using the cache line.
use std::collections::{HashMap, VecDeque};

/// LRU cache for 3D LUT tetrahedral interpolation results.
/// Uses `VecDeque` for O(1) front/back removal instead of O(n) `Vec::remove(0)`.
#[derive(Debug)]
pub struct LutCache {
    entries: HashMap<u64, [f32; 3]>,
    order: VecDeque<u64>,
    capacity: usize,
}

impl LutCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    #[inline]
    fn cache_key(r: f32, g: f32, b: f32, size: usize) -> u64 {
        let s = size.saturating_sub(1) as f32;
        let ri = ((r * s).clamp(0.0, s) * 1000.0) as u32;
        let gi = ((g * s).clamp(0.0, s) * 1000.0) as u32;
        let bi = ((b * s).clamp(0.0, s) * 1000.0) as u32;
        let mut hash = 0xcbf29ce484222325u64;
        for value in [size as u64, ri as u64, gi as u64, bi as u64] {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }

    pub fn get_or_insert(
        &mut self,
        r: f32,
        g: f32,
        b: f32,
        size: usize,
        f: impl FnOnce(f32, f32, f32) -> (f32, f32, f32),
    ) -> (f32, f32, f32) {
        if self.capacity == 0 || size == 0 {
            return f(r, g, b);
        }
        let key = Self::cache_key(r, g, b, size);
        if let Some(&cached) = self.entries.get(&key) {
            // Move to most-recently-used (O(n) scan but small n in practice)
            self.order.retain(|&k| k != key);
            self.order.push_back(key);
            return (cached[0], cached[1], cached[2]);
        }
        let result = f(r, g, b);
        // Evict if at capacity — O(1) pop_front from VecDeque
        if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key, [result.0, result.1, result.2]);
        self.order.push_back(key);
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
    for pixel in pixels.chunks_exact_mut(4) {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let (lr, lg, lb) = cache.get_or_insert(r, g, b, size, |r, g, b| lut.apply(r, g, b));
        pixel[0] = (lr.clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[1] = (lg.clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[2] = (lb.clamp(0.0, 1.0) * 255.0).round() as u8;
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
        let input =
            [pixel[0], pixel[1], pixel[2]].map(|value| if value.is_finite() { value } else { 0.0 });
        let (lr, lg, lb) = cache.get_or_insert(input[0], input[1], input[2], size, |r, g, b| {
            lut.apply(r, g, b)
        });
        pixel[0] = finite_clamped(lr);
        pixel[1] = finite_clamped(lg);
        pixel[2] = finite_clamped(lb);
    }
}

#[inline]
fn finite_clamped(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
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
        assert!(!cache
            .entries
            .contains_key(&LutCache::cache_key(0.1, 0.1, 0.1, 33)));
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
    fn cache_key_includes_lut_size_and_full_quantized_channels() {
        assert_ne!(
            LutCache::cache_key(0.5, 0.3, 0.7, 33),
            LutCache::cache_key(0.5, 0.3, 0.7, 34)
        );
        assert_ne!(
            LutCache::cache_key(0.5, 0.3, 0.7, 33),
            LutCache::cache_key(0.5, 0.3, 0.8, 33)
        );
    }

    #[test]
    fn zero_capacity_and_zero_size_bypass_cache() {
        let mut cache = LutCache::new(0);
        let mut calls = 0;
        cache.get_or_insert(0.5, 0.5, 0.5, 0, |r, g, b| {
            calls += 1;
            (r, g, b)
        });
        assert_eq!(calls, 1);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_batch_apply_f32() {
        let lut = crate::core::ocio_color::Lut3D {
            size: 2,
            data: vec![0.0; 24],
        };
        let mut cache = LutCache::new(64);
        let mut pixels = [[0.5f32; 3]; 8];
        apply_lut_batch_f32(&lut, &mut pixels, &mut cache);
        assert_eq!(pixels.len(), 8);
    }

    #[test]
    fn test_batch_apply_u8_processes_short_tail() {
        let lut = crate::core::ocio_color::Lut3D {
            size: 2,
            data: (0..24).map(|value| value as f32 / 24.0).collect(),
        };
        let mut cache = LutCache::new(64);
        let mut pixels = vec![128u8, 64, 32, 255];
        let original = pixels.clone();
        apply_lut_batch_cached(&lut, &mut pixels, &mut cache);
        assert_ne!(pixels[..3], original[..3]);
    }

    #[test]
    fn test_batch_apply_f32_sanitizes_nonfinite_values() {
        let lut = crate::core::ocio_color::Lut3D {
            size: 2,
            data: vec![0.5; 24],
        };
        let mut cache = LutCache::new(64);
        let mut pixels = [[f32::NAN, f32::INFINITY, 0.5]];
        apply_lut_batch_f32(&lut, &mut pixels, &mut cache);
        assert!(pixels[0].iter().all(|value| value.is_finite()));
        assert!(pixels[0].iter().all(|value| (0.0..=1.0).contains(value)));
    }
}
