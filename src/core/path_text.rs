#![allow(dead_code)]
use crate::core::mask::MaskVertex;

/// Alignment settings for text placed along a path (AE Path Text options).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathTextAlignment {
    Left,
    Center,
    Right,
    ForceJustify,
}

/// Options for layout of text glyphs along Bezier spline paths.
#[derive(Debug, Clone)]
pub struct PathTextOptions {
    pub first_margin: f32,
    pub last_margin: f32,
    pub force_alignment: bool,
    pub alignment: PathTextAlignment,
    pub perpendicular_to_path: bool,
    pub reverse_path: bool,
}

impl Default for PathTextOptions {
    fn default() -> Self {
        Self {
            first_margin: 0.0,
            last_margin: 0.0,
            force_alignment: false,
            alignment: PathTextAlignment::Left,
            perpendicular_to_path: true,
            reverse_path: false,
        }
    }
}

/// A positioned text character along a 2D Bezier path.
#[derive(Debug, Clone)]
pub struct PlacedGlyph {
    pub char_code: char,
    pub position: [f32; 2],
    pub tangent: [f32; 2],
    pub normal: [f32; 2],
    pub rotation_deg: f32,
}

/// Computes arc-length of cubic Bezier curve using 5-point Gauss-Legendre Quadrature for high precision.
pub fn calculate_bezier_arc_length(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> f32 {
    // Gauss-Legendre 5-point quadrature weights and nodes on [-1, 1]
    let nodes = [
        -0.9061798459, -0.5384693101, 0.0, 0.5384693101, 0.9061798459,
    ];
    let weights = [
        0.2369268851, 0.4786286705, 0.5688888889, 0.4786286705, 0.2369268851,
    ];

    let mut length = 0.0f32;

    for i in 0..5 {
        let t = ((nodes[i] + 1.0) * 0.5) as f32; // Map to [0, 1]

        // Derivative dP/dt of cubic Bezier
        let inv_t = 1.0 - t;
        let d0 = 3.0 * inv_t * inv_t * (p1[0] - p0[0]) + 6.0 * inv_t * t * (p2[0] - p1[0]) + 3.0 * t * t * (p3[0] - p2[0]);
        let d1 = 3.0 * inv_t * inv_t * (p1[1] - p0[1]) + 6.0 * inv_t * t * (p2[1] - p1[1]) + 3.0 * t * t * (p3[1] - p2[1]);

        let speed = (d0 * d0 + d1 * d1).sqrt();
        length += speed * weights[i] as f32;
    }

    length * 0.5
}

/// Evaluates point, tangent, and normal along cubic Bezier segment at parameter t in [0.0, 1.0].
pub fn sample_cubic_bezier(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    t: f32,
) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let t = t.clamp(0.0, 1.0);
    let inv_t = 1.0 - t;

    let b0 = inv_t * inv_t * inv_t;
    let b1 = 3.0 * inv_t * inv_t * t;
    let b2 = 3.0 * inv_t * t * t;
    let b3 = t * t * t;

    let pos = [
        b0 * p0[0] + b1 * p1[0] + b2 * p2[0] + b3 * p3[0],
        b0 * p0[1] + b1 * p1[1] + b2 * p2[1] + b3 * p3[1],
    ];

    let d0 = 3.0 * inv_t * inv_t * (p1[0] - p0[0]) + 6.0 * inv_t * t * (p2[0] - p1[0]) + 3.0 * t * t * (p3[0] - p2[0]);
    let d1 = 3.0 * inv_t * inv_t * (p1[1] - p0[1]) + 6.0 * inv_t * t * (p2[1] - p1[1]) + 3.0 * t * t * (p3[1] - p2[1]);

    let speed = (d0 * d0 + d1 * d1).sqrt().max(0.0001);
    let tangent = [d0 / speed, d1 / speed];
    let normal = [-tangent[1], tangent[0]];

    (pos, tangent, normal)
}

/// Places text characters smoothly along a set of Bezier mask path vertices.
pub fn layout_text_along_path(
    text: &str,
    font_size: f32,
    vertices: &[MaskVertex],
    closed: bool,
    options: &PathTextOptions,
) -> Vec<PlacedGlyph> {
    if text.is_empty() || vertices.len() < 2 {
        return Vec::new();
    }

    let mut path_vertices = vertices.to_vec();
    if options.reverse_path {
        path_vertices.reverse();
    }

    // Step 1: Compute cumulative arc-lengths for all Bezier segments
    let num_segments = if closed { path_vertices.len() } else { path_vertices.len() - 1 };
    let mut total_path_length = 0.0f32;
    let mut segment_lengths = Vec::with_capacity(num_segments);

    for i in 0..num_segments {
        let v0 = &path_vertices[i];
        let v1 = &path_vertices[(i + 1) % path_vertices.len()];

        let p0 = v0.position;
        let p1 = [p0[0] + v0.tangent_out[0], p0[1] + v0.tangent_out[1]];
        let p3 = v1.position;
        let p2 = [p3[0] + v1.tangent_in[0], p3[1] + v1.tangent_in[1]];

        let seg_len = calculate_bezier_arc_length(p0, p1, p2, p3);
        segment_lengths.push(seg_len);
        total_path_length += seg_len;
    }

    if total_path_length <= 0.001 {
        return Vec::new();
    }

    // Step 2: Lay out glyphs along total arc-length
    let char_width = font_size * 0.6; // Average glyph spacing estimate
    let mut glyphs = Vec::new();
    let mut current_dist = options.first_margin;

    for ch in text.chars() {
        if current_dist > total_path_length {
            break;
        }

        // Find which Bezier segment contains current_dist using arc-length search
        let mut accum_d = 0.0f32;
        for i in 0..num_segments {
            let seg_len = segment_lengths[i];
            if current_dist <= accum_d + seg_len || i == num_segments - 1 {
                let local_d = (current_dist - accum_d).clamp(0.0, seg_len);
                let seg_t = if seg_len > 0.0001 { local_d / seg_len } else { 0.0 };

                let v0 = &path_vertices[i];
                let v1 = &path_vertices[(i + 1) % path_vertices.len()];

                let p0 = v0.position;
                let p1 = [p0[0] + v0.tangent_out[0], p0[1] + v0.tangent_out[1]];
                let p3 = v1.position;
                let p2 = [p3[0] + v1.tangent_in[0], p3[1] + v1.tangent_in[1]];

                let (pos, tangent, normal) = sample_cubic_bezier(p0, p1, p2, p3, seg_t);
                let rot_deg = if options.perpendicular_to_path {
                    tangent[1].atan2(tangent[0]).to_degrees()
                } else {
                    0.0
                };

                glyphs.push(PlacedGlyph {
                    char_code: ch,
                    position: pos,
                    tangent,
                    normal,
                    rotation_deg: rot_deg,
                });
                break;
            }
            accum_d += seg_len;
        }

        current_dist += char_width;
    }

    glyphs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_text_along_straight_path() {
        let vertices = vec![
            MaskVertex::new(0.0, 0.0),
            MaskVertex::new(500.0, 0.0),
        ];

        let options = PathTextOptions::default();
        let glyphs = layout_text_along_path("AURA", 24.0, &vertices, false, &options);

        assert_eq!(glyphs.len(), 4);
        assert_eq!(glyphs[0].position[0], 0.0);
        assert!((glyphs[0].rotation_deg - 0.0).abs() < 0.1);
    }
}
