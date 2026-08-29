//! The Wiggler: Procedural Noise Keyframe Generator (AE Parity).
//!
//! Generates baked keyframe motion across a layer property using Perlin (Smooth)
//! or Uniform (Jagged) noise at a given frequency and magnitude.

use crate::core::keyframe::{Keyframe, InterpolationType, BezierControlPoint};
use crate::core::turbulent_displace::perlin_noise_2d;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiggleNoiseType {
    Smooth,
    Jagged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiggleDimension {
    AllSame,
    AllIndependent,
}

/// Generate 2D vector wiggle keyframes
pub fn generate_wiggle_vec2(
    base_val: [f32; 2],
    start_frame: u32,
    end_frame: u32,
    fps: u32,
    frequency: f32,
    magnitude: f32,
    noise_type: WiggleNoiseType,
    dimensions: WiggleDimension,
    seed: u32,
) -> Vec<Keyframe<[f32; 2]>> {
    let mut keyframes = Vec::new();
    let frame_interval = (fps as f32 / frequency.max(0.1)).round().max(1.0) as u32;

    let mut f = start_frame;
    while f <= end_frame {
        let t = f as f32 / fps.max(1) as f32;
        let (offset_x, offset_y) = match noise_type {
            WiggleNoiseType::Smooth => {
                let nx = (perlin_noise_2d((t * frequency + seed as f32 * 10.0) as f64, 0.0) as f32) * magnitude;
                let ny = match dimensions {
                    WiggleDimension::AllSame => nx,
                    WiggleDimension::AllIndependent => {
                        (perlin_noise_2d((t * frequency + (seed + 1) as f32 * 10.0) as f64, 5.0) as f32) * magnitude
                    }
                };
                (nx, ny)
            }
            WiggleNoiseType::Jagged => {
                let pseudo_rand = |seed_offset: u32| -> f32 {
                    let h = (f.wrapping_mul(374761393) ^ seed.wrapping_add(seed_offset).wrapping_mul(668265263))
                        .wrapping_mul(1274126177);
                    let norm = (h >> 16) as f32 / 65535.0;
                    (norm * 2.0 - 1.0) * magnitude
                };
                let rx = pseudo_rand(0);
                let ry = match dimensions {
                    WiggleDimension::AllSame => rx,
                    WiggleDimension::AllIndependent => pseudo_rand(1),
                };
                (rx, ry)
            }
        };

        let interp = match noise_type {
            WiggleNoiseType::Smooth => InterpolationType::Bezier {
                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
            },
            WiggleNoiseType::Jagged => InterpolationType::Linear,
        };

        keyframes.push(Keyframe::new(f, [base_val[0] + offset_x, base_val[1] + offset_y], interp));
        f += frame_interval;
    }

    keyframes
}

/// Generate 1D scalar wiggle keyframes
pub fn generate_wiggle_scalar(
    base_val: f32,
    start_frame: u32,
    end_frame: u32,
    fps: u32,
    frequency: f32,
    magnitude: f32,
    noise_type: WiggleNoiseType,
    seed: u32,
) -> Vec<Keyframe<f32>> {
    let mut keyframes = Vec::new();
    let frame_interval = (fps as f32 / frequency.max(0.1)).round().max(1.0) as u32;

    let mut f = start_frame;
    while f <= end_frame {
        let t = f as f32 / fps.max(1) as f32;
        let offset = match noise_type {
            WiggleNoiseType::Smooth => {
                (perlin_noise_2d((t * frequency + seed as f32 * 10.0) as f64, 0.0) as f32) * magnitude
            }
            WiggleNoiseType::Jagged => {
                let h = (f.wrapping_mul(374761393) ^ seed.wrapping_mul(668265263))
                    .wrapping_mul(1274126177);
                let norm = (h >> 16) as f32 / 65535.0;
                (norm * 2.0 - 1.0) * magnitude
            }
        };

        let interp = match noise_type {
            WiggleNoiseType::Smooth => InterpolationType::Bezier {
                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
            },
            WiggleNoiseType::Jagged => InterpolationType::Linear,
        };

        keyframes.push(Keyframe::new(f, base_val + offset, interp));
        f += frame_interval;
    }

    keyframes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiggler_generates_bounded_keyframes() {
        let kfs = generate_wiggle_vec2(
            [100.0, 200.0],
            0,
            60,
            30,
            5.0,
            20.0,
            WiggleNoiseType::Smooth,
            WiggleDimension::AllIndependent,
            42,
        );

        assert!(!kfs.is_empty());
        for k in &kfs {
            assert!((k.value[0] - 100.0).abs() <= 20.0 + 1e-4);
            assert!((k.value[1] - 200.0).abs() <= 20.0 + 1e-4);
        }
    }
}
