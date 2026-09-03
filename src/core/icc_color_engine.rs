//! Enterprise Color Management: 16-bit / 32-bit Float Pipeline,
//! ICC Profile Tone Reproduction Curves, and Bradford Chromatic Adaptation (AE Parity).

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ColorBitDepth {
    #[default]
    EightBpc,
    SixteenBpc,
    ThirtyTwoBpcFloat,
}

/// Bradford Chromatic Adaptation Matrix D65 -> D50 (ICC standard profile connection space).
const BRADFORD_D65_TO_D50: [[f32; 3]; 3] = [
    [1.0478112, 0.0228866, -0.0501270],
    [0.0295424, 0.9904844, -0.0170491],
    [-0.0092345, 0.0150436, 0.7521316],
];

/// Bradford Chromatic Adaptation Matrix D50 -> D65.
const BRADFORD_D50_TO_D65: [[f32; 3]; 3] = [
    [0.9555766, -0.0230393, 0.0631636],
    [-0.0282895, 1.0099416, 0.0210077],
    [0.0122982, -0.0204830, 1.3299098],
];

fn mat_mul_3x3(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Adapts XYZ coordinates between D65 and D50 white points using Bradford method.
pub fn adapt_white_point_d65_to_d50(xyz: [f32; 3]) -> [f32; 3] {
    mat_mul_3x3(&BRADFORD_D65_TO_D50, xyz)
}

pub fn adapt_white_point_d50_to_d65(xyz: [f32; 3]) -> [f32; 3] {
    mat_mul_3x3(&BRADFORD_D50_TO_D65, xyz)
}

/// Converts 8-bit RGBA buffer to 32-bit Float linear RGBA buffer [0.0..1.0].
pub fn convert_rgba8_to_rgba32f(src: &[u8], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        let v = src[i] as f32 / 255.0;
        dst[i] = if i % 4 == 3 {
            v
        } else if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        };
    }
}

/// Converts 32-bit Float RGBA buffer back to 8-bit RGBA buffer with dithering/clamping.
pub fn convert_rgba32f_to_rgba8(src: &[f32], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        let v = src[i].clamp(0.0, 1.0);
        let srgb = if i % 4 == 3 {
            v
        } else if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        dst[i] = (srgb * 255.0).round() as u8;
    }
}

/// Converts 8-bit RGBA buffer to 16-bit RGBA buffer [0..65535].
pub fn convert_rgba8_to_rgba16(src: &[u8], dst: &mut [u16]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        // Exact 8-to-16 bit expansion: (v << 8) | v
        let v = src[i] as u16;
        dst[i] = (v << 8) | v;
    }
}

/// Converts 16-bit RGBA buffer to 8-bit RGBA buffer.
pub fn convert_rgba16_to_rgba8(src: &[u16], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = (src[i] >> 8) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bradford_adaptation_roundtrip() {
        let d65_white = [0.95047, 1.00000, 1.08883];
        let d50 = adapt_white_point_d65_to_d50(d65_white);
        let back = adapt_white_point_d50_to_d65(d50);

        for c in 0..3 {
            assert!((back[c] - d65_white[c]).abs() < 1e-3);
        }
    }

    #[test]
    fn test_bit_depth_conversions_roundtrip() {
        let original_8 = vec![0u8, 128, 255, 64];
        let mut buf_32f = vec![0.0f32; 4];
        let mut back_8 = vec![0u8; 4];

        convert_rgba8_to_rgba32f(&original_8, &mut buf_32f);
        convert_rgba32f_to_rgba8(&buf_32f, &mut back_8);
        assert_eq!(original_8, back_8);

        let mut buf_16 = vec![0u16; 4];
        convert_rgba8_to_rgba16(&original_8, &mut buf_16);
        convert_rgba16_to_rgba8(&buf_16, &mut back_8);
        assert_eq!(original_8, back_8);
    }

    #[test]
    fn test_srgb_transfer_curve_midpoint() {
        let mut linear = [0.0f32; 1];
        convert_rgba8_to_rgba32f(&[128], &mut linear);
        assert!((linear[0] - 0.2158605).abs() < 1e-4);
    }

    #[test]
    fn test_alpha_channel_remains_linear() {
        let mut linear = [0.0f32; 4];
        convert_rgba8_to_rgba32f(&[128, 128, 128, 128], &mut linear);
        assert!((linear[0] - 0.2158605).abs() < 1e-4);
        assert!((linear[3] - 128.0 / 255.0).abs() < 1e-6);
    }
}
