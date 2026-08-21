#![allow(dead_code)]
/// 3D Camera Depth of Field (DoF) and Circle of Confusion (CoF) parameters.
#[derive(Debug, Clone)]
pub struct CameraDofSettings {
    pub focus_distance: f32, // Distance to sharp focal plane
    pub aperture: f32,       // Lens aperture / iris size
    pub f_stop: f32,         // f-stop ratio
    pub blur_level: f32,     // Global DoF blur multiplier percentage (100.0 = standard)
}

impl Default for CameraDofSettings {
    fn default() -> Self {
        Self {
            focus_distance: 1000.0,
            aperture: 50.0,
            f_stop: 2.8,
            blur_level: 100.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_of_confusion_at_focal_plane() {
        let settings = CameraDofSettings { focus_distance: 500.0, aperture: 50.0, f_stop: 2.8, blur_level: 100.0 };
        let blur = calculate_circle_of_confusion(500.0, &settings);
        assert_eq!(blur, 0.0);
    }

    #[test]
    fn test_circle_of_confusion_out_of_focus() {
        let settings = CameraDofSettings { focus_distance: 500.0, aperture: 50.0, f_stop: 2.8, blur_level: 100.0 };
        let blur = calculate_circle_of_confusion(1000.0, &settings);
        assert!(blur > 0.0);
    }
}
