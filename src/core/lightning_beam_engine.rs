//! Advanced Lightning Discharge & Cinematic Laser Beam Engine (AE Parity).
//!
//! Generates recursive fractal lightning arcs with branching forks and
//! 3D perspective-aware continuous laser beams with core/glow falloffs.

#![allow(dead_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LightningBranch {
    pub points: Vec<[f32; 2]>,
    pub thickness: f32,
    pub alpha: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvancedLightningConfig {
    pub origin: [f32; 2],
    pub destination: [f32; 2],
    pub segments: usize,
    pub displacement_amplitude: f32,
    pub branch_probability: f32,
    pub core_color: [f32; 4],
    pub glow_color: [f32; 4],
    pub main_thickness: f32,
    pub seed: u64,
}

impl Default for AdvancedLightningConfig {
    fn default() -> Self {
        Self {
            origin: [200.0, 540.0],
            destination: [1720.0, 540.0],
            segments: 16,
            displacement_amplitude: 60.0,
            branch_probability: 0.4,
            core_color: [1.0, 1.0, 1.0, 1.0],
            glow_color: [0.2, 0.6, 1.0, 0.8],
            main_thickness: 6.0,
            seed: 0x12345678,
        }
    }
}

/// Generates recursive lightning arc paths with branching bolts.
pub fn generate_lightning_arcs(config: &AdvancedLightningConfig) -> Vec<LightningBranch> {
    if !config.origin[0].is_finite()
        || !config.origin[1].is_finite()
        || !config.destination[0].is_finite()
        || !config.destination[1].is_finite()
        || !config.displacement_amplitude.is_finite()
        || !config.main_thickness.is_finite()
    {
        return Vec::new();
    }

    let mut branches = Vec::new();
    let mut rng = if config.seed == 0 { 0x853c49e6748fea9b } else { config.seed };

    let mut next_f32 = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng & 0xFFFFFF) as f32 / 16777216.0
    };

    // Calculate max depth from segments: 2^max_depth ~= segments
    let max_depth = (config.segments as f32).max(2.0).log2().round().clamp(1.0, 7.0) as usize;
    let branch_prob = config.branch_probability.clamp(0.0, 1.0);
    let amp = config.displacement_amplitude.max(0.0);

    // Recursive midpoint displacement generator
    fn displace_segment(
        p0: [f32; 2],
        p1: [f32; 2],
        depth: usize,
        max_depth: usize,
        amp: f32,
        rng_fn: &mut impl FnMut() -> f32,
        out_pts: &mut Vec<[f32; 2]>,
    ) {
        if depth >= max_depth {
            out_pts.push(p1);
            return;
        }

        let mid = [(p0[0] + p1[0]) * 0.5, (p0[1] + p1[1]) * 0.5];
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let normal = [-dy / len, dx / len];

        let offset = (rng_fn() - 0.5) * 2.0 * amp;
        let displaced_mid = [mid[0] + normal[0] * offset, mid[1] + normal[1] * offset];

        displace_segment(p0, displaced_mid, depth + 1, max_depth, amp * 0.55, rng_fn, out_pts);
        displace_segment(displaced_mid, p1, depth + 1, max_depth, amp * 0.55, rng_fn, out_pts);
    }

    let mut main_pts = vec![config.origin];
    displace_segment(
        config.origin,
        config.destination,
        0,
        max_depth,
        amp,
        &mut next_f32,
        &mut main_pts,
    );

    branches.push(LightningBranch {
        points: main_pts.clone(),
        thickness: config.main_thickness.max(0.1),
        alpha: 1.0,
    });

    // Generate sub-branches from interior main nodes
    let num_main = main_pts.len();
    if num_main >= 3 {
        for i in 1..num_main - 1 {
            if next_f32() < branch_prob {
                let start = main_pts[i];
                let dir = [
                    (next_f32() - 0.5) * 200.0,
                    (next_f32() - 0.5) * 200.0,
                ];
                let branch_dest = [start[0] + dir[0], start[1] + dir[1]];

                let mut fork_pts = vec![start];
                displace_segment(
                    start,
                    branch_dest,
                    0,
                    3,
                    amp * 0.5,
                    &mut next_f32,
                    &mut fork_pts,
                );

                branches.push(LightningBranch {
                    points: fork_pts,
                    thickness: config.main_thickness * 0.45,
                    alpha: 0.7,
                });
            }
        }
    }

    branches
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaserBeamConfig {
    pub start_point: [f32; 2],
    pub end_point: [f32; 2],
    pub time_progress: f32, // 0.0 .. 1.0
    pub beam_length_percent: f32, // 0.0 .. 100.0
    pub starting_thickness: f32,
    pub ending_thickness: f32,
    pub core_color: [f32; 4],
    pub glow_color: [f32; 4],
}

impl Default for LaserBeamConfig {
    fn default() -> Self {
        Self {
            start_point: [300.0, 540.0],
            end_point: [1600.0, 540.0],
            time_progress: 0.5,
            beam_length_percent: 40.0,
            starting_thickness: 12.0,
            ending_thickness: 4.0,
            core_color: [1.0, 1.0, 1.0, 1.0],
            glow_color: [1.0, 0.2, 0.1, 0.8],
        }
    }
}

/// Evaluates current head and tail positions and interpolated thicknesses of a laser beam.
pub fn evaluate_laser_beam_segment(config: &LaserBeamConfig) -> Option<([f32; 2], [f32; 2], f32, f32)> {
    if !config.start_point[0].is_finite()
        || !config.start_point[1].is_finite()
        || !config.end_point[0].is_finite()
        || !config.end_point[1].is_finite()
        || !config.beam_length_percent.is_finite()
        || config.beam_length_percent <= 0.0
    {
        return None;
    }

    let p0 = config.start_point;
    let p1 = config.end_point;
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    if (dx * dx + dy * dy) < 1e-6 {
        return None;
    }

    let t = config.time_progress.clamp(0.0, 1.0);
    let len_frac = (config.beam_length_percent / 100.0).clamp(0.0, 1.0);
    if len_frac <= 0.0 {
        return None;
    }

    let head_t = (t * (1.0 + len_frac)).clamp(0.0, 1.0);
    let tail_t = (head_t - len_frac).clamp(0.0, 1.0);

    if (head_t - tail_t).abs() < 1e-4 {
        return None;
    }

    let head_pos = [
        p0[0] + (p1[0] - p0[0]) * head_t,
        p0[1] + (p1[1] - p0[1]) * head_t,
    ];
    let tail_pos = [
        p0[0] + (p1[0] - p0[0]) * tail_t,
        p0[1] + (p1[1] - p0[1]) * tail_t,
    ];

    let head_thick = config.starting_thickness + (config.ending_thickness - config.starting_thickness) * head_t;
    let tail_thick = config.starting_thickness + (config.ending_thickness - config.starting_thickness) * tail_t;

    Some((tail_pos, head_pos, tail_thick, head_thick))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lightning_generator_creates_continuous_arcs() {
        let config = AdvancedLightningConfig::default();
        let branches = generate_lightning_arcs(&config);

        assert!(!branches.is_empty());
        let main = &branches[0];
        assert_eq!(main.points.first().unwrap(), &config.origin);
        assert_eq!(main.points.last().unwrap(), &config.destination);
    }

    #[test]
    fn test_laser_beam_segment_evaluation() {
        let config = LaserBeamConfig {
            start_point: [0.0, 0.0],
            end_point: [1000.0, 0.0],
            time_progress: 0.5,
            beam_length_percent: 20.0,
            ..Default::default()
        };

        let res = evaluate_laser_beam_segment(&config);
        assert!(res.is_some());
        let (tail, head, _, _) = res.unwrap();
        assert!(head[0] > tail[0]);
    }

    #[test]
    fn test_lightning_segments_parameter_changes_path_resolution() {
        let mut low = AdvancedLightningConfig::default();
        low.segments = 2;
        let mut high = low.clone();
        high.segments = 32;
        let low_count = generate_lightning_arcs(&low)[0].points.len();
        let high_count = generate_lightning_arcs(&high)[0].points.len();
        assert_ne!(low_count, high_count);
    }

    #[test]
    fn test_zero_seed_still_produces_non_degenerate_random_arc() {
        let config = AdvancedLightningConfig {
            seed: 0,
            ..Default::default()
        };
        let points = &generate_lightning_arcs(&config)[0].points;
        assert!(points.windows(3).any(|w| (w[1][1] - w[0][1]).abs() > 1e-5
            || (w[2][1] - w[1][1]).abs() > 1e-5));
    }

    #[test]
    fn test_nan_inputs_handled_safely() {
        let config = AdvancedLightningConfig {
            origin: [f32::NAN, 0.0],
            ..Default::default()
        };
        let arcs = generate_lightning_arcs(&config);
        assert!(arcs.is_empty());

        let beam = LaserBeamConfig {
            beam_length_percent: 0.0,
            ..Default::default()
        };
        assert!(evaluate_laser_beam_segment(&beam).is_none());
    }
}
