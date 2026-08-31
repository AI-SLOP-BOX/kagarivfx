#![allow(dead_code)]
/// 3D Camera Depth of Field (DoF) and Circle of Confusion (CoF) parameters.
#[derive(Debug, Clone)]
pub struct CameraDofSettings {
    pub focus_distance: f32, // Distance to sharp focal plane
    pub aperture: f32,       // Lens aperture / iris size
    pub f_stop: f32,         // f-stop ratio
    pub blur_level: f32,     // Global DoF blur multiplier percentage (100.0 = standard)
    /// Iris shape: 0=circle, 3=triangle, 5=pentagon, 6=hexagon, 8=octagon
    pub iris_sides: u32,
    /// Anamorphic ratio / squeeze factor (1.0 = spherical, 2.0 = standard 2x anamorphic lens)
    pub anamorphic_ratio: f32,
    /// Optical vignetting / Cat's Eye bokeh intensity towards frame corners (0.0..1.0)
    pub optical_vignetting: f32,
}

impl Default for CameraDofSettings {
    fn default() -> Self {
        Self {
            focus_distance: 1000.0,
            aperture: 50.0,
            f_stop: 2.8,
            blur_level: 100.0,
            iris_sides: 0,
            anamorphic_ratio: 1.0,
            optical_vignetting: 0.0,
        }
    }
}

/// Evaluates Circle of Confusion (CoF) blur radius for a 3D layer based on Z-depth distance.
pub fn calculate_circle_of_confusion(layer_z: f32, settings: &CameraDofSettings) -> f32 {
    let focus_dist = settings.focus_distance.max(1.0);
    let delta_z = (layer_z - focus_dist).abs();

    if delta_z < 0.1 {
        return 0.0;
    }

    // CoF radius = aperture * |Z - focus_dist| / Z
    let raw_cof = (settings.aperture * delta_z) / layer_z.abs().max(1.0);
    (raw_cof * (settings.blur_level / 100.0)).clamp(0.0, 150.0)
}

/// Generate bokeh sample offsets for a given iris shape with optional anamorphic squeeze.
/// Returns [(x, y, weight)] sample points within the CoF radius.
pub fn generate_bokeh_samples(
    iris_sides: u32,
    cof_radius: f32,
    num_samples: u32,
    anamorphic_ratio: f32,
) -> Vec<(f32, f32, f32)> {
    if cof_radius < 0.5 || num_samples == 0 {
        return vec![(0.0, 0.0, 1.0)];
    }

    let squeeze = anamorphic_ratio.clamp(0.2, 5.0);
    let mut samples = Vec::with_capacity(num_samples as usize);

    if iris_sides == 0 || iris_sides == 1 {
        // Circle / Anamorphic Ellipse: distribute samples in concentric rings
        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            let angle = t * std::f32::consts::TAU;
            let r = (t * 0.7 + 0.3) * cof_radius;
            let x = angle.cos() * r / squeeze; // Horizontal squeeze for vertical ellipse
            let y = angle.sin() * r;
            let weight = 1.0 - (r / cof_radius) * 0.3;
            samples.push((x, y, weight));
        }
    } else {
        // Polygon: distribute samples across filled concentric polygon rings
        let n = iris_sides as f32;
        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            let side = (t * n).floor() as u32 % iris_sides;
            let edge_t = (t * n).fract();
            let radial_scale = (t * 0.8 + 0.2) * cof_radius;

            let a1 = (side as f32 / n) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let a2 = ((side + 1) as f32 / n) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;

            let x1 = a1.cos() * radial_scale / squeeze;
            let y1 = a1.sin() * radial_scale;
            let x2 = a2.cos() * radial_scale / squeeze;
            let y2 = a2.sin() * radial_scale;

            let x = x1 + (x2 - x1) * edge_t;
            let y = y1 + (y2 - y1) * edge_t;
            let weight = 1.0;
            samples.push((x, y, weight));
        }
    }

    samples
}

/// Applies optical camera depth of field bokeh blur to an RGBA pixel buffer based on layer Z distance.
pub fn apply_camera_dof_bokeh_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    layer_z: f32,
    settings: &CameraDofSettings,
) {
    let cof = calculate_circle_of_confusion(layer_z, settings);
    if cof < 0.5 || width == 0 || height == 0 {
        return;
    }

    let num_samples = (cof * 2.0).clamp(8.0, 32.0) as u32;
    let samples = generate_bokeh_samples(
        settings.iris_sides,
        cof,
        num_samples,
        settings.anamorphic_ratio,
    );
    let src = pixels.to_vec();
    let w = width as i32;
    let h = height as i32;

    let sample_pixel = |x: i32, y: i32| -> [f32; 4] {
        let cx = x.clamp(0, w - 1) as usize;
        let cy = y.clamp(0, h - 1) as usize;
        let idx = (cy * width as usize + cx) * 4;
        [
            src[idx] as f32,
            src[idx + 1] as f32,
            src[idx + 2] as f32,
            src[idx + 3] as f32,
        ]
    };

    let total_weight: f32 = samples.iter().map(|s| s.2).sum();
    if total_weight <= f32::EPSILON {
        return;
    }

    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for &(sx, sy, sw) in &samples {
                let px = (x as f32 + sx).round() as i32;
                let py = (y as f32 + sy).round() as i32;
                let p = sample_pixel(px, py);
                acc[0] += p[0] * sw;
                acc[1] += p[1] * sw;
                acc[2] += p[2] * sw;
                acc[3] += p[3] * sw;
            }

            let idx = (y as usize * width as usize + x as usize) * 4;
            pixels[idx] = (acc[0] / total_weight).round().clamp(0.0, 255.0) as u8;
            pixels[idx + 1] = (acc[1] / total_weight).round().clamp(0.0, 255.0) as u8;
            pixels[idx + 2] = (acc[2] / total_weight).round().clamp(0.0, 255.0) as u8;
            pixels[idx + 3] = (acc[3] / total_weight).round().clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_of_confusion_at_focal_plane() {
        let settings = CameraDofSettings {
            focus_distance: 500.0,
            aperture: 50.0,
            f_stop: 2.8,
            blur_level: 100.0,
            iris_sides: 0,
            anamorphic_ratio: 1.0,
            optical_vignetting: 0.0,
        };
        let blur = calculate_circle_of_confusion(500.0, &settings);
        assert_eq!(blur, 0.0);
    }

    #[test]
    fn test_circle_of_confusion_out_of_focus() {
        let settings = CameraDofSettings {
            focus_distance: 500.0,
            aperture: 50.0,
            f_stop: 2.8,
            blur_level: 100.0,
            iris_sides: 0,
            anamorphic_ratio: 1.0,
            optical_vignetting: 0.0,
        };
        let blur = calculate_circle_of_confusion(1000.0, &settings);
        assert!(blur > 0.0);
    }

    #[test]
    fn test_bokeh_circle_samples() {
        let samples = generate_bokeh_samples(0, 10.0, 8, 1.0);
        assert_eq!(samples.len(), 8);
        for (x, y, w) in &samples {
            let r = (x * x + y * y).sqrt();
            assert!(r <= 10.0, "sample outside CoF radius: {}", r);
            assert!(*w > 0.0);
        }
    }

    #[test]
    fn test_bokeh_anamorphic_ellipse_samples() {
        let samples = generate_bokeh_samples(0, 10.0, 8, 2.0); // 2x squeeze
        assert_eq!(samples.len(), 8);
        for (x, _y, _w) in &samples {
            // Horizontal coordinate should be squeezed by half
            assert!(x.abs() <= 5.05);
        }
    }

    #[test]
    fn test_bokeh_hexagon_samples() {
        let samples = generate_bokeh_samples(6, 10.0, 12, 1.0);
        assert_eq!(samples.len(), 12);
    }

    #[test]
    fn test_apply_camera_dof_bokeh_blur() {
        let settings = CameraDofSettings {
            focus_distance: 100.0,
            aperture: 5.0,
            f_stop: 2.8,
            blur_level: 100.0,
            iris_sides: 0,
            anamorphic_ratio: 1.0,
            optical_vignetting: 0.0,
        };
        let mut pixels = vec![0u8; 20 * 20 * 4];
        let center_idx = (10 * 20 + 10) * 4;
        pixels[center_idx] = 255;
        pixels[center_idx + 1] = 255;
        pixels[center_idx + 2] = 255;
        pixels[center_idx + 3] = 255;

        apply_camera_dof_bokeh_blur(&mut pixels, 20, 20, 200.0, &settings);

        let non_zero_count = pixels.iter().filter(|&&p| p > 0).count();
        assert!(non_zero_count > 0);
    }
}
