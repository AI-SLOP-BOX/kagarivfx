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
}

impl Default for CameraDofSettings {
    fn default() -> Self {
        Self {
            focus_distance: 1000.0,
            aperture: 50.0,
            f_stop: 2.8,
            blur_level: 100.0,
            iris_sides: 0,
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

/// Generate bokeh sample offsets for a given iris shape.
/// Returns [(x, y, weight)] sample points within the CoF radius.
pub fn generate_bokeh_samples(iris_sides: u32, cof_radius: f32, num_samples: u32) -> Vec<(f32, f32, f32)> {
    if cof_radius < 0.5 || num_samples == 0 {
        return vec![(0.0, 0.0, 1.0)];
    }

    let mut samples = Vec::with_capacity(num_samples as usize);

    if iris_sides == 0 || iris_sides == 1 {
        // Circle: distribute samples in concentric rings
        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            let angle = t * std::f32::consts::TAU;
            let r = (t * 0.7 + 0.3) * cof_radius; // bias towards edge for nicer bokeh
            let x = angle.cos() * r;
            let y = angle.sin() * r;
            let weight = 1.0 - (r / cof_radius) * 0.3; // slight center bias
            samples.push((x, y, weight));
        }
    } else {
        // Polygon: distribute samples at vertices and edges
        let n = iris_sides as f32;
        let polygon_r = cof_radius * 0.8;
        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            let side = (t * n).floor() as u32 % iris_sides;
            let edge_t = (t * n).fract();

            let a1 = (side as f32 / n) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let a2 = ((side + 1) as f32 / n) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;

            let x1 = a1.cos() * polygon_r;
            let y1 = a1.sin() * polygon_r;
            let x2 = a2.cos() * polygon_r;
            let y2 = a2.sin() * polygon_r;

            let x = x1 + (x2 - x1) * edge_t;
            let y = y1 + (y2 - y1) * edge_t;
            let weight = 1.0;
            samples.push((x, y, weight));
        }
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_of_confusion_at_focal_plane() {
        let settings = CameraDofSettings { focus_distance: 500.0, aperture: 50.0, f_stop: 2.8, blur_level: 100.0, iris_sides: 0 };
        let blur = calculate_circle_of_confusion(500.0, &settings);
        assert_eq!(blur, 0.0);
    }

    #[test]
    fn test_circle_of_confusion_out_of_focus() {
        let settings = CameraDofSettings { focus_distance: 500.0, aperture: 50.0, f_stop: 2.8, blur_level: 100.0, iris_sides: 0 };
        let blur = calculate_circle_of_confusion(1000.0, &settings);
        assert!(blur > 0.0);
    }

    #[test]
    fn test_bokeh_circle_samples() {
        let samples = generate_bokeh_samples(0, 10.0, 8);
        assert_eq!(samples.len(), 8);
        for (x, y, w) in &samples {
            let r = (x * x + y * y).sqrt();
            assert!(r <= 10.0, "sample outside CoF radius: {}", r);
            assert!(*w > 0.0);
        }
    }

    #[test]
    fn test_bokeh_hexagon_samples() {
        let samples = generate_bokeh_samples(6, 10.0, 12);
        assert_eq!(samples.len(), 12);
    }
}
