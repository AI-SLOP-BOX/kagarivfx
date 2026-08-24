#![allow(dead_code)]
use crate::core::timeline::Layer;

/// Auto-Orient mode matching After Effects layer orientation settings.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub enum AutoOrientMode {
    #[default]
    Off,
    /// Rotation follows the direction of the position motion path.
    OrientAlongPath,
    /// Rotation points at a fixed composition coordinate.
    OrientTowardsPoint { target_point: [f32; 2] },
}


/// Evaluates automatic rotation angle (in degrees) for a layer along its motion path.
pub fn evaluate_auto_orient_rotation(
    layer: &Layer,
    frame: u32,
    mode: AutoOrientMode,
) -> Option<f32> {
    match mode {
        AutoOrientMode::Off => None,
        AutoOrientMode::OrientAlongPath => {
            let p_curr = layer.transform.position.evaluate(frame);

            // Forward difference sample (delta = +1 frame)
            let p_next = layer.transform.position.evaluate(frame + 1);
            let dx = p_next[0] - p_curr[0];
            let dy = p_next[1] - p_curr[1];

            if dx.abs() < 0.001 && dy.abs() < 0.001 {
                // If stationary at current frame, try backward difference sample
                if frame > 0 {
                    let p_prev = layer.transform.position.evaluate(frame - 1);
                    let dx_prev = p_curr[0] - p_prev[0];
                    let dy_prev = p_curr[1] - p_prev[1];
                    if dx_prev.abs() > 0.001 || dy_prev.abs() > 0.001 {
                        return Some(dy_prev.atan2(dx_prev).to_degrees());
                    }
                }
                None
            } else {
                Some(dy.atan2(dx).to_degrees())
            }
        }
        AutoOrientMode::OrientTowardsPoint { target_point } => {
            let p_curr = layer.transform.position.evaluate(frame);
            let dx = target_point[0] - p_curr[0];
            let dy = target_point[1] - p_curr[1];
            if dx.abs() < 0.001 && dy.abs() < 0.001 {
                None
            } else {
                Some(dy.atan2(dx).to_degrees())
            }
        }
    }
}

/// Evaluates exponential scale interpolation: visually uniform zoom without linear acceleration perception.
pub fn evaluate_exponential_scale(start_scale: [f32; 2], end_scale: [f32; 2], progress: f32) -> [f32; 2] {
    let t = progress.clamp(0.0, 1.0);
    [
        start_scale[0] * (end_scale[0] / start_scale[0].max(0.0001)).powf(t),
        start_scale[1] * (end_scale[1] / start_scale[1].max(0.0001)).powf(t),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::Transform2D;
    use crate::core::keyframe::InterpolationType;

    #[test]
    fn test_orient_along_path_horizontal() {
        let mut layer = Layer::new("layer_1".to_string(), "Test".to_string(), crate::core::timeline::LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] }, 100);
        layer.transform = Transform2D::default();
        // Moving right along X axis
        layer.transform.position = Animatable::new_animated(vec![
            crate::core::keyframe::Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
            crate::core::keyframe::Keyframe::new(10, [100.0, 0.0], InterpolationType::Linear),
        ]);

        let rot = evaluate_auto_orient_rotation(&layer, 5, AutoOrientMode::OrientAlongPath);
        assert!(rot.is_some());
        assert!((rot.unwrap() - 0.0).abs() < 0.1); // 0 degrees (pointing right)
    }

    #[test]
    fn test_exponential_scale_midpoint() {
        let start = [10.0, 10.0];
        let end = [1000.0, 1000.0];
        let mid = evaluate_exponential_scale(start, end, 0.5);

        // Geometric mean sqrt(10 * 1000) = 100.0
        assert!((mid[0] - 100.0).abs() < 0.1);
    }
}
