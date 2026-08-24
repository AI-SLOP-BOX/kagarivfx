//! Spatial keyframes with Rove Across Time (AE spatial interpolation).
//!
//! Wired end-to-end via the Command Palette entry
//! "Keyframe Assistant: Rove Across Time (Position)", which feeds the
//! selected layer's position track through [`smooth_keyframe_velocity`].
use crate::core::keyframe::Keyframe;

/// Spatial keyframe with optional Rove Across Time capability (AE Spatial Interpolation).
#[derive(Debug, Clone)]
pub struct SpatialKeyframe2D {
    pub frame: u32,
    pub position: [f32; 2],
    pub handle_in: [f32; 2],  // Relative spatial tangent in
    pub handle_out: [f32; 2], // Relative spatial tangent out
    pub rove_across_time: bool,
}

/// Evaluates spatial Euclidean arc-length distance between two 2D spatial keyframes.
pub fn calculate_segment_distance(k0: &SpatialKeyframe2D, k1: &SpatialKeyframe2D) -> f32 {
    let p0 = k0.position;
    let p3 = k1.position;

    // Check if Bezier spatial curve
    let is_bezier = k0.handle_out[0].abs() > 0.001
        || k0.handle_out[1].abs() > 0.001
        || k1.handle_in[0].abs() > 0.001
        || k1.handle_in[1].abs() > 0.001;

    if !is_bezier {
        let dx = p3[0] - p0[0];
        let dy = p3[1] - p0[1];
        return (dx * dx + dy * dy).sqrt();
    }

    // Cubic Bezier numerical integration (16 steps)
    let p1 = [p0[0] + k0.handle_out[0], p0[1] + k0.handle_out[1]];
    let p2 = [p3[0] + k1.handle_in[0], p3[1] + k1.handle_in[1]];

    let steps = 16;
    let mut total_dist = 0.0f32;
    let mut prev_pt = p0;

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let inv_t = 1.0 - t;
        let b0 = inv_t * inv_t * inv_t;
        let b1 = 3.0 * inv_t * inv_t * t;
        let b2 = 3.0 * inv_t * t * t;
        let b3 = t * t * t;

        let pt = [
            b0 * p0[0] + b1 * p1[0] + b2 * p2[0] + b3 * p3[0],
            b0 * p0[1] + b1 * p1[1] + b2 * p2[1] + b3 * p3[1],
        ];

        let dx = pt[0] - prev_pt[0];
        let dy = pt[1] - prev_pt[1];
        total_dist += (dx * dx + dy * dy).sqrt();
        prev_pt = pt;
    }

    total_dist
}

/// Core Rove Across Time Algorithm:
/// Automatically adjusts the frame timings of intermediate roving keyframes to ensure constant motion velocity along spatial paths.
pub fn apply_rove_across_time(keyframes: &mut [SpatialKeyframe2D]) {
    if keyframes.len() < 3 {
        return;
    }

    let total_count = keyframes.len();
    let mut idx = 0;

    while idx < total_count {
        // Find sequence of roving keyframes bounded by fixed keyframes
        if !keyframes[idx].rove_across_time {
            let start_idx = idx;
            let mut end_idx = start_idx + 1;

            while end_idx < total_count && keyframes[end_idx].rove_across_time {
                end_idx += 1;
            }

            if end_idx < total_count && end_idx > start_idx + 1 {
                // We found a roving segment between start_idx and end_idx
                let start_frame = keyframes[start_idx].frame;
                let end_frame = keyframes[end_idx].frame;
                let total_frame_delta = (end_frame - start_frame) as f32;

                // Step 1: Accumulate spatial distance along the sub-path
                let mut segment_distances = Vec::with_capacity(end_idx - start_idx);
                let mut accumulated_dist = 0.0f32;

                for i in start_idx..end_idx {
                    let seg_d = calculate_segment_distance(&keyframes[i], &keyframes[i + 1]);
                    accumulated_dist += seg_d;
                    segment_distances.push(accumulated_dist);
                }

                // Step 2: Re-allocate frame numbers proportionally to spatial arc-length
                if accumulated_dist > 0.001 {
                    for (seg_idx, kf) in keyframes[start_idx + 1..end_idx].iter_mut().enumerate() {
                        let dist_ratio = segment_distances[seg_idx] / accumulated_dist;
                        let new_frame = start_frame + (total_frame_delta * dist_ratio).round() as u32;
                        kf.frame = new_frame;
                    }
                }
            }

            idx = end_idx;
        } else {
            idx += 1;
        }
    }
}

/// Converts standard position keyframes into spatial keyframes and applies roving velocity smoothing.
pub fn smooth_keyframe_velocity(keyframes: &mut [Keyframe<[f32; 2]>], rove_indices: &[usize]) {
    if keyframes.len() < 3 {
        return;
    }

    let mut spatial_kfs: Vec<SpatialKeyframe2D> = keyframes
        .iter()
        .enumerate()
        .map(|(idx, kf)| SpatialKeyframe2D {
            frame: kf.frame,
            position: kf.value,
            handle_in: [0.0, 0.0],
            handle_out: [0.0, 0.0],
            rove_across_time: rove_indices.contains(&idx),
        })
        .collect();

    apply_rove_across_time(&mut spatial_kfs);

    for (idx, skf) in spatial_kfs.into_iter().enumerate() {
        keyframes[idx].frame = skf.frame;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rove_across_time_equal_spacing() {
        let mut kfs = vec![
            SpatialKeyframe2D { frame: 0, position: [0.0, 0.0], handle_in: [0.0, 0.0], handle_out: [0.0, 0.0], rove_across_time: false },
            SpatialKeyframe2D { frame: 10, position: [50.0, 0.0], handle_in: [0.0, 0.0], handle_out: [0.0, 0.0], rove_across_time: true },
            SpatialKeyframe2D { frame: 100, position: [100.0, 0.0], handle_in: [0.0, 0.0], handle_out: [0.0, 0.0], rove_across_time: false },
        ];

        apply_rove_across_time(&mut kfs);

        // Middle keyframe at 50% distance should be adjusted to 50% of time (frame 50)
        assert_eq!(kfs[1].frame, 50);
    }
}
