/// Image layer loader and cache for JPEG/PNG images.
///
/// Loads image files from disk, decodes them to RGBA pixel buffers,
/// and caches them for use by the software renderer.
use std::collections::HashMap;
use std::sync::OnceLock;

/// Cached decoded image data.
#[derive(Debug, Clone)]
pub struct CachedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Thread-safe image cache.
pub struct ImageCache {
    cache: HashMap<String, CachedImage>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Load an image from a file path. Returns cached result on subsequent calls.
    pub fn load_image(&mut self, path: &str) -> Option<&CachedImage> {
        if self.cache.contains_key(path) {
            return self.cache.get(path);
        }

        let img = Self::decode_image_file(path)?;
        self.cache.insert(path.to_string(), img);
        self.cache.get(path)
    }

    /// Decode an image file to RGBA pixels.
    fn decode_image_file(path: &str) -> Option<CachedImage> {
        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(CachedImage {
            width,
            height,
            pixels: rgba.into_raw(),
        })
    }

    /// Get a cached image without loading.
    pub fn get(&self, path: &str) -> Option<&CachedImage> {
        self.cache.get(path)
    }
}

/// Global image cache instance.
static GLOBAL_IMAGE_CACHE: OnceLock<std::sync::Mutex<ImageCache>> = OnceLock::new();

/// Access the global image cache.
pub fn with_image_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut ImageCache) -> R,
{
    let cache = GLOBAL_IMAGE_CACHE.get_or_init(|| {
        std::sync::Mutex::new(ImageCache::new())
    });
    let mut lock = cache.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut lock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_cache_creation() {
        let c = ImageCache::new();
        assert!(c.cache.is_empty());
    }

    #[test]
    fn test_load_nonexistent_image() {
        let mut c = ImageCache::new();
        assert!(c.load_image("/nonexistent/path/image.png").is_none());
    }
}
