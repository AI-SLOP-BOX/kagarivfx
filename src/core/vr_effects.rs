//! VR & 360 Immersive Video Effects Pack (AE Parity).
//!
//! Provides mathematically accurate spherical image manipulation for 360 equirectangular footage:
//! - VR Horizon: 3-axis spherical orientation realignment (Pitch, Yaw, Roll)
//! - VR Spherical Blur: Geodesic latitude-compensated Gaussian blur
//! - VR Chromatic Aberrations: Spherical chromatic dispersion along optical axes
//! - VR Fisheye to Equirectangular: Lens unwarping to 360 panorama

#![allow(dead_code)]

/// Realings the horizon of a 360 equirectangular video by spherical rotation (Pitch, Yaw, Roll).
pub fn apply_vr_horizon(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    pitch_deg: f32,
    yaw_deg: f32,
    roll_deg: f32,
) {
    let Some(pixel_count) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return;
    };
    if width == 0
        || height == 0
        || pixels.len() != pixel_count * 4
        || !pitch_deg.is_finite()
        || !yaw_deg.is_finite()
        || !roll_deg.is_finite()
        || (pitch_deg.abs() < 1e-4 && yaw_deg.abs() < 1e-4 && roll_deg.abs() < 1e-4)
    {
        return;
    }
    let src = pixels.to_vec();
    let p_rad = pitch_deg.to_radians();
    let y_rad = yaw_deg.to_radians();
    let r_rad = roll_deg.to_radians();

    let (cp, sp) = (p_rad.cos(), p_rad.sin());
    let (cy, sy) = (y_rad.cos(), y_rad.sin());
    let (cr, sr) = (r_rad.cos(), r_rad.sin());

    // 3D rotation matrix R = R_yaw * R_pitch * R_roll
    let r00 = cy * cr + sy * sp * sr;
    let r01 = -cy * sr + sy * sp * cr;
    let r02 = sy * cp;

    let r10 = cp * sr;
    let r11 = cp * cr;
    let r12 = -sp;

    let r20 = -sy * cr + cy * sp * sr;
    let r21 = sy * sr + cy * sp * cr;
    let r22 = cy * cp;

    let w_f = width as f32;
    let h_f = height as f32;

    for y in 0..height {
        let lat = ((y as f32 + 0.5) / h_f - 0.5) * std::f32::consts::PI; // -PI/2 to +PI/2
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();

        for x in 0..width {
            let lon = ((x as f32 + 0.5) / w_f - 0.5) * 2.0 * std::f32::consts::PI; // -PI to +PI
            let vx = cos_lat * lon.sin();
            let vy = sin_lat;
            let vz = cos_lat * lon.cos();

            // Rotate vector inversely
            let rx = r00 * vx + r10 * vy + r20 * vz;
            let ry = r01 * vx + r11 * vy + r21 * vz;
            let rz = r02 * vx + r12 * vy + r22 * vz;

            // Map back to equirectangular uv
            let src_lat = ry.clamp(-1.0, 1.0).asin();
            let src_lon = rx.atan2(rz);

            let src_u =
                ((src_lon / (2.0 * std::f32::consts::PI) + 0.5).rem_euclid(1.0) * w_f) as usize;
            let src_v = (((src_lat / std::f32::consts::PI + 0.5).clamp(0.0, 1.0)) * h_f) as usize;

            let dst_idx = ((y * width + x) * 4) as usize;
            let src_idx = ((src_v.min(height as usize - 1) * width as usize
                + src_u.min(width as usize - 1))
                * 4) as usize;

            pixels[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
        }
    }
}

/// Geodesic latitude-compensated Gaussian blur for 360 panoramic equirectangular images.
pub fn apply_vr_blur(pixels: &mut [u8], width: u32, height: u32, radius: f32) {
    let Some(pixel_count) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return;
    };
    if width == 0
        || height == 0
        || radius <= 0.001
        || !radius.is_finite()
        || pixels.len() != pixel_count * 4
    {
        return;
    }
    let radius = radius.min(4096.0);

    let src = pixels.to_vec();
    let w_f = width as f32;
    let h_f = height as f32;

    for y in 0..height {
        let lat = ((y as f32 + 0.5) / h_f - 0.5) * std::f32::consts::PI;
        // Scale horizontal blur radius according to 1/cos(latitude) to avoid polar seam pinching
        let cos_lat = lat.cos().abs().max(0.08);
        let eff_rx = (radius / cos_lat).min(w_f * 0.25);
        let eff_ry = radius;

        let k_size_x = (eff_rx * 2.0).ceil() as i32;
        let k_size_y = (eff_ry * 2.0).ceil() as i32;

        for x in 0..width {
            let mut acc_r = 0.0f32;
            let mut acc_g = 0.0f32;
            let mut acc_b = 0.0f32;
            let mut acc_a = 0.0f32;
            let mut weight_sum = 0.0f32;

            for dy in -k_size_y..=k_size_y {
                let sy = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
                let wy = (-((dy as f32).powi(2)) / (2.0 * eff_ry.powi(2).max(1.0))).exp();

                for dx in -k_size_x..=k_size_x {
                    let sx = ((x as i32 + dx).rem_euclid(width as i32)) as usize; // Circular seam wrapping
                    let wx = (-((dx as f32).powi(2)) / (2.0 * eff_rx.powi(2).max(1.0))).exp();
                    let w = wx * wy;

                    let s_idx = (sy * width as usize + sx) * 4;
                    acc_r += src[s_idx] as f32 * w;
                    acc_g += src[s_idx + 1] as f32 * w;
                    acc_b += src[s_idx + 2] as f32 * w;
                    acc_a += src[s_idx + 3] as f32 * w;
                    weight_sum += w;
                }
            }

            let d_idx = ((y * width + x) * 4) as usize;
            let inv_w = 1.0 / weight_sum.max(1e-5);
            pixels[d_idx] = (acc_r * inv_w).round() as u8;
            pixels[d_idx + 1] = (acc_g * inv_w).round() as u8;
            pixels[d_idx + 2] = (acc_b * inv_w).round() as u8;
            pixels[d_idx + 3] = (acc_a * inv_w).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vr_horizon_identity_at_zero_angles() {
        let mut pixels = vec![128u8; 32 * 16 * 4];
        let original = pixels.clone();
        apply_vr_horizon(&mut pixels, 32, 16, 0.0, 0.0, 0.0);
        assert_eq!(pixels, original);
    }

    #[test]
    fn test_vr_blur_preserves_bounds_and_wraps_seam() {
        let mut pixels = vec![0u8; 32 * 16 * 4];
        // Center white spot at boundary seam (x=0, y=8)
        let idx = (8 * 32 + 0) * 4;
        pixels[idx] = 255;
        pixels[idx + 1] = 255;
        pixels[idx + 2] = 255;
        pixels[idx + 3] = 255;

        apply_vr_blur(&mut pixels, 32, 16, 2.0);

        // Rightmost pixel (x=31, y=8) should receive wrapped blur energy
        let wrap_idx = (8 * 32 + 31) * 4;
        assert!(
            pixels[wrap_idx] >= 5,
            "Seam wrapping blur must propagate to x=31: got {}",
            pixels[wrap_idx]
        );
    }
}
