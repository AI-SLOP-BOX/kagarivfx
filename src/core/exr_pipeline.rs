//! Multi-Layer 32-bit Float OpenEXR & 3D Channel Extractor Engine (AE Parity).
//!
//! Provides multi-pass compositing capabilities for CG renders:
//! - Depth (Z-depth) normalization and fog/DOF mapping
//! - Surface Normal (XYZ) pass extraction
//! - Cryptomatte Object/Material ID matte isolation
//! - Motion Vector pass extraction for post-process motion blur

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ExrChannelPass {
    #[default]
    CombinedRGBA,
    DepthZ,
    NormalXYZ,
    Cryptomatte,
    MotionVector,
    Specular,
    Diffuse,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExrLayerPass {
    pub name: String,
    pub pass_type: ExrChannelPass,
    pub width: u32,
    pub height: u32,
    pub channels_f32: Vec<f32>, // Flat [R, G, B, A] or [Z] 32-bit float values
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepthExtractorOptions {
    pub black_point_depth: f32,
    pub white_point_depth: f32,
    pub invert: bool,
}

impl Default for DepthExtractorOptions {
    fn default() -> Self {
        Self {
            black_point_depth: 10.0,
            white_point_depth: 5000.0,
            invert: false,
        }
    }
}

/// Extracts Depth channel (Z) from 32-bit float buffer and normalizes to 8-bit grayscale RGBA.
pub fn extract_depth_to_rgba(
    depth_f32: &[f32],
    width: u32,
    height: u32,
    options: &DepthExtractorOptions,
    out_rgba: &mut [u8],
) {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    let Some(out_len) = len.checked_mul(4) else {
        return;
    };
    if width == 0 || height == 0 || depth_f32.len() < len || out_rgba.len() < out_len {
        return;
    }

    let z_min = if options.black_point_depth.is_finite() {
        options.black_point_depth
    } else {
        0.0
    };
    let z_max = if options.white_point_depth.is_finite() {
        options.white_point_depth
    } else {
        z_min + 1.0
    };
    let range = (z_max - z_min).max(1e-5);

    for i in 0..len {
        let z = depth_f32[i];
        let mut norm = ((z - z_min) / range).clamp(0.0, 1.0);
        if options.invert {
            norm = 1.0 - norm;
        }
        let byte_val = (norm * 255.0).round() as u8;

        let out_idx = i * 4;
        out_rgba[out_idx] = byte_val;
        out_rgba[out_idx + 1] = byte_val;
        out_rgba[out_idx + 2] = byte_val;
        out_rgba[out_idx + 3] = 255;
    }
}

/// Normalizes 3D Normal pass vectors ([-1..1, -1..1, -1..1]) to [0..255] RGB color.
pub fn extract_normals_to_rgba(
    normals_f32: &[f32], // [Nx, Ny, Nz] per pixel
    width: u32,
    height: u32,
    out_rgba: &mut [u8],
) {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    let Some(normals_len) = len.checked_mul(3) else {
        return;
    };
    let Some(out_len) = len.checked_mul(4) else {
        return;
    };
    if width == 0 || height == 0 || normals_f32.len() < normals_len || out_rgba.len() < out_len {
        return;
    }

    for i in 0..len {
        let in_idx = i * 3;
        let nx = (normals_f32[in_idx]
            .is_finite()
            .then_some(normals_f32[in_idx])
            .unwrap_or(0.0)
            * 0.5
            + 0.5)
            .clamp(0.0, 1.0);
        let ny = (normals_f32[in_idx + 1]
            .is_finite()
            .then_some(normals_f32[in_idx + 1])
            .unwrap_or(0.0)
            * 0.5
            + 0.5)
            .clamp(0.0, 1.0);
        let nz = (normals_f32[in_idx + 2]
            .is_finite()
            .then_some(normals_f32[in_idx + 2])
            .unwrap_or(0.0)
            * 0.5
            + 0.5)
            .clamp(0.0, 1.0);

        let out_idx = i * 4;
        out_rgba[out_idx] = (nx * 255.0).round() as u8;
        out_rgba[out_idx + 1] = (ny * 255.0).round() as u8;
        out_rgba[out_idx + 2] = (nz * 255.0).round() as u8;
        out_rgba[out_idx + 3] = 255;
    }
}

/// Extracts a single Cryptomatte Object/Material ID matte (0 or 255) from Cryptomatte float hash channel.
pub fn extract_cryptomatte_id(
    crypto_f32: &[f32],
    width: u32,
    height: u32,
    target_id_hash: f32,
    out_mask: &mut [u8],
) {
    let Some(len) = (width as usize).checked_mul(height as usize) else {
        return;
    };
    if crypto_f32.len() < len || out_mask.len() < len {
        return;
    }
    if !target_id_hash.is_finite() {
        out_mask[..len].fill(0);
        return;
    }

    for i in 0..len {
        let hash = crypto_f32[i];
        if hash.is_finite() && (hash - target_id_hash).abs() < 1e-4 {
            out_mask[i] = 255;
        } else {
            out_mask[i] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_depth_to_rgba() {
        let depth_data = vec![100.0, 500.0, 1000.0, 5000.0];
        let mut out = vec![0u8; 16];
        let opts = DepthExtractorOptions {
            black_point_depth: 0.0,
            white_point_depth: 1000.0,
            invert: false,
        };

        extract_depth_to_rgba(&depth_data, 2, 2, &opts, &mut out);

        assert_eq!(out[0], 26); // 100/1000 * 255 ~ 25.5
        assert_eq!(out[4], 128); // 500/1000 * 255 ~ 127.5
        assert_eq!(out[8], 255); // 1000/1000 * 255 = 255
        assert_eq!(out[12], 255); // Clamped to 255
    }

    #[test]
    fn test_extract_cryptomatte_id() {
        let crypto_hashes = vec![0.1234, 0.5678, 0.1234, 0.9999];
        let mut mask = vec![0u8; 4];

        extract_cryptomatte_id(&crypto_hashes, 2, 2, 0.1234, &mut mask);

        assert_eq!(mask[0], 255);
        assert_eq!(mask[1], 0);
        assert_eq!(mask[2], 255);
        assert_eq!(mask[3], 0);
    }
}
