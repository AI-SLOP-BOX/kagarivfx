#![allow(dead_code)]
/// Corner Pin options matching After Effects Corner Pin effect.
#[derive(Debug, Clone)]
pub struct CornerPinOptions {
    pub top_left: [f32; 2],
    pub top_right: [f32; 2],
    pub bottom_right: [f32; 2],
    pub bottom_left: [f32; 2],
}

impl CornerPinOptions {
    pub fn default_for_size(width: f32, height: f32) -> Self {
        Self {
            top_left: [0.0, 0.0],
            top_right: [width, 0.0],
            bottom_right: [width, height],
            bottom_left: [0.0, height],
        }
    }
}

/// Solves 3x3 Homography matrix H using Gaussian elimination for 4-point correspondence:
/// maps source rectangle (0,0)-(w,h) to arbitrary target quadrilateral pins.
#[allow(clippy::needless_range_loop)]
fn compute_homography_matrix(src_pts: &[[f32; 2]; 4], dst_pts: &[[f32; 2]; 4]) -> Option<[[f64; 3]; 3]> {
    // Solve system A * h = b for 8 parameters of 3x3 homography matrix (with h33 = 1.0)
    let mut a = [[0.0f64; 8]; 8];
    let mut b = [0.0f64; 8];

    for i in 0..4 {
        let u = src_pts[i][0] as f64;
        let v = src_pts[i][1] as f64;
        let x = dst_pts[i][0] as f64;
        let y = dst_pts[i][1] as f64;

        let row1 = i * 2;
        let row2 = row1 + 1;

        a[row1] = [u, v, 1.0, 0.0, 0.0, 0.0, -u * x, -v * x];
        b[row1] = x;

        a[row2] = [0.0, 0.0, 0.0, u, v, 1.0, -u * y, -v * y];
        b[row2] = y;
    }

    // Gaussian elimination with partial pivoting
    for i in 0..8 {
        let mut max_row = i;
        for k in (i + 1)..8 {
            if a[k][i].abs() > a[max_row][i].abs() {
                max_row = k;
            }
        }
        a.swap(i, max_row);
        b.swap(i, max_row);

        if a[i][i].abs() < 1e-9 {
            return None; // Degenerate quadrilateral
        }

        let pivot = a[i][i];
        for val in &mut a[i][i..8] {
            *val /= pivot;
        }
        b[i] /= pivot;

        for k in 0..8 {
            if k != i {
                let factor = a[k][i];
                for j in i..8 {
                    a[k][j] -= factor * a[i][j];
                }
                b[k] -= factor * b[i];
            }
        }
    }

    Some([
        [b[0], b[1], b[2]],
        [b[3], b[4], b[5]],
        [b[6], b[7], 1.0],
    ])
}

/// Computes inverse 3x3 matrix for perspective backward pixel sampling.
fn invert_3x3_matrix(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-9 {
        return None;
    }

    let invdet = 1.0 / det;
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * invdet,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * invdet,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * invdet,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * invdet,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * invdet,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * invdet,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * invdet,
            (m[0][1] * m[1][0] - m[0][0] * m[2][1]) * invdet,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * invdet,
        ],
    ])
}

/// Applies rigorous 8-DOF Homography perspective warping (Corner Pin) to RGBA buffer.
pub fn apply_corner_pin(
    pixels: &[u8],
    width: u32,
    height: u32,
    options: &CornerPinOptions,
) -> Vec<u8> {
    let num_pixels = (width * height) as usize;
    if pixels.len() != num_pixels * 4 {
        return pixels.to_vec();
    }

    let w_f64 = width as f64;
    let h_f64 = height as f64;

    let src_pts = [
        [0.0, 0.0],
        [w_f64 as f32, 0.0],
        [w_f64 as f32, h_f64 as f32],
        [0.0, h_f64 as f32],
    ];
    let dst_pts = [
        options.top_left,
        options.top_right,
        options.bottom_right,
        options.bottom_left,
    ];

    let h_mat = match compute_homography_matrix(&src_pts, &dst_pts) {
        Some(h) => h,
        None => return pixels.to_vec(),
    };

    let inv_h = match invert_3x3_matrix(&h_mat) {
        Some(ih) => ih,
        None => return pixels.to_vec(),
    };

    let mut out_pixels = vec![0u8; num_pixels * 4];

    for y in 0..height {
        let y_f = y as f64;
        for x in 0..width {
            let x_f = x as f64;

            // Homogeneous backward coordinate transformation (X', Y', Z') = H^-1 * (x, y, 1)
            let z_prime = inv_h[2][0] * x_f + inv_h[2][1] * y_f + inv_h[2][2];
            if z_prime.abs() < 1e-7 {
                continue;
            }

            let u_src = (inv_h[0][0] * x_f + inv_h[0][1] * y_f + inv_h[0][2]) / z_prime;
            let v_src = (inv_h[1][0] * x_f + inv_h[1][1] * y_f + inv_h[1][2]) / z_prime;

            if u_src >= 0.0 && u_src < w_f64 - 1.0 && v_src >= 0.0 && v_src < h_f64 - 1.0 {
                // Bilinear interpolation sampling
                let x0 = u_src.floor() as u32;
                let y0 = v_src.floor() as u32;
                let x1 = x0 + 1;
                let y1 = y0 + 1;

                let tx = (u_src - x0 as f64) as f32;
                let ty = (v_src - y0 as f64) as f32;

                let idx00 = ((y0 * width + x0) * 4) as usize;
                let idx10 = ((y0 * width + x1) * 4) as usize;
                let idx01 = ((y1 * width + x0) * 4) as usize;
                let idx11 = ((y1 * width + x1) * 4) as usize;
                let idx_out = ((y * width + x) * 4) as usize;

                for c in 0..4 {
                    let p00 = pixels[idx00 + c] as f32;
                    let p10 = pixels[idx10 + c] as f32;
                    let p01 = pixels[idx01 + c] as f32;
                    let p11 = pixels[idx11 + c] as f32;

                    let top = p00 + (p10 - p00) * tx;
                    let bottom = p01 + (p11 - p01) * tx;
                    let val = top + (bottom - top) * ty;

                    out_pixels[idx_out + c] = val.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }

    out_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corner_pin_homography_identity() {
        let pixels = vec![255u8; 64]; // 4x4 RGBA
        let options = CornerPinOptions::default_for_size(4.0, 4.0);
        let out = apply_corner_pin(&pixels, 4, 4, &options);
        assert_eq!(out.len(), 64);
        assert_eq!(out[0], 255);
    }
}
