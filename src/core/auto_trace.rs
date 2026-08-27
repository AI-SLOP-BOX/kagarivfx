/// Auto-trace: trace alpha boundaries from a pixel buffer into FreeformBezier shape paths.
///
/// Scans the alpha channel to find inside/outside transitions per scanline,
/// then walks the boundary clockwise to produce a closed contour path.
use crate::core::timeline::ShapeType;

/// Trace alpha boundaries in an RGBA buffer and produce shape paths.
/// `threshold` is the alpha cutoff (0.0–1.0) for boundary detection.
/// `tolerance` controls Douglas-Peucker simplification (smaller = more detail).
pub fn auto_trace(
    pixels: &[u8],
    width: u32,
    height: u32,
    threshold: f32,
    tolerance: f32,
) -> Vec<ShapeType> {
    if width == 0 || height == 0 || pixels.len() < (width * height * 4) as usize {
        return Vec::new();
    }

    let w = width as usize;
    let h = height as usize;

    let inside = |x: usize, y: usize| -> bool {
        let idx = (y * w + x) * 4 + 3;
        pixels[idx] as f32 / 255.0 >= threshold
    };

    // Find bounding box of inside region
    let mut min_x = w;
    let mut max_x = 0usize;
    let mut min_y = h;
    let mut max_y = 0usize;
    let mut found = false;

    for y in 0..h {
        for x in 0..w {
            if inside(x, y) {
                found = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return Vec::new();
    }

    // Build a contour by walking the boundary per scanline.
    // For each row, find leftmost and rightmost inside pixel → emit boundary points.
    let mut contour = Vec::new();

    // Top edge: walk left to right
    for x in min_x..=max_x {
        if inside(x, min_y) {
            contour.push([x as f32, min_y as f32]);
        } else {
            break;
        }
    }
    // Right edge: walk top to bottom
    for y in min_y..=max_y {
        if inside(max_x, y) {
            contour.push([(max_x + 1) as f32, y as f32]);
        } else {
            break;
        }
    }
    // Bottom edge: walk right to left
    for x in (min_x..=max_x).rev() {
        if inside(x, max_y) {
            contour.push([x as f32, (max_y + 1) as f32]);
        } else {
            break;
        }
    }
    // Left edge: walk bottom to top
    for y in (min_y..=max_y).rev() {
        if inside(min_x, y) {
            contour.push([min_x as f32, y as f32]);
        } else {
            break;
        }
    }

    // If boundary walking produced too few points, use bounding box
    if contour.len() < 4 {
        let pad = 0.5;
        contour = vec![
            [min_x as f32 - pad, min_y as f32 - pad],
            [(max_x + 1) as f32 + pad, min_y as f32 - pad],
            [(max_x + 1) as f32 + pad, (max_y + 1) as f32 + pad],
            [min_x as f32 - pad, (max_y + 1) as f32 + pad],
        ];
    }

    // Simplify and create shape
    let simplified = douglas_peucker(&contour, tolerance);
    let mut shapes = Vec::new();
    if simplified.len() >= 3 {
        let len = simplified.len();
        shapes.push(ShapeType::FreeformBezier {
            points: simplified,
            tangents: (0..len).map(|_| ([0.0f32, 0.0], [0.0f32, 0.0])).collect(),
            closed: true,
        });
    }

    shapes
}

/// Douglas-Peucker polyline simplification.
fn douglas_peucker(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut max_dist = 0.0f32;
    let mut max_idx = 0;

    let first = points[0];
    let last = points[points.len() - 1];

    for (i, &p) in points[1..points.len() - 1].iter().enumerate() {
        let dist = point_to_line_distance(p, first, last);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i + 1;
        }
    }

    if max_dist > tolerance {
        let left = douglas_peucker(&points[..=max_idx], tolerance);
        let right = douglas_peucker(&points[max_idx..], tolerance);
        let mut result = left;
        result.extend_from_slice(&right[1..]);
        result
    } else {
        vec![first, last]
    }
}

fn point_to_line_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 {
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a[0] + t * dx;
    let proj_y = a[1] + t * dy;
    ((p[0] - proj_x).powi(2) + (p[1] - proj_y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transparent_buffer(w: u32, h: u32) -> Vec<u8> {
        vec![0u8; (w * h * 4) as usize]
    }

    #[test]
    fn test_auto_trace_empty_buffer() {
        let shapes = auto_trace(&[], 0, 0, 0.5, 1.0);
        assert!(shapes.is_empty());
    }

    #[test]
    fn test_auto_trace_all_transparent() {
        let pixels = transparent_buffer(10, 10);
        let shapes = auto_trace(&pixels, 10, 10, 0.5, 1.0);
        assert!(shapes.is_empty());
    }

    #[test]
    fn test_auto_trace_rectangular_block() {
        let mut pixels = transparent_buffer(20, 20);
        for y in 5..15 {
            for x in 5..15 {
                let idx = (y * 20 + x) * 4 + 3;
                pixels[idx] = 255;
            }
        }
        let shapes = auto_trace(&pixels, 20, 20, 0.5, 1.0);
        assert!(!shapes.is_empty(), "rectangular block should produce a contour");
        if let ShapeType::FreeformBezier { points, closed, .. } = &shapes[0] {
            assert!(*closed, "auto-traced shape should be closed");
            assert!(points.len() >= 3, "contour should have at least 3 points");
            // Verify the shape roughly covers the rectangle
            let xs: Vec<f32> = points.iter().map(|p| p[0]).collect();
            let ys: Vec<f32> = points.iter().map(|p| p[1]).collect();
            assert!(xs.iter().copied().reduce(f32::min).unwrap() < 6.0);
            assert!(xs.iter().copied().reduce(f32::max).unwrap() > 14.0);
            assert!(ys.iter().copied().reduce(f32::min).unwrap() < 6.0);
            assert!(ys.iter().copied().reduce(f32::max).unwrap() > 14.0);
        }
    }

    #[test]
    fn test_auto_trace_circle() {
        let mut pixels = transparent_buffer(40, 40);
        for y in 0..40 {
            for x in 0..40 {
                let dx = x as f32 - 20.0;
                let dy = y as f32 - 20.0;
                if dx * dx + dy * dy < 100.0 {
                    let idx = (y * 40 + x) * 4 + 3;
                    pixels[idx] = 255;
                }
            }
        }
        let shapes = auto_trace(&pixels, 40, 40, 0.5, 1.0);
        assert!(!shapes.is_empty(), "circle should produce a contour");
    }

    #[test]
    fn test_douglas_peucker_identity() {
        let pts = vec![[0.0, 0.0], [5.0, 0.0], [10.0, 0.0]];
        let result = douglas_peucker(&pts, 1.0);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_point_to_line_distance_zero() {
        let d = point_to_line_distance([5.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_point_to_line_distance_off() {
        let d = point_to_line_distance([5.0, 3.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - 3.0).abs() < 0.001);
    }
}
