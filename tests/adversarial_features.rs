use aftereffects_oss::core::advanced_motion_blur::*;
use aftereffects_oss::core::advanced_particles_3d::*;
use aftereffects_oss::core::audio_dsp::*;
use aftereffects_oss::core::bend_warp_engine::*;
use aftereffects_oss::core::calculations_composite::*;
use aftereffects_oss::core::camera_tracker_3d::*;
use aftereffects_oss::core::cylinder_sphere_warp::*;
use aftereffects_oss::core::icc_color_engine::*;
use aftereffects_oss::core::lightning_beam_engine::*;
use aftereffects_oss::core::mask::point_in_polygon;
use aftereffects_oss::core::obj_loader::*;
use aftereffects_oss::core::optical_flow_timewarp::*;
use aftereffects_oss::core::roto_brush_engine::*;
use aftereffects_oss::core::shape_boolean::*;

fn finite_points(branches: &[LightningBranch]) -> bool {
    branches
        .iter()
        .flat_map(|b| b.points.iter())
        .flatten()
        .all(|v| v.is_finite())
}

#[test]
fn lightning_is_deterministic_for_same_seed() {
    let config = AdvancedLightningConfig::default();
    let a = generate_lightning_arcs(&config);
    let b = generate_lightning_arcs(&config);
    assert_eq!(a.len(), b.len());
    assert_eq!(a[0].points, b[0].points);
}

#[test]
fn lightning_rejects_non_finite_configuration_output() {
    let config = AdvancedLightningConfig {
        displacement_amplitude: f32::NAN,
        ..Default::default()
    };
    assert!(finite_points(&generate_lightning_arcs(&config)));
}

#[test]
fn laser_zero_length_is_empty() {
    let config = LaserBeamConfig {
        start_point: [4.0, 4.0],
        end_point: [4.0, 4.0],
        ..Default::default()
    };
    assert!(evaluate_laser_beam_segment(&config).is_none());
}

#[test]
fn particles_are_finite_after_extreme_but_valid_step() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 20.0,
        initial_speed: 100.0,
        gravity: [0.0, -1000.0, 0.0],
        ..Default::default()
    };
    sim.update(0.25, &config);
    assert!(sim.particles.iter().all(|p| {
        p.position
            .iter()
            .chain(p.velocity.iter())
            .all(|v| v.is_finite())
    }));
}

#[test]
fn particles_with_zero_lifespan_do_not_survive() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        lifespan_sec: 0.0,
        birth_rate_per_sec: 100.0,
        ..Default::default()
    };
    sim.update(0.1, &config);
    assert!(sim.particles.is_empty());
}

#[test]
fn particles_with_zero_length_collision_normal_are_safe() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 1.0,
        initial_speed: 0.0,
        collision_planes: vec![CollisionPlane {
            origin: [0.0; 3],
            normal: [0.0; 3],
            bounce_restitution: 1.0,
            friction: 0.0,
        }],
        ..Default::default()
    };
    sim.update(1.0, &config);
    assert!(sim
        .particles
        .iter()
        .all(|p| p.position.iter().all(|v| v.is_finite())));
}

#[test]
fn obj_quad_triangulation_has_valid_indices() {
    let mesh = parse_obj_str("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
    assert_eq!(mesh.indices.len(), 6);
    assert!(mesh
        .indices
        .iter()
        .all(|i| (*i as usize) < mesh.vertices.len()));
}

#[test]
fn obj_malformed_numeric_vertex_is_rejected() {
    assert!(parse_obj_str("v nope 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").is_err());
}

#[test]
fn color_conversion_is_monotonic() {
    let mut previous = 0.0;
    for value in 0..=255u8 {
        let src = [value, value, value, 255];
        let mut linear = [0.0; 4];
        convert_rgba8_to_rgba32f(&src, &mut linear);
        assert!(linear[0] >= previous);
        assert_eq!(linear[0], linear[1]);
        assert_eq!(linear[1], linear[2]);
        assert_eq!(linear[3], 1.0);
        previous = linear[0];
    }
}

#[test]
fn color_conversion_rejects_nan_on_output() {
    let mut out = [0u8; 3];
    convert_rgba32f_to_rgba8(&[f32::NAN, f32::INFINITY, f32::NEG_INFINITY], &mut out);
    assert_eq!(out, [0, 255, 0]);
}

#[test]
fn roto_empty_frame_has_expected_size() {
    let result = generate_rotobrush_matte(&[], 0, 0, &[], &Default::default());
    assert!(result.is_empty());
}

#[test]
fn roto_output_is_binary_for_binary_input() {
    let pixels = vec![0u8; 4 * 4 * 4];
    let stroke = RotoStroke {
        stroke_type: RotoStrokeType::Background,
        points: vec![[1.0, 1.0]],
        radius: 1.0,
    };
    let result = generate_rotobrush_matte(&pixels, 4, 4, &[stroke], &Default::default());
    assert_eq!(result.len(), 16);
    assert!(result.iter().all(|v| *v == 0 || *v == 255));
}

#[test]
fn polygon_intersection_is_inside_both_inputs() {
    let a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];
    let result = polygon_intersect(&a, &b);
    assert!(result.iter().all(|p| p
        .iter()
        .all(|q| point_in_polygon(q[0], q[1], &a) && point_in_polygon(q[0], q[1], &b))));
}

#[test]
fn polygon_subtract_does_not_fill_cutout() {
    let a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];
    let result = polygon_subtract(&a, &b);
    assert!(!result.iter().any(|p| point_in_polygon(10.0, 10.0, p)));
}

#[test]
fn audio_keyframes_have_one_entry_per_requested_frame() {
    let options = AudioKeyframeOptions::default();
    let result = extract_multiband_audio_keyframes(&[0.2; 100], 48_000, 24, 17, &options);
    assert_eq!(result.master.len(), 17);
    assert_eq!(result.bass.len(), 17);
    assert_eq!(result.mid.len(), 17);
    assert_eq!(result.treble.len(), 17);
}

#[test]
fn motion_blur_weights_are_normalized_and_times_are_finite() {
    let settings = SubframeMotionBlurSettings::default();
    let samples = evaluate_subframe_samples(100, 30, &settings);
    let sum: f32 = samples.iter().map(|s| s.weight).sum();
    assert!((sum - 1.0).abs() < 1e-5);
    assert!(samples
        .iter()
        .all(|s| s.subframe_time_sec.is_finite() && s.weight.is_finite()));
}

#[test]
fn motion_blur_negative_weights_cannot_darken_or_invert_pixels() {
    let mut output = vec![0u8; 4];
    accumulate_motion_blur_buffers(&[(&[100, 100, 100, 255], -1.0)], 1, 1, &mut output);
    assert_eq!(output, vec![0, 0, 0, 0]);
}

#[test]
fn motion_blur_zero_fps_does_not_create_infinite_time() {
    let samples = evaluate_subframe_samples(1, 0, &SubframeMotionBlurSettings::default());
    assert!(samples.iter().all(|s| s.subframe_time_sec.is_finite()));
}

#[test]
fn bend_zero_amount_preserves_every_pixel_including_edges() {
    let w = 5u32;
    let h = 5u32;
    let src: Vec<u8> = (0..w * h * 4).map(|v| (v % 251) as u8).collect();
    let mut dst = vec![0u8; src.len()];
    apply_bend_warp(
        &src,
        w,
        h,
        &mut dst,
        &BendWarpConfig {
            bend_amount: 0.0,
            ..Default::default()
        },
    );
    assert_eq!(dst, src);
}

#[test]
fn bend_warp_rejects_mismatched_buffers_without_writing() {
    let src = vec![7u8; 16];
    let mut dst = vec![9u8; 15];
    apply_bend_warp(&src, 2, 2, &mut dst, &BendWarpConfig::default());
    assert!(dst.iter().all(|v| *v == 9));
}

#[test]
fn bend_warp_output_is_finite_and_same_size_for_all_modes() {
    for warp_type in [BendWarpType::Bend, BendWarpType::Pinch, BendWarpType::Twist] {
        let src = vec![128u8; 16 * 16 * 4];
        let mut dst = vec![0u8; src.len()];
        apply_bend_warp(
            &src,
            16,
            16,
            &mut dst,
            &BendWarpConfig {
                warp_type,
                ..Default::default()
            },
        );
        assert_eq!(dst.len(), src.len());
        assert_eq!(dst.len(), 16 * 16 * 4);
    }
}

#[test]
fn lightning_branch_probability_is_bounded() {
    let config = AdvancedLightningConfig {
        branch_probability: 1000.0,
        ..Default::default()
    };
    let branches = generate_lightning_arcs(&config);
    assert!(branches.len() <= config.segments.max(1));
}

#[test]
fn particle_update_with_negative_dt_does_not_age_particles_backwards() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 10.0,
        ..Default::default()
    };
    sim.update(0.1, &config);
    let before = sim.particles.iter().map(|p| p.age_sec).collect::<Vec<_>>();
    sim.update(-1.0, &config);
    assert!(sim
        .particles
        .iter()
        .enumerate()
        .all(|(i, p)| p.age_sec >= before.get(i).copied().unwrap_or(0.0)));
}

#[test]
fn obj_parser_preserves_uv_and_normal_cardinality() {
    let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 3/3/1\n";
    let mesh = parse_obj_str(obj).unwrap();
    assert_eq!(mesh.uvs.len(), mesh.vertices.len());
    assert_eq!(mesh.normals.len(), mesh.vertices.len());
}

#[test]
fn lightning_seeds_produce_repeatable_but_distinct_paths() {
    let a = generate_lightning_arcs(&AdvancedLightningConfig {
        seed: 1,
        ..Default::default()
    });
    let b = generate_lightning_arcs(&AdvancedLightningConfig {
        seed: 2,
        ..Default::default()
    });
    assert_ne!(a[0].points, b[0].points);
    assert_eq!(
        a[0].points,
        generate_lightning_arcs(&AdvancedLightningConfig {
            seed: 1,
            ..Default::default()
        })[0]
            .points
    );
}

#[test]
fn laser_segment_stays_between_endpoints() {
    let config = LaserBeamConfig {
        start_point: [-10.0, 5.0],
        end_point: [90.0, 25.0],
        time_progress: 0.7,
        beam_length_percent: 30.0,
        ..Default::default()
    };
    let (tail, head, _, _) = evaluate_laser_beam_segment(&config).unwrap();
    for p in [tail, head] {
        let t = (p[0] - config.start_point[0]) / (config.end_point[0] - config.start_point[0]);
        assert!((0.0..=1.0).contains(&t));
        assert!((p[1] - (config.start_point[1] + t * 20.0)).abs() < 1e-4);
    }
}

#[test]
fn particle_count_is_bounded_for_multiple_small_steps() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 100.0,
        lifespan_sec: 0.2,
        ..Default::default()
    };
    for _ in 0..200 {
        sim.update(0.01, &config);
    }
    assert!(sim.particles.len() <= 100);
}

#[test]
fn particle_sphere_emitter_positions_are_within_radius() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        emitter_type: EmitterType3D::Sphere { radius: 10.0 },
        birth_rate_per_sec: 100.0,
        initial_speed: 0.0,
        gravity: [0.0; 3],
        ..Default::default()
    };
    sim.update(0.2, &config);
    assert!(sim.particles.iter().all(|p| {
        p.position
            .iter()
            .zip(config.position.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            <= 100.001
    }));
}

#[test]
fn obj_empty_and_comments_are_safe() {
    let mesh = parse_obj_str("# only comments\n\n").unwrap();
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
    assert!(mesh.ray_intersect([0.0; 3], [1.0, 0.0, 0.0]).is_none());
}

#[test]
fn color_16_bit_expansion_is_exact() {
    let src = [0u8, 1, 2, 127, 128, 254, 255];
    let mut wide = [0u16; 7];
    let mut back = [0u8; 7];
    convert_rgba8_to_rgba16(&src, &mut wide);
    convert_rgba16_to_rgba8(&wide, &mut back);
    assert_eq!(back, src);
    assert_eq!(wide, [0, 257, 514, 32639, 32896, 65278, 65535]);
}

#[test]
fn roto_matte_is_deterministic() {
    let pixels = (0..(16 * 16 * 4))
        .map(|i| (i % 251) as u8)
        .collect::<Vec<_>>();
    let strokes = vec![RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[4.0, 4.0], [8.0, 8.0]],
        radius: 2.0,
    }];
    let settings = RotoBrushSettings::default();
    assert_eq!(
        generate_rotobrush_matte(&pixels, 16, 16, &strokes, &settings),
        generate_rotobrush_matte(&pixels, 16, 16, &strokes, &settings)
    );
}

#[test]
fn motion_blur_single_buffer_is_identity() {
    let src = vec![37u8; 4 * 4 * 4];
    let mut out = vec![0u8; src.len()];
    accumulate_motion_blur_buffers(&[(&src, 1.0)], 4, 4, &mut out);
    assert_eq!(out, src);
}

#[test]
fn bend_warp_handles_all_small_dimensions() {
    for (w, h) in [(1, 1), (1, 8), (8, 1), (2, 2), (3, 5)] {
        let src = vec![128u8; w * h * 4];
        let mut dst = vec![0u8; src.len()];
        apply_bend_warp(
            &src,
            w as u32,
            h as u32,
            &mut dst,
            &BendWarpConfig::default(),
        );
        assert_eq!(dst.len(), src.len());
    }
}

#[test]
fn empty_audio_returns_empty_keyframe_sets() {
    let result =
        extract_multiband_audio_keyframes(&[], 48_000, 24, 10, &AudioKeyframeOptions::default());
    assert!(result.master.is_empty());
    assert!(result.bass.is_empty());
    assert!(result.mid.is_empty());
    assert!(result.treble.is_empty());
}

#[test]
fn audio_keyframe_values_stay_inside_multiplier_range() {
    let options = AudioKeyframeOptions {
        multiplier: 73.0,
        ..Default::default()
    };
    let result = extract_multiband_audio_keyframes(&[1.0; 4096], 48_000, 24, 10, &options);
    for values in [&result.master, &result.bass, &result.mid, &result.treble] {
        assert!(values.iter().all(|kf| (0.0..=73.0).contains(&kf.value)));
    }
}

#[test]
fn motion_blur_buffer_size_mismatch_is_noop() {
    let mut out = vec![11u8; 16];
    accumulate_motion_blur_buffers(&[(&[1u8; 3], 1.0)], 2, 2, &mut out);
    assert_eq!(out, vec![0u8; 16]);
}

#[test]
fn motion_blur_ignores_invalid_samples_and_keeps_valid_result() {
    let valid = vec![80u8; 4];
    let invalid = vec![200u8; 3];
    let mut out = vec![0u8; 4];
    accumulate_motion_blur_buffers(&[(&invalid, 1.0), (&valid, 1.0)], 1, 1, &mut out);
    assert_eq!(out, valid);
}

#[test]
fn lightning_output_has_endpoints_for_extreme_segment_settings() {
    for segments in [0, 1, 2, 128, 10_000] {
        let config = AdvancedLightningConfig {
            segments,
            ..Default::default()
        };
        let branches = generate_lightning_arcs(&config);
        assert!(!branches.is_empty());
        assert_eq!(branches[0].points.first(), Some(&config.origin));
        assert_eq!(branches[0].points.last(), Some(&config.destination));
    }
}

#[test]
fn particle_zero_dt_does_not_change_existing_state() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 10.0,
        ..Default::default()
    };
    sim.update(0.1, &config);
    let before = sim.particles.clone();
    sim.update(0.0, &config);
    assert_eq!(sim.particles.len(), before.len());
    assert!(sim
        .particles
        .iter()
        .zip(before.iter())
        .all(|(a, b)| a.position == b.position && a.age_sec == b.age_sec));
}

#[test]
fn particle_restitution_and_friction_are_clamped() {
    let mut sim = ParticleSimulation3D::new();
    let config = EmitterConfig3D {
        birth_rate_per_sec: 10.0,
        collision_planes: vec![CollisionPlane {
            origin: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            bounce_restitution: 99.0,
            friction: -99.0,
        }],
        ..Default::default()
    };
    sim.update(0.1, &config);
    assert!(sim
        .particles
        .iter()
        .all(|p| p.velocity.iter().all(|v| v.is_finite())));
}

#[test]
fn obj_normals_are_unit_length_when_generated() {
    let mesh = parse_obj_str("v 0 0 0\nv 3 0 0\nv 0 4 0\nf 1 2 3\n").unwrap();
    assert!(mesh.normals.iter().all(|n| {
        let length = n.iter().map(|v| v * v).sum::<f32>().sqrt();
        (length - 1.0).abs() < 1e-4
    }));
}

#[test]
fn roto_mismatched_input_returns_exact_expected_length() {
    let result = generate_rotobrush_matte(&[1, 2, 3], 10, 10, &[], &Default::default());
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|v| *v == 0));
}

#[test]
fn bend_warp_mismatched_output_does_not_partially_write() {
    let src = vec![1u8; 4 * 4 * 4];
    let mut dst = vec![77u8; 4 * 4 * 4 - 1];
    apply_bend_warp(&src, 4, 4, &mut dst, &BendWarpConfig::default());
    assert!(dst.iter().all(|v| *v == 77));
}

macro_rules! contract_test {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() $body
    };
}

contract_test!(lightning_default_has_main_branch, {
    assert!(!generate_lightning_arcs(&AdvancedLightningConfig::default()).is_empty());
});
contract_test!(lightning_negative_amplitude_stays_finite, {
    let c = AdvancedLightningConfig {
        displacement_amplitude: -100.0,
        ..Default::default()
    };
    assert!(finite_points(&generate_lightning_arcs(&c)));
});
contract_test!(lightning_zero_branch_probability_has_one_branch, {
    let c = AdvancedLightningConfig {
        branch_probability: 0.0,
        ..Default::default()
    };
    assert_eq!(generate_lightning_arcs(&c).len(), 1);
});
contract_test!(laser_start_progress_has_nonnegative_length, {
    let c = LaserBeamConfig {
        time_progress: 0.0,
        ..Default::default()
    };
    let (tail, head, _, _) = evaluate_laser_beam_segment(&c).unwrap();
    assert!(head != tail);
});
contract_test!(laser_end_progress_has_nonnegative_length, {
    let c = LaserBeamConfig {
        time_progress: 1.0,
        ..Default::default()
    };
    let (tail, head, _, _) = evaluate_laser_beam_segment(&c).unwrap();
    assert!(head[0] >= tail[0]);
});
contract_test!(motion_blur_sample_count_is_clamped_low, {
    let c = SubframeMotionBlurSettings {
        samples_per_frame: 0,
        ..Default::default()
    };
    assert_eq!(evaluate_subframe_samples(0, 24, &c).len(), 1);
});
contract_test!(motion_blur_sample_count_is_clamped_high, {
    let c = SubframeMotionBlurSettings {
        samples_per_frame: usize::MAX,
        ..Default::default()
    };
    assert_eq!(evaluate_subframe_samples(0, 24, &c).len(), 64);
});
contract_test!(motion_blur_zero_shutter_has_equal_time_samples, {
    let c = SubframeMotionBlurSettings {
        shutter_angle_deg: 0.0,
        ..Default::default()
    };
    let s = evaluate_subframe_samples(2, 30, &c);
    assert!(s
        .windows(2)
        .all(|w| w[0].subframe_time_sec == w[1].subframe_time_sec));
});
contract_test!(bend_empty_buffers_are_safe, {
    let mut dst = Vec::new();
    apply_bend_warp(&[], 0, 0, &mut dst, &BendWarpConfig::default());
    assert!(dst.is_empty());
});
contract_test!(bend_negative_amount_is_supported, {
    let src = vec![10u8; 8 * 8 * 4];
    let mut dst = vec![0u8; src.len()];
    apply_bend_warp(
        &src,
        8,
        8,
        &mut dst,
        &BendWarpConfig {
            bend_amount: -100.0,
            ..Default::default()
        },
    );
    assert_eq!(dst.len(), src.len());
});
contract_test!(bend_large_amount_does_not_change_buffer_size, {
    let src = vec![10u8; 8 * 8 * 4];
    let mut dst = vec![0u8; src.len()];
    apply_bend_warp(
        &src,
        8,
        8,
        &mut dst,
        &BendWarpConfig {
            bend_amount: 1000.0,
            ..Default::default()
        },
    );
    assert_eq!(dst.len(), src.len());
});
contract_test!(particles_empty_step_stays_empty, {
    let mut s = ParticleSimulation3D::new();
    s.update(0.0, &EmitterConfig3D::default());
    assert!(s.particles.is_empty());
});
contract_test!(particles_box_emitter_is_deterministic_from_new_state, {
    let c = EmitterConfig3D {
        emitter_type: EmitterType3D::Box { size: [10.0; 3] },
        birth_rate_per_sec: 20.0,
        ..Default::default()
    };
    let mut a = ParticleSimulation3D::new();
    let mut b = ParticleSimulation3D::new();
    a.update(0.2, &c);
    b.update(0.2, &c);
    assert_eq!(a.particles[0].position, b.particles[0].position);
});
contract_test!(particles_negative_lifespan_does_not_survive, {
    let mut s = ParticleSimulation3D::new();
    s.update(
        0.1,
        &EmitterConfig3D {
            lifespan_sec: -1.0,
            birth_rate_per_sec: 5.0,
            ..Default::default()
        },
    );
    assert!(s.particles.is_empty());
});
contract_test!(obj_face_with_too_few_vertices_has_no_indices, {
    let m = parse_obj_str("v 0 0 0\nv 1 0 0\nf 1 2\n").unwrap();
    assert!(m.indices.is_empty());
});
contract_test!(obj_extra_face_components_are_accepted, {
    assert!(parse_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1/1/extra 2/2/1 3/3/1\n").is_ok());
});
contract_test!(obj_generated_bounds_are_finite, {
    let m = parse_obj_str("v -1 -2 -3\nv 4 5 6\n").unwrap();
    assert!(m
        .bbox_min
        .iter()
        .chain(m.bbox_max.iter())
        .all(|v| v.is_finite()));
});
contract_test!(color_zero_maps_to_zero_linear, {
    let mut d = [1.0];
    convert_rgba8_to_rgba32f(&[0], &mut d);
    assert_eq!(d[0], 0.0);
});
contract_test!(color_white_maps_to_one_linear, {
    let mut d = [0.0];
    convert_rgba8_to_rgba32f(&[255], &mut d);
    assert_eq!(d[0], 1.0);
});
contract_test!(color_float_clamps_above_one, {
    let mut d = [0];
    convert_rgba32f_to_rgba8(&[2.0], &mut d);
    assert_eq!(d[0], 255);
});
contract_test!(color_float_clamps_below_zero, {
    let mut d = [0];
    convert_rgba32f_to_rgba8(&[-1.0], &mut d);
    assert_eq!(d[0], 0);
});
contract_test!(roto_no_strokes_returns_zero_matte, {
    let result = generate_rotobrush_matte(&[255; 16], 2, 2, &[], &Default::default());
    assert_eq!(result, vec![0; 4]);
});
contract_test!(roto_out_of_bounds_stroke_is_safe, {
    let stroke = RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[999.0, -999.0]],
        radius: 5.0,
    };
    let result = generate_rotobrush_matte(&[0; 4 * 4 * 4], 4, 4, &[stroke], &Default::default());
    assert_eq!(result.len(), 16);
});
contract_test!(roto_negative_radius_is_safe, {
    let stroke = RotoStroke {
        stroke_type: RotoStrokeType::Foreground,
        points: vec![[1.0, 1.0]],
        radius: -5.0,
    };
    assert_eq!(
        generate_rotobrush_matte(&[0; 4 * 4 * 4], 4, 4, &[stroke], &Default::default()).len(),
        16
    );
});
contract_test!(boolean_empty_subject_union_returns_clip, {
    let clip = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert_eq!(
        apply_polygon_boolean(&[], &clip, BooleanOp::Union),
        vec![clip]
    );
});
contract_test!(boolean_empty_intersection_is_empty, {
    let a = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert!(apply_polygon_boolean(&a, &[], BooleanOp::Intersect).is_empty());
});
contract_test!(boolean_offset_zero_is_identity, {
    let a = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert_eq!(offset_polygon_path(&a, 0.0), a);
});
contract_test!(audio_invalid_fps_returns_empty, {
    assert!(extract_multiband_audio_keyframes(
        &[1.0, 1.0],
        48_000,
        0,
        3,
        &AudioKeyframeOptions::default()
    )
    .master
    .is_empty());
});
contract_test!(audio_invalid_sample_rate_returns_empty, {
    assert!(extract_multiband_audio_keyframes(
        &[1.0, 1.0],
        0,
        24,
        3,
        &AudioKeyframeOptions::default()
    )
    .master
    .is_empty());
});
contract_test!(audio_keyframe_frames_are_monotonic, {
    let r = extract_multiband_audio_keyframes(
        &[0.2; 1000],
        48_000,
        24,
        8,
        &AudioKeyframeOptions::default(),
    );
    assert!(r.master.windows(2).all(|w| w[0].frame < w[1].frame));
});

contract_test!(calculations_nan_opacity_is_safe, {
    let mut pixel = [128u8, 64, 32, 255];
    let before = pixel;
    apply_calculations_composite(
        &mut pixel,
        None,
        1,
        1,
        &CalculationsConfig {
            second_layer_opacity: f32::NAN,
            ..Default::default()
        },
    );
    assert_eq!(pixel[0], before[0]);
    assert_eq!(pixel[3], before[3]);
});
contract_test!(camera_tracker_rejects_zero_dimensions, {
    let tracks = (0..8)
        .map(|id| FeatureTrack2D {
            id,
            observations: vec![(0, [1.0, 1.0]), (1, [2.0, 2.0])],
        })
        .collect::<Vec<_>>();
    assert!(solve_camera_motion_3d(&tracks, 0, 0, None).is_none());
});
contract_test!(camera_tracker_rejects_nonfinite_observation, {
    let tracks = (0..8)
        .map(|id| FeatureTrack2D {
            id,
            observations: vec![(0, [f32::NAN, 1.0]), (1, [2.0, 2.0])],
        })
        .collect::<Vec<_>>();
    assert!(solve_camera_motion_3d(&tracks, 100, 100, None).is_none());
});
contract_test!(camera_tracker_rejects_unbounded_frame_span, {
    let tracks = (0..8)
        .map(|id| FeatureTrack2D {
            id,
            observations: vec![(0, [1.0, 1.0]), (u32::MAX, [2.0, 2.0])],
        })
        .collect::<Vec<_>>();
    assert!(solve_camera_motion_3d(&tracks, 100, 100, None).is_none());
});
contract_test!(cylinder_zero_dimensions_are_safe, {
    let mut out = Vec::new();
    apply_cylinder_projection(&[], 0, 0, &mut out, &CylinderProjectionConfig::default());
    assert!(out.is_empty());
});
contract_test!(sphere_nonfinite_radius_clears_output, {
    let src = vec![255u8; 16];
    let mut out = vec![99u8; 16];
    apply_sphere_projection(
        &src,
        2,
        2,
        &mut out,
        &SphereProjectionConfig {
            radius: f32::NAN,
            ..Default::default()
        },
    );
    assert!(out.iter().all(|v| *v == 0));
});
contract_test!(optical_flow_mismatched_input_returns_zero_flow, {
    let flow = compute_dense_optical_flow(&[], &[], 4, 4, 1, 2);
    assert_eq!(flow.vectors, vec![[0.0, 0.0]; 16]);
});
contract_test!(timewarp_one_pixel_frame_is_safe, {
    let a = [0u8, 0, 0, 255];
    let b = [255u8, 255, 255, 255];
    let flow = DenseFlowField::new(1, 1);
    let mut out = [0u8; 4];
    interpolate_timewarp_frame(&a, &b, 1, 1, &flow, &flow, 0.5, &mut out);
    assert!((out[0] as i16 - 128).abs() <= 1);
    assert_eq!(out[3], 255);
});
