//! The Smoother: Spatial-Temporal Ramer-Douglas-Peucker (RDP) Keyframe Curve Reduction & Smoothing Engine (AE Parity).
//!
//! Takes dense keyframe sequences (e.g., from Motion Sketch, Tracking, or Wiggle baking)
//! and simplifies them within a user-defined tolerance, preserving both spatial trajectory
//! and temporal pacing, calculating smooth Bezier velocity handles.

use crate::core::keyframe::{BezierControlPoint, InterpolationType, Keyframe};

/// 2D Point distance to line segment
fn point_to_segment_dist_2d(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        let ex = p[0] - a[0];
        let ey = p[1] - a[1];
        return (ex * ex + ey * ey).sqrt();
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = a[0] + t * dx;
    let proj_y = a[1] + t * dy;
    let rx = p[0] - proj_x;
    let ry = p[1] - proj_y;
    (rx * rx + ry * ry).sqrt()
}

/// 3D Point distance to line segment (Used for spatio-temporal [time, x, y] RDP)
fn point_to_segment_dist_3d(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    let len_sq = dx * dx + dy * dy + dz * dz;
    if len_sq < 1e-6 {
        let ex = p[0] - a[0];
        let ey = p[1] - a[1];
        let ez = p[2] - a[2];
        return (ex * ex + ey * ey + ez * ez).sqrt();
    }
    let t =
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy + (p[2] - a[2]) * dz) / len_sq).clamp(0.0, 1.0);
    let proj_x = a[0] + t * dx;
    let proj_y = a[1] + t * dy;
    let proj_z = a[2] + t * dz;
    let rx = p[0] - proj_x;
    let ry = p[1] - proj_y;
    let rz = p[2] - proj_z;
    (rx * rx + ry * ry + rz * rz).sqrt()
}

/// Simplify a slice of [f32; 2] keyframes using Spatio-Temporal Ramer-Douglas-Peucker.
/// Preserves timing inflections and spatial path curvature.
pub fn simplify_rdp_vec2(
    keyframes: &[Keyframe<[f32; 2]>],
    tolerance: f32,
) -> Vec<Keyframe<[f32; 2]>> {
    if keyframes.len() <= 2 {
        return keyframes.to_vec();
    }

    let mut keep = vec![false; keyframes.len()];
    keep[0] = true;
    *keep.last_mut().unwrap() = true;

    // Time weighting factor: small normalized weight to prevent stationary frames from collapsing
    let time_scale = 0.5f32;

    fn rdp_step(
        kfs: &[Keyframe<[f32; 2]>],
        start: usize,
        end: usize,
        epsilon: f32,
        time_scale: f32,
        keep: &mut [bool],
    ) {
        if end <= start + 1 {
            return;
        }
        let a = [
            kfs[start].frame as f32 * time_scale,
            kfs[start].value[0],
            kfs[start].value[1],
        ];
        let b = [
            kfs[end].frame as f32 * time_scale,
            kfs[end].value[0],
            kfs[end].value[1],
        ];

        let mut max_dist = 0.0f32;
        let mut max_idx = start;

        for i in (start + 1)..end {
            let p = [
                kfs[i].frame as f32 * time_scale,
                kfs[i].value[0],
                kfs[i].value[1],
            ];
            let d = point_to_segment_dist_3d(p, a, b);
            if d > max_dist {
                max_dist = d;
                max_idx = i;
            }
        }

        if max_dist > epsilon {
            keep[max_idx] = true;
            rdp_step(kfs, start, max_idx, epsilon, time_scale, keep);
            rdp_step(kfs, max_idx, end, epsilon, time_scale, keep);
        }
    }

    rdp_step(
        keyframes,
        0,
        keyframes.len() - 1,
        tolerance,
        time_scale,
        &mut keep,
    );

    let mut result = Vec::new();
    for (i, &should_keep) in keep.iter().enumerate() {
        if should_keep {
            let mut kf = keyframes[i].clone();
            // Automatically assign smooth Bezier interpolation
            kf.interpolation = InterpolationType::Bezier {
                outgoing: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                incoming: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
            };
            result.push(kf);
        }
    }

    result
}

/// Simplify a slice of f32 scalar keyframes using 2D (time, value) RDP
pub fn simplify_rdp_scalar(keyframes: &[Keyframe<f32>], tolerance: f32) -> Vec<Keyframe<f32>> {
    if keyframes.len() <= 2 {
        return keyframes.to_vec();
    }

    let mut keep = vec![false; keyframes.len()];
    keep[0] = true;
    *keep.last_mut().unwrap() = true;

    fn rdp_step_scalar(
        kfs: &[Keyframe<f32>],
        start: usize,
        end: usize,
        epsilon: f32,
        keep: &mut [bool],
    ) {
        if end <= start + 1 {
            return;
        }
        let a = [kfs[start].frame as f32, kfs[start].value];
        let b = [kfs[end].frame as f32, kfs[end].value];

        let mut max_dist = 0.0f32;
        let mut max_idx = start;

        for i in (start + 1)..end {
            let p = [kfs[i].frame as f32, kfs[i].value];
            let d = point_to_segment_dist_2d(p, a, b);
            if d > max_dist {
                max_dist = d;
                max_idx = i;
            }
        }

        if max_dist > epsilon {
            keep[max_idx] = true;
            rdp_step_scalar(kfs, start, max_idx, epsilon, keep);
            rdp_step_scalar(kfs, max_idx, end, epsilon, keep);
        }
    }

    rdp_step_scalar(keyframes, 0, keyframes.len() - 1, tolerance, &mut keep);

    let mut result = Vec::new();
    for (i, &should_keep) in keep.iter().enumerate() {
        if should_keep {
            let mut kf = keyframes[i].clone();
            kf.interpolation = InterpolationType::Bezier {
                outgoing: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                incoming: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
            };
            result.push(kf);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdp_simplification_reduces_collinear_points() {
        let mut kfs = Vec::new();
        for f in 0..=100 {
            // Straight line y = 2x with minor noise
            kfs.push(Keyframe::new(
                f,
                [f as f32 * 10.0, f as f32 * 20.0],
                InterpolationType::Linear,
            ));
        }

        let simplified = simplify_rdp_vec2(&kfs, 2.0);
        // Linear path should be reduced to start and end points
        assert_eq!(simplified.len(), 2);
        assert_eq!(simplified[0].frame, 0);
        assert_eq!(simplified[1].frame, 100);
    }

    #[test]
    fn test_rdp_preserves_curve_extremas() {
        let mut kfs = Vec::new();
        // Triangle wave: 0->50->0
        for f in 0..=50 {
            kfs.push(Keyframe::new(
                f,
                [f as f32, f as f32 * 2.0],
                InterpolationType::Linear,
            ));
        }
        for f in 51..=100 {
            kfs.push(Keyframe::new(
                f,
                [f as f32, (100 - f) as f32 * 2.0],
                InterpolationType::Linear,
            ));
        }

        let simplified = simplify_rdp_vec2(&kfs, 1.0);
        assert_eq!(simplified.len(), 3);
        assert_eq!(simplified[1].frame, 50);
        assert_eq!(simplified[1].value, [50.0, 100.0]);
    }

    #[test]
    fn test_rdp_preserves_stationary_and_timing_steps() {
        let mut kfs = Vec::new();
        // Stationary for 30 frames, then jump
        for f in 0..=30 {
            kfs.push(Keyframe::new(f, [100.0, 100.0], InterpolationType::Linear));
        }
        for f in 31..=60 {
            kfs.push(Keyframe::new(f, [200.0, 200.0], InterpolationType::Linear));
        }

        let simplified = simplify_rdp_vec2(&kfs, 2.0);
        assert!(simplified.len() >= 3);
    }
}
