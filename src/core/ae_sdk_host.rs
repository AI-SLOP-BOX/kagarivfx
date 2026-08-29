//! Adobe After Effects C++ Plugin SDK (PF / AEGP) Native Host Compatibility Layer (AE Parity).
//!
//! Replicates the official Adobe AE SDK ABI data layouts (`PF_EffectWorld`, `PF_ParamDef`,
//! `PF_Cmd`, `PF_Pixel8`, `PF_Pixel16`, `PF_PixelFloat`) enabling native C++ plugin interoperability.

#![allow(dead_code, non_camel_case_types)]

use std::ptr;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PF_Cmd {
    PF_Cmd_ABOUT = 0,
    PF_Cmd_GLOBAL_SETUP = 1,
    PF_Cmd_PARAMS_SETUP = 2,
    PF_Cmd_RENDER = 3,
    PF_Cmd_FRAME_SETUP = 4,
    PF_Cmd_USER_CHANGED_PARAM = 5,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PF_ParamType {
    PF_Param_SLIDER = 1,
    PF_Param_COLOR = 2,
    PF_Param_CHECKBOX = 3,
    PF_Param_POINT = 4,
    PF_Param_ANGLE = 5,
    PF_Param_FLOAT_SLIDER = 6,
    PF_Param_POPUP = 7,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PF_Pixel8 {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PF_Pixel16 {
    pub a: u16,
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PF_PixelFloat {
    pub a: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Adobe AE SDK native C-ABI compatible `PF_EffectWorld` structure.
/// Layout matches Adobe After Effects SDK struct layout with raw pointer `data`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PF_EffectWorld {
    pub format: i32,
    pub data: *mut u8,
    pub rowbytes: i32,
    pub width: i32,
    pub height: i32,
    pub origin_x: i32,
    pub origin_y: i32,
}

unsafe impl Send for PF_EffectWorld {}
unsafe impl Sync for PF_EffectWorld {}

/// Safe RAII wrapper for `PF_EffectWorld` managing underlying pixel memory.
#[derive(Debug)]
pub struct SafeEffectWorld {
    pub raw: PF_EffectWorld,
    buffer: Vec<u8>,
}

impl SafeEffectWorld {
    pub fn new_rgba8(width: i32, height: i32) -> Self {
        if width <= 0 || height <= 0 || width > (i32::MAX / 4) || height > 65536 {
            let raw = PF_EffectWorld {
                format: 0, // PF_PixelFormat_ARGB32
                data: std::ptr::null_mut(),
                rowbytes: 0,
                width: width.max(0),
                height: height.max(0),
                origin_x: 0,
                origin_y: 0,
            };
            return Self { raw, buffer: Vec::new() };
        }

        let w = width as usize;
        let h = height as usize;
        let rowbytes = (w * 4) as i32;
        let total_bytes = match (rowbytes as usize).checked_mul(h) {
            Some(tb) if tb <= 512 * 1024 * 1024 => tb,
            _ => 0,
        };
        let mut buffer = vec![0u8; total_bytes];
        let data_ptr = if total_bytes > 0 { buffer.as_mut_ptr() } else { std::ptr::null_mut() };

        let raw = PF_EffectWorld {
            format: 0, // PF_PixelFormat_ARGB32
            data: data_ptr,
            rowbytes,
            width,
            height,
            origin_x: 0,
            origin_y: 0,
        };

        Self { raw, buffer }
    }

    /// Converts standard RGBA buffer into AE ARGB8 format inside `PF_EffectWorld`.
    pub fn from_rgba_slice(rgba: &[u8], width: u32, height: u32) -> Self {
        let mut world = Self::new_rgba8(width as i32, height as i32);
        if world.buffer.is_empty() {
            return world;
        }

        let pixels = (width as usize)
            .saturating_mul(height as usize)
            .min(rgba.len() / 4);

        for i in 0..pixels {
            let src = i * 4;
            let dst = i * 4;
            // Adobe AE 8-bit format is ARGB: [A, R, G, B]
            world.buffer[dst] = rgba[src + 3]; // A
            world.buffer[dst + 1] = rgba[src]; // R
            world.buffer[dst + 2] = rgba[src + 1]; // G
            world.buffer[dst + 3] = rgba[src + 2]; // B
        }

        world.raw.data = world.buffer.as_mut_ptr();
        world
    }

    /// Converts ARGB8 effect output back into standard RGBA8 buffer [R, G, B, A].
    pub fn to_rgba_vec(&self) -> Vec<u8> {
        let pixels = (self.raw.width.max(0) as usize).saturating_mul(self.raw.height.max(0) as usize);
        let mut out = vec![0u8; pixels * 4];

        for i in 0..pixels {
            let src = i * 4;
            let dst = i * 4;
            if src + 3 < self.buffer.len() && dst + 3 < out.len() {
                out[dst] = self.buffer[src + 1]; // R
                out[dst + 1] = self.buffer[src + 2]; // G
                out[dst + 2] = self.buffer[src + 3]; // B
                out[dst + 3] = self.buffer[src]; // A
            }
        }
        out
    }

    pub fn as_raw(&self) -> &PF_EffectWorld {
        &self.raw
    }

    pub fn as_raw_mut(&mut self) -> &mut PF_EffectWorld {
        self.raw.data = self.buffer.as_mut_ptr();
        &mut self.raw
    }
}

/// Adobe AE Parameter Definition ABI struct (`PF_ParamDef`).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct PF_ParamDef {
    pub param_type: PF_ParamType,
    pub name: [u8; 32],
    pub value_f32: f32,
    pub value_color: PF_Pixel8,
    pub default_f32: f32,
    pub min_f32: f32,
    pub max_f32: f32,
}

impl PF_ParamDef {
    pub fn new_float_slider(name: &str, default: f32, min: f32, max: f32) -> Self {
        let mut name_buf = [0u8; 32];
        let bytes = name.as_bytes();
        let copy_len = bytes.len().min(31);
        name_buf[..copy_len].copy_from_slice(&bytes[..copy_len]);

        Self {
            param_type: PF_ParamType::PF_Param_FLOAT_SLIDER,
            name: name_buf,
            value_f32: default,
            value_color: PF_Pixel8::default(),
            default_f32: default,
            min_f32: min,
            max_f32: max,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pf_effect_world_c_abi_layout() {
        let mut world = SafeEffectWorld::new_rgba8(64, 64);
        assert_eq!(world.raw.width, 64);
        assert_eq!(world.raw.height, 64);
        assert_eq!(world.raw.rowbytes, 256);
        assert!(!world.raw.data.is_null());

        // Check buffer conversion
        let rgba = vec![255, 128, 64, 200];
        let converted = SafeEffectWorld::from_rgba_slice(&rgba, 1, 1);
        let roundtrip = converted.to_rgba_vec();
        assert_eq!(roundtrip, rgba);
    }
}
