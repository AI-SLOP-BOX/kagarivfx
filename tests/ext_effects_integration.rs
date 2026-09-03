//! Integration tests for the 🟢-scope effect modules (Parts 27–28,
//! color correction, particle forces, audio spectrum, text animator stack).
//!
//! Focus: cross-module pipelines stay deterministic, panic-free under
//! parameter sweeps, and compose correctly end-to-end.

use aftereffects_oss::core::ae_effects_pack_v27::{
    apply_cc_lens_pro, apply_optics_compensation, apply_polar_coordinates_pro, apply_wave_warp_pro,
    CcLensParams, OpticsCompensationParams, PinKind, PolarMode, WaveType, WaveWarpParams,
};
use aftereffects_oss::core::ae_effects_pack_v28::{
    apply_cc_bend_it_pro, apply_glow_pro, apply_light_sweep, LightSweepParams,
};
use aftereffects_oss::core::audio_spectrum::{
    extract_waveform, render_spectrum, AudioSpectrumOptions, AudioSpectrumType, SpectrumAnalyzer,
};
use aftereffects_oss::core::color_correction::{
    apply_channel_mixer, apply_color_balance, apply_curves, ChannelCurves, ChannelMixer,
    ColorBalance, ToneCurve,
};
use aftereffects_oss::core::effect_registry_ext::ExtEffect;
use aftereffects_oss::core::particle_forces::{apply_drag, resolve_bounds_collision, LifeCurve};
use aftereffects_oss::core::particle_system::{ParticleEmitter, ParticleSystem};

fn gradient(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            v.push(((x * 255) / w.max(1)) as u8);
            v.push(((y * 255) / h.max(1)) as u8);
            v.push(128);
            v.push(255);
        }
    }
    v
}

#[test]
fn test_full_distortion_and_grade_pipeline_is_deterministic() {
    let run = || {
        let mut img = gradient(48, 48);
        // Distortion stage.
        apply_wave_warp_pro(
            &mut img,
            48,
            48,
            &WaveWarpParams {
                wave_height: 6.0,
                wave_width: 16.0,
                speed: 1.0,
                time: 0.75,
                wave_type: WaveType::Sine,
                pinning: PinKind::All,
                ..Default::default()
            },
        );
        apply_cc_lens_pro(
            &mut img,
            48,
            48,
            &CcLensParams {
                convergence: 35.0,
                zoom: 1.0,
            },
        );
        apply_polar_coordinates_pro(&mut img, 48, 48, PolarMode::RectToPolar, 0.5);
        apply_optics_compensation(
            &mut img,
            48,
            48,
            &OpticsCompensationParams {
                field_of_view_deg: 45.0,
                reverse: false,
                zoom: 1.0,
            },
        );
        apply_cc_bend_it_pro(&mut img, 48, 48, 2.0, -2.0);
        // Stylize stage.
        apply_light_sweep(
            &mut img,
            48,
            48,
            &LightSweepParams {
                direction_deg: 30.0,
                center: 0.4,
                width: 0.3,
                sweep_intensity: 0.5,
                edge_intensity: 0.2,
            },
        );
        apply_glow_pro(&mut img, 48, 48, 0.55, 3, 0.8);
        // Grade stage.
        let s_curve = ToneCurve::new(vec![
            [0.0, 0.0],
            [0.25, 0.18],
            [0.5, 0.5],
            [0.75, 0.82],
            [1.0, 1.0],
        ]);
        apply_curves(
            &mut img,
            &ChannelCurves {
                master: Some(s_curve),
                ..Default::default()
            },
        );
        apply_color_balance(
            &mut img,
            &ColorBalance {
                shadows: [-15.0, 5.0, 20.0],
                midtones: [10.0, 0.0, -10.0],
                highlights: [25.0, -5.0, -15.0],
                preserve_luminosity: true,
            },
        );
        apply_channel_mixer(
            &mut img,
            &ChannelMixer {
                matrix: [[105.0, -5.0, 0.0], [0.0, 100.0, 0.0], [0.0, 5.0, 95.0]],
                monochrome: false,
            },
        );
        img
    };
    let a = run();
    let b = run();
    assert_eq!(a, b, "full pipeline must be byte-deterministic");
    assert_eq!(a.len(), 48 * 48 * 4);
}

#[test]
fn test_registry_parameter_sweep_never_panics() {
    let variants = vec![
        ExtEffect::WaveWarp {
            wave_height: 40.0,
            wave_width: 8.0,
            speed: 3.0,
            direction_deg: 270.0,
            wave_type: 3,
            pinning: 3,
        },
        ExtEffect::CcLens {
            convergence: -100.0,
            zoom: 3.0,
        },
        ExtEffect::PolarCoordinates {
            to_polar: false,
            interpolation: 100.0,
        },
        ExtEffect::OpticsCompensation {
            field_of_view_deg: -179.0,
            reverse: true,
            zoom: 0.05,
        },
        ExtEffect::Curves {
            master: vec![[0.0, 1.0], [1.0, 0.0]],
            red: vec![],
            green: vec![],
            blue: vec![],
        },
        ExtEffect::ColorBalance {
            shadows: [100.0; 3],
            midtones: [-100.0; 3],
            highlights: [100.0; 3],
            preserve_luminosity: false,
        },
        ExtEffect::ChannelMixer {
            matrix: [[300.0; 3]; 3],
            monochrome: true,
        },
        ExtEffect::LightSweep {
            direction_deg: 180.0,
            center: 0.0,
            width: 1.0,
            sweep_intensity: 1.0,
            edge_intensity: 1.0,
        },
        ExtEffect::RadialFastBlur {
            amount: 1.0,
            samples: 64,
        },
        ExtEffect::BendIt {
            top_offset: 50.0,
            bottom_offset: -50.0,
        },
        ExtEffect::Tiler {
            scale_percent: 5000.0,
            mirror: false,
        },
        ExtEffect::GlowPro {
            threshold: 0.0,
            radius: 128,
            intensity: 4.0,
        },
    ];
    for e in &variants {
        for t in [0.0f32, 0.33, 1.7] {
            let mut img = gradient(32, 32);
            e.apply(&mut img, 32, 32, t);
            assert_eq!(
                img.len(),
                32 * 32 * 4,
                "{} changed buffer size",
                e.type_id()
            );
        }
    }
}

#[test]
fn test_particle_simulation_with_all_forces_renders() {
    let emitter = ParticleEmitter {
        rate: 400.0,
        lifetime: 2.0,
        lifetime_variance: 0.3,
        speed: 120.0,
        spread_degrees: 360.0,
        gravity: [0.0, 250.0],
        gravity_curve: LifeCurve(vec![0.1, 1.6, 0.6]),
        wind: [80.0, -10.0],
        wind_gust_strength: 45.0,
        wind_gust_frequency: 1.5,
        drag: 0.6,
        turbulence: 12.0,
        collision_enabled: true,
        collision_bounds: [0.0, 0.0, 320.0, 240.0],
        restitution: 0.5,
        surface_friction: 0.85,
        max_particles: 600,
        ..Default::default()
    };
    let mut ps = ParticleSystem::new(emitter);
    for _ in 0..90 {
        ps.update(1.0 / 30.0, 160.0, 60.0);
    }
    assert!(!ps.particles.is_empty(), "particles must be alive");
    for p in &ps.particles {
        assert!(
            p.x.is_finite() && p.y.is_finite(),
            "positions must stay finite"
        );
        assert!(
            p.x >= -1.0 && p.x <= 321.0,
            "collision must contain x: {}",
            p.x
        );
        assert!(
            p.y >= -1.0 && p.y <= 241.0,
            "collision must contain y: {}",
            p.y
        );
    }
    let mut buf = vec![0u8; 320 * 240 * 4];
    ps.render(&mut buf, 320, 240, 1.0);
    assert!(
        buf.chunks(4).any(|px| px[3] > 0),
        "rendered frame must contain particles"
    );
}

#[test]
fn test_particle_force_helpers_compose() {
    // Drag then collision then curve evaluation in one step sequence.
    let mut pos = [10.0f32, 250.0];
    let mut vel = [30.0f32, 60.0];
    let curve = LifeCurve(vec![0.0, 2.0]);
    for step in 0..120u32 {
        let t = step as f32 / 120.0;
        let gmul = curve.eval(t);
        vel[1] += 300.0 * gmul * (1.0 / 60.0);
        let [vx, vy] = &mut vel;
        apply_drag(vx, vy, 0.8, 1.0 / 60.0);
        pos[0] += vel[0] * (1.0 / 60.0);
        pos[1] += vel[1] * (1.0 / 60.0);
        resolve_bounds_collision(&mut pos, &mut vel, [0.0, 0.0, 200.0, 200.0], 0.6, 0.9);
        assert!(pos[1] <= 200.0 + 1e-3);
    }
}

#[test]
fn test_audio_pipeline_end_to_end() {
    // 440 Hz tone → analyzer → bands → rendered spectrum pixels.
    let sr = 44100u32;
    let pcm: Vec<f32> = (0..2048)
        .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / sr as f32).sin() * 0.8)
        .collect();
    let opts = AudioSpectrumOptions {
        spectrum_type: AudioSpectrumType::DigitalBands,
        frequency_bands: 32,
        fft_size: 2048,
        max_height: 80.0,
        ..Default::default()
    };
    let mut analyzer = SpectrumAnalyzer::new(opts.frequency_bands);
    let bands = analyzer.analyze(&pcm, sr, &opts);
    assert_eq!(bands.len(), 32);
    assert!(bands.iter().any(|&b| b > 0.3), "tone must register energy");

    let peaks = analyzer.peaks().to_vec();
    assert_eq!(peaks.len(), 32);

    let mut buf = vec![0u8; 256 * 128 * 4];
    render_spectrum(
        &mut buf,
        256,
        128,
        &bands,
        &opts,
        [10, 30, 80],
        [120, 230, 255],
    );
    assert!(
        buf.chunks(4).any(|px| px[3] == 255),
        "spectrum must draw bars"
    );

    let wf = extract_waveform(&pcm, 16);
    assert_eq!(wf.len(), 16);
    assert!(
        wf.iter().any(|&v| v > 0.4),
        "waveform envelope must capture the tone"
    );
}

#[test]
fn test_text_animator_stack_with_color_channels() {
    use aftereffects_oss::core::text_animator::{RangeSelector, SelectorShape};
    use aftereffects_oss::core::text_animator_advanced::{
        AnimatorStack, SelectorUnit, TextAnimatorAdvanced,
    };

    // Square selector covering only the second word (unit pct 50%).
    let mut pop = TextAnimatorAdvanced {
        selector: RangeSelector {
            shape: SelectorShape::Square,
            start: 34.0,
            end: 100.0,
            ..Default::default()
        },
        unit: SelectorUnit::Words,
        position: [0.0, -30.0],
        opacity: 0.0,
        ..Default::default()
    };
    pop.advanced.fill_color = Some([1.0, 0.4, 0.1, 1.0]);
    pop.advanced.skew = 12.0;

    let mut tracking_anim = TextAnimatorAdvanced {
        selector: RangeSelector {
            shape: SelectorShape::Square,
            start: 0.0,
            end: 100.0,
            ..Default::default()
        },
        tracking: 8.0,
        ..Default::default()
    };
    tracking_anim.advanced.stroke_color = Some([0.1, 0.1, 0.1, 1.0]);
    tracking_anim.advanced.stroke_width = 2.0;

    let stack = AnimatorStack {
        animators: vec![pop, tracking_anim],
    };
    let composed = stack.compose("hello world");
    assert_eq!(composed.len(), 11);

    // Word 1 ("world") must carry the full effect set; word 0 none of the
    // word-scoped channels.
    let first = &composed[0];
    assert!(first.fill_mix.abs() < 1e-4, "word 0 must be unselected");
    let last = &composed[10];
    assert!(
        (last.fill_mix - 1.0).abs() < 1e-4,
        "word 1 fill fully applied"
    );
    assert_eq!(last.fill_color, Some([1.0, 0.4, 0.1, 1.0]));
    assert!((last.skew_deg - 12.0).abs() < 1e-3);
    assert!((last.stroke_width_add - 2.0).abs() < 1e-4);
    assert!((last.base.tracking_offset - 8.0).abs() < 1e-4);

    // Separator space keeps zero stroke width add from the square animator
    // but inherits nothing else problematic.
    let space = &composed[5];
    assert!(space.base.position_offset[1].abs() < 1e-4 || space.fill_mix.abs() < 1e-4);
}

#[test]
fn test_histogram_feeds_scope_rendering_contract() {
    let img = gradient(64, 64);
    let luma = aftereffects_oss::core::color_correction::compute_luma_histogram(&img);
    let total: u64 = luma.iter().map(|&v| v as u64).sum();
    assert_eq!(total, (64 * 64) as u64, "every pixel counted exactly once");
    let rgb = aftereffects_oss::core::color_correction::compute_rgb_histograms(&img);
    for ch in &rgb {
        let t: u64 = ch.iter().map(|&v| v as u64).sum();
        assert_eq!(t, (64 * 64) as u64);
    }
}
