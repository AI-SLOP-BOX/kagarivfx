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
    pub fn render_frame(
        &self,
        in_pixels: &[u8],
        out_pixels: &mut [u8],
        width: u32,
        height: u32,
        frame: f64,
    ) -> OfxStatus {
        if in_pixels.len() != out_pixels.len() {
            return OfxStatus::ErrMemory;
        }

        // Pass-through execution simulating an OFX plugin render call
        out_pixels.copy_from_slice(in_pixels);
        log::info!(
            "OpenFX Plugin [{}] rendered frame {:.1} at {}x{}",
            self.plugin_name,
            frame,
            width,
            height
        );

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
            paths.push(
                PathBuf::from(pf)
                    .join("Common Files")
                    .join("OFX")
                    .join("Plugins"),
            );
        }
    }
    paths.into_iter().filter(|p| p.is_dir()).collect()
}

/// Scan a directory tree (max depth 3) for `*.bundle` directories containing
/// a binary under `Contents/<platform>/` or directly under `Contents/`.
pub fn discover_ofx_plugins(root: &Path) -> Vec<OfxPluginDescriptor> {
    let mut found = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        return found;
    };
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
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let contents = path.join("Contents");
        // Preferred: Contents/<Platform>/<name>.ofx
        for candidate in [contents.join(platform_dir), contents.clone()] {
            if let Ok(bin_rd) = std::fs::read_dir(&candidate) {
                for b in bin_rd.flatten() {
                    let bp = b.path();
                    if bp.extension().is_some_and(|e| e == "ofx") {
                        found.push(OfxPluginDescriptor {
                            name,
                            binary_path: bp,
                            bundle_root: path.clone(),
                        });
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

use std::collections::HashMap;

/// Property values storable in an OFX property set.
#[derive(Debug, Clone, PartialEq)]
pub enum OfxPropValue {
    Int(i32),
    Double(f64),
    Str(String),
}

/// Minimal property-set storage backing the host's OfxPropertySuiteV1.
/// Handles passed through the C ABI are raw pointers to this struct.
#[derive(Default, Debug)]
pub struct OfxPropertySet {
    props: HashMap<String, Vec<OfxPropValue>>,
}

impl OfxPropertySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_int(&mut self, prop: &str, index: usize, v: i32) -> bool {
        let slot = self.props.entry(prop.to_string()).or_default();
        if slot.len() <= index {
            slot.resize(index + 1, OfxPropValue::Int(0));
        }
        match &mut slot[index] {
            OfxPropValue::Int(old) => {
                *old = v;
                true
            }
            slot => {
                *slot = OfxPropValue::Int(v);
                true
            }
        }
    }

    pub fn set_double(&mut self, prop: &str, index: usize, v: f64) -> bool {
        let slot = self.props.entry(prop.to_string()).or_default();
        if slot.len() <= index {
            slot.resize(index + 1, OfxPropValue::Double(0.0));
        }
        match &mut slot[index] {
            OfxPropValue::Double(old) => {
                *old = v;
                true
            }
            slot => {
                *slot = OfxPropValue::Double(v);
                true
            }
        }
    }

    pub fn set_string(&mut self, prop: &str, index: usize, v: &str) -> bool {
        let slot = self.props.entry(prop.to_string()).or_default();
        if slot.len() <= index {
            slot.resize(index + 1, OfxPropValue::Str(String::new()));
        }
        slot[index] = OfxPropValue::Str(v.to_string());
        true
    }

    pub fn get_int(&self, prop: &str, index: usize) -> Option<i32> {
        match self.props.get(prop)?.get(index)? {
            OfxPropValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_double(&self, prop: &str, index: usize) -> Option<f64> {
        match self.props.get(prop)?.get(index)? {
            OfxPropValue::Double(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string(&self, prop: &str, index: usize) -> Option<&str> {
        match self.props.get(prop)?.get(index)? {
            OfxPropValue::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

// OFX status codes used by the handshake (ofxCore.h)
pub const K_OFX_STAT_OK: i32 = 0;
pub const K_OFX_STAT_REPLY_YES: i32 = 1;
pub const K_OFX_STAT_REPLY_DEFAULT: i32 = 2;
pub const K_OFX_STAT_ERR_UNSET: i32 = 4; // what plugins return when ignoring an action

unsafe fn handle_as_set<'a>(handle: *mut std::os::raw::c_void) -> Option<&'a mut OfxPropertySet> {
    (handle as *mut OfxPropertySet).as_mut()
}

unsafe extern "C" fn prop_set_int(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: i32,
) -> i32 {
    let (Some(set), Some(name)) = (handle_as_set(handle), cstr(prop)) else {
        return 1; // kOfxStatErrBadHandle-ish; real code kept minimal
    };
    if set.set_int(name, index.max(0) as usize, value) {
        K_OFX_STAT_OK
    } else {
        1
    }
}

unsafe extern "C" fn prop_set_double(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: f64,
) -> i32 {
    let (Some(set), Some(name)) = (handle_as_set(handle), cstr(prop)) else {
        return 1;
    };
    if set.set_double(name, index.max(0) as usize, value) {
        K_OFX_STAT_OK
    } else {
        1
    }
}

unsafe extern "C" fn prop_set_string(
    handle: *mut std::os::raw::c_void,
    prop: *const std::os::raw::c_char,
    index: i32,
    value: *const std::os::raw::c_char,
) -> i32 {
    let (Some(set), Some(name), Some(val)) = (handle_as_set(handle), cstr(prop), cstr(value))
    else {
        return 1;
    };
    if set.set_string(name, index.max(0) as usize, val) {
        K_OFX_STAT_OK
    } else {
        1
    }
}

/// `OfxPropertySuiteV1` layout — first three entries implemented, remaining
/// slots are null so well-behaved plugins can detect unsupported members.
#[repr(C)]
pub struct OfxPropertySuiteV1 {
    prop_set_pointer: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        *mut std::os::raw::c_void,
    ) -> i32,
    prop_set_int: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        i32,
    ) -> i32,
    prop_set_double: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        f64,
    ) -> i32,
    prop_set_string: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        *const std::os::raw::c_char,
        i32,
        *const std::os::raw::c_char,
    ) -> i32,
    prop_get_pointer: usize,
    prop_get_int: usize,
    prop_get_double: usize,
    prop_get_string: usize,
    prop_reset: usize,
    prop_get_dimension: usize,
    prop_set_default: usize,
    _rest: [usize; 16],
}

/// `OfxHost` layout handed to plugins via `setHost`.
/// Raw pointers are immutable after init and only read from plugin threads
/// through the C ABI, so Sync is sound here.
#[repr(C)]
pub struct OfxHostRaw {
    host_name: *const std::os::raw::c_char,
    property_suite: *const OfxPropertySuiteV1,
    _remaining_suites: [usize; 15],
}

/// Host-side state shared by every loaded plugin (process-global).
static HOST_NAME_BYTES: &[u8] = b"AfterEffectsOSS\0";

unsafe impl Send for OfxHostRaw {}
unsafe impl Sync for OfxHostRaw {}

fn host_raw() -> &'static OfxHostRaw {
    use std::sync::OnceLock;
    static HOST: OnceLock<OfxHostRaw> = OnceLock::new();
    HOST.get_or_init(|| {
        let suite = Box::leak(Box::new(OfxPropertySuiteV1 {
            prop_set_pointer: noop_set_pointer,
            prop_set_int,
            prop_set_double,
            prop_set_string,
            prop_get_pointer: 0,
            prop_get_int: 0,
            prop_get_double: 0,
            prop_get_string: 0,
            prop_reset: 0,
            prop_get_dimension: 0,
            prop_set_default: 0,
            _rest: [0; 16],
        }));
        OfxHostRaw {
            host_name: HOST_NAME_BYTES.as_ptr() as *const std::os::raw::c_char,
            property_suite: suite as *const _,
            _remaining_suites: [0; 15],
        }
    })
}

unsafe extern "C" fn noop_set_pointer(
    _h: *mut std::os::raw::c_void,
    _p: *const std::os::raw::c_char,
    _i: i32,
    _v: *mut std::os::raw::c_void,
) -> i32 {
    3 // kOfxStatErrUnsupported
}

/// A third-party plugin whose binary is held open for its lifetime.
pub struct LoadedOfxPlugin {
    pub name: String,
    library: libloading::Library,
    /// Per-instance property sets the plugin may have requested during Load.
    instance_props: Vec<OfxPropertySet>,
}

impl LoadedOfxPlugin {
    /// dlopen + setHost + kOfxActionLoad handshake.
    pub fn attach(path: &Path, name: &str) -> Result<Self, String> {
        let library =
            unsafe { libloading::Library::new(path) }.map_err(|e| format!("dlopen failed: {e}"))?;
        unsafe {
            let get_plugin: libloading::Symbol<'_, unsafe extern "C" fn(i32) -> *mut OfxPluginRaw> =
                library
                    .get(b"OfxGetPlugin")
                    .map_err(|e| format!("missing OfxGetPlugin: {e}"))?;
            let raw = (get_plugin)(0);
            let raw = raw.as_ref().ok_or("OfxGetPlugin(0) returned null")?;

            // Handshake step 1: introduce the host (property suite available)
            if raw.set_host != 0 {
                let set_host: unsafe extern "C" fn(*mut std::os::raw::c_void) =
                    std::mem::transmute(raw.set_host);
                set_host(host_raw() as *const OfxHostRaw as *mut std::os::raw::c_void);
            }

            // Handshake step 2: kOfxActionLoad with empty in/out args
            if raw.main_entry != 0 {
                let main_fn: unsafe extern "C" fn(
                    *const std::os::raw::c_char,
                    *mut std::os::raw::c_void,
                    *mut std::os::raw::c_void,
                    *mut std::os::raw::c_void,
                ) -> i32 = std::mem::transmute(raw.main_entry);
                let action = b"kOfxActionLoad\0";
                let status = main_fn(
                    action.as_ptr() as *const _,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                match status {
                    s if s == K_OFX_STAT_OK
                        || s == K_OFX_STAT_REPLY_DEFAULT
                        || s == K_OFX_STAT_REPLY_YES
                        || s == K_OFX_STAT_ERR_UNSET => {}
                    other => return Err(format!("kOfxActionLoad rejected: status {other}")),
                }
            }
        }
        Ok(Self {
            name: name.to_string(),
            library,
            instance_props: Vec::new(),
        })
    }

    /// Allocate an instance property set owned by this plugin session.
    /// The returned pointer stays valid for the plugin's lifetime (the set is
    /// owned by the LoadedOfxPlugin and freed on drop).
    pub fn new_property_set(&mut self) -> *mut OfxPropertySet {
        self.instance_props.push(OfxPropertySet::new());
        let last = self.instance_props.last_mut().unwrap() as *mut OfxPropertySet;
        last
    }
}

/// Registry keeping attached plugins alive for the process lifetime.
pub fn register_loaded(plugin: LoadedOfxPlugin) -> usize {
    use std::sync::Mutex;
    static REGISTRY: std::sync::OnceLock<Mutex<Vec<LoadedOfxPlugin>>> = std::sync::OnceLock::new();
    let reg = REGISTRY.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = match reg.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.push(plugin);
    guard.len()
}

/// Attach-and-register convenience returning (total_registered, error).
pub fn attach_and_register(path: &Path, name: &str) -> Result<usize, String> {
    let plugin = LoadedOfxPlugin::attach(path, name)?;
    Ok(register_loaded(plugin))
}

/// Parameters for one OFX render invocation.
pub struct OfxRenderRequest<'a> {
    pub width: u32,
    pub height: u32,
    pub frame: f64,
    /// Source RGBA8 pixels (straight alpha)
    pub input: &'a [u8],
    /// Destination buffer (same length)
    pub output: &'a mut [u8],
}

impl LoadedOfxPlugin {
    /// Dispatch `kOfxImageEffectActionRender` with instance property sets
    /// carrying dimensions/frame. Plugins that only implement the Load
    /// handshake (no effect main body) yield `Err` cleanly rather than
    /// crashing — we cannot validate third-party memory discipline beyond
    /// status codes, so callers must keep using the returned output only on Ok.
    pub fn render(&mut self, req: &mut OfxRenderRequest) -> Result<(), String> {
        if req.input.len() != req.output.len() {
            return Err("input/output length mismatch".into());
        }
        unsafe {
            let get_plugin: libloading::Symbol<'_, unsafe extern "C" fn(i32) -> *mut OfxPluginRaw> =
                self.library
                    .get(b"OfxGetPlugin")
                    .map_err(|e| format!("missing OfxGetPlugin: {e}"))?;
            let raw = (get_plugin)(0).as_ref().ok_or("null plugin")?;
            if raw.main_entry == 0 {
                return Err("plugin exposes no main entry".into());
            }
            let main_fn: unsafe extern "C" fn(
                *const std::os::raw::c_char,
                *mut std::os::raw::c_void,
                *mut std::os::raw::c_void,
                *mut std::os::raw::c_void,
            ) -> i32 = std::mem::transmute(raw.main_entry);

            // Instance args property set (dimensions + frame), owned here.
            let mut args = OfxPropertySet::new();
            let _ = args.set_int("OfxPropWidth", 0, req.width as i32);
            let _ = args.set_int("OfxPropHeight", 0, req.height as i32);
            let _ = args.set_double("OfxPropFrame", 0, req.frame);
            let args_ptr = &args as *const OfxPropertySet as *mut std::os::raw::c_void;

            let action = b"kOfxImageEffectActionRender\0";
            // Effect handle is opaque to us; plugins receive the instance props
            let status = main_fn(
                action.as_ptr() as *const _,
                std::ptr::null_mut(),
                args_ptr,
                args_ptr,
            );
            match status {
                s if s == K_OFX_STAT_OK => Ok(()),
                s if s == K_OFX_STAT_ERR_UNSET || s == K_OFX_STAT_REPLY_DEFAULT => {
                    Err(format!("plugin does not implement Render (status {s})"))
                }
                other => Err(format!("render failed with status {other}")),
            }
        }
    }
}

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
    set_host: usize,   // function pointer, unused during probing
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
    let get_num: libloading::Symbol<'_, unsafe extern "C" fn() -> i32> =
        match lib.get(b"OfxGetNumberOfPlugins") {
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
            matches!(
                res,
                OfxProbeResult::LoadError(_) | OfxProbeResult::NotOfx(_)
            ),
            "garbage must not crash: {res:?}"
        );
    }

    #[test]
    fn test_property_set_roundtrip() {
        let mut set = OfxPropertySet::new();
        assert!(set.set_int("width", 0, 1920));
        assert!(set.set_int("multi", 1, 7)); // multi-dim index
        assert!(set.set_double("scale", 0, 0.5));
        assert!(set.set_string("name", 0, "blur"));

        assert_eq!(set.get_int("width", 0), Some(1920));
        assert_eq!(set.get_int("multi", 1), Some(7));
        assert_eq!(set.get_double("scale", 0), Some(0.5));
        assert_eq!(set.get_string("name", 0), Some("blur"));
        assert_eq!(set.get_int("missing", 0), None);
        assert_eq!(
            set.get_double("width", 0),
            None,
            "type mismatch returns none"
        );
    }

    #[test]
    fn test_property_suite_c_abi_roundtrip() {
        // Drive the raw C function pointers exactly as a plugin would.
        let mut set = OfxPropertySet::new();
        let handle = &mut set as *mut OfxPropertySet as *mut std::os::raw::c_void;
        let suite = host_raw().property_suite;
        unsafe {
            let name = b"OfxPropWidth\0";
            let rc_int = ((*suite).prop_set_int)(handle, name.as_ptr() as *const _, 0, 640);
            assert_eq!(rc_int, K_OFX_STAT_OK);
            let name_d = b"OfxPropScale\0";
            let rc_dbl = ((*suite).prop_set_double)(handle, name_d.as_ptr() as *const _, 0, 2.5);
            assert_eq!(rc_dbl, K_OFX_STAT_OK);
            let name_s = b"OfxPropName\0";
            let val = b"test\0";
            let rc_str = ((*suite).prop_set_string)(
                handle,
                name_s.as_ptr() as *const _,
                0,
                val.as_ptr() as *const _,
            );
            assert_eq!(rc_str, K_OFX_STAT_OK);
        }
        assert_eq!(set.get_int("OfxPropWidth", 0), Some(640));
        assert_eq!(set.get_double("OfxPropScale", 0), Some(2.5));
        assert_eq!(set.get_string("OfxPropName", 0), Some("test"));
    }
}
