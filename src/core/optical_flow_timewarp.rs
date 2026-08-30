//! Pixel Motion Timewarp & Dense Optical Flow Interpolation Engine (AE Timewarp Parity).
//!
//! Computes bidirectional dense optical flow fields and performs forward/backward
//! motion-compensated pixel warping for artifact-free slow motion and retiming.

#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct DenseFlowField {
    pub width: u32,
    pub height: u32,
    pub vectors: Vec<[f32; 2]>, // [dx, dy] per pixel
}

impl DenseFlowField {
    pub fn new(width: u32, height: u32) -> Self {
        const MAX_FLOW_PIXELS: usize = 16_777_216;
        let size = (width as usize)
            .checked_mul(height as usize)
            .filter(|&count| count <= MAX_FLOW_PIXELS)
            .unwrap_or(0);
        Self {
            width,
            height,
            vectors: vec![[0.0, 0.0]; size],
        }
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> [f32; 2] {
        if x < self.width && y < self.height {
            self.vectors
                .get(y as usize * self.width as usize + x as usize)
                .copied()
                .unwrap_or([0.0, 0.0])
        } else {
            [0.0, 0.0]
        }
    }
}

/// Computes block-matching dense optical flow vectors from source to target frame.
pub fn compute_dense_optical_flow(
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
) -> DenseFlowField {
    let mut flow = DenseFlowField::new(width, height);
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return flow;
    };
    if width == 0 || height == 0 || src_rgba.len() != size || tgt_rgba.len() != size {
        return flow;
    }
    if flow.vectors.len() != (width as usize).saturating_mul(height as usize) {
        return flow;
    }
    let w = width as i32;
    let h = height as i32;
    let block_radius = block_radius.clamp(0, 64);
    let search_radius = search_radius.clamp(0, 128);

    let get_luma = |buf: &[u8], x: i32, y: i32| -> f32 {
        let cx = x.clamp(0, w - 1) as usize;
        let cy = y.clamp(0, h - 1) as usize;
        let idx = (cy * width as usize + cx) * 4;
        0.299 * buf[idx] as f32 + 0.587 * buf[idx + 1] as f32 + 0.114 * buf[idx + 2] as f32
    };

    // Subsampled dense matching for high performance
    let step = 4;
    for by in (0..h).step_by(step as usize) {
        for bx in (0..w).step_by(step as usize) {
            let mut best_sad = f32::INFINITY;
            let mut best_dx = 0.0f32;
            let mut best_dy = 0.0f32;

            for sdy in -search_radius..=search_radius {
                for sdx in -search_radius..=search_radius {
                    let mut sad = 0.0f32;
                    for py in -block_radius..=block_radius {
                        for px in -block_radius..=block_radius {
                            let src_val = get_luma(src_rgba, bx + px, by + py);
                            let tgt_val = get_luma(tgt_rgba, bx + px + sdx, by + py + sdy);
                            sad += (src_val - tgt_val).abs();
                        }
                    }

                    if sad < best_sad {
                        best_sad = sad;
                        best_dx = sdx as f32;
                        best_dy = sdy as f32;
                    }
                }
            }

            // Populate block cells
            for dy in 0..step {
                let y = by + dy;
                if y >= h {
                    continue;
                }
                for dx in 0..step {
                    let x = bx + dx;
                    if x >= w {
                        continue;
                    }
                    flow.vectors[(y as u32 * width + x as u32) as usize] = [best_dx, best_dy];
                }
            }
        }
    }

    flow
}

/// Interpolates an intermediate frame at fractional position `t` (0.0 .. 1.0)
/// using bidirectional forward and backward flow fields.
pub fn interpolate_timewarp_frame(
    frame_a: &[u8],
    frame_b: &[u8],
    width: u32,
    height: u32,
    flow_a_to_b: &DenseFlowField,
    flow_b_to_a: &DenseFlowField,
    t: f32,
    out_rgba: &mut [u8],
) {
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return;
    };
    if width == 0
        || height == 0
        || frame_a.len() != size
        || frame_b.len() != size
        || out_rgba.len() != size
    {
        return;
    }

    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let w = width as f32;
    let h = height as f32;

    let sample_bilinear = |buf: &[u8], sx: f32, sy: f32| -> [f32; 4] {
        let x = sx.clamp(0.0, (w - 1.0).max(0.0));
        let y = sy.clamp(0.0, (h - 1.0).max(0.0));

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(width as usize - 1);
        let y1 = (y0 + 1).min(height as usize - 1);

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let idx00 = (y0 * width as usize + x0) * 4;
        let idx10 = (y0 * width as usize + x1) * 4;
        let idx01 = (y1 * width as usize + x0) * 4;
        let idx11 = (y1 * width as usize + x1) * 4;

        let mut res = [0.0f32; 4];
        for c in 0..4 {
            let v00 = buf[idx00 + c] as f32;
            let v10 = buf[idx10 + c] as f32;
            let v01 = buf[idx01 + c] as f32;
            let v11 = buf[idx11 + c] as f32;
            res[c] = (1.0 - fx) * (1.0 - fy) * v00
                + fx * (1.0 - fy) * v10
                + (1.0 - fx) * fy * v01
                + fx * fy * v11;
        }
        res
    };

    for y in 0..height {
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;

            // Flow vectors
            let v_ab = flow_a_to_b.get(x, y);
            let v_ba = flow_b_to_a.get(x, y);

            // Backward-warp positions
            let src_a_x = px - v_ab[0] * t;
            let src_a_y = py - v_ab[1] * t;

            let src_b_x = px + v_ba[0] * (1.0 - t);
            let src_b_y = py + v_ba[1] * (1.0 - t);

            let col_a = sample_bilinear(frame_a, src_a_x, src_a_y);
            let col_b = sample_bilinear(frame_b, src_b_x, src_b_y);

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            for c in 0..4 {
                let blended = col_a[c] * (1.0 - t) + col_b[c] * t;
                out_rgba[dst_idx + c] = blended.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_flow_bounds_allocation_and_rejects_oversized_compute() {
        let field = DenseFlowField::new(u32::MAX, u32::MAX);
        assert!(field.vectors.is_empty());
        assert_eq!(field.get(0, 0), [0.0, 0.0]);
        let flow = compute_dense_optical_flow(&[0; 4], &[0; 4], u32::MAX, u32::MAX, 1, 1);
        assert!(flow.vectors.is_empty());
    }

    #[test]
    fn test_dense_optical_flow_translation() {
        let width = 16u32;
        let height = 16u32;
        let mut frame_a = vec![0u8; (width * height * 4) as usize];
        let mut frame_b = vec![0u8; (width * height * 4) as usize];

        // Draw a 4x4 white patch moving from (4, 4) to (6, 4)
        for y in 4..8 {
            for x in 4..8 {
                let idx = (y * width + x) as usize * 4;
                frame_a[idx] = 255;
                frame_a[idx + 1] = 255;
                frame_a[idx + 2] = 255;
                frame_a[idx + 3] = 255;
            }
        }

        for y in 4..8 {
            for x in 6..10 {
                let idx = (y * width + x) as usize * 4;
                frame_b[idx] = 255;
                frame_b[idx + 1] = 255;
                frame_b[idx + 2] = 255;
                frame_b[idx + 3] = 255;
            }
        }

        let flow = compute_dense_optical_flow(&frame_a, &frame_b, width, height, 1, 3);
        let center_vec = flow.get(5, 5);
        assert_eq!(center_vec[0], 2.0);
        assert_eq!(center_vec[1], 0.0);
    }
}
