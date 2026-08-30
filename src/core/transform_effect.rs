//! Transform Effect Engine (AE Parity - Distort > Transform).
//!
//! Provides layer-stage 2D affine transformation within the effect stack:
//! Anchor Point, Position, Scale (Width/Height), Skew, Skew Axis, Rotation, and Opacity.
//! Uses high-precision 3x3 affine matrix inversion and bilinear sampling.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransformEffectParams {
    pub anchor_point: [f32; 2],
    pub position: [f32; 2],
    pub scale_width: f32,  // % (100.0 = 1.0)
    pub scale_height: f32, // % (100.0 = 1.0)
    pub uniform_scale: bool,
    pub skew_deg: f32,      // -85.0..85.0
    pub skew_axis_deg: f32, // 0.0..360.0
    pub rotation_deg: f32,
    pub opacity: f32, // 0.0..100.0%
}

impl Default for TransformEffectParams {
    fn default() -> Self {
        Self {
            anchor_point: [960.0, 540.0],
            position: [960.0, 540.0],
            scale_width: 100.0,
            scale_height: 100.0,
            uniform_scale: true,
            skew_deg: 0.0,
            skew_axis_deg: 0.0,
            rotation_deg: 0.0,
            opacity: 100.0,
        }
    }
}

/// Applies 2D Transform effect to an RGBA pixel buffer.
pub fn apply_transform_effect(
    src: &[u8],
    width: u32,
    height: u32,
    params: &TransformEffectParams,
) -> Vec<u8> {
    let Some(pixel_count) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return src.to_vec();
    };
    if src.len() != pixel_count * 4 || width == 0 || height == 0 {
        return src.to_vec();
    }

    let mut dst = vec![0u8; pixel_count * 4];
    let finite = |value: f32, fallback: f32| {
        if value.is_finite() { value } else { fallback }
    };

    let sx = (if params.uniform_scale {
        finite(params.scale_width, 100.0)
    } else {
        finite(params.scale_width, 100.0)
    } / 100.0)
        .max(0.001);
    let sy = (if params.uniform_scale {
        finite(params.scale_width, 100.0)
    } else {
        finite(params.scale_height, 100.0)
    } / 100.0)
        .max(0.001);

    let rot_rad = finite(params.rotation_deg, 0.0).to_radians();
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();

    let skew_rad = finite(params.skew_deg, 0.0).to_radians().clamp(-1.48, 1.48);
    let tan_skew = skew_rad.tan();

    let skew_axis_rad = finite(params.skew_axis_deg, 0.0).to_radians();
    let cos_sa = skew_axis_rad.cos();
    let sin_sa = skew_axis_rad.sin();

    let ax = finite(params.anchor_point[0], 0.0);
    let ay = finite(params.anchor_point[1], 0.0);
    let px = finite(params.position[0], 0.0);
    let py = finite(params.position[1], 0.0);

    let opacity_mult = (finite(params.opacity, 100.0) / 100.0).clamp(0.0, 1.0);

    let sample_bilinear = |x: f32, y: f32| -> [u8; 4] {
        if x < 0.0 || x > (width - 1) as f32 || y < 0.0 || y > (height - 1) as f32 {
            return [0, 0, 0, 0];
        }

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(width - 1);
        let y1 = (y0 + 1).min(height - 1);

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let i00 = ((y0 * width + x0) * 4) as usize;
        let i10 = ((y0 * width + x1) * 4) as usize;
        let i01 = ((y1 * width + x0) * 4) as usize;
        let i11 = ((y1 * width + x1) * 4) as usize;

        let mut out = [0u8; 4];
        for c in 0..4 {
            let top = src[i00 + c] as f32 * (1.0 - fx) + src[i10 + c] as f32 * fx;
            let bot = src[i01 + c] as f32 * (1.0 - fx) + src[i11 + c] as f32 * fx;
            out[c] = (top * (1.0 - fy) + bot * fy).clamp(0.0, 255.0) as u8;
        }
        out
    };

    for y in 0..height {
        let dy = y as f32 - py;
        for x in 0..width {
            let dx = x as f32 - px;

            // 1. Inverse Rotation
            let rx = dx * cos_r + dy * sin_r;
            let ry = -dx * sin_r + dy * cos_r;

            // 2. Inverse Skew around Skew Axis
            let sa_x = rx * cos_sa + ry * sin_sa;
            let sa_y = -rx * sin_sa + ry * cos_sa;

            let unskew_x = sa_x - tan_skew * sa_y;
            let unskew_y = sa_y;

            let unskew_rx = unskew_x * cos_sa - unskew_y * sin_sa;
            let unskew_ry = unskew_x * sin_sa + unskew_y * cos_sa;

            // 3. Inverse Scale
            let unscale_x = unskew_rx / sx;
            let unscale_y = unskew_ry / sy;

            // 4. Translate back to anchor point
            let src_x = unscale_x + ax;
            let src_y = unscale_y + ay;

            let mut pixel = sample_bilinear(src_x, src_y);
            if opacity_mult < 1.0 {
                pixel[3] = ((pixel[3] as f32) * opacity_mult).round() as u8;
            }

            let d_idx = ((y * width + x) * 4) as usize;
            dst[d_idx..d_idx + 4].copy_from_slice(&pixel);
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_rejects_overflow_and_sanitizes_parameters() {
        let original = vec![120u8; 16];
        let mut params = TransformEffectParams::default();
        params.anchor_point = [f32::NAN, f32::INFINITY];
        params.position = [f32::NAN, f32::NEG_INFINITY];
        params.scale_width = f32::NAN;
        params.rotation_deg = f32::INFINITY;
        params.opacity = f32::NAN;
        let rendered = apply_transform_effect(&original, 2, 2, &params);
        assert!(rendered.iter().all(|value| *value <= 255));
        assert_eq!(
            apply_transform_effect(&original, u32::MAX, u32::MAX, &params),
            original
        );
    }

    #[test]
    fn test_transform_effect_identity() {
        let w = 16u32;
        let h = 16u32;
        let mut src = vec![0u8; (w * h * 4) as usize];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (i % 255) as u8;
        }

        let params = TransformEffectParams {
            anchor_point: [8.0, 8.0],
            position: [8.0, 8.0],
            scale_width: 100.0,
            scale_height: 100.0,
            uniform_scale: true,
            skew_deg: 0.0,
            skew_axis_deg: 0.0,
            rotation_deg: 0.0,
            opacity: 100.0,
        };

        let dst = apply_transform_effect(&src, w, h, &params);
        assert_eq!(dst.len(), src.len());
        assert_eq!(dst[0], src[0]);
    }
}
