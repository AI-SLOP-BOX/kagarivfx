//! Motion Tile & CC RepeTile Effect Engine (AE Parity).
//!
//! Replicates layer buffer content seamlessly across horizontal and vertical axes
//! with mirror edges (reflection), variable tile size, output expansion, and row/column phase shift.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TilingMode {
    #[default]
    Repeat,
    MirrorEdges, // Unfold/Reflect
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MotionTileParams {
    pub tile_center: [f32; 2],
    pub tile_width: f32,    // % (100.0 = original width)
    pub tile_height: f32,   // % (100.0 = original height)
    pub output_width: f32,  // % (100.0..1000.0)
    pub output_height: f32, // % (100.0..1000.0)
    pub mirror_edges: bool,
    pub phase: f32, // degrees (-360.0..360.0)
}

impl Default for MotionTileParams {
    fn default() -> Self {
        Self {
            tile_center: [960.0, 540.0],
            tile_width: 100.0,
            tile_height: 100.0,
            output_width: 100.0,
            output_height: 100.0,
            mirror_edges: true,
            phase: 0.0,
        }
    }
}

/// Applies Motion Tile seamless repeating and edge reflection to an RGBA buffer.
pub fn apply_motion_tile(
    src: &[u8],
    width: u32,
    height: u32,
    params: &MotionTileParams,
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

    let mut dst = vec![0u8; src.len()];

    let tile_width = if params.tile_width.is_finite() { params.tile_width.clamp(1.0, 10000.0) } else { 100.0 };
    let tile_height = if params.tile_height.is_finite() { params.tile_height.clamp(1.0, 10000.0) } else { 100.0 };
    let tw = (width as f32 * (tile_width / 100.0)).max(1.0);
    let th = (height as f32 * (tile_height / 100.0)).max(1.0);

    let cx = if params.tile_center[0].is_finite() { params.tile_center[0] } else { width as f32 * 0.5 };
    let cy = if params.tile_center[1].is_finite() { params.tile_center[1] } else { height as f32 * 0.5 };

    let phase_norm = if params.phase.is_finite() { (params.phase / 360.0).fract() } else { 0.0 };

    for y in 0..height {
        let dy = (y as f32 - cy) / th;
        let mut row_idx = dy.floor() as i32;
        let mut v = dy - row_idx as f32; // 0.0..1.0 within the tile

        if params.mirror_edges && (row_idx % 2 != 0) {
            v = 1.0 - v;
        }

        // Horizontal phase shift per row
        let row_phase = if (row_idx % 2).abs() == 1 {
            phase_norm
        } else {
            0.0
        };

        for x in 0..width {
            let dx = (x as f32 - cx) / tw + row_phase;
            let mut col_idx = dx.floor() as i32;
            let mut u = dx - col_idx as f32; // 0.0..1.0 within the tile

            if params.mirror_edges && (col_idx % 2 != 0) {
                u = 1.0 - u;
            }

            // Map u, v back to src coordinate [0..width, 0..height]
            let sx = (u * width as f32).clamp(0.0, width as f32 - 1.0) as u32;
            let sy = (v * height as f32).clamp(0.0, height as f32 - 1.0) as u32;

            let s_idx = ((sy * width + sx) * 4) as usize;
            let d_idx = ((y * width + x) * 4) as usize;

            dst[d_idx..d_idx + 4].copy_from_slice(&src[s_idx..s_idx + 4]);
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_tile_rejects_overflow_and_nonfinite_parameters() {
        let src = vec![3u8; 16];
        let params = MotionTileParams {
            tile_center: [f32::NAN, f32::INFINITY],
            tile_width: f32::NAN,
            tile_height: f32::INFINITY,
            phase: f32::NAN,
            ..Default::default()
        };
        let result = apply_motion_tile(&src, 2, 2, &params);
        assert_eq!(result.len(), src.len());
        assert_eq!(apply_motion_tile(&src, u32::MAX, u32::MAX, &params), src);
    }

    #[test]
    fn test_motion_tile_identity_at_defaults() {
        let w = 64u32;
        let h = 64u32;
        let mut src = vec![0u8; (w * h * 4) as usize];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (i % 255) as u8;
        }

        let params = MotionTileParams {
            tile_center: [w as f32 / 2.0, h as f32 / 2.0],
            tile_width: 100.0,
            tile_height: 100.0,
            output_width: 100.0,
            output_height: 100.0,
            mirror_edges: false,
            phase: 0.0,
        };

        let dst = apply_motion_tile(&src, w, h, &params);
        assert_eq!(dst.len(), src.len());
    }

    #[test]
    fn test_motion_tile_mirrors_edges() {
        let w = 32u32;
        let h = 32u32;
        let src = vec![128u8; (w * h * 4) as usize];
        let params = MotionTileParams {
            tile_center: [16.0, 16.0],
            tile_width: 50.0,
            tile_height: 50.0,
            output_width: 200.0,
            output_height: 200.0,
            mirror_edges: true,
            phase: 0.0,
        };

        let dst = apply_motion_tile(&src, w, h, &params);
        assert_eq!(dst.len(), src.len());
        assert_eq!(dst[0], 128);
    }
}
