#![allow(dead_code)]
use crate::core::mask::MaskVertex;

/// Pucker & Bloat options matching After Effects Shape Pucker & Bloat modifier.
#[derive(Debug, Clone)]
pub struct PuckerBloatOptions {
    pub amount: f32, // Percentage (-100.0 = full Pucker, +100.0 = full Bloat)
}

/// Zig Zag options matching After Effects Shape Zig Zag modifier.
#[derive(Debug, Clone)]
pub struct ZigZagOptions {
    pub size: f32,               // Ridge amplitude in pixels
    pub ridges_per_segment: u32, // Number of ridges per path segment
    pub smooth: bool,            // Smooth curve vs sharp corner points
}

/// Applies Pucker & Bloat distortion to a vector shape path.
pub fn apply_pucker_bloat(
    vertices: &[MaskVertex],
    options: &PuckerBloatOptions,
) -> Vec<MaskVertex> {
    if vertices.is_empty() || options.amount.abs() < 0.001 {
        return vertices.to_vec();
    }

    // Compute centroid of shape vertices
    let mut center = [0.0f32, 0.0f32];
    for v in vertices {
        center[0] += v.position[0];
        center[1] += v.position[1];
    }
    let n = vertices.len() as f32;
    center[0] /= n;
    center[1] /= n;

    let factor = options.amount * 0.01;
    let mut modified = Vec::with_capacity(vertices.len());

    for v in vertices {
        let mut v_new = v.clone();

        // Vector from center to vertex position
        let vx = v.position[0] - center[0];
        let vy = v.position[1] - center[1];

        // Shift vertex towards or away from center
        v_new.position[0] += vx * factor;
        v_new.position[1] += vy * factor;

        // Scale Bezier tangents inversely for Pucker / Bloat organic curve bulging
        v_new.tangent_in[0] *= 1.0 - factor;
        v_new.tangent_in[1] *= 1.0 - factor;
        v_new.tangent_out[0] *= 1.0 - factor;
        v_new.tangent_out[1] *= 1.0 - factor;

        modified.push(v_new);
    }

    modified
}

/// Applies Zig Zag (sawtooth / wave) geometric sub-division distortion to a Bezier path.
pub fn apply_zig_zag(
    vertices: &[MaskVertex],
    options: &ZigZagOptions,
) -> Vec<MaskVertex> {
    if vertices.len() < 2 || options.ridges_per_segment == 0 || options.size <= 0.001 {
        return vertices.to_vec();
    }

    let mut result = Vec::new();
    let ridges = options.ridges_per_segment as usize;

    for i in 0..vertices.len() - 1 {
        let v0 = &vertices[i];
        let v1 = &vertices[i + 1];

        result.push(v0.clone());

        let dx = v1.position[0] - v0.position[0];
        let dy = v1.position[1] - v0.position[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);

        let normal = [-dy / len, dx / len];

        for r in 1..ridges {
            let t = r as f32 / ridges as f32;
            let px = v0.position[0] + dx * t;
            let py = v0.position[1] + dy * t;

            let side = if r % 2 == 1 { 1.0 } else { -1.0 };
            let offset_x = normal[0] * options.size * side;
            let offset_y = normal[1] * options.size * side;

            let mut ridge_vertex = MaskVertex::new(px + offset_x, py + offset_y);
            if !options.smooth {
                ridge_vertex.tangent_in = [0.0, 0.0];
                ridge_vertex.tangent_out = [0.0, 0.0];
            }
            result.push(ridge_vertex);
        }
    }

    if let Some(last) = vertices.last() {
        result.push(last.clone());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pucker_bloat_displacement() {
        let vertices = vec![
            MaskVertex::new(-10.0, -10.0),
            MaskVertex::new(10.0, 10.0),
        ];

        let options = PuckerBloatOptions { amount: 50.0 };
        let bloat = apply_pucker_bloat(&vertices, &options);

        assert_eq!(bloat.len(), 2);
        assert!(bloat[1].position[0] > 10.0);
    }
}
