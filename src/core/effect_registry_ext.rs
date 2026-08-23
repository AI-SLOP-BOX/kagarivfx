#![allow(dead_code)]
//! Unified registry for the 🟢-scope CPU effect kernels (Parts 27–28 +
//! color correction). Provides a single serializable enum with an `apply`
//! dispatch so the render pipeline can integrate every effect through one
//! call site, and exposes preset metadata for data-driven UI catalogs
//! (ARCHITECTURE_GUIDELINES Rule 8.2).

use serde::{Deserialize, Serialize};

use crate::core::ae_effects_pack_v27::{
    apply_cc_lens_pro, apply_optics_compensation, apply_polar_coordinates_pro,
    apply_wave_warp_pro, CcLensParams, OpticsCompensationParams, PinKind, PolarMode,
    WaveType, WaveWarpParams,
};
use crate::core::ae_effects_pack_v28::{
    apply_cc_bend_it_pro, apply_cc_tiler_pro, apply_glow_pro, apply_light_sweep,
    apply_radial_fast_blur, LightSweepParams, TileEdgeMode,
};
use crate::core::color_correction::{
    apply_channel_mixer, apply_color_balance, apply_curves, apply_vibrance,
    apply_white_balance, ChannelCurves, ColorBalance, ChannelMixer, ToneCurve,
};

/// Serializable extended effect. All parameters are plain values; animation
/// is driven by the caller evaluating them per frame before dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExtEffect {
    /// AE Wave Warp (Pro rebuild).
    WaveWarp {
        #[serde(default = "d50")]
        wave_height: f32,
        #[serde(default = "d100")]
        wave_width: f32,
        #[serde(default)]
        speed: f32,
        #[serde(default = "d90")]
        direction_deg: f32,
        /// 0=Sine 1=Triangle 2=Square 3=Sawtooth
        #[serde(default)]
        wave_type: u8,
        /// 0=All 1=LeftRight 2=TopBottom 3=None
        #[serde(default)]
        pinning: u8,
    },
    /// CC Lens fisheye.
    CcLens {
        #[serde(default = "d50")]
        convergence: f32,
        #[serde(default = "d1")]
        zoom: f32,
    },
    /// Polar Coordinates conversion.
    PolarCoordinates {
        #[serde(default)]
        to_polar: bool,
        #[serde(default = "d100")]
        interpolation: f32,
    },
    /// Lens distortion compensation.
    OpticsCompensation {
        #[serde(default)]
        field_of_view_deg: f32,
        #[serde(default)]
        reverse: bool,
        #[serde(default = "d1")]
        zoom: f32,
    },
    /// Spline tone curves. Empty vec = channel untouched.
    Curves {
        #[serde(default)]
        master: Vec<[f32; 2]>,
        #[serde(default)]
        red: Vec<[f32; 2]>,
        #[serde(default)]
        green: Vec<[f32; 2]>,
        #[serde(default)]
        blue: Vec<[f32; 2]>,
    },
    /// Three-way color balance (-100..100 per slider).
    ColorBalance {
        #[serde(default)]
        shadows: [f32; 3],
        #[serde(default)]
        midtones: [f32; 3],
        #[serde(default)]
        highlights: [f32; 3],
        #[serde(default = "dtrue")]
        preserve_luminosity: bool,
    },
    /// Channel mixer matrix in percent units.
    ChannelMixer {
        #[serde(default = "identity_matrix")]
        matrix: [[f32; 3]; 3],
        #[serde(default)]
        monochrome: bool,
    },
    /// CC Light Sweep specular band.
    LightSweep {
        #[serde(default)]
        direction_deg: f32,
        #[serde(default = "dhalf")]
        center: f32,
        #[serde(default = "dqtr")]
        width: f32,
        #[serde(default = "dsixty")]
        sweep_intensity: f32,
        #[serde(default = "dthirty")]
        edge_intensity: f32,
    },
    /// Radial (zoom) fast blur.
    RadialFastBlur {
        #[serde(default = "dhalf")]
        amount: f32,
        #[serde(default = "dsamples")]
        samples: u32,
    },
    /// Vertical-axis bend between top/bottom offsets (px).
    BendIt {
        #[serde(default)]
        top_offset: f32,
        #[serde(default)]
        bottom_offset: f32,
    },
    /// Tiling with edge mode (0=Repeat 1=Mirror).
    Tiler {
        #[serde(default = "d200")]
        scale_percent: f32,
        #[serde(default)]
        mirror: bool,
    },
    /// Threshold bloom glow.
    GlowPro {
        #[serde(default = "dseventy")]
        threshold: f32,
        #[serde(default = "dfour")]
        radius: u32,
        #[serde(default = "d1")]
        intensity: f32,
    },
    /// Saturation boost protecting skin tones (−100..100).
    Vibrance {
        #[serde(default = "dhalf")]
        amount: f32,
    },
    /// Temperature/Tint white balance (−100..100 each).
    WhiteBalance {
        #[serde(default)]
        temperature: f32,
        #[serde(default)]
        tint: f32,
    },
}

fn d50() -> f32 { 50.0 }
fn d100() -> f32 { 100.0 }
fn d90() -> f32 { 90.0 }
fn d1() -> f32 { 1.0 }
fn dhalf() -> f32 { 0.5 }
fn dqtr() -> f32 { 0.25 }
fn dsixty() -> f32 { 0.6 }
fn dthirty() -> f32 { 0.3 }
fn dsamples() -> u32 { 12 }
fn d200() -> f32 { 200.0 }
fn dseventy() -> f32 { 0.7 }
fn dfour() -> u32 { 4 }
fn dtrue() -> bool { true }
fn identity_matrix() -> [[f32; 3]; 3] {
    [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 100.0]]
}

impl ExtEffect {
    /// Stable identifier used by project files and UI catalogs.
    pub fn type_id(&self) -> &'static str {
        match self {
            ExtEffect::WaveWarp { .. } => "wave_warp",
            ExtEffect::CcLens { .. } => "cc_lens",
            ExtEffect::PolarCoordinates { .. } => "polar_coordinates",
            ExtEffect::OpticsCompensation { .. } => "optics_compensation",
            ExtEffect::Curves { .. } => "curves",
            ExtEffect::ColorBalance { .. } => "color_balance",
            ExtEffect::ChannelMixer { .. } => "channel_mixer",
            ExtEffect::LightSweep { .. } => "light_sweep",
            ExtEffect::RadialFastBlur { .. } => "radial_fast_blur",
            ExtEffect::BendIt { .. } => "bend_it",
            ExtEffect::Tiler { .. } => "tiler",
            ExtEffect::GlowPro { .. } => "glow_pro",
            ExtEffect::Vibrance { .. } => "vibrance",
            ExtEffect::WhiteBalance { .. } => "white_balance",
        }
    }

    /// Human-readable display name for effect browsers.
    pub fn display_name(&self) -> &'static str {
        match self {
            ExtEffect::WaveWarp { .. } => "Wave Warp",
            ExtEffect::CcLens { .. } => "CC Lens",
            ExtEffect::PolarCoordinates { .. } => "Polar Coordinates",
            ExtEffect::OpticsCompensation { .. } => "Optics Compensation",
            ExtEffect::Curves { .. } => "Curves",
            ExtEffect::ColorBalance { .. } => "Color Balance",
            ExtEffect::ChannelMixer { .. } => "Channel Mixer",
            ExtEffect::LightSweep { .. } => "CC Light Sweep",
            ExtEffect::RadialFastBlur { .. } => "CC Radial Fast Blur",
            ExtEffect::BendIt { .. } => "CC Bend It",
            ExtEffect::Tiler { .. } => "CC Tiler",
            ExtEffect::GlowPro { .. } => "Glow",
            ExtEffect::Vibrance { .. } => "Vibrance",
            ExtEffect::WhiteBalance { .. } => "White Balance",
        }
    }

    /// Category for the effects library browser.
    pub fn category(&self) -> &'static str {
        match self {
            ExtEffect::WaveWarp { .. }
            | ExtEffect::CcLens { .. }
            | ExtEffect::PolarCoordinates { .. }
            | ExtEffect::OpticsCompensation { .. }
            | ExtEffect::BendIt { .. }
            | ExtEffect::Tiler { .. } => "Distort",
            ExtEffect::LightSweep { .. } | ExtEffect::GlowPro { .. } => "Stylize",
            ExtEffect::RadialFastBlur { .. } => "Blur & Sharpen",
            ExtEffect::Curves { .. }
            | ExtEffect::ColorBalance { .. }
            | ExtEffect::ChannelMixer { .. }
            | ExtEffect::Vibrance { .. }
            | ExtEffect::WhiteBalance { .. } => "Color Correction",
        }
    }

    /// Apply this effect to a packed RGBA8 buffer. `time` drives animated
    /// parameters (currently Wave Warp travel phase).
    pub fn apply(&self, pixels: &mut [u8], width: u32, height: u32, time: f32) {
        match self {
            ExtEffect::WaveWarp { wave_height, wave_width, speed, direction_deg, wave_type, pinning } => {
                let params = WaveWarpParams {
                    wave_height: *wave_height,
                    wave_width: *wave_width,
                    speed: *speed,
                    time,
                    direction_deg: *direction_deg,
                    wave_type: match wave_type {
                        1 => WaveType::Triangle,
                        2 => WaveType::Square,
                        3 => WaveType::Sawtooth,
                        _ => WaveType::Sine,
                    },
                    pinning: match pinning {
                        1 => PinKind::LeftRight,
                        2 => PinKind::TopBottom,
                        3 => PinKind::None,
                        _ => PinKind::All,
                    },
                    ..Default::default()
                };
                apply_wave_warp_pro(pixels, width, height, &params);
            }
            ExtEffect::CcLens { convergence, zoom } => {
                apply_cc_lens_pro(pixels, width, height, &CcLensParams {
                    convergence: *convergence, zoom: *zoom,
                });
            }
            ExtEffect::PolarCoordinates { to_polar, interpolation } => {
                let mode = if *to_polar { PolarMode::RectToPolar } else { PolarMode::PolarToRect };
                apply_polar_coordinates_pro(pixels, width, height, mode, *interpolation);
            }
            ExtEffect::OpticsCompensation { field_of_view_deg, reverse, zoom } => {
                apply_optics_compensation(pixels, width, height, &OpticsCompensationParams {
                    field_of_view_deg: *field_of_view_deg, reverse: *reverse, zoom: *zoom,
                });
            }
            ExtEffect::Curves { master, red, green, blue } => {
                let curve = |pts: &Vec<[f32; 2]>| -> Option<ToneCurve> {
                    if pts.len() < 2 { None } else { Some(ToneCurve::new(pts.clone())) }
                };
                let cc = ChannelCurves {
                    master: curve(master),
                    red: curve(red),
                    green: curve(green),
                    blue: curve(blue),
                };
                apply_curves(pixels, &cc);
            }
            ExtEffect::ColorBalance { shadows, midtones, highlights, preserve_luminosity } => {
                apply_color_balance(pixels, &ColorBalance {
                    shadows: *shadows,
                    midtones: *midtones,
                    highlights: *highlights,
                    preserve_luminosity: *preserve_luminosity,
                });
            }
            ExtEffect::ChannelMixer { matrix, monochrome } => {
                apply_channel_mixer(pixels, &ChannelMixer {
                    matrix: *matrix, monochrome: *monochrome,
                });
            }
            ExtEffect::LightSweep { direction_deg, center, width: sweep_width, sweep_intensity, edge_intensity } => {
                apply_light_sweep(pixels, width, height, &LightSweepParams {
                    direction_deg: *direction_deg,
                    center: *center,
                    width: *sweep_width,
                    sweep_intensity: *sweep_intensity,
                    edge_intensity: *edge_intensity,
                });
            }
            ExtEffect::RadialFastBlur { amount, samples } => {
                let cx = width as f32 * 0.5;
                let cy = height as f32 * 0.5;
                apply_radial_fast_blur(pixels, width, height, [cx, cy], *amount, *samples);
            }
            ExtEffect::BendIt { top_offset, bottom_offset } => {
                apply_cc_bend_it_pro(pixels, width, height, *top_offset, *bottom_offset);
            }
            ExtEffect::Tiler { scale_percent, mirror } => {
                let mode = if *mirror { TileEdgeMode::Mirror } else { TileEdgeMode::Repeat };
                apply_cc_tiler_pro(pixels, width, height, *scale_percent, mode);
            }
            ExtEffect::GlowPro { threshold, radius, intensity } => {
                apply_glow_pro(pixels, width, height, *threshold, *radius, *intensity);
            }
            ExtEffect::Vibrance { amount } => {
                apply_vibrance(pixels, *amount);
            }
            ExtEffect::WhiteBalance { temperature, tint } => {
                apply_white_balance(
                    pixels,
                    &crate::core::color_correction::WhiteBalance {
                        temperature: *temperature,
                        tint: *tint,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn sample_effects() -> Vec<ExtEffect> {
        vec![
            ExtEffect::WaveWarp { wave_height: 5.0, wave_width: 10.0, speed: 1.0, direction_deg: 90.0, wave_type: 0, pinning: 0 },
            ExtEffect::CcLens { convergence: 40.0, zoom: 1.0 },
            ExtEffect::PolarCoordinates { to_polar: true, interpolation: 100.0 },
            ExtEffect::OpticsCompensation { field_of_view_deg: 60.0, reverse: false, zoom: 1.0 },
            ExtEffect::Curves { master: vec![[0.0, 0.0], [0.5, 0.45], [1.0, 1.0]], red: vec![], green: vec![], blue: vec![] },
            ExtEffect::ColorBalance { shadows: [-20.0, 0.0, 20.0], midtones: [0.0; 3], highlights: [10.0, 0.0, -10.0], preserve_luminosity: true },
            ExtEffect::ChannelMixer { matrix: identity_matrix(), monochrome: false },
            ExtEffect::LightSweep { direction_deg: 0.0, center: 0.5, width: 0.25, sweep_intensity: 0.6, edge_intensity: 0.3 },
            ExtEffect::RadialFastBlur { amount: 0.3, samples: 8 },
            ExtEffect::BendIt { top_offset: 4.0, bottom_offset: -4.0 },
            ExtEffect::Tiler { scale_percent: 250.0, mirror: true },
            ExtEffect::GlowPro { threshold: 0.5, radius: 3, intensity: 0.9 },
            ExtEffect::Vibrance { amount: 40.0 },
            ExtEffect::WhiteBalance { temperature: 30.0, tint: -10.0 },
        ]
    }

    #[test]
    fn test_all_variants_apply_without_panic_and_deterministic() {
        for e in sample_effects() {
            let run = || {
                let mut img = gradient(24, 24);
                e.apply(&mut img, 24, 24, 1.0);
                img
            };
            let a = run();
            let b = run();
            assert_eq!(a, b, "{} not deterministic", e.type_id());
            assert_eq!(a.len(), 24 * 24 * 4, "{} changed buffer size", e.type_id());
        }
    }

    #[test]
    fn test_metadata_catalog_complete() {
        for e in sample_effects() {
            assert!(!e.type_id().is_empty());
            assert!(!e.display_name().is_empty());
            assert!(
                ["Distort", "Stylize", "Blur & Sharpen", "Color Correction"].contains(&e.category()),
                "unknown category {}",
                e.category()
            );
        }
    }

    #[test]
    fn test_serde_roundtrip_preserves_params() {
        for e in sample_effects() {
            let json = serde_json::to_string(&e).unwrap_or_default();
            let back: ExtEffect = serde_json::from_str(&json).unwrap_or(ExtEffect::BendIt {
                top_offset: f32::NAN,
                bottom_offset: 0.0,
            });
            assert_eq!(&back, &e, "{} roundtrip mismatch", e.type_id());
        }
    }

    #[test]
    fn test_serde_backward_compat_minimal_json() {
        // Old/lean JSON relying on serde defaults must deserialize.
        let json = r#"{"WaveWarp":{}}"#;
        let e: ExtEffect = serde_json::from_str(json).unwrap_or(ExtEffect::BendIt { top_offset: 0.0, bottom_offset: 0.0 });
        match e {
            ExtEffect::WaveWarp { wave_height, wave_width, direction_deg, .. } => {
                assert_eq!(wave_height, 50.0);
                assert_eq!(wave_width, 100.0);
                assert_eq!(direction_deg, 90.0);
            }
            other => panic!("wrong variant deserialized: {other:?}"),
        }

        let json2 = r#"{"ChannelMixer":{"monochrome":true}}"#;
        let e2: ExtEffect = serde_json::from_str(json2).unwrap_or(ExtEffect::BendIt { top_offset: 0.0, bottom_offset: 0.0 });
        match e2 {
            ExtEffect::ChannelMixer { matrix, monochrome } => {
                assert!(monochrome);
                assert_eq!(matrix, identity_matrix());
            }
            other => panic!("wrong variant deserialized: {other:?}"),
        }
    }

    #[test]
    fn test_identity_params_leave_image_unchanged() {
        let src = gradient(16, 16);

        let mut out = src.clone();
        ExtEffect::CcLens { convergence: 0.0, zoom: 1.0 }.apply(&mut out, 16, 16, 0.0);
        assert_eq!(out, src);

        let mut out = src.clone();
        ExtEffect::OpticsCompensation { field_of_view_deg: 0.0, reverse: false, zoom: 1.0 }
            .apply(&mut out, 16, 16, 0.0);
        assert_eq!(out, src);

        let mut out = src.clone();
        ExtEffect::ChannelMixer { matrix: identity_matrix(), monochrome: false }
            .apply(&mut out, 16, 16, 0.0);
        assert_eq!(out, src);

        let mut out = src.clone();
        ExtEffect::ColorBalance { shadows: [0.0; 3], midtones: [0.0; 3], highlights: [0.0; 3], preserve_luminosity: true }
            .apply(&mut out, 16, 16, 0.0);
        assert_eq!(out, src);
    }

    #[test]
    fn test_wave_warp_time_drives_animation() {
        let mk = |t: f32| {
            let mut img = gradient(32, 32);
            ExtEffect::WaveWarp { wave_height: 8.0, wave_width: 12.0, speed: 1.0, direction_deg: 90.0, wave_type: 0, pinning: 3 }
                .apply(&mut img, 32, 32, t);
            img
        };
        let t0 = mk(0.0);
        let t_half = mk(0.5);
        assert_ne!(t0, t_half, "time must advance the wave phase");
    }
}