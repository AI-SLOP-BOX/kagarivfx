#![allow(dead_code)]
use crate::core::timeline::{Layer, LayerType};

/// Evaluates effective transformation matrix and vector bounding box for continuously rasterized layers (✸ switch enabled).
#[derive(Debug, Clone, Copy)]
pub struct ContinuouslyRasterizedTransform {
    pub scale_factor: [f32; 2],
    pub composite_matrix: [[f32; 3]; 3],
    pub world_bounds: [f32; 4], // [min_x, min_y, max_x, max_y]
}

/// 2D Homogeneous 3x3 Matrix Multiplication helper.
pub fn multiply_matrix_3x3(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0f32; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            out[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c];
        }
    }
    out
}

/// Creates a 2D affine transformation matrix for [position, scale, rotation_deg, anchor].
pub fn create_affine_matrix(
    position: [f32; 2],
    scale: [f32; 2],
    rotation_deg: f32,
    anchor: [f32; 2],
) -> [[f32; 3]; 3] {
    let rad = rotation_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    // 1. Translate by (-anchor)
    let t_anchor = [
        [1.0, 0.0, -anchor[0]],
        [0.0, 1.0, -anchor[1]],
        [0.0, 0.0, 1.0],
    ];

    // 2. Scale & Rotate
    let s_rot = [
        [scale[0] * cos, -scale[1] * sin, 0.0],
        [scale[0] * sin, scale[1] * cos, 0.0],
        [0.0, 0.0, 1.0],
    ];

    // 3. Translate by position
    let t_pos = [
        [1.0, 0.0, position[0]],
        [0.0, 1.0, position[1]],
        [0.0, 0.0, 1.0],
    ];

    let step1 = multiply_matrix_3x3(&s_rot, &t_anchor);
    multiply_matrix_3x3(&t_pos, &step1)
}

/// Computes sharp rasterization bounds and transform matrices for vector layers or PreComps with continuous rasterization enabled.
pub fn evaluate_continuous_rasterization(
    layer: &Layer,
    frame: u32,
    viewport_size: [f32; 2],
) -> ContinuouslyRasterizedTransform {
    let pos = layer.transform.position.evaluate(frame);
    let scale = layer.transform.scale.evaluate(frame);
    let rot = layer.transform.rotation.evaluate(frame);
    let anchor = layer.transform.anchor_point.evaluate(frame);

    let affine_matrix = create_affine_matrix(pos, scale, rot, anchor);

    // Default base dimensions for vector shape / raster layer
    let (base_w, base_h) = match &layer.layer_type {
        LayerType::Solid { .. } => (viewport_size[0], viewport_size[1]),
        LayerType::Shape { .. } => (viewport_size[0], viewport_size[1]),
        LayerType::PreComp { .. } => (viewport_size[0], viewport_size[1]),
        _ => (100.0, 100.0),
    };

    // Calculate transformed bounding box corners
    let corners = [
        [0.0, 0.0, 1.0],
        [base_w, 0.0, 1.0],
        [base_w, base_h, 1.0],
        [0.0, base_h, 1.0],
    ];

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for corner in &corners {
        let tx = affine_matrix[0][0] * corner[0] + affine_matrix[0][1] * corner[1] + affine_matrix[0][2];
        let ty = affine_matrix[1][0] * corner[0] + affine_matrix[1][1] * corner[1] + affine_matrix[1][2];

        min_x = min_x.min(tx);
        min_y = min_y.min(ty);
        max_x = max_x.max(tx);
        max_y = max_y.max(ty);
    }

    ContinuouslyRasterizedTransform {
        scale_factor: [scale[0].abs().max(0.001), scale[1].abs().max(0.001)],
        composite_matrix: affine_matrix,
        world_bounds: [min_x, min_y, max_x, max_y],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affine_matrix_identity() {
        let mat = create_affine_matrix([0.0, 0.0], [1.0, 1.0], 0.0, [0.0, 0.0]);
        assert_eq!(mat[0][0], 1.0);
        assert_eq!(mat[1][1], 1.0);
        assert_eq!(mat[2][2], 1.0);
    }
}
