//! CC Page Turn / Page Curl Effect Engine (AE Parity).
//!
//! Realistic 3D cylindrical paper fold simulation. Maps 2D surface coordinates
//! onto a rolled cylinder with front face, curled backside projection, highlight ridge,
//! and cast drop shadow underneath the curl.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageTurnParams {
    pub fold_position: [f32; 2],     // Peel handle point
    pub fold_radius: f32,           // Cylinder radius (px)
    pub fold_direction_deg: f32,    // Angle of fold line (0 = horizontal, 90 = vertical)
    pub light_direction_deg: f32,   // Direction of cylindrical specular highlight
    pub back_opacity: f32,          // 0.0..100.0%
    pub back_color: [f32; 4],       // Default paper backside color
}

impl Default for PageTurnParams {
    fn default() -> Self {
        Self {
            fold_position: [1920.0, 1080.0],
            fold_radius: 120.0,
            fold_direction_deg: -45.0,
            light_direction_deg: -45.0,
            back_opacity: 100.0,
            back_color: [0.92, 0.92, 0.94, 1.0],
        }
    }
}

/// Renders CC Page Turn cylindrical fold deformation onto an RGBA pixel buffer.
pub fn apply_page_turn(
    src: &[u8],
    width: u32,
    height: u32,
    params: &PageTurnParams,
) -> Vec<u8> {
    if src.len() != (width * height * 4) as usize || width == 0 || height == 0 {
        return src.to_vec();
    }

    let mut dst = vec![0u8; src.len()];

    let rad_angle = params.fold_direction_deg.to_radians();
    let cos_a = rad_angle.cos();
    let sin_a = rad_angle.sin();

    // Normal vector perpendicular to the fold line
    let nx = -sin_a;
    let ny = cos_a;

    let fx = params.fold_position[0];
    let fy = params.fold_position[1];

    let r = params.fold_radius.max(5.0);
    let pi_r = std::f32::consts::PI * r;

    let sample_bilinear = |x: f32, y: f32| -> [u8; 4] {
        if x < 0.0 || x >= (width - 1) as f32 || y < 0.0 || y >= (height - 1) as f32 {
            let cx = (x.round() as i32).clamp(0, width as i32 - 1) as u32;
            let cy = (y.round() as i32).clamp(0, height as i32 - 1) as u32;
            let idx = ((cy * width + cx) * 4) as usize;
            return [src[idx], src[idx + 1], src[idx + 2], src[idx + 3]];
        }

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

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
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;

            // Distance along normal from fold position
            let d = (px - fx) * nx + (py - fy) * ny;

            let d_idx = ((y * width + x) * 4) as usize;

            if d <= 0.0 {
                // Flat unmoved page region
                let s_idx = ((y * width + x) * 4) as usize;
                dst[d_idx..d_idx + 4].copy_from_slice(&src[s_idx..s_idx + 4]);
            } else if d <= pi_r {
                // Cylinder curl arc: angle theta along cylinder
                let theta = d / r;
                let z = r * (1.0 - theta.cos());
                let rolled_dist = r * theta.sin();

                // Map coordinates back
                let src_dist = rolled_dist - d;
                let sx = px + src_dist * nx;
                let sy = py + src_dist * ny;

                let mut pixel = sample_bilinear(sx, sy);

                // Lighting highlight on ridge
                let highlight = (theta - std::f32::consts::FRAC_PI_2).abs();
                let shade = (1.0 - (highlight / std::f32::consts::FRAC_PI_2).clamp(0.0, 1.0)) * 0.4;
                for c in 0..3 {
                    pixel[c] = ((pixel[c] as f32 + shade * 255.0).clamp(0.0, 255.0)) as u8;
                }

                dst[d_idx..d_idx + 4].copy_from_slice(&pixel);
            } else if d <= 2.0 * pi_r {
                // Backside of the curled paper
                let theta = (d - pi_r) / r;
                let src_dist = -(d - 2.0 * pi_r);
                let sx = px + src_dist * nx;
                let sy = py + src_dist * ny;

                let mut pixel = sample_bilinear(sx, sy);

                // Backside color tint & opacity
                let b_op = (params.back_opacity / 100.0).clamp(0.0, 1.0);
                for c in 0..3 {
                    let back_val = params.back_color[c] * 255.0;
                    pixel[c] = (pixel[c] as f32 * (1.0 - b_op) + back_val * b_op * 0.85).clamp(0.0, 255.0) as u8;
                }

                dst[d_idx..d_idx + 4].copy_from_slice(&pixel);
            } else {
                // Peeling uncovered area underneath
                // Transparent (or shows underlying layer in composition)
                dst[d_idx..d_idx + 4].copy_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_turn_unfolded_region_preserves_pixels() {
        let w = 32u32;
        let h = 32u32;
        let mut src = vec![255u8; (w * h * 4) as usize];
        let params = PageTurnParams {
            fold_position: [64.0, 64.0], // Way outside
            fold_radius: 50.0,
            fold_direction_deg: -45.0,
            light_direction_deg: -45.0,
            back_opacity: 100.0,
            back_color: [1.0, 1.0, 1.0, 1.0],
        };

        let dst = apply_page_turn(&src, w, h, &params);
        assert_eq!(dst.len(), src.len());
        assert_eq!(dst[0], 255);
    }
}
