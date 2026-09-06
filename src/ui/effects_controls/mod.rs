mod audio;
mod blur;
mod channel_util;
mod color_correction;
mod common;
mod custom_shader;
mod distort;
mod expression_controls;
mod generate;
mod halftone;
mod keying;
mod particle;
mod path_shape;
pub mod presets;
mod sharpen_edge;
mod stylize;
mod transform;
mod transition;

pub use presets::EffectPreset;
pub use particle::draw_particle_emitter_controls;

use crate::core::timeline::EffectType;
use eframe::egui;

pub fn get_all_effect_presets() -> &'static [EffectPreset] {
    presets::get_all_effect_presets()
}

pub fn draw_effect_type_ui(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        // ── Color Correction ──
        EffectType::GaussianBlur { .. }
        | EffectType::ColorTint { .. }
        | EffectType::Levels { .. }
        | EffectType::HueSaturation { .. }
        | EffectType::ColorGradeLUT { .. }
        | EffectType::ColorSpaceConvert { .. }
        | EffectType::FilmGrain { .. }
        | EffectType::Posterize { .. }
        | EffectType::Invert { .. }
        | EffectType::Threshold { .. }
        | EffectType::ShiftChannels { .. }
        | EffectType::Curves { .. }
        | EffectType::ColorBalance { .. }
        | EffectType::ChannelMixer { .. }
        | EffectType::Tritone { .. }
        | EffectType::Colorama { .. }
        | EffectType::Vibrance { .. }
        | EffectType::WhiteBalance { .. }
        | EffectType::HslAdjust { .. }
        | EffectType::FilmEmulation { .. }
        | EffectType::GradientMap { .. } => {
            color_correction::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Blur ──
        EffectType::DirectionalBlur { .. }
        | EffectType::RadialBlur { .. }
        | EffectType::CompoundBlur { .. }
        | EffectType::RadialBlurZoom { .. }
        | EffectType::TiltShift { .. }
        | EffectType::CameraLensBlur { .. }
        | EffectType::MotionBlur { .. } => {
            blur::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Distort ──
        EffectType::MeshWarp { .. }
        | EffectType::CornerPin { .. }
        | EffectType::Twirl { .. }
        | EffectType::Bulge { .. }
        | EffectType::Spherize { .. }
        | EffectType::TurbulentDisplace { .. }
        | EffectType::WaveWarp { .. }
        | EffectType::CcLens { .. }
        | EffectType::PolarCoordinates { .. }
        | EffectType::OpticsCompensation { .. }
        | EffectType::BendIt { .. }
        | EffectType::Tiler { .. }
        | EffectType::Vortex { .. }
        | EffectType::HeatDistortion { .. }
        | EffectType::RainRipples { .. }
        | EffectType::Fisheye { .. }
        | EffectType::LensCorrection { .. }
        | EffectType::GlitchDisplacement { .. }
        | EffectType::PinchPunch { .. }
        | EffectType::RefractionLens { .. }
        | EffectType::GlassEdgeBevel { .. }
        | EffectType::RadialFastBlur { .. }
        | EffectType::ScanlineGlitch { .. } => {
            distort::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Generate ──
        EffectType::FractalNoise { .. }
        | EffectType::StarField { .. }
        | EffectType::LightningArc { .. }
        | EffectType::LaserBeam { .. }
        | EffectType::FireAutomaton { .. }
        | EffectType::PerlinFlow { .. }
        | EffectType::FbmTurbulence { .. }
        | EffectType::LightLeak { .. }
        | EffectType::GodRays { .. } => {
            generate::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Stylize ──
        EffectType::DropShadow { .. }
        | EffectType::ChromaticAberration { .. }
        | EffectType::Vignette { .. }
        | EffectType::Glow { .. }
        | EffectType::GlowPro { .. }
        | EffectType::Emboss { .. }
        | EffectType::NightVision { .. }
        | EffectType::CrtScanlines { .. }
        | EffectType::BevelAlpha { .. }
        | EffectType::LensFlare { .. }
        | EffectType::OpticalFlares { .. }
        | EffectType::ReflectionMap { .. } => {
            stylize::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Keying ──
        EffectType::ChromaKey { .. }
        | EffectType::LumaKeyRange { .. }
        | EffectType::SimpleChoker { .. }
        | EffectType::MatteChokeSpread { .. }
        | EffectType::AlphaFeather { .. }
        | EffectType::AlphaFromLuminance { .. }
        | EffectType::MatteChoker { .. }
        | EffectType::SetMatte { .. }
        | EffectType::LinearColorKey { .. } => {
            keying::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Transition ──
        EffectType::LinearWipe { .. }
        | EffectType::IrisWipe { .. }
        | EffectType::RadialWipe { .. }
        | EffectType::VenetianBlinds { .. }
        | EffectType::PageTurn { .. }
        | EffectType::LightSweep { .. } => {
            transition::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Transform ──
        EffectType::Offset { .. }
        | EffectType::MotionTile { .. }
        | EffectType::Transform { .. }
        | EffectType::DisplacementMap { .. } => {
            transform::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Sharpen / Edge ──
        EffectType::Sharpen { .. }
        | EffectType::SobelEdges { .. }
        | EffectType::DirectionalSharpen { .. }
        | EffectType::Mosaic { .. }
        | EffectType::MedianFilter { .. }
        | EffectType::FindEdges { .. }
        | EffectType::Minimax { .. } => {
            sharpen_edge::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Halftone ──
        EffectType::Halftone { .. }
        | EffectType::Solarize { .. }
        | EffectType::PixelSort { .. }
        | EffectType::CrossHatch { .. }
        | EffectType::CmykHalftone { .. } => {
            halftone::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Audio ──
        EffectType::AudioSpectrum { .. }
        | EffectType::BassTreble { .. }
        | EffectType::Flanger { .. }
        | EffectType::Chorus { .. }
        | EffectType::ParametricEQ { .. }
        | EffectType::Echo { .. } => {
            audio::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Expression Controls ──
        EffectType::SliderControl { .. }
        | EffectType::AngleControl { .. }
        | EffectType::PointControl { .. }
        | EffectType::Letterbox { .. }
        | EffectType::ColorControl { .. }
        | EffectType::CheckboxControl { .. }
        | EffectType::DropdownControl { .. }
        | EffectType::Point3DControl { .. } => {
            expression_controls::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Path / Shape ──
        EffectType::MergePaths { .. } | EffectType::OffsetPath { .. } => {
            path_shape::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Custom Shader ──
        EffectType::CustomShader { .. } => {
            custom_shader::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }

        // ── Channel Utilities ──
        EffectType::ChannelCombiner { .. } => {
            channel_util::draw(effect_type, ui, current_frame, project_changed, next_frame);
        }
    }
}
