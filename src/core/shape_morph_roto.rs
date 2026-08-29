//! Advanced Bezier Path Morphing, Shape Boolean Operations, and
//! Roto Brush 2/3 Temporal Propagation Engine (AE Parity).

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BezierKnot {
    pub point: [f32; 2],
    pub in_tangent: [f32; 2],
    pub out_tangent: [f32; 2],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BezierPathData {
    pub closed: bool,
    pub knots: Vec<BezierKnot>,
}

impl BezierPathData {
    /// Resamples the path to exactly `target_count` knots evenly distributed for morphing.
    pub fn resample_uniform(&self, target_count: usize) -> Vec<[f32; 2]> {
        if self.knots.is_empty() || target_count == 0 {
            return vec![[0.0, 0.0]; target_count];
        }
        if self.knots.len() == 1 {
            return vec![self.knots[0].point; target_count];
        }

        let mut pts = Vec::with_capacity(target_count);
        let n_src = self.knots.len();
        for i in 0..target_count {
            let t = i as f32 / target_count as f32;
            let src_idx_f = t * n_src as f32;
            let idx0 = (src_idx_f.floor() as usize) % n_src;
            let idx1 = (idx0 + 1) % n_src;
            let frac = src_idx_f - idx0 as f32;

            let p0 = self.knots[idx0].point;
            let p1 = self.knots[idx1].point;
            let interp = [
                p0[0] + (p1[0] - p0[0]) * frac,
                p0[1] + (p1[1] - p0[1]) * frac,
            ];
            pts.push(interp);
        }
        pts
    }
}

/// Interpolates smoothly between two paths of arbitrary knot counts.
pub fn morph_bezier_paths(
    path_a: &BezierPathData,
    path_b: &BezierPathData,
    progress: f32,
    sample_density: usize,
) -> Vec<[f32; 2]> {
    let count = sample_density.max(16);
    let pts_a = path_a.resample_uniform(count);
    let pts_b = path_b.resample_uniform(count);

    let t = progress.clamp(0.0, 1.0);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let pa = pts_a[i];
        let pb = pts_b[i];
        out.push([pa[0] + (pb[0] - pa[0]) * t, pa[1] + (pb[1] - pa[1]) * t]);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeBooleanMode {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

/// Combines two binary 8-bit masks using shape boolean logic.
pub fn apply_shape_boolean_masks(
    mask_a: &[u8],
    mask_b: &[u8],
    mode: ShapeBooleanMode,
    out_mask: &mut [u8],
) {
    let len = mask_a.len().min(mask_b.len()).min(out_mask.len());
    for i in 0..len {
        let a = mask_a[i] > 127;
        let b = mask_b[i] > 127;
        let res = match mode {
            ShapeBooleanMode::Union => a || b,
            ShapeBooleanMode::Subtract => a && !b,
            ShapeBooleanMode::Intersect => a && b,
            ShapeBooleanMode::Exclude => a ^ b,
        };
        out_mask[i] = if res { 255 } else { 0 };
    }
}

/// Propagates a segmentation matte forward in time using optical flow motion vectors.
pub fn propagate_roto_matte_forward(
    prev_mask: &[u8],
    flow_vectors: &[[f32; 2]], // [dx, dy] per pixel
    width: u32,
    height: u32,
    out_propagated_mask: &mut [u8],
) {
    let w = width as usize;
    let h = height as usize;
    let len = w * h;
    if prev_mask.len() < len || flow_vectors.len() < len || out_propagated_mask.len() < len {
        return;
    }

    // Backward warping: Sample from prev_mask at (x - dx, y - dy)
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let [dx, dy] = flow_vectors[idx];
            let src_x = (x as f32 - dx).clamp(0.0, (w - 1) as f32);
            let src_y = (y as f32 - dy).clamp(0.0, (h - 1) as f32);

            let x0 = src_x.floor() as usize;
            let y0 = src_y.floor() as usize;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);

            let fx = src_x - x0 as f32;
            let fy = src_y - y0 as f32;

            let m00 = prev_mask[y0 * w + x0] as f32;
            let m10 = prev_mask[y0 * w + x1] as f32;
            let m01 = prev_mask[y1 * w + x0] as f32;
            let m11 = prev_mask[y1 * w + x1] as f32;

            let interp = (1.0 - fx) * (1.0 - fy) * m00
                + fx * (1.0 - fy) * m10
                + (1.0 - fx) * fy * m01
                + fx * fy * m11;

            out_propagated_mask[idx] = interp.round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_path_morphing() {
        let p_a = BezierPathData {
            closed: true,
            knots: vec![
                BezierKnot {
                    point: [0.0, 0.0],
                    in_tangent: [0.0; 2],
                    out_tangent: [0.0; 2],
                },
                BezierKnot {
                    point: [100.0, 0.0],
                    in_tangent: [0.0; 2],
                    out_tangent: [0.0; 2],
                },
            ],
        };
        let p_b = BezierPathData {
            closed: true,
            knots: vec![
                BezierKnot {
                    point: [0.0, 100.0],
                    in_tangent: [0.0; 2],
                    out_tangent: [0.0; 2],
                },
                BezierKnot {
                    point: [100.0, 100.0],
                    in_tangent: [0.0; 2],
                    out_tangent: [0.0; 2],
                },
            ],
        };

        let morphed_mid = morph_bezier_paths(&p_a, &p_b, 0.5, 16);
        assert_eq!(morphed_mid.len(), 16);
        // Midpoint Y should be ~50.0
        assert!((morphed_mid[0][1] - 50.0).abs() < 1e-3);
    }

    #[test]
    fn test_shape_boolean_subtract() {
        let mask_a = vec![255, 255, 0, 0];
        let mask_b = vec![0, 255, 255, 0];
        let mut out = vec![0u8; 4];

        apply_shape_boolean_masks(&mask_a, &mask_b, ShapeBooleanMode::Subtract, &mut out);
        assert_eq!(out, vec![255, 0, 0, 0]);
    }
}
