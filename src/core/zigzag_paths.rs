//! Procedural Zig Zag Shape Modifier (AE Parity).
//!
//! Subdivides 2D vector path segments into oscillating ridges (Corner / Smooth peaks)
//! with configurable amplitude (size) and frequency (ridges_per_segment).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ZigZagPointType {
    #[default]
    Corner,
    Smooth,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZigZagParams {
    /// Amplitude / peak height in pixels (0.0 .. 500.0)
    pub size: f32,
    /// Number of zig-zag ridges per path segment (1 .. 100)
    pub ridges_per_segment: u32,
    /// Waveform style: Corner (triangular peaks) or Smooth (sinusoidal curves)
    pub point_type: ZigZagPointType,
}

impl Default for ZigZagParams {
    fn default() -> Self {
        Self {
            size: 10.0,
            ridges_per_segment: 5,
            point_type: ZigZagPointType::Corner,
        }
    }
}

/// Applies Zig Zag displacement to a polyline / polygon path points sequence.
pub fn apply_zigzag_to_points(points: &[[f32; 2]], params: &ZigZagParams, is_closed: bool) -> Vec<[f32; 2]> {
    if points.len() < 2 || params.size.abs() < 1e-4 || params.ridges_per_segment == 0 {
        return points.to_vec();
    }

    let ridges = params.ridges_per_segment.clamp(1, 100) as usize;
    let num_subdivisions = ridges * 2;
    let mut out = Vec::with_capacity(points.len() * (num_subdivisions + 1));

    let count = if is_closed { points.len() } else { points.len() - 1 };

    for i in 0..count {
        let p0 = points[i];
        let p1 = points[(i + 1) % points.len()];

        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt();

        if len < 1e-4 {
            out.push(p0);
            continue;
        }

        // Unit normal perpendicular to the segment (rotated 90 deg counter-clockwise)
        let nx = -dy / len;
        let ny = dx / len;

        out.push(p0);

        for step in 1..num_subdivisions {
            let t = step as f32 / num_subdivisions as f32;
            let base_x = p0[0] + dx * t;
            let base_y = p0[1] + dy * t;

            // Oscillate alternating +size and -size
            let phase = if step % 2 == 1 { 1.0f32 } else { -1.0f32 };
            let displacement = match params.point_type {
                ZigZagPointType::Corner => params.size * phase,
                ZigZagPointType::Smooth => {
                    // Smooth sinusoidal wave
                    let angle = t * std::f32::consts::PI * (ridges as f32 * 2.0);
                    params.size * angle.sin()
                }
            };

            out.push([base_x + nx * displacement, base_y + ny * displacement]);
        }
    }

    if !is_closed {
        if let Some(last) = points.last() {
            out.push(*last);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_empty_or_single_point() {
        let params = ZigZagParams::default();
        assert_eq!(apply_zigzag_to_points(&[], &params, false).len(), 0);
        let single = vec![[10.0, 20.0]];
        assert_eq!(apply_zigzag_to_points(&single, &params, false), single);
    }

    #[test]
    fn test_zigzag_corner_subdivision_count() {
        let points = vec![[0.0, 0.0], [100.0, 0.0]];
        let params = ZigZagParams {
            size: 10.0,
            ridges_per_segment: 3,
            point_type: ZigZagPointType::Corner,
        };
        let res = apply_zigzag_to_points(&points, &params, false);
        // 2 base points + 3*2-1 intermediate peaks = 7 total points
        assert_eq!(res.len(), 7);
        // Middle peaks must displace along Y axis
        assert!(res[1][1].abs() > 0.0);
    }

    #[test]
    fn test_zigzag_smooth_preserves_finite() {
        let points = vec![[0.0, 0.0], [100.0, 100.0]];
        let params = ZigZagParams {
            size: 25.0,
            ridges_per_segment: 4,
            point_type: ZigZagPointType::Smooth,
        };
        let res = apply_zigzag_to_points(&points, &params, false);
        for pt in res {
            assert!(pt[0].is_finite());
            assert!(pt[1].is_finite());
        }
    }
}
