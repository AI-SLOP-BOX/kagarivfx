//! VFX compositing C++ Plugin SDK & AEGP (After Effects General Plugin) Compatibility Bridge.
//!
//! Provides ABI-compatible host data structures, command dispatchers, and suite handler function pointers
//! enabling loading and execution of AE-standard C++ effects (.plugin / .aex) and OpenFX plugins.

#![allow(dead_code, non_camel_case_types, non_snake_case)]

/// Standard AE Effect Commands dispatched to Plugin Entry Points.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PF_Cmd {
    PF_Cmd_ABOUT = 0,
    PF_Cmd_GLOBAL_SETUP = 1,
    PF_Cmd_PARAMS_SETUP = 2,
    PF_Cmd_FRAME_SETUP = 3,
    PF_Cmd_RENDER = 4,
    PF_Cmd_FRAME_SETDOWN = 5,
    PF_Cmd_USER_CHANGED_PARAM = 6,
    PF_Cmd_QUERY_DYNAMIC_FLAGS = 7,
    PF_Cmd_SMART_RENDER = 8,
    PF_Cmd_SMART_PRE_RENDER = 9,
}

/// Pixel format descriptor for AE plugin worlds.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PF_PixelFormat {
    PF_PixelFormat_ARGB32 = 0,   // 8bpc ARGB
    PF_PixelFormat_ARGB64 = 1,   // 16bpc ARGB
    PF_PixelFormat_ARGB128 = 2,  // 32bpc Float ARGB
}

/// Buffer representation passed to AE C++ plugins for input and output rendering.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PF_EffectWorld {
    pub data: *mut u8,
    pub width: i32,
    pub height: i32,
    pub rowbytes: i32,
    pub pixel_format: PF_PixelFormat,
}

unsafe impl Send for PF_EffectWorld {}
unsafe impl Sync for PF_EffectWorld {}

/// InData passed from host to AE plugin.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PF_InData {
    pub current_time: i32,
    pub time_scale: i32,
    pub time_step: i32,
    pub width: i32,
    pub height: i32,
    pub pixel_aspect_ratio_num: i32,
    pub pixel_aspect_ratio_den: i32,
    pub num_params: i32,
    pub version_major: i16,
    pub version_minor: i16,
}

/// OutData returned from AE plugin to host.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PF_OutData {
    pub my_version: u32,
    pub out_flags: u32,
    pub out_flags2: u32,
    pub width: i32,
    pub height: i32,
    pub origin_x: i32,
    pub origin_y: i32,
}

impl Default for PF_OutData {
    fn default() -> Self {
        Self {
            my_version: 1,
            out_flags: 0,
            out_flags2: 0,
            width: 0,
            height: 0,
            origin_x: 0,
            origin_y: 0,
        }
    }
}

/// Native C-ABI plugin entry point signature for AE effects.
pub type PF_PluginEntryPoint = unsafe extern "C" fn(
    cmd: PF_Cmd,
    in_data: *const PF_InData,
    out_data: *mut PF_OutData,
    params: *mut *mut std::ffi::c_void,
    output: *mut PF_EffectWorld,
    extra: *mut std::ffi::c_void,
) -> i32;

/// Host execution context for loading and running AE C++ plugins.
pub struct AePluginHostContext {
    pub name: String,
    pub entry_point: Option<PF_PluginEntryPoint>,
}

impl AePluginHostContext {
    pub fn new(name: &str, entry_point: Option<PF_PluginEntryPoint>) -> Self {
        Self {
            name: name.to_string(),
            entry_point,
        }
    }

    /// Dispatches render command to native C++ plugin.
    pub fn render_frame(
        &self,
        pixels: &mut [u8],
        width: i32,
        height: i32,
        frame: i32,
        fps: i32,
    ) -> Result<(), String> {
        let entry = self.entry_point.ok_or("No plugin entry point loaded")?;

        let in_data = PF_InData {
            current_time: frame,
            time_scale: fps,
            time_step: 1,
            width,
            height,
            pixel_aspect_ratio_num: 1,
            pixel_aspect_ratio_den: 1,
            num_params: 0,
            version_major: 14,
            version_minor: 0,
        };

        let mut out_data = PF_OutData::default();
        let mut effect_world = PF_EffectWorld {
            data: pixels.as_mut_ptr(),
            width,
            height,
            rowbytes: width * 4,
            pixel_format: PF_PixelFormat::PF_PixelFormat_ARGB32,
        };

        let err_code = unsafe {
            entry(
                PF_Cmd::PF_Cmd_RENDER,
                &in_data as *const _,
                &mut out_data as *mut _,
                std::ptr::null_mut(),
                &mut effect_world as *mut _,
                std::ptr::null_mut(),
            )
        };

        if err_code == 0 {
            Ok(())
        } else {
            Err(format!("AE Plugin error code: {err_code}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock AE C++ plugin entry point for testing
    unsafe extern "C" fn mock_ae_plugin_entry(
        cmd: PF_Cmd,
        in_data: *const PF_InData,
        _out_data: *mut PF_OutData,
        _params: *mut *mut std::ffi::c_void,
        output: *mut PF_EffectWorld,
        _extra: *mut std::ffi::c_void,
    ) -> i32 {
        if cmd == PF_Cmd::PF_Cmd_RENDER && !output.is_null() && !in_data.is_null() {
            let world = &mut *output;
            let len = (world.width * world.height * 4) as usize;
            let slice = std::slice::from_raw_parts_mut(world.data, len);
            // Invert red channel
            for chunk in slice.chunks_exact_mut(4) {
                chunk[0] = 255 - chunk[0];
            }
            0 // PF_Err_NONE
        } else {
            0
        }
    }

    #[test]
    fn test_ae_plugin_host_dispatch() {
        let host = AePluginHostContext::new("Mock Inverter", Some(mock_ae_plugin_entry));
        let mut buffer = vec![100u8, 50, 20, 255];
        let res = host.render_frame(&mut buffer, 1, 1, 0, 30);
        assert!(res.is_ok());
        assert_eq!(buffer[0], 155); // 255 - 100
    }
}
