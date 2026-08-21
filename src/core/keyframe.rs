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
#[derive(Default)]
pub enum InterpolationType {
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EasePreset {
    Standard,           // Easy Ease [0.25, 0.1, 0.25, 1.0]
    FastIn,           // Acceleration [0.42, 0.0, 1.0, 1.0]
    SmoothOut,        // Deceleration [0.0, 0.0, 0.58, 1.0]
    Overshoot,        // Bounce Spring [0.68, -0.55, 0.265, 1.55]
    Sine,             // Ultra Smooth Sine [0.37, 0.0, 0.63, 1.0]
    FastOut,          // Fast Deceleration [0.0, 0.0, 0.35, 1.0]
    SlowIn,           // Slow Acceleration [0.43, 0.0, 0.9, 1.0]
    CustomEase,       // Custom bezier defined via custom_bezier
    MirrorEase,       // Mirror the first half [0.5, 0, 1, 1]
    // Legacy AE presets (backward compat)
    EaseIn,           // Quadratic ease-in [0.5, 0, 1, 1]
    EaseOut,        // Quadratic ease-out [0, 0, 0.58, 1]
    // New presets
    Elastic,          // Elastic bounce [0.5, 0, 0.8, 0.3]
    Bounce,           // Single bounce [0.5, 0.3, 0.7, 0.1]
    Cycle,            // Cycling [0.5, 0.5, 0.5, 0.5]
    MirrorEase2,      // Mirror back and forth [0.5, 0, 1, 1]
    Custom0,          // [0.0, 0, 1, 1] - default
    Custom1,          // [0.25, 0.1, 0.25, 1] - slightly eased
    Custom2,          // [0.42, 0, 1, 1] - fast start
    Custom3,          // [0, 0, 0.58, 1] - slow start
}

impl EasePreset {
    pub fn control_points(self) -> [f32; 4] {
        match self {
            EasePreset::Standard => [0.25, 0.1, 0.25, 1.0],
            EasePreset::FastIn => [0.42, 0.0, 1.0, 1.0],
            EasePreset::SmoothOut => [0.0, 0.0, 0.58, 1.0],
            EasePreset::Overshoot => [0.68, -0.4, 0.265, 1.4],
            EasePreset::Sine => [0.37, 0.0, 0.63, 1.0],
            EasePreset::EaseIn => [0.5, 0.0, 1.0, 1.0],
            EasePreset::EaseOut => [0.0, 0.0, 0.58, 1.0],
            EasePreset::FastOut => [0.0, 0.0, 0.35, 1.0],
            EasePreset::SlowIn => [0.43, 0.0, 0.9, 1.0],
            EasePreset::CustomEase => [0.5, 0.0, 0.5, 1.0],
            EasePreset::MirrorEase => [0.5, 0.0, 1.0, 1.0],
            EasePreset::Elastic => [0.5, 0.2, 0.6, 0.8],
            EasePreset::Bounce => [0.5, 0.3, 0.7, 0.1],
            EasePreset::Cycle => [0.5, 0.5, 0.5, 0.5],
            EasePreset::MirrorEase2 => [0.5, 0.0, 1.0, 1.0],
            //
            EasePreset::Custom0 => [0.0, 0.0, 1.0, 1.0],
            EasePreset::Custom1 => [0.25, 0.1, 0.25, 1.0],
            EasePreset::Custom2 => [0.42, 0.0, 1.0, 1.0],
            EasePreset::Custom3 => [0.0, 0.0, 0.58, 1.0],
        }
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

    // Newton-Raphson iteration (12 steps, 1e-7 epsilon tolerance for sub-pixel accuracy)
    let mut t_guess = x;
    for _ in 0..12 {
        let x_guess = sample_curve_x(t_guess) - x;
        if x_guess.abs() < 1e-7 {
            break;
        }
        let d = sample_curve_derivative_x(t_guess);
        if d.abs() < 1e-7 {
            break;
        }
        // Clamp to [0, 1] to prevent divergence on extreme easing curves (influence ~100%)
        t_guess = (t_guess - x_guess / d).clamp(0.0, 1.0);
    }

    // Fallback to high-precision binary search (24 steps) if Newton-Raphson did not converge
    if (sample_curve_x(t_guess) - x).abs() > 1e-6 {
        let mut t_lower = 0.0;
        let mut t_upper = 1.0;
        t_guess = x;
        for _ in 0..24 {
            let x_guess = sample_curve_x(t_guess);
            if (x_guess - x).abs() < 1e-6 {
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

    let y1 = if delta_val.abs() > 1e-6 {
        (outgoing.speed * dt * x1) / delta_val
    } else {
        0.0
    };
    let y2 = if delta_val.abs() > 1e-6 {
        1.0 - (incoming.speed * dt * (1.0 - x2)) / delta_val
    } else {
        1.0
    };

    [x1, y1.clamp(-2.0, 3.0), x2, y2.clamp(-2.0, 3.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_bezier_eased_time_precision() {
        let t0 = solve_bezier_eased_time(0.0, 0.25, 0.1, 0.25, 1.0);
        let t1 = solve_bezier_eased_time(1.0, 0.25, 0.1, 0.25, 1.0);
        let t_mid = solve_bezier_eased_time(0.5, 0.25, 0.1, 0.25, 1.0);

        assert!((t0 - 0.0).abs() < 1e-5);
        assert!((t1 - 1.0).abs() < 1e-5);
        assert!(t_mid > 0.0 && t_mid < 1.0);
    }

    #[test]
    fn test_compute_ae_bezier_control_points() {
        let outgoing = BezierControlPoint { influence: 0.33, speed: 100.0 };
        let incoming = BezierControlPoint { influence: 0.33, speed: 0.0 };
        let pts = compute_ae_bezier_control_points(&outgoing, &incoming, 30.0, 100.0, 30.0);
        assert!(pts[0] >= 0.0 && pts[0] <= 1.0);
        assert!(pts[2] >= 0.0 && pts[2] <= 1.0);
    }

    #[test]
    fn test_ease_preset_control_points() {
        let std_pts = EasePreset::Standard.control_points();
        assert_eq!(std_pts, [0.25, 0.1, 0.25, 1.0]);

        let fast_pts = EasePreset::FastIn.control_points();
        assert_eq!(fast_pts, [0.42, 0.0, 1.0, 1.0]);

        // Evaluate solve_bezier_eased_time for Overshoot preset
        let over_pts = EasePreset::Overshoot.control_points();
        let eased = solve_bezier_eased_time(0.5, over_pts[0], over_pts[1], over_pts[2], over_pts[3]);
        assert!(!eased.is_nan());
    }

    #[test]
    fn test_bezier_mirror_math() {
        let original: [f32; 4] = [0.25, 0.1, 0.75, 0.9];
        let mirrored = [1.0 - original[2], 1.0 - original[3], 1.0 - original[0], 1.0 - original[1]];
        assert!((mirrored[0] - 0.25).abs() < 1e-5);
        assert!((mirrored[1] - 0.1).abs() < 1e-5);
        assert!((mirrored[2] - 0.75).abs() < 1e-5);
        assert!((mirrored[3] - 0.9).abs() < 1e-5);

        let fast_in: [f32; 4] = [0.42, 0.0, 1.0, 1.0];
        let mirrored_fast_in = [1.0 - fast_in[2], 1.0 - fast_in[3], 1.0 - fast_in[0], 1.0 - fast_in[1]];
        assert!((mirrored_fast_in[0] - 0.0).abs() < 1e-5);
        assert!((mirrored_fast_in[1] - 0.0).abs() < 1e-5);
        assert!((mirrored_fast_in[2] - 0.58).abs() < 1e-5);
        assert!((mirrored_fast_in[3] - 1.0).abs() < 1e-5);
    }

}

// ── Bezier Keyframe Interpolation ──────────────────────────────────────

/// Bezier tangent handle for easing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BezierHandle {
    /// Incoming tangent X (time offset, normalized 0..1)
    pub in_x: f32,
    /// Incoming tangent Y (value offset, normalized 0..1)
    pub in_y: f32,
    /// Outgoing tangent X (time offset, normalized 0..1)
    pub out_x: f32,
    /// Outgoing tangent Y (value offset, normalized 0..1)
    pub out_y: f32,
}

impl Default for BezierHandle {
    fn default() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 1.0,
        }
    }
}

impl BezierHandle {
    /// Linear interpolation (no easing)
    pub fn linear() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 1.0,
        }
    }
    /// Ease in (slow start)
    pub fn ease_in() -> Self {
        Self {
            in_x: 0.42,
            in_y: 0.0,
            out_x: 1.0,
            out_y: 1.0,
        }
    }
    /// Ease out (slow end)
    pub fn ease_out() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 0.58,
            out_y: 1.0,
        }
    }
    /// Ease in-out
    pub fn ease_in_out() -> Self {
        Self {
            in_x: 0.42,
            in_y: 0.0,
            out_x: 0.58,
            out_y: 1.0,
        }
    }
    /// Ease out back (overshoot)
    pub fn ease_out_back() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 0.34,
            out_y: 1.3,
        }
    }
    /// Bounce ease (approximation via cubic bezier)
    pub fn bounce() -> Self {
        Self {
            in_x: 0.0,
            in_y: 0.0,
            out_x: 0.34,
            out_y: 1.2,
        }
    }
}

/// Evaluate cubic bezier at parameter t ∈ [0,1]
pub fn cubic_bezier(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    mt3 * p0 + 3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3 * p3
}

/// Interpolate between two values using bezier handles
pub fn interpolate_bezier(
    from_val: f32,
    to_val: f32,
    from_time: f32,
    to_time: f32,
    current_time: f32,
    out_handle: &BezierHandle,
    in_handle: &BezierHandle,
) -> f32 {
    // Normalize time to [0,1]
    let t = if (to_time - from_time).abs() < 1e-6 {
        0.0
    } else {
        ((current_time - from_time) / (to_time - from_time)).clamp(0.0, 1.0)
    };

    // Use the outgoing handle of the start key and incoming handle of the end key
    // Map bezier control points: P0=(0,0), P1=(out_handle.out_x, out_handle.out_y),
    //                            P2=(in_handle.in_x, in_handle.in_y), P3=(1,1)
    let t_eased = cubic_bezier(0.0, out_handle.out_y, in_handle.in_y, 1.0, t);

    from_val + (to_val - from_val) * t_eased
}

#[cfg(test)]
mod tests_bezier_handle {
    use super::*;

    #[test]
    fn test_bezier_handle_defaults() {
        let h = BezierHandle::default();
        assert!((h.in_x - 0.0).abs() < 1e-6);
        assert!((h.in_y - 0.0).abs() < 1e-6);
        assert!((h.out_x - 1.0).abs() < 1e-6);
        assert!((h.out_y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cubic_bezier_endpoints() {
        // At t=0 should return p0, at t=1 should return p3
        let v0 = cubic_bezier(0.0, 0.5, 0.5, 1.0, 0.0);
        let v1 = cubic_bezier(0.0, 0.5, 0.5, 1.0, 1.0);
        assert!((v0 - 0.0).abs() < 1e-6);
        assert!((v1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_bezier_linear() {
        let lin = BezierHandle::linear();
        let v = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 5.0, &lin, &lin);
        assert!((v - 50.0).abs() < 1e-3);
    }

    #[test]
    fn test_interpolate_bezier_ease_in() {
        let ease_in = BezierHandle { in_x: 0.42, in_y: 0.0, out_x: 1.0, out_y: 0.8 };
        let lin = BezierHandle::linear();
        let v_mid = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 5.0, &ease_in, &lin);
        // With out_y=0.8 the curve is below linear at midpoint → value < 50
        assert!(v_mid < 50.0);
        // At endpoints the value should match exactly
        let v_start = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 0.0, &ease_in, &lin);
        let v_end = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 10.0, &ease_in, &lin);
        assert!((v_start - 0.0).abs() < 1e-3);
        assert!((v_end - 100.0).abs() < 1e-3);
    }
}

#[cfg(test)]
mod tests_ease_presets {
    use super::*;

    fn handle(x: f32, y: f32) -> BezierHandle {
        BezierHandle { in_x: x, in_y: y, out_x: x, out_y: y }
    }

    #[test]
    fn test_new_presets_have_valid_control_points() {
        let presets = [
            EasePreset::FastOut,
            EasePreset::SlowIn,
            EasePreset::CustomEase,
            EasePreset::MirrorEase,
            EasePreset::Elastic,
            EasePreset::Bounce,
            EasePreset::Cycle,
            EasePreset::MirrorEase2,
            EasePreset::Custom0,
            EasePreset::Custom1,
            EasePreset::Custom2,
            EasePreset::Custom3,
            EasePreset::EaseIn,
            EasePreset::EaseOut,
        ];
        for p in presets {
            let pts = p.control_points();
            assert!((0.0..=1.0).contains(&pts[0]), "{:?} x1 out of range", p);
            assert!((0.0..=1.0).contains(&pts[2]), "{:?} x2 out of range", p);
            assert!(pts[0] <= pts[2], "{:?} x1 must be <= x2", p);
        }
    }

    #[test]
    fn test_new_presets_interpolate_monotonically() {
        for preset in [EasePreset::FastOut, EasePreset::SlowIn, EasePreset::Sine, EasePreset::Cycle] {
            let [x1, y1, x2, y2] = preset.control_points();
            let out_h = handle(x1, y1);
            let in_h = handle(x2, y2);
            let mut prev = interpolate_bezier(0.0, 1.0, 0.0, 10.0, 0.0, &out_h, &in_h);
            for i in 1..=10 {
                let t = i as f32 / 10.0;
                let v = interpolate_bezier(0.0, 1.0, 0.0, 10.0, 10.0 * t, &out_h, &in_h);
                assert!(v >= prev - 1e-3, "{:?} not monotonic at t={}", preset, t);
                prev = v;
            }
        }
    }

    #[test]
    fn test_ease_in_out_endpoints() {
        for preset in [EasePreset::EaseIn, EasePreset::EaseOut, EasePreset::FastOut, EasePreset::SlowIn] {
            let [x1, y1, x2, y2] = preset.control_points();
            let out_h = handle(x1, y1);
            let in_h = handle(x2, y2);
            let v_start = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 0.0, &out_h, &in_h);
            let v_end = interpolate_bezier(0.0, 100.0, 0.0, 10.0, 10.0, &out_h, &in_h);
            assert!(v_start.abs() < 1e-3, "{:?} start != 0, got {}", preset, v_start);
            assert!((v_end - 100.0).abs() < 1e-3, "{:?} end != 100, got {}", preset, v_end);
        }
    }
}
