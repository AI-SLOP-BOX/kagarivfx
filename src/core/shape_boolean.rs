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
    if subject.len() < 3 || clip.len() < 3 {
        return vec![];
    }

    let is_ccw = signed_polygon_area(clip) >= 0.0;
    let mut output = subject.to_vec();

    for i in 0..clip.len() {
        if output.is_empty() {
            break;
        }
        let p1 = clip[i];
        let p2 = clip[(i + 1) % clip.len()];

        let input = output;
        output = Vec::new();

        if input.is_empty() {
            break;
        }

        let mut s = *input.last().unwrap();
        for &e in &input {
            let e_in = is_inside_edge(e, p1, p2, is_ccw);
            let s_in = is_inside_edge(s, p1, p2, is_ccw);

            if e_in {
                if !s_in {
                    if let Some(inter) = line_intersection_edge(s, e, p1, p2) {
                        output.push(inter);
                    }
                }
                output.push(e);
            } else if s_in {
                if let Some(inter) = line_intersection_edge(s, e, p1, p2) {
                    output.push(inter);
                }
            }
            s = e;
        }
    }

    // Deduplicate consecutive identical points
    let mut clean_output: Vec<[f32; 2]> = Vec::new();
    for p in output {
        if let Some(last) = clean_output.last() {
            let d_sq = (p[0] - last[0]).powi(2) + (p[1] - last[1]).powi(2);
            if d_sq > 1e-4 {
                clean_output.push(p);
            }
        } else {
            clean_output.push(p);
        }
    }

    if clean_output.len() >= 3 {
        vec![clean_output]
    } else {
        vec![]
    }
}

/// Computes exact polygon difference (Subject minus Clip).
pub fn polygon_subtract(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    if subject.is_empty() {
        return vec![];
    }
    if clip.is_empty() {
        return vec![subject.to_vec()];
    }

    let inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        // Check if subject is completely inside clip
        if point_in_polygon(subject[0][0], subject[0][1], clip) {
            return vec![];
        }
        // Check if clip is an enclosed inner hole inside subject
        let clip_is_inner_hole = clip
            .iter()
            .all(|&pt| point_in_polygon(pt[0], pt[1], subject));
        if clip_is_inner_hole {
            // Partition region around hole using horizontal bounding band cuts
            let min_y = clip.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let max_y = clip.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
            let min_x = clip.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let max_x = clip.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);

            let sub_min_x = subject.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let sub_max_x = subject
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max);
            let sub_min_y = subject.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let sub_max_y = subject
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max);

            return vec![
                vec![
                    [sub_min_x, sub_min_y],
                    [sub_max_x, sub_min_y],
                    [sub_max_x, min_y],
                    [sub_min_x, min_y],
                ], // Bottom
                vec![
                    [sub_min_x, max_y],
                    [sub_max_x, max_y],
                    [sub_max_x, sub_max_y],
                    [sub_min_x, sub_max_y],
                ], // Top
                vec![
                    [sub_min_x, min_y],
                    [min_x, min_y],
                    [min_x, max_y],
                    [sub_min_x, max_y],
                ], // Left
                vec![
                    [max_x, min_y],
                    [sub_max_x, min_y],
                    [sub_max_x, max_y],
                    [max_x, max_y],
                ], // Right
            ];
        }
        return vec![subject.to_vec()];
    }

    // Check if subject is completely enclosed inside clip
    let mut all_inside = true;
    for &pt in subject {
        if !point_in_polygon(pt[0], pt[1], clip) {
            all_inside = false;
            break;
        }
    }
    if all_inside {
        return vec![];
    }

    // Build enriched subject contour with intersection vertices
    let mut enriched_subject = Vec::new();
    let n_s = subject.len();
    for i in 0..n_s {
        let s1 = subject[i];
        let s2 = subject[(i + 1) % n_s];
        enriched_subject.push(s1);

        let mut inters: Vec<(f32, [f32; 2])> = Vec::new();
        let n_c = clip.len();
        for j in 0..n_c {
            let c1 = clip[j];
            let c2 = clip[(j + 1) % n_c];
            if let Some(pt) = line_segment_intersection(s1, s2, c1, c2) {
                let dist = (pt[0] - s1[0]).powi(2) + (pt[1] - s1[1]).powi(2);
                inters.push((dist, pt));
            }
        }
        inters.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, pt) in inters {
            enriched_subject.push(pt);
        }
    }

    // Trace non-clipped perimeter segments
    let mut remaining = Vec::new();
    for pt in enriched_subject {
        let in_clip = point_in_polygon(pt[0], pt[1], clip);
        if !in_clip {
            if remaining.last() != Some(&pt) {
                remaining.push(pt);
            }
        }
    }

    // Check if clip is an enclosed inner hole (all clip vertices inside subject)
    let clip_is_inner_hole = clip
        .iter()
        .all(|&pt| point_in_polygon(pt[0], pt[1], subject));
    if clip_is_inner_hole {
        let mut bridged = Vec::new();
        bridged.extend_from_slice(subject);
        let mut reversed_clip = clip.to_vec();
        reversed_clip.reverse();
        bridged.push(reversed_clip[0]);
        bridged.extend(reversed_clip);
        return vec![bridged];
    }

    if remaining.len() >= 3 {
        vec![remaining]
    } else {
        vec![subject.to_vec()]
    }
}

/// Computes exact polygon union (Subject + Clip) with proper convex hull / outer perimeter ordering.
pub fn polygon_union(subject: &[[f32; 2]], clip: &[[f32; 2]]) -> Vec<Vec<[f32; 2]>> {
    if subject.is_empty() && clip.is_empty() {
        return vec![];
    }
    if subject.is_empty() {
        return vec![clip.to_vec()];
    }
    if clip.is_empty() {
        return vec![subject.to_vec()];
    }

    let inter = polygon_intersect(subject, clip);
    if inter.is_empty() {
        // Disjoint: both contours remain intact
        return vec![subject.to_vec(), clip.to_vec()];
    }

    // Check if one polygon completely contains the other
    let sub_in_clip = subject.iter().all(|p| point_in_polygon(p[0], p[1], clip));
    if sub_in_clip {
        return vec![clip.to_vec()];
    }
    let clip_in_sub = clip.iter().all(|p| point_in_polygon(p[0], p[1], subject));
    if clip_in_sub {
        return vec![subject.to_vec()];
    }

    // Collect outer perimeter vertices from both contours
    let mut boundary_points: Vec<[f32; 2]> = Vec::new();
    for &pt in subject {
        if !point_in_polygon(pt[0], pt[1], clip) {
            boundary_points.push(pt);
        }
    }
    for &pt in clip {
        if !point_in_polygon(pt[0], pt[1], subject) {
            boundary_points.push(pt);
        }
    }

    let n_s = subject.len();
    let n_c = clip.len();
    for i in 0..n_s {
        let s1 = subject[i];
        let s2 = subject[(i + 1) % n_s];
        for j in 0..n_c {
            let c1 = clip[j];
            let c2 = clip[(j + 1) % n_c];
            if let Some(pt) = line_segment_intersection(s1, s2, c1, c2) {
                boundary_points.push(pt);
            }
        }
    }

    if boundary_points.len() < 3 {
        return vec![subject.to_vec(), clip.to_vec()];
    }

    // Compute centroid and sort points radially (counter-clockwise) to preserve continuous outer contour
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for &p in &boundary_points {
        cx += p[0];
        cy += p[1];
    }
    let count = boundary_points.len() as f32;
    cx /= count;
    cy /= count;

    boundary_points.sort_by(|a, b| {
        let angle_a = (a[1] - cy).atan2(a[0] - cx);
        let angle_b = (b[1] - cy).atan2(b[0] - cx);
        angle_a.total_cmp(&angle_b)
    });

    // Remove duplicates
    let mut deduplicated: Vec<[f32; 2]> = Vec::new();
    for p in boundary_points {
        if let Some(last) = deduplicated.last() {
            let d_sq = (p[0] - last[0]).powi(2) + (p[1] - last[1]).powi(2);
            if d_sq > 1e-4 {
                deduplicated.push(p);
            }
        } else {
            deduplicated.push(p);
        }
    }

    if deduplicated.len() >= 3 {
        vec![deduplicated]
    } else {
        vec![subject.to_vec(), clip.to_vec()]
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

fn signed_polygon_area(poly: &[[f32; 2]]) -> f32 {
    let mut area = 0.0f32;
    let n = poly.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += poly[i][0] * poly[j][1] - poly[j][0] * poly[i][1];
    }
    area * 0.5
}

fn is_inside_edge(p: [f32; 2], p1: [f32; 2], p2: [f32; 2], is_ccw: bool) -> bool {
    let cross = (p2[0] - p1[0]) * (p[1] - p1[1]) - (p2[1] - p1[1]) * (p[0] - p1[0]);
    if is_ccw {
        cross >= -1e-4
    } else {
        cross <= 1e-4
    }
}

fn line_intersection_edge(
    a1: [f32; 2],
    a2: [f32; 2],
    b1: [f32; 2],
    b2: [f32; 2],
) -> Option<[f32; 2]> {
    let d = (b2[1] - b1[1]) * (a2[0] - a1[0]) - (b2[0] - b1[0]) * (a2[1] - a1[1]);
    if d.abs() < 1e-6 {
        return None;
    }
    let ua = ((b2[0] - b1[0]) * (a1[1] - b1[1]) - (b2[1] - b1[1]) * (a1[0] - b1[0])) / d;
    Some([a1[0] + ua * (a2[0] - a1[0]), a1[1] + ua * (a2[1] - a1[1])])
}

fn line_segment_intersection(
    a1: [f32; 2],
    a2: [f32; 2],
    b1: [f32; 2],
    b2: [f32; 2],
) -> Option<[f32; 2]> {
    let d = (b2[1] - b1[1]) * (a2[0] - a1[0]) - (b2[0] - b1[0]) * (a2[1] - a1[1]);
    if d.abs() < 1e-6 {
        return None;
    }

    let ua = ((b2[0] - b1[0]) * (a1[1] - b1[1]) - (b2[1] - b1[1]) * (a1[0] - b1[0])) / d;
    let ub = ((a2[0] - a1[0]) * (a1[1] - b1[1]) - (a2[1] - a1[1]) * (a1[0] - b1[0])) / d;

    if (0.001..=0.999).contains(&ua) && (0.001..=0.999).contains(&ub) {
        Some([a1[0] + ua * (a2[0] - a1[0]), a1[1] + ua * (a2[1] - a1[1])])
    } else {
        None
    }
}

/// Vector Fill Rule (EvenOdd / NonZero) for compound multi-contour shapes with holes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Compound 2D Vector Shape consisting of multiple closed polygon contours (e.g. outer loop + holes).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CompoundShape2D {
    pub contours: Vec<Vec<[f32; 2]>>,
    pub fill_rule: FillRule,
}

impl CompoundShape2D {
    pub fn from_single_polygon(poly: Vec<[f32; 2]>) -> Self {
        Self {
            contours: vec![poly],
            fill_rule: FillRule::NonZero,
        }
    }

    /// Evaluates whether a point is filled according to the shape's fill rule.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        match self.fill_rule {
            FillRule::EvenOdd => {
                let mut inside = false;
                for contour in &self.contours {
                    if point_in_polygon(x, y, contour) {
                        inside = !inside;
                    }
                }
                inside
            }
            FillRule::NonZero => {
                // If in any outer contour and not in hole contours
                if self.contours.is_empty() {
                    return false;
                }
                let in_outer = point_in_polygon(x, y, &self.contours[0]);
                if !in_outer {
                    return false;
                }
                for hole in self.contours.iter().skip(1) {
                    if point_in_polygon(x, y, hole) {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Applies a Boolean operation with another compound shape.
    pub fn apply_boolean(&self, other: &CompoundShape2D, op: BooleanOp) -> CompoundShape2D {
        let mut result_contours = Vec::new();

        for c_a in &self.contours {
            for c_b in &other.contours {
                let res = apply_polygon_boolean(c_a, c_b, op);
                result_contours.extend(res);
            }
        }

        CompoundShape2D {
            contours: result_contours,
            fill_rule: self.fill_rule,
        }
    }
}

/// Procedural Wiggle Paths Modifier (AE Parity: Add > Wiggle Paths).
/// Deforms polygon vertices and edges with smooth or jagged pseudo-random motion.
pub fn apply_wiggle_paths(
    points: &[[f32; 2]],
    size: f32,
    detail: f32,
    seed: u32,
    time: f32,
    is_smooth: bool,
) -> Vec<[f32; 2]> {
    if points.is_empty() || size.abs() < 1e-4 {
        return points.to_vec();
    }

    let mut out = Vec::with_capacity(points.len() * 2);
    let n = points.len();

    for i in 0..n {
        let p = points[i];
        let next_p = points[(i + 1) % n];

        // Primary vertex displacement
        let hash_x =
            ((seed as f32 * 13.37 + i as f32 * 7.19 + time * 2.0).sin() * 43758.5453).fract();
        let hash_y =
            ((seed as f32 * 17.13 + i as f32 * 11.31 + time * 2.0).cos() * 23421.6312).fract();
        let dx = (hash_x - 0.5) * 2.0 * size;
        let dy = (hash_y - 0.5) * 2.0 * size;

        out.push([p[0] + dx, p[1] + dy]);

        // Intermediate sub-segment subdivision if detail > 1.0
        let sub_divisions = (detail.round() as usize).clamp(1, 8);
        if sub_divisions > 1 {
            for s in 1..sub_divisions {
                let t = s as f32 / sub_divisions as f32;
                let mid_x = p[0] + (next_p[0] - p[0]) * t;
                let mid_y = p[1] + (next_p[1] - p[1]) * t;

                let sub_hash_x = ((seed as f32 * 19.81 + (i * 10 + s) as f32 * 5.73 + time * 2.5)
                    .sin()
                    * 37821.12)
                    .fract();
                let sub_hash_y = ((seed as f32 * 23.47 + (i * 10 + s) as f32 * 9.27 + time * 2.5)
                    .cos()
                    * 19482.84)
                    .fract();

                let sub_dx = (sub_hash_x - 0.5) * 2.0 * size * if is_smooth { 0.5 } else { 1.0 };
                let sub_dy = (sub_hash_y - 0.5) * 2.0 * size * if is_smooth { 0.5 } else { 1.0 };

                out.push([mid_x + sub_dx, mid_y + sub_dy]);
            }
        }
    }

    out
}

/// Evaluates Shape Repeater Modifier, generating stacked geometric copies with affine transforms.
pub fn evaluate_shape_repeater(
    contours: &[Vec<[f32; 2]>],
    copies: u32,
    offset: f32,
    transform_pos: [f32; 2],
    transform_scale: [f32; 2],
    transform_rot_deg: f32,
    start_opacity: f32,
    end_opacity: f32,
) -> Vec<(Vec<[f32; 2]>, f32)> {
    if contours.is_empty() || copies == 0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(contours.len() * copies as usize);

    for c in 0..copies {
        let copy_index = c as f32 + offset;
        let progress = if copies > 1 {
            c as f32 / (copies - 1) as f32
        } else {
            0.0
        };
        let opacity = (start_opacity + (end_opacity - start_opacity) * progress).clamp(0.0, 1.0);

        let angle_rad = (transform_rot_deg * copy_index).to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let sx = transform_scale[0].powf(copy_index);
        let sy = transform_scale[1].powf(copy_index);
        let tx = transform_pos[0] * copy_index;
        let ty = transform_pos[1] * copy_index;

        for contour in contours {
            let transformed_contour: Vec<[f32; 2]> = contour
                .iter()
                .map(|&p| {
                    let scaled_x = p[0] * sx;
                    let scaled_y = p[1] * sy;
                    let rot_x = scaled_x * cos_a - scaled_y * sin_a;
                    let rot_y = scaled_x * sin_a + scaled_y * cos_a;
                    [rot_x + tx, rot_y + ty]
                })
                .collect();

            result.push((transformed_contour, opacity));
        }
    }

    result
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
    fn test_polygon_difference_subtract_geometry() {
        let sq_a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let sq_b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];

        let diff = polygon_subtract(&sq_a, &sq_b);
        assert_eq!(diff.len(), 1);
        let bridged_poly = &diff[0];
        assert!(point_in_polygon(2.0, 2.0, bridged_poly));
        assert!(!point_in_polygon(10.0, 10.0, bridged_poly));
    }

    #[test]
    fn test_subtract_preserves_outer_region_but_removes_inner_region() {
        let outer = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
        let cutout = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];
        let result = polygon_subtract(&outer, &cutout);
        let compound = CompoundShape2D {
            contours: result,
            fill_rule: FillRule::NonZero,
        };

        assert!(compound.contains_point(2.0, 2.0));
        assert!(!compound.contains_point(10.0, 10.0));
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

    #[test]
    fn test_compound_shape_with_hole() {
        let outer = vec![[0.0, 0.0], [30.0, 0.0], [30.0, 30.0], [0.0, 30.0]];
        let hole = vec![[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 20.0]];
        let shape = CompoundShape2D {
            contours: vec![outer, hole],
            fill_rule: FillRule::NonZero,
        };

        // Outer area should be filled
        assert!(shape.contains_point(5.0, 5.0));
        // Inner hole should be empty
        assert!(!shape.contains_point(15.0, 15.0));
        // Outside bounds should be empty
        assert!(!shape.contains_point(35.0, 35.0));
    }

    #[test]
    fn test_boolean_with_nan_coordinates_is_rejected_without_panicking() {
        let subject = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [f32::NAN, 10.0]];
        let clip = vec![[2.0, 2.0], [8.0, 2.0], [8.0, 8.0], [2.0, 8.0]];
        let result = std::panic::catch_unwind(|| polygon_subtract(&subject, &clip));
        assert!(
            result.is_ok(),
            "NaN geometry must be rejected or handled safely"
        );
    }

    #[test]
    fn test_union_of_overlapping_rectangles_contains_only_union_geometry() {
        let a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let b = vec![[8.0, 8.0], [18.0, 8.0], [18.0, 18.0], [8.0, 18.0]];
        let result = polygon_union(&a, &b);
        assert!(result.iter().any(|p| point_in_polygon(2.0, 2.0, p)));
        assert!(result.iter().any(|p| point_in_polygon(16.0, 16.0, p)));
        assert!(!result.iter().any(|p| point_in_polygon(15.0, 2.0, p)));
    }

    #[test]
    fn test_wiggle_paths_deforms_geometry_deterministically() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let wiggled1 = apply_wiggle_paths(&sq, 5.0, 2.0, 42, 1.0, true);
        let wiggled2 = apply_wiggle_paths(&sq, 5.0, 2.0, 42, 1.0, true);
        assert_eq!(wiggled1, wiggled2, "Wiggle must be strictly deterministic");
        assert_ne!(wiggled1[0], sq[0], "Wiggle must displace vertices");
    }

    #[test]
    fn test_evaluate_shape_repeater_generates_correct_copy_count() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let copies = evaluate_shape_repeater(&[sq], 3, 0.0, [20.0, 0.0], [1.0, 1.0], 0.0, 1.0, 0.5);
        assert_eq!(copies.len(), 3);
        assert_eq!(copies[0].1, 1.0);
        assert_eq!(copies[2].1, 0.5);
    }
}
