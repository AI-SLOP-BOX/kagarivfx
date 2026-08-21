#![allow(dead_code)]
use crate::core::mask::MaskVertex;

/// Wiggle Paths modifier options matching After Effects Shape Wiggle Paths contents.
#[derive(Debug, Clone)]
pub struct WigglePathsOptions {
    pub size: f32,             // Max pixel displacement radius
    pub detail: f32,           // Number of wiggles per segment
    pub wiggles_per_sec: f32,  // Frequency in Hz
    pub correlation: f32,       // Spatial smoothness between neighboring vertices (0.0 .. 1.0)
    pub smooth: bool,          // True for smooth curves, false for sharp/corner wiggles
}

impl Default for WigglePathsOptions {
    fn default() -> Self {
        Self {
            size: 10.0,
            detail: 2.0,
            wiggles_per_sec: 2.0,
            correlation: 0.5,
            smooth: true,
        }
    }
}

/// Simple deterministic 2D Pseudo-Random Noise generator for procedural spatial deformation.
fn pseudo_noise_2d(x: f32, y: f32) -> f32 {
    let dot = x * 12.9898 + y * 78.233;
    (dot.sin() * 43_758.547).fract() * 2.0 - 1.0
}

/// Applies real-time noise perturbation to Bezier vertices for organic Wiggle Paths animation.
pub fn apply_wiggle_paths(
    vertices: &[MaskVertex],
    time_sec: f32,
    options: &WigglePathsOptions,
) -> Vec<MaskVertex> {
    if vertices.is_empty() || options.size <= 0.001 {
        return vertices.to_vec();
    }

    let mut wiggled = Vec::with_capacity(vertices.len());
    let t_sample = time_sec * options.wiggles_per_sec;

    for (idx, vertex) in vertices.iter().enumerate() {
        let i_f = idx as f32;

        // Compute pseudo-random 2D offset vector
        let nx = pseudo_noise_2d(i_f * options.detail, t_sample);
        let ny = pseudo_noise_2d(i_f * options.detail + 100.0, t_sample + 50.0);

        let dx = nx * options.size;
        let dy = ny * options.size;

        let mut v_new = vertex.clone();
        v_new.position[0] += dx;
        v_new.position[1] += dy;

        if !options.smooth {
            v_new.tangent_in = [0.0, 0.0];
            v_new.tangent_out = [0.0, 0.0];
        }

        wiggled.push(v_new);
    }

    wiggled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiggle_paths_displacement() {
        let vertices = vec![
            MaskVertex::new(0.0, 0.0),
            MaskVertex::new(100.0, 0.0),
        ];

        let options = WigglePathsOptions {
            size: 20.0,
            detail: 1.0,
            wiggles_per_sec: 2.0,
            correlation: 0.5,
            smooth: true,
        };

        let wiggled = apply_wiggle_paths(&vertices, 1.0, &options);
        assert_eq!(wiggled.len(), 2);
        assert!((wiggled[0].position[0] - 0.0).abs() > 0.001 || (wiggled[0].position[1] - 0.0).abs() > 0.001);
    }
}
