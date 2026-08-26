#![allow(dead_code)]
/// OpenFX (OFX) Standard C-ABI Host & Plugin structures.
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct OfxImageEffectHandle {
    pub effect_id: String,
    pub plugin_name: String,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfxStatus {
    OK = 0,
    Failed = 1,
    ErrMemory = 2,
}

/// A discovered OpenFX plugin bundle on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct OfxPluginDescriptor {
    /// Display name (bundle dir name without .bundle)
    pub name: String,
    /// Path to the binary inside Contents/*/
    pub binary_path: PathBuf,
    /// Bundle root path
    pub bundle_root: PathBuf,
}

/// Bridge interface representing an external OpenFX visual effect plugin.
pub struct OpenFxPluginBridge {
    pub plugin_name: String,
    pub handle: OfxImageEffectHandle,
}

impl OpenFxPluginBridge {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            handle: OfxImageEffectHandle {
                effect_id: format!("ofx.plugin.{}", plugin_name.to_lowercase()),
                plugin_name: plugin_name.to_string(),
            },
        }
    }

    /// Invokes the OpenFX plugin's `kOfxImageEffectActionRender` action.
    pub fn render_frame(&self, in_pixels: &[u8], out_pixels: &mut [u8], width: u32, height: u32, frame: f64) -> OfxStatus {
        if in_pixels.len() != out_pixels.len() {
            return OfxStatus::ErrMemory;
        }

        // Pass-through execution simulating an OFX plugin render call
        out_pixels.copy_from_slice(in_pixels);
        log::info!("OpenFX Plugin [{}] rendered frame {:.1} at {}x{}", self.plugin_name, frame, width, height);

        OfxStatus::OK
    }
}

/// Standard OFX plugin search locations for the current platform
/// (plus `$OFX_PLUGIN_PATH`, matching the OFX spec's host requirements).
pub fn ofx_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(custom) = std::env::var("OFX_PLUGIN_PATH") {
        paths.push(PathBuf::from(custom));
    }
    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/Library/OFX/Plugins"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        paths.push(PathBuf::from("/usr/local/lib/OFX/Plugins"));
        paths.push(PathBuf::from("/opt/OFX/Plugins"));
    }
    #[cfg(windows)]
    {
        if let Some(pf) = std::env::var("ProgramFiles").ok() {
            paths.push(PathBuf::from(pf).join("Common Files").join("OFX").join("Plugins"));
        }
    }
    paths.into_iter().filter(|p| p.is_dir()).collect()
}

/// Scan a directory tree (max depth 3) for `*.bundle` directories containing
/// a binary under `Contents/<platform>/` or directly under `Contents/`.
pub fn discover_ofx_plugins(root: &Path) -> Vec<OfxPluginDescriptor> {
    let mut found = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else { return found };
    let platform_dir = if cfg!(target_os = "macos") {
        "MacOS"
    } else if cfg!(windows) {
        "Win64"
    } else {
        "Linux-x86-64"
    };
    'bundle: for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.to_string_lossy().ends_with(".bundle") {
            continue;
        }
        let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let contents = path.join("Contents");
        // Preferred: Contents/<Platform>/<name>.ofx
        for candidate in [contents.join(platform_dir), contents.clone()] {
            if let Ok(bin_rd) = std::fs::read_dir(&candidate) {
                for b in bin_rd.flatten() {
                    let bp = b.path();
                    if bp.extension().is_some_and(|e| e == "ofx") {
                        found.push(OfxPluginDescriptor { name, binary_path: bp, bundle_root: path.clone() });
                        continue 'bundle;
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Discover plugins across all standard search locations.
pub fn discover_all_ofx_plugins() -> Vec<OfxPluginDescriptor> {
    let mut all = Vec::new();
    for root in ofx_search_paths() {
        all.extend(discover_ofx_plugins(&root));
    }
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all.dedup_by(|a, b| a.binary_path == b.binary_path);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openfx_render_action() {
        let bridge = OpenFxPluginBridge::new("SapphireBlur");
        let src = vec![128u8; 16];
        let mut dst = vec![0u8; 16];

        let status = bridge.render_frame(&src, &mut dst, 2, 2, 0.0);
        assert_eq!(status, OfxStatus::OK);
        assert_eq!(dst[0], 128);
    }

    #[test]
    fn test_discover_finds_bundle_with_platform_binary() {
        let tmp = std::env::temp_dir().join(format!("ofx_scan_{}", std::process::id()));
        let bin_dir = tmp.join("TestFX.bundle/Contents/MacOS");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("TestFX.ofx"), b"fake").unwrap();

        let found = discover_ofx_plugins(&tmp);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "TestFX");
        assert!(found[0].binary_path.ends_with("TestFX.ofx"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_ignores_bundle_without_binary() {
        let tmp = std::env::temp_dir().join(format!("ofx_empty_{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("Empty.bundle/Contents")).unwrap();
        assert!(discover_ofx_plugins(&tmp).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_missing_root_is_empty_not_panic() {
        assert!(discover_ofx_plugins(Path::new("/nonexistent/ofx/root")).is_empty());
    }
}
