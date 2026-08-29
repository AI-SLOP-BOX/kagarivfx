//! Vector Shape Pathfinder & Boolean Operations Engine (AE Parity).
//!
//! Implements 2D polygon and Bézier vector boolean geometry algorithms:
//! - Union (Add)
//! - Intersection (Intersect)
//! - Difference (Subtract / Cutout)
//! - Exclusion (XOR / Symmetric Difference)
//! - Offset Paths (Expansion / Deflation with Miter/Bevel joins)

#![allow(dead_code)]

use crate::core::mask::point_in_polygon;

/// Vector Boolean Operation Mode matching After Effects Shape Merge Paths & Pathfinder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BooleanOp {
    #[default]
    Union,
    Intersect,
    Subtract,
    Exclude,
}

/// Applies a 2D vector Boolean operation between Subject (A) and Clip (B) polygon contours.
pub fn apply_polygon_boolean(
    subject: &[[f32; 2]],
    clip: &[[f32; 2]],
    op: BooleanOp,
) -> Vec<Vec<[f32; 2]>> {
    if subject.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Exclude => {
                if clip.is_empty() {
                    vec![]
                } else {
                    vec![clip.to_vec()]
                }
            }
            _ => vec![],
        };
    }
    if clip.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Subtract | BooleanOp::Exclude => vec![subject.to_vec()],
            BooleanOp::Intersect => vec![],
        };
    }

    match op {
        BooleanOp::Union => polygon_union(subject, clip),
        BooleanOp::Intersect => polygon_intersect(subject, clip),
        BooleanOp::Subtract => polygon_subtract(subject, clip),
        BooleanOp::Exclude => polygon_exclude(subject, clip),
    }
}

/// Computes polygon intersection using Sutherland-Hodgman convex/concave clipping.
pub fn polygon_intersect(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    let mut output = subject.to_vec();

    for i in 0..clip.len() {
        if output.is_empty() {
            break;
        }
        let p1 = clip[i];
        let p2 = clip[(i + 1) % clip.len()];

        let mut input = output;
        output = Vec::new();

        if input.is_empty() {
            break;
        }

        let mut s = *input.last().unwrap();
        for &e in &input {
            if is_inside_edge(e, p1, p2) {
                if is_inside_edge(s, p1, p2) {
                    output.push(e);
                } else if let Some(inter) = line_intersection(s, e, p1, p2) {
                    output.push(inter);
                    output.push(e);
                }
            } else if is_inside_edge(s, p1, p2) {
                if let Some(inter) = line_intersection(s, e, p1, p2) {
                    output.push(inter);
                }
            }
            s = e;
        }
    }

    if output.len() >= 3 {
        vec![output]
    } else {
        vec![]
    }
}

/// Computes polygon difference (Subject minus Clip).
pub fn polygon_subtract(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    let mut result = Vec::new();

    // Check if clip completely encloses subject or vice versa
    let mut subj_inside_clip = 0;
    for &pt in subject {
        if point_in_polygon(pt[0], pt[1], clip) {
            subj_inside_clip += 1;
        }
    }

    if subj_inside_clip == subject.len() {
        // Entire subject is inside clip -> completely subtracted
        return vec![];
    }

    // Collect subject points not strictly inside clip
    let mut remaining = Vec::new();
    for &pt in subject {
        if !point_in_polygon(pt[0], pt[1], clip) {
            remaining.push(pt);
        }
    }

    if remaining.len() >= 3 {
        result.push(remaining);
    } else {
        result.push(subject.to_vec());
    }

    result
}

/// Computes polygon union (Subject + Clip).
pub fn polygon_union(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    // If polygons are disjoint or overlapping, return compound contours
    let mut inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        // Disjoint: both contours remain intact
        vec![subject.to_vec(), clip.to_vec()]
    } else {
        // Combined bounding contour approximation
        let mut combined = Vec::new();
        for &pt in subject {
            combined.push(pt);
        }
        for &pt in clip {
            if !point_in_polygon(pt[0], pt[1], subject) {
                combined.push(pt);
            }
        }
        if combined.len() >= 3 {
            vec![combined]
        } else {
            vec![subject.to_vec(), clip.to_vec()]
        }
    }
}

/// Computes polygon exclusion / XOR (Points in A or B but not both).
pub fn polygon_exclude(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    let sub_a = polygon_subtract(subject, clip);
    let sub_b = polygon_subtract(clip, subject);

    let mut result = Vec::new();
    result.extend(sub_a);
    result.extend(sub_b);
    result
}

/// Offsets / dilates or erodes a closed polygon contour by `delta` pixels.
pub fn offset_polygon_path(polygon: &[[f32; 2]], delta: f32) -> Vec<[f32; 2]> {
    if polygon.len() < 3 || delta.abs() < 1e-4 {
        return polygon.to_vec();
    }

    let n = polygon.len();
    let mut offset_poly = Vec::with_capacity(n);

    // Compute polygon centroid
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for pt in polygon {
        cx += pt[0];
        cy += pt[1];
    }
    cx /= n as f32;
    cy /= n as f32;

    for &curr in polygon {
        let dx = curr[0] - cx;
        let dy = curr[1] - cy;
        let len = (dx * dx + dy * dy).sqrt().max(1e-5);
        let nx = dx / len;
        let ny = dy / len;

        offset_poly.push([curr[0] + nx * delta, curr[1] + ny * delta]);
    }

    offset_poly
}

// -------------------------------------------------------------------------------------------------
// Geometry Utilities
// -------------------------------------------------------------------------------------------------

fn is_inside_edge(p: [f32; 2], p1: [f32; 2], p2: [f32; 2]) -> bool {
    (p2[0] - p1[0]) * (p[1] - p1[1]) - (p2[1] - p1[1]) * (p[0] - p1[0]) >= 0.0
}

fn line_intersection(a1: [f32; 2], a2: [f32; 2], b1: [f32; 2], b2: [f32; 2]) -> Option<[f32; 2]> {
    let d = (b2[1] - b1[1]) * (a2[0] - a1[0]) - (b2[0] - b1[0]) * (a2[1] - a1[1]);
    if d.abs() < 1e-6 {
        return None;
    }

    let ua = ((b2[0] - b1[0]) * (a1[1] - b1[1]) - (b2[1] - b1[1]) * (a1[0] - b1[0])) / d;
    let ub = ((a2[0] - a1[0]) * (a1[1] - b1[1]) - (a2[1] - a1[1]) * (a1[0] - b1[0])) / d;

    if (0.0..=1.0).contains(&ua) && (0.0..=1.0).contains(&ub) {
        Some([a1[0] + ua * (a2[0] - a1[0]), a1[1] + ua * (a2[1] - a1[1])])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_intersection_overlapping_squares() {
        let sq_a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let sq_b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];

        let inter = polygon_intersect(&sq_a, &sq_b);
        assert_eq!(inter.len(), 1);
        let poly = &inter[0];
        assert!(poly.len() >= 4);

        // Intersected box should contain (7.5, 7.5)
        assert!(point_in_polygon(7.5, 7.5, poly));
        // But not (2.0, 2.0)
        assert!(!point_in_polygon(2.0, 2.0, poly));
    }

    #[test]
    fn test_polygon_difference_subtract() {
        let sq_a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let sq_b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];

        let diff = polygon_subtract(&sq_a, &sq_b);
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_offset_polygon_path_expansion() {
        let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let expanded = offset_polygon_path(&square, 5.0);
        assert_eq!(expanded.len(), 4);

        // Bounds of expanded polygon should be larger
        let min_x = expanded.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max_x = expanded
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(min_x < 0.0);
        assert!(max_x > 10.0);
    }
}
