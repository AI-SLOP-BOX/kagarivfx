use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BezierControlPoint {
    /// Time influence, typically between 0.0 and 1.0 (X axis in AE velocity graph)
    pub influence: f32,
    /// Value speed, change per second (Y axis in AE velocity graph)
    pub speed: f32,
}

impl Default for BezierControlPoint {
    fn default() -> Self {
        Self {
            influence: 0.333,
            speed: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InterpolationType {
    Linear,
    Hold,
    Bezier {
        /// Control point outgoing from the current keyframe (influence/speed)
        outgoing: BezierControlPoint,
        /// Control point incoming to the next keyframe (influence/speed)
        incoming: BezierControlPoint,
        /// Easing parameters mapped to CSS-like control points (x1, y1, x2, y2)
        /// if computed, otherwise default to a standard ease.
        #[serde(default)]
        custom_bezier: Option<[f32; 4]>,
    },
}

impl Default for InterpolationType {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub frame: u32,
    pub value: T,
    pub interpolation: InterpolationType,
}

impl<T> Keyframe<T> {
    pub fn new(frame: u32, value: T, interpolation: InterpolationType) -> Self {
        Self {
            frame,
            value,
            interpolation,
        }
    }
}

/// Evaluates a cubic bezier curve at x using Newton-Raphson or binary search.
/// Control points are (0,0), (x1, y1), (x2, y2), (1,1).
pub fn solve_bezier_eased_time(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Coefficients of the cubic bezier equation for X(t) = x
    // X(t) = 3*(1-t)^2*t*x1 + 3*(1-t)*t^2*x2 + t^3
    //      = t^3 * (1 - 3*x2 + 3*x1) + t^2 * (3*x2 - 6*x1) + t * (3*x1)
    let c = 3.0 * x1;
    let b = 3.0 * (x2 - x1) - c;
    let a = 1.0 - 3.0 * x2 + c;

    // Helper functions
    let sample_curve_x = |t: f32| ((a * t + b) * t + c) * t;
    let sample_curve_derivative_x = |t: f32| (3.0 * a * t + 2.0 * b) * t + c;

    // Newton-Raphson iteration
    let mut t_guess = x;
    for _ in 0..8 {
        let x_guess = sample_curve_x(t_guess) - x;
        if x_guess.abs() < 1e-6 {
            break;
        }
        let d = sample_curve_derivative_x(t_guess);
        if d.abs() < 1e-6 {
            break;
        }
        // Clamp to [0, 1] to prevent divergence on extreme easing curves (influence ~100%)
        t_guess = (t_guess - x_guess / d).clamp(0.0, 1.0);
    }

    // Fallback to binary search if Newton-Raphson did not converge
    if (sample_curve_x(t_guess) - x).abs() > 1e-4 {
        let mut t_lower = 0.0;
        let mut t_upper = 1.0;
        t_guess = x;
        for _ in 0..16 {
            let x_guess = sample_curve_x(t_guess);
            if (x_guess - x).abs() < 1e-5 {
                break;
            }
            if x < x_guess {
                t_upper = t_guess;
            } else {
                t_lower = t_guess;
            }
            t_guess = (t_lower + t_upper) * 0.5;
        }
    }

    // Compute Y(t)
    // Y(t) = 3*(1-t)^2*t*y1 + 3*(1-t)*t^2*y2 + t^3
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - 3.0 * y2 + cy;

    ((ay * t_guess + by) * t_guess + cy) * t_guess
}

/// Compute cubic Bezier control points (x1, y1, x2, y2) from AE keyframe speed and influence parameters.
#[allow(dead_code)]
pub fn compute_ae_bezier_control_points(
    outgoing: &BezierControlPoint,
    incoming: &BezierControlPoint,
    delta_frame: f32,
    delta_val: f32,
    fps: f32,
) -> [f32; 4] {
    let dt = (delta_frame / fps.max(1.0)).max(0.001);
    let x1 = outgoing.influence.clamp(0.0, 1.0);
    let x2 = 1.0 - incoming.influence.clamp(0.0, 1.0);

    let average_speed = delta_val / dt;
    let y1 = if average_speed.abs() > 1e-6 {
        (outgoing.speed * dt * x1) / delta_val
    } else {
        0.0
    };
    let y2 = if average_speed.abs() > 1e-6 {
        1.0 - (incoming.speed * dt * (1.0 - x2)) / delta_val
    } else {
        1.0
    };

    [x1, y1.clamp(-2.0, 3.0), x2, y2.clamp(-2.0, 3.0)]
}
