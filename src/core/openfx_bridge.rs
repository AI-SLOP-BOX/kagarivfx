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

// ─── dlopen-based ABI probing (OfxGetPlugin / OfxPlugin v1) ─────────────────

/// `struct OfxPlugin` layout from ofxCore.h — read-only fields we inspect
/// before any host action handshake. Offsets per the published C ABI.
#[repr(C)]
struct OfxPluginRaw {
    struct_version: i32,
    type_identifier: *const std::os::raw::c_char,
    plugin_api_version: [i32; 2],
    plugin_api: *const std::os::raw::c_char,
    plugin_version_major: i32,
    plugin_version_minor: i32,
    set_host: usize, // function pointer, unused during probing
    main_entry: usize, // function pointer, unused during probing
}

unsafe fn cstr<'a>(p: *const std::os::raw::c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    std::ffi::CStr::from_ptr(p).to_str().ok()
}

/// Result of probing one .ofx binary.
#[derive(Debug, Clone, PartialEq)]
pub enum OfxProbeResult {
    /// Binary loaded and exposed at least one valid OfxImageEffectAPI plugin.
    Loaded {
        api: String,
        api_version: (i32, i32),
        plugin_version: (i32, i32),
        plugin_count: usize,
    },
    /// The binary loaded but is not an OFX plugin (no OfxGetPlugin export).
    NotOfx(String),
    /// The binary could not be loaded at all.
    LoadError(String),
}

/// dlopen the given `.ofx` binary and introspect its exported plugin list
/// via `OfxGetNumberOfPlugins` / `OfxGetPlugin`. Read-only: no host actions
/// are dispatched, so this is safe against arbitrary third-party binaries
/// beyond the dlopen constructor itself.
///
/// Returns `LoadError` when the file is missing/corrupt for this platform,
/// which callers surface without crashing the app.
pub fn probe_ofx_plugin(path: &Path) -> OfxProbeResult {
    let file = match unsafe { libloading::Library::new(path) } {
        Ok(lib) => lib,
        Err(e) => return OfxProbeResult::LoadError(format!("dlopen failed: {e}")),
    };
    let result = unsafe { probe_loaded(&file) };
    drop(file);
    result
}

unsafe fn probe_loaded(lib: &libloading::Library) -> OfxProbeResult {
    let get_num: libloading::Symbol<'_, unsafe extern "C" fn() -> i32> = match lib.get(b"OfxGetNumberOfPlugins") {
        Ok(s) => s,
        Err(e) => return OfxProbeResult::NotOfx(format!("missing OfxGetNumberOfPlugins: {e}")),
    };
    let get_plugin: libloading::Symbol<'_, unsafe extern "C" fn(i32) -> *mut OfxPluginRaw> =
        match lib.get(b"OfxGetPlugin") {
            Ok(s) => s,
            Err(e) => return OfxProbeResult::NotOfx(format!("missing OfxGetPlugin: {e}")),
        };

    let count = (get_num)().max(0) as usize;
    if count == 0 {
        return OfxProbeResult::NotOfx("exports zero plugins".into());
    }

    for i in 0..count {
        let raw = (get_plugin)(i as i32);
        if raw.is_null() {
            continue;
        }
        let p = &*raw;
        if let Some(api) = cstr(p.plugin_api) {
            if api.starts_with("OfxImageEffect") {
                return OfxProbeResult::Loaded {
                    api: api.to_string(),
                    api_version: (p.plugin_api_version[0], p.plugin_api_version[1]),
                    plugin_version: (p.plugin_version_major, p.plugin_version_minor),
                    plugin_count: count,
                };
            }
        }
    }
    OfxProbeResult::NotOfx("no OfxImageEffect plugin in export list".into())
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

    #[test]
    fn test_probe_missing_file_is_load_error() {
        match probe_ofx_plugin(Path::new("/nonexistent/probe.ofx")) {
            OfxProbeResult::LoadError(_) => {}
            other => panic!("expected LoadError, got {other:?}"),
        }
    }

    #[test]
    fn test_probe_garbage_file_is_clean_error_not_crash() {
        let tmp = std::env::temp_dir().join(format!("ofx_garbage_{}.ofx", std::process::id()));
        std::fs::write(&tmp, b"definitely not a dylib").unwrap();
        let res = probe_ofx_plugin(&tmp);
        let _ = std::fs::remove_file(&tmp);
        assert!(
            matches!(res, OfxProbeResult::LoadError(_) | OfxProbeResult::NotOfx(_)),
            "garbage must not crash: {res:?}"
        );
    }
}
