#![allow(dead_code)]
/// OBS-Studio inspired plugin trait system for render effects.
///
/// # Current Architecture (Adapter Pattern)
/// `EffectParams` is a flat struct that mirrors the `LayerUniform` GPU buffer.
/// `evaluate_effects()` acts as an adapter: it iterates `EffectType` enum variants
/// and writes the results into the flat `EffectParams`, which is then uploaded to the GPU.
///
/// Adding a new effect currently requires:
///   1. A new `EffectType` variant in `timeline.rs`
///   2. New fields in `EffectParams` (this file) and `LayerUniform` in `renderer.rs`
///   3. New WGSL shader logic in `shader.wgsl`
///
/// # Future Roadmap (True Plugin Architecture)
/// The goal is to evolve toward a system where each plugin carries its own
/// WGSL shader fragment and an arbitrary GPU buffer, enabling multi-pass
/// compositing without touching the core renderer. This mirrors how Nuke's
/// Blink script system or OBS's source plugin model works.
use crate::core::timeline::{EffectType, ColorConversionMode};

/// The GPU-facing data that an effect plugin can read and modify.
/// Mirrors the relevant fields of LayerUniform without GPU-specific types.
#[derive(Debug, Clone, Copy)]
pub struct EffectParams {
    // Blur
    pub blur_enabled: u32,
    pub blur_radius: f32,

    // Tint
    pub tint_enabled: u32,
    pub tint_color: [f32; 4],
    pub tint_intensity: f32,

    // Drop Shadow
    pub shadow_enabled: u32,
    pub shadow_color: [f32; 4],
    pub shadow_opacity: f32,
    pub shadow_direction: f32,
    pub shadow_distance: f32,
    pub shadow_softness: f32,

    // Chromatic Aberration
    pub chromatic_enabled: u32,
    pub chromatic_shift_r: f32,
    pub chromatic_shift_b: f32,
    pub chromatic_edge_falloff: f32,

    // Vignette
    pub vignette_enabled: u32,
    pub vignette_intensity: f32,
    pub vignette_roundness: f32,
    pub vignette_feather: f32,
    pub vignette_color: [f32; 4],

    // Cinematic Color Grading (NextVFX integration)
    pub lut_enabled: u32,
    pub lut_intensity: f32,
    pub color_convert_mode: u32,

    // Physical Film Grain (NextVFX integration)
    pub grain_enabled: u32,
    pub grain_intensity: f32,
    pub grain_size: f32,

    // Levels Adjustment
    pub levels_enabled: u32,
    pub levels_in_black: f32,
    pub levels_in_white: f32,
    pub levels_gamma: f32,
    pub levels_out_black: f32,
    pub levels_out_white: f32,

    // Hue / Saturation / Lightness
    pub huesat_enabled: u32,
    pub huesat_hue: f32,
    pub huesat_sat: f32,
    pub huesat_light: f32,

    // Glow / Bloom
    pub glow_enabled: u32,
    pub glow_threshold: f32,
    pub glow_radius: f32,
    pub glow_intensity: f32,
    pub glow_color: [f32; 4],

    // Mesh Warp / Corner Pin
    pub meshwarp_enabled: u32,
    pub corner_top_left: [f32; 2],
    pub corner_top_right: [f32; 2],
    pub corner_bottom_left: [f32; 2],
    pub corner_bottom_right: [f32; 2],

    // Motion Blur
    pub motionblur_enabled: u32,
    pub motionblur_shutter: f32,
    pub motionblur_velocity_x: f32,
    pub motionblur_velocity_y: f32,
    pub motionblur_samples: u32,

    // Lens Flare
    pub flare_enabled: u32,
    pub flare_pos_x: f32,
    pub flare_pos_y: f32,
    pub flare_intensity: f32,
    pub flare_threshold: f32,
    pub flare_color: [f32; 4],
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            blur_enabled: 0,
            blur_radius: 0.0,
            tint_enabled: 0,
            tint_color: [1.0; 4],
            tint_intensity: 0.0,
            shadow_enabled: 0,
            shadow_color: [0.0, 0.0, 0.0, 1.0],
            shadow_opacity: 0.0,
            shadow_direction: 135.0,
            shadow_distance: 5.0,
            shadow_softness: 5.0,
            chromatic_enabled: 0,
            chromatic_shift_r: 0.0,
            chromatic_shift_b: 0.0,
            chromatic_edge_falloff: 1.0,
            vignette_enabled: 0,
            vignette_intensity: 0.0,
            vignette_roundness: 1.0,
            vignette_feather: 50.0,
            vignette_color: [0.0, 0.0, 0.0, 1.0],
            lut_enabled: 0,
            lut_intensity: 0.0,
            color_convert_mode: 0,
            grain_enabled: 0,
            grain_intensity: 0.0,
            grain_size: 1.5,
            levels_enabled: 0,
            levels_in_black: 0.0,
            levels_in_white: 1.0,
            levels_gamma: 1.0,
            levels_out_black: 0.0,
            levels_out_white: 1.0,
            huesat_enabled: 0,
            huesat_hue: 0.0,
            huesat_sat: 1.0,
            huesat_light: 1.0,
            glow_enabled: 0,
            glow_threshold: 0.7,
            glow_radius: 10.0,
            glow_intensity: 0.0,
            glow_color: [1.0, 1.0, 1.0, 1.0],
            meshwarp_enabled: 0,
            corner_top_left: [0.0, 0.0],
            corner_top_right: [100.0, 0.0],
            corner_bottom_left: [0.0, 100.0],
            corner_bottom_right: [100.0, 100.0],
            motionblur_enabled: 0,
            motionblur_shutter: 0.5,
            motionblur_velocity_x: 0.0,
            motionblur_velocity_y: 0.0,
            motionblur_samples: 8,
            flare_enabled: 0,
            flare_pos_x: 0.5,
            flare_pos_y: 0.5,
            flare_intensity: 1.0,
            flare_threshold: 0.8,
            flare_color: [1.0, 0.95, 0.9, 1.0],
        }
    }
}

/// The core trait every render effect plugin must implement.
#[allow(dead_code)]
pub trait RenderEffectPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn type_id(&self) -> &str;
    fn apply_to_params(&self, frame: u32, params: &mut EffectParams);
}

/// Adapts a serializable `EffectType` enum variant into the plugin trait.
pub struct EnumEffectPlugin {
    pub effect_type: EffectType,
}

impl RenderEffectPlugin for EnumEffectPlugin {
    fn name(&self) -> &str {
        match &self.effect_type {
            EffectType::GaussianBlur { .. } => "Gaussian Blur",
            EffectType::ColorTint { .. } => "Color Tint",
            EffectType::DropShadow { .. } => "Drop Shadow",
            EffectType::ChromaticAberration { .. } => "Chromatic Aberration",
            EffectType::Vignette { .. } => "Vignette",
            EffectType::Levels { .. } => "Levels",
            EffectType::HueSaturation { .. } => "Hue / Saturation",
            EffectType::Glow { .. } => "Glow",
            EffectType::MotionBlur { .. } => "Motion Blur",
            EffectType::MeshWarp { .. } => "Mesh Warp",
            EffectType::CornerPin { .. } => "Corner Pin",
            EffectType::ColorGradeLUT { .. } => "3D LUT Color Grading",
            EffectType::ColorSpaceConvert { .. } => "Color Space Converter",
            EffectType::FilmGrain { .. } => "Physical Film Grain",
            EffectType::Twirl { .. } => "Twirl",
            EffectType::Bulge { .. } => "Bulge",
            EffectType::Posterize { .. } => "Posterize",
            EffectType::Invert { .. } => "Invert",
            EffectType::Offset { .. } => "Offset",
            EffectType::DirectionalBlur { .. } => "Directional Blur",
            EffectType::RadialBlur { .. } => "Radial Blur",
            EffectType::Sharpen { .. } => "Sharpen",
            EffectType::Threshold { .. } => "Threshold",
            EffectType::LinearWipe { .. } => "Linear Wipe",
            EffectType::SimpleChoker { .. } => "Simple Choker",
            EffectType::ChromaKey { .. } => "Chroma Key",
            EffectType::Spherize { .. } => "Spherize",
            EffectType::TurbulentDisplace { .. } => "Turbulent Displace",
            EffectType::Colorama { .. } => "Colorama",
            EffectType::FractalNoise { .. } => "Fractal Noise",
            EffectType::Curves { .. } => "Curves",
            EffectType::DisplacementMap { .. } => "Displacement Map",
            EffectType::CompoundBlur { .. } => "Compound Blur",
            EffectType::Minimax { .. } => "Minimax",
            EffectType::ShiftChannels { .. } => "Shift Channels",
            EffectType::WaveWarp { .. } => "Wave Warp",
            EffectType::CcLens { .. } => "CC Lens",
            EffectType::PolarCoordinates { .. } => "Polar Coordinates",
            EffectType::OpticsCompensation { .. } => "Optics Compensation",
            EffectType::ColorBalance { .. } => "Color Balance",
            EffectType::ChannelMixer { .. } => "Channel Mixer",
            EffectType::LightSweep { .. } => "CC Light Sweep",
            EffectType::RadialFastBlur { .. } => "CC Radial Fast Blur",
            EffectType::BendIt { .. } => "CC Bend It",
            EffectType::Tiler { .. } => "CC Tiler",
            EffectType::Tritone { .. } => "Tritone",
            EffectType::MatteChoker { .. } => "Matte Choker",
            EffectType::VenetianBlinds { .. } => "Venetian Blinds",
            EffectType::Vibrance { .. } => "Vibrance",
            EffectType::WhiteBalance { .. } => "White Balance",
            EffectType::HslAdjust { .. } => "HSL Adjust",
            EffectType::GlowPro { .. } => "Glow",
            EffectType::CrtScanlines { .. } => "CRT Scanlines",
            EffectType::Vortex { .. } => "Vortex Distortion",
            EffectType::HeatDistortion { .. } => "Heat Distortion",
            EffectType::RainRipples { .. } => "Rain Ripples",
            EffectType::Fisheye { .. } => "Fisheye",
            EffectType::LensCorrection { .. } => "Lens Correction",
            EffectType::GlitchDisplacement { .. } => "Glitch Displacement",
            EffectType::MatteChokeSpread { .. } => "Matte Choke / Spread",
            EffectType::AlphaFeather { .. } => "Alpha Feather",
            EffectType::AlphaFromLuminance { .. } => "Alpha From Luminance",
            EffectType::NightVision { .. } => "Night Vision",
            EffectType::IrisWipe { .. } => "Iris Wipe",
            EffectType::RadialWipe { .. } => "Radial Wipe",
            EffectType::FilmEmulation { .. } => "Film Emulation",
            EffectType::GodRays { .. } => "God Rays",
            EffectType::RadialBlurZoom { .. } => "Zoom Blur",
            EffectType::MedianFilter { .. } => "Median Filter",
            EffectType::SobelEdges { .. } => "Sobel Edges",
            EffectType::Mosaic { .. } => "Mosaic",
            EffectType::TiltShift { .. } => "Tilt Shift",
            EffectType::Emboss { .. } => "Emboss",
            EffectType::StarField { .. } => "Star Field",
            EffectType::LightningArc { .. } => "Lightning",
            EffectType::FireAutomaton { .. } => "Fire",
            EffectType::LumaKeyRange { .. } => "Luma Key Range",
            EffectType::Halftone { .. } => "Halftone",
            EffectType::Solarize { .. } => "Solarize",
            EffectType::PixelSort { .. } => "Pixel Sort",
            EffectType::PinchPunch { .. } => "Pinch / Punch",
            EffectType::ScanlineGlitch { .. } => "Scanline Glitch",
            EffectType::GlassEdgeBevel { .. } => "Glass Edge Bevel",
            EffectType::DirectionalSharpen { .. } => "Directional Sharpen",
            EffectType::RefractionLens { .. } => "Refraction Lens",
            EffectType::GradientMap { .. } => "Gradient Map",
            EffectType::LightLeak { .. } => "Light Leak",
            EffectType::BevelAlpha { .. } => "Bevel Alpha 3D",
            EffectType::CrossHatch { .. } => "Cross Hatch",
            EffectType::CmykHalftone { .. } => "CMYK Halftone",
            EffectType::ReflectionMap { .. } => "Reflection Map",
            EffectType::PerlinFlow { .. } => "Perlin Flow Noise",
            EffectType::FbmTurbulence { .. } => "FBM Turbulence",
            EffectType::SliderControl { .. } => "Slider Control",
            EffectType::AngleControl { .. } => "Angle Control",
            EffectType::PointControl { .. } => "Point Control",
            EffectType::ColorControl { .. } => "Color Control",
            EffectType::CheckboxControl { .. } => "Checkbox Control",
            EffectType::DropdownControl { .. } => "Dropdown Control",
            EffectType::Point3DControl { .. } => "3D Point Control",
            EffectType::LensFlare { .. } => "Lens Flare",
            EffectType::AudioSpectrum { .. } => "Audio Spectrum",
            EffectType::Letterbox { .. } => "Letterbox (Cinema Bars)",
            EffectType::CustomShader { .. } => "Custom Shader (WGSL)",
            EffectType::MergePaths { .. } => "Merge Paths",
            EffectType::OffsetPath { .. } => "Offset Path",
            EffectType::BassTreble { .. } => "Bass & Treble",
            EffectType::Flanger { .. } => "Flanger",
            EffectType::Chorus { .. } => "Chorus",
            EffectType::ParametricEQ { .. } => "Parametric EQ",
            EffectType::OpticalFlares { .. } => "Optical Flares",
            EffectType::MotionTile { .. } => "Motion Tile",
            EffectType::PageTurn { .. } => "CC Page Turn",
            EffectType::SetMatte { .. } => "Set Matte",
            EffectType::Echo { .. } => "Echo",
            EffectType::FindEdges { .. } => "Find Edges",
            EffectType::Transform { .. } => "Transform",
        }
    }

    fn type_id(&self) -> &str {
        match &self.effect_type {
            EffectType::GaussianBlur { .. } => "gaussian_blur",
            EffectType::ColorTint { .. } => "color_tint",
            EffectType::DropShadow { .. } => "drop_shadow",
            EffectType::ChromaticAberration { .. } => "chromatic_aberration",
            EffectType::Vignette { .. } => "vignette",
            EffectType::Levels { .. } => "levels",
            EffectType::HueSaturation { .. } => "hue_saturation",
            EffectType::Glow { .. } => "glow",
            EffectType::MotionBlur { .. } => "motion_blur",
            EffectType::MeshWarp { .. } => "mesh_warp",
            EffectType::ColorGradeLUT { .. } => "color_grade_lut",
            EffectType::ColorSpaceConvert { .. } => "color_space_convert",
            EffectType::FilmGrain { .. } => "film_grain",
            EffectType::Twirl { .. } => "twirl",
            EffectType::Bulge { .. } => "bulge",
            EffectType::Posterize { .. } => "posterize",
            EffectType::Invert { .. } => "invert",
            EffectType::Offset { .. } => "offset",
            EffectType::DirectionalBlur { .. } => "directional_blur",
            EffectType::RadialBlur { .. } => "radial_blur",
            EffectType::Sharpen { .. } => "sharpen",
            EffectType::Threshold { .. } => "threshold",
            EffectType::LinearWipe { .. } => "linear_wipe",
            EffectType::SimpleChoker { .. } => "simple_choker",
            EffectType::ChromaKey { .. } => "chroma_key",
            EffectType::Spherize { .. } => "spherize",
            EffectType::CornerPin { .. } => "corner_pin",
            EffectType::TurbulentDisplace { .. } => "turbulent_displace",
            EffectType::Colorama { .. } => "colorama",
            EffectType::FractalNoise { .. } => "fractal_noise",
            EffectType::Curves { .. } => "curves",
            EffectType::DisplacementMap { .. } => "displacement_map",
            EffectType::CompoundBlur { .. } => "compound_blur",
            EffectType::Minimax { .. } => "minimax",
            EffectType::ShiftChannels { .. } => "shift_channels",
            EffectType::WaveWarp { .. } => "wave_warp",
            EffectType::CcLens { .. } => "cc_lens",
            EffectType::PolarCoordinates { .. } => "polar_coordinates",
            EffectType::OpticsCompensation { .. } => "optics_compensation",
            EffectType::ColorBalance { .. } => "color_balance",
            EffectType::ChannelMixer { .. } => "channel_mixer",
            EffectType::LightSweep { .. } => "light_sweep",
            EffectType::RadialFastBlur { .. } => "radial_fast_blur",
            EffectType::BendIt { .. } => "bend_it",
            EffectType::Tiler { .. } => "tiler",
            EffectType::Tritone { .. } => "tritone",
            EffectType::MatteChoker { .. } => "matte_choker",
            EffectType::VenetianBlinds { .. } => "venetian_blinds",
            EffectType::Vibrance { .. } => "vibrance",
            EffectType::WhiteBalance { .. } => "white_balance",
            EffectType::HslAdjust { .. } => "hsl_adjust",
            EffectType::GlowPro { .. } => "glow_pro",
            EffectType::CrtScanlines { .. } => "crt_scanlines",
            EffectType::Vortex { .. } => "vortex_distortion",
            EffectType::HeatDistortion { .. } => "heat_distortion",
            EffectType::RainRipples { .. } => "rain_ripples",
            EffectType::Fisheye { .. } => "fisheye",
            EffectType::LensCorrection { .. } => "lens_correction",
            EffectType::GlitchDisplacement { .. } => "glitch_displacement",
            EffectType::MatteChokeSpread { .. } => "matte_choke_spread",
            EffectType::AlphaFeather { .. } => "alpha_feather",
            EffectType::AlphaFromLuminance { .. } => "alpha_from_luminance",
            EffectType::NightVision { .. } => "night_vision",
            EffectType::IrisWipe { .. } => "iris_wipe",
            EffectType::RadialWipe { .. } => "radial_wipe",
            EffectType::FilmEmulation { .. } => "film_emulation",
            EffectType::GodRays { .. } => "god_rays",
            EffectType::RadialBlurZoom { .. } => "radial_blur_zoom",
            EffectType::MedianFilter { .. } => "median_filter",
            EffectType::SobelEdges { .. } => "sobel_edges",
            EffectType::Mosaic { .. } => "mosaic",
            EffectType::TiltShift { .. } => "tilt_shift",
            EffectType::Emboss { .. } => "emboss",
            EffectType::StarField { .. } => "star_field",
            EffectType::LightningArc { .. } => "lightning_arc",
            EffectType::FireAutomaton { .. } => "fire_automaton",
            EffectType::LumaKeyRange { .. } => "luma_key_range",
            EffectType::Halftone { .. } => "halftone",
            EffectType::Solarize { .. } => "solarize",
            EffectType::PixelSort { .. } => "pixel_sort",
            EffectType::PinchPunch { .. } => "pinch_punch",
            EffectType::ScanlineGlitch { .. } => "scanline_glitch",
            EffectType::GlassEdgeBevel { .. } => "glass_edge_bevel",
            EffectType::DirectionalSharpen { .. } => "directional_sharpen",
            EffectType::RefractionLens { .. } => "refraction_lens",
            EffectType::GradientMap { .. } => "gradient_map",
            EffectType::LightLeak { .. } => "light_leak",
            EffectType::BevelAlpha { .. } => "bevel_alpha",
            EffectType::CrossHatch { .. } => "cross_hatch",
            EffectType::CmykHalftone { .. } => "cmyk_halftone",
            EffectType::ReflectionMap { .. } => "reflection_map",
            EffectType::PerlinFlow { .. } => "perlin_flow",
            EffectType::FbmTurbulence { .. } => "fbm_turbulence",
            EffectType::SliderControl { .. } => "slider_control",
            EffectType::AngleControl { .. } => "angle_control",
            EffectType::PointControl { .. } => "point_control",
            EffectType::ColorControl { .. } => "color_control",
            EffectType::CheckboxControl { .. } => "checkbox_control",
            EffectType::DropdownControl { .. } => "dropdown_control",
            EffectType::Point3DControl { .. } => "point3d_control",
            EffectType::LensFlare { .. } => "lens_flare",
            EffectType::AudioSpectrum { .. } => "audio_spectrum",
            EffectType::Letterbox { .. } => "letterbox",
            EffectType::CustomShader { .. } => "custom_shader",
            EffectType::MergePaths { .. } => "merge_paths",
            EffectType::OffsetPath { .. } => "offset_path",
            EffectType::BassTreble { .. } => "bass_treble",
            EffectType::Flanger { .. } => "flanger",
            EffectType::Chorus { .. } => "chorus",
            EffectType::ParametricEQ { .. } => "parametric_eq",
            EffectType::OpticalFlares { .. } => "optical_flares",
            EffectType::MotionTile { .. } => "motion_tile",
            EffectType::PageTurn { .. } => "page_turn",
            EffectType::SetMatte { .. } => "set_matte",
            EffectType::Echo { .. } => "echo",
            EffectType::FindEdges { .. } => "find_edges",
            EffectType::Transform { .. } => "transform",
        }
    }

    fn apply_to_params(&self, frame: u32, params: &mut EffectParams) {
        match &self.effect_type {
            EffectType::GaussianBlur { blur_radius } => {
                params.blur_enabled = 1;
                params.blur_radius = blur_radius.evaluate(frame);
            }
            EffectType::ColorTint { color, intensity } => {
                params.tint_enabled = 1;
                params.tint_color = color.evaluate(frame);
                params.tint_intensity = intensity.evaluate(frame) / 100.0;
            }
            EffectType::DropShadow {
                color, opacity, direction, distance, softness
            } => {
                params.shadow_enabled = 1;
                params.shadow_color = color.evaluate(frame);
                params.shadow_opacity = opacity.evaluate(frame) / 100.0;
                params.shadow_direction = direction.evaluate(frame);
                params.shadow_distance = distance.evaluate(frame);
                params.shadow_softness = softness.evaluate(frame);
            }
            EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff, iris_linked: _ } => {
                params.chromatic_enabled = 1;
                params.chromatic_shift_r = shift_r.evaluate(frame);
                params.chromatic_shift_b = shift_b.evaluate(frame);
                params.chromatic_edge_falloff = edge_falloff.evaluate(frame);
            }
            EffectType::Vignette { intensity, roundness, feather, color } => {
                params.vignette_enabled = 1;
                params.vignette_intensity = intensity.evaluate(frame) / 100.0;
                params.vignette_roundness = roundness.evaluate(frame);
                params.vignette_feather = feather.evaluate(frame) / 100.0;
                params.vignette_color = color.evaluate(frame);
            }
            EffectType::ColorGradeLUT { intensity, .. } => {
                params.lut_enabled = 1;
                params.lut_intensity = intensity.evaluate(frame) / 100.0;
            }
            EffectType::ColorSpaceConvert { mode } => {
                let mode_val = match mode {
                    ColorConversionMode::LogCToLinear => 1,
                    ColorConversionMode::LinearToLogC => 2,
                    ColorConversionMode::SLog3ToLinear => 3,
                    ColorConversionMode::LinearToSLog3 => 4,
                };
                params.color_convert_mode = mode_val;
            }
            EffectType::FilmGrain { intensity, grain_size, .. } => {
                params.grain_enabled = 1;
                params.grain_intensity = intensity.evaluate(frame) / 100.0;
                params.grain_size = *grain_size;
            }
            EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
                params.levels_enabled = 1;
                params.levels_in_black = input_black.evaluate(frame);
                params.levels_in_white = input_white.evaluate(frame);
                params.levels_gamma = gamma.evaluate(frame);
                params.levels_out_black = output_black.evaluate(frame);
                params.levels_out_white = output_white.evaluate(frame);
            }
            EffectType::HueSaturation { hue_shift, saturation, lightness } => {
                params.huesat_enabled = 1;
                // Map percentages or values to HSL shift ratios
                params.huesat_hue = hue_shift.evaluate(frame);
                params.huesat_sat = 1.0 + (saturation.evaluate(frame) / 100.0);
                params.huesat_light = 1.0 + (lightness.evaluate(frame) / 100.0);
            }
            EffectType::Glow { threshold, radius, intensity, color } => {
                params.glow_enabled = 1;
                params.glow_threshold = threshold.evaluate(frame) / 100.0;
                params.glow_radius = radius.evaluate(frame);
                params.glow_intensity = intensity.evaluate(frame) / 100.0;
                params.glow_color = color.evaluate(frame);
            }
            EffectType::GlowPro { threshold, radius, intensity } => {
                // Reuse the GPU glow/bloom path; white tint comes from the default.
                params.glow_enabled = 1;
                params.glow_threshold = threshold.evaluate(frame).clamp(0.0, 1.0);
                params.glow_radius = radius.evaluate(frame);
                params.glow_intensity = (intensity.evaluate(frame) / 4.0).clamp(0.0, 1.0);
            }
            EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
                params.meshwarp_enabled = 1;
                params.corner_top_left = top_left.evaluate(frame);
                params.corner_top_right = top_right.evaluate(frame);
                params.corner_bottom_left = bottom_left.evaluate(frame);
                params.corner_bottom_right = bottom_right.evaluate(frame);
            }
            EffectType::MotionBlur { shutter_angle, samples } => {
                params.motionblur_enabled = 1;
                params.motionblur_shutter = shutter_angle.evaluate(frame) / 360.0;
                params.motionblur_samples = *samples;
            }
            EffectType::LensFlare { enabled, position_x, position_y, intensity, threshold, color, .. } => {
                params.flare_enabled = if enabled.evaluate(frame) > 0.5 { 1 } else { 0 };
                params.flare_pos_x = position_x.evaluate(frame);
                params.flare_pos_y = position_y.evaluate(frame);
                params.flare_intensity = intensity.evaluate(frame);
                params.flare_threshold = threshold.evaluate(frame);
                let c = color.evaluate(frame);
                params.flare_color = [c[0], c[1], c[2], c[3]];
            }
            _ => {}
        }
    }
}

/// Evaluate all active plugins on a layer's effects into a single EffectParams.
pub fn evaluate_effects(effects: &[crate::core::timeline::Effect], frame: u32) -> EffectParams {
    let mut params = EffectParams::default();
    for effect in effects {
        if effect.enabled {
            let plugin = EnumEffectPlugin { effect_type: effect.effect_type.clone() };
            plugin.apply_to_params(frame, &mut params);
        }
    }
    params
}

/// Thread-safe central registry for dynamic effect plugins.
pub struct EffectPluginRegistry {
    plugins: std::sync::RwLock<std::collections::HashMap<String, Box<dyn RenderEffectPlugin>>>,
}

impl Default for EffectPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn register(&self, plugin: Box<dyn RenderEffectPlugin>) {
        let key = plugin.type_id().to_string();
        if let Ok(mut guard) = self.plugins.write() {
            guard.insert(key, plugin);
        }
    }

    pub fn apply_all(&self, plugin_ids: &[String], frame: u32, params: &mut EffectParams) {
        if let Ok(guard) = self.plugins.read() {
            for id in plugin_ids {
                if let Some(plugin) = guard.get(id) {
                    plugin.apply_to_params(frame, params);
                }
            }
        }
    }
}
