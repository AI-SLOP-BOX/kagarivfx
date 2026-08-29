/// Image layer loader and cache for JPEG/PNG images.
///
/// Loads image files from disk, decodes them to RGBA pixel buffers,
/// and caches them for use by the software renderer.
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::SystemTime;

/// Cached decoded image data.
#[derive(Debug, Clone)]
pub struct CachedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileRevision {
    len: u64,
    modified: Option<SystemTime>,
}

struct CachedFile {
    revision: FileRevision,
    image: CachedImage,
}

/// Thread-safe image cache.
pub struct ImageCache {
    cache: HashMap<String, CachedFile>,
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
        let revision = Self::file_revision(path)?;
        if self
            .cache
            .get(path)
            .is_some_and(|cached| cached.revision == revision)
        {
            return self.cache.get(path).map(|cached| &cached.image);
        }

        let img = Self::decode_image_file(path)?;
        self.cache.insert(
            path.to_string(),
            CachedFile {
                revision,
                image: img,
            },
        );
        self.cache.get(path).map(|cached| &cached.image)
    }

    /// Maximum pixels per image (16384 x 16384): guards against decompression bombs
    /// and runaway allocations from corrupted or malicious image files.
    pub const MAX_IMAGE_PIXELS: u64 = 16384 * 16384;

    /// Decode an image file to RGBA pixels.
    fn decode_image_file(path: &str) -> Option<CachedImage> {
        // Check declared dimensions BEFORE decoding to avoid huge allocations
        let reader = image::ImageReader::open(path).ok()?;
        let (declared_w, declared_h) = reader.into_dimensions().ok()?;
        let total = declared_w as u64 * declared_h as u64;
        if total == 0 || total > Self::MAX_IMAGE_PIXELS {
            log::warn!(
                "[ImageCache] Rejecting image {} with dimensions {}x{} ({} px > limit)",
                path, declared_w, declared_h, total
            );
            return None;
        }

        let img = image::open(path).ok()?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(CachedImage {
            width,
            height,
            pixels: rgba.into_raw(),
        })
    }

    fn file_revision(path: &str) -> Option<FileRevision> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(FileRevision {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }

    /// Get a cached image without loading.
    pub fn get(&self, path: &str) -> Option<&CachedImage> {
        self.cache.get(path).map(|cached| &cached.image)
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

    #[test]
    fn test_reloads_image_when_file_changes() {
        let dir = std::env::temp_dir().join(format!(
            "aevfx_image_cache_revision_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("replace.png");

        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let mut cache = ImageCache::new();
        assert_eq!(
            cache.load_image(path.to_str().unwrap()).unwrap().pixels[0],
            255
        );

        image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 255, 0, 255]))
            .save(&path)
            .unwrap();
        let reloaded = cache.load_image(path.to_str().unwrap()).unwrap();
        assert_eq!((reloaded.width, reloaded.height), (2, 1));
        assert_eq!(&reloaded.pixels[..4], &[0, 255, 0, 255]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod robustness_tests {
    use super::*;

    #[test]
    fn test_missing_and_invalid_files_return_none() {
        let mut cache = ImageCache::new();
        // Missing file
        assert!(cache.load_image("/nonexistent/path/img.png").is_none());
        // Directory instead of file
        assert!(cache.load_image("/tmp").is_none());
        // Corrupted image payload with valid extension
        let bad = std::env::temp_dir().join("aevfx_bad_image.png");
        std::fs::write(&bad, b"not a real png at all").unwrap();
        assert!(cache.load_image(bad.to_str().unwrap()).is_none());
        let _ = std::fs::remove_file(&bad);
    }

}
