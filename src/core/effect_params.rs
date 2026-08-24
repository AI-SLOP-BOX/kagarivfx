//! Centralized reflection over `EffectType` keyframeable parameters.
//!
//! The timeline property rows need mutable access to every `Animatable`
//! track inside an effect. Maintaining that match in the UI layer meant
//! new effect variants silently got no rows. This module is the single
//! place to register params: extend the match here when adding variants.
//!
//! Variants not listed fall through the catch-all and simply expose no
//! timeline rows (same as before) — adding a variant never breaks the build.

use crate::core::property::Animatable;
use crate::core::timeline::EffectType;

/// Mutable borrow of one keyframeable parameter track.
pub enum ParamRef<'a> {
    Scalar(&'a mut Animatable<f32>),
    Vec2(&'a mut Animatable<[f32; 2]>),
    Vec4Color(&'a mut Animatable<[f32; 4]>),
}

impl EffectType {
    /// All keyframeable parameters of this effect, in display order.
    pub fn animatable_params(&mut self) -> Vec<(&'static str, ParamRef<'_>)> {
        let mut out: Vec<(&'static str, ParamRef<'_>)> = Vec::new();
        macro_rules! push {
            ($label:expr, $field:expr, Scalar) => { out.push(($label, ParamRef::Scalar($field))) };
            ($label:expr, $field:expr, Vec2) => { out.push(($label, ParamRef::Vec2($field))) };
            ($label:expr, $field:expr, Color) => { out.push(($label, ParamRef::Vec4Color($field))) };
        }
        match self {
            EffectType::GaussianBlur { blur_radius } => push!("Blur Radius", blur_radius, Scalar),
            EffectType::ColorTint { color, intensity } => {
                push!("Tint Color", color, Color);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::DropShadow { opacity, direction, distance, softness, .. } => {
                push!("Opacity", opacity, Scalar);
                push!("Direction", direction, Scalar);
                push!("Distance", distance, Scalar);
                push!("Softness", softness, Scalar);
            }
            EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
                push!("Red Shift", shift_r, Scalar);
                push!("Blue Shift", shift_b, Scalar);
                push!("Edge Falloff", edge_falloff, Scalar);
            }
            EffectType::Vignette { intensity, roundness, feather, .. } => {
                push!("Intensity", intensity, Scalar);
                push!("Roundness", roundness, Scalar);
                push!("Feather", feather, Scalar);
            }
            EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
                push!("Input Black", input_black, Scalar);
                push!("Input White", input_white, Scalar);
                push!("Gamma", gamma, Scalar);
                push!("Output Black", output_black, Scalar);
                push!("Output White", output_white, Scalar);
            }
            EffectType::HueSaturation { hue_shift, saturation, lightness } => {
                push!("Hue Shift", hue_shift, Scalar);
                push!("Saturation", saturation, Scalar);
                push!("Lightness", lightness, Scalar);
            }
            EffectType::Glow { threshold, radius, intensity, .. } => {
                push!("Threshold", threshold, Scalar);
                push!("Radius", radius, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::MotionBlur { shutter_angle, .. } => push!("Shutter Angle", shutter_angle, Scalar),
            EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
                push!("Top Left", top_left, Vec2);
                push!("Top Right", top_right, Vec2);
                push!("Bottom Left", bottom_left, Vec2);
                push!("Bottom Right", bottom_right, Vec2);
            }
            EffectType::CornerPin { top_left, top_right, bottom_right, bottom_left } => {
                push!("Top Left", top_left, Vec2);
                push!("Top Right", top_right, Vec2);
                push!("Bottom Right", bottom_right, Vec2);
                push!("Bottom Left", bottom_left, Vec2);
            }
            EffectType::FilmGrain { intensity, .. } => push!("Grain Intensity", intensity, Scalar),
            EffectType::Twirl { angle, radius } => {
                push!("Angle", angle, Scalar);
                push!("Radius", radius, Scalar);
            }
            EffectType::Bulge { amount, radius } => {
                push!("Amount", amount, Scalar);
                push!("Radius", radius, Scalar);
            }
            EffectType::Posterize { levels } => push!("Levels", levels, Scalar),
            EffectType::Offset { shift_x, shift_y } => {
                push!("Shift X", shift_x, Scalar);
                push!("Shift Y", shift_y, Scalar);
            }
            EffectType::DirectionalBlur { angle, length } => {
                push!("Angle", angle, Scalar);
                push!("Length", length, Scalar);
            }
            EffectType::RadialBlur { amount } => push!("Amount", amount, Scalar),
            EffectType::RadialFastBlur { amount, .. } => push!("Amount", amount, Scalar),
            EffectType::Sharpen { amount } => push!("Amount", amount, Scalar),
            EffectType::Threshold { threshold } => push!("Threshold", threshold, Scalar),
            EffectType::LinearWipe { completion, angle } => {
                push!("Completion", completion, Scalar);
                push!("Angle", angle, Scalar);
            }
            EffectType::SimpleChoker { choke_amount } => push!("Choke Amount", choke_amount, Scalar),
            EffectType::ChromaKey { screen_gain, clip_black, clip_white, .. } => {
                push!("Screen Gain", screen_gain, Scalar);
                push!("Clip Black", clip_black, Scalar);
                push!("Clip White", clip_white, Scalar);
            }
            EffectType::Spherize { radius, refractive_index } => {
                push!("Radius", radius, Scalar);
                push!("Refractive Index", refractive_index, Scalar);
            }
            EffectType::TurbulentDisplace { amount, size, evolution, .. } => {
                push!("Amount", amount, Scalar);
                push!("Size", size, Scalar);
                push!("Evolution", evolution, Scalar);
            }
            EffectType::DisplacementMap { .. } => {}
            EffectType::CompoundBlur { .. } => {}
            EffectType::Minimax { .. } => {}
            EffectType::ShiftChannels { .. } => {}
            EffectType::WaveWarp { .. } => {}
            EffectType::CcLens { .. } => {}
            EffectType::PolarCoordinates { .. } => {}
            EffectType::OpticsCompensation { .. } => {}
            EffectType::ColorBalance { .. } => {}
            EffectType::ChannelMixer { .. } => {}
            EffectType::LightSweep { .. } => {}
            EffectType::BendIt { .. } => {}
            EffectType::Tiler { .. } => {}
            EffectType::Tritone { .. } => {}
            EffectType::MatteChoker { .. } => {}
            EffectType::VenetianBlinds { .. } => {}
            EffectType::Vibrance { amount } => push!("Amount", amount, Scalar),
            EffectType::WhiteBalance { .. } => {}
            EffectType::HslAdjust { .. } => {}
            EffectType::GlowPro { .. } => {}
            EffectType::CrtScanlines { .. } => {}
            EffectType::Vortex { .. } => {}
            EffectType::HeatDistortion { .. } => {}
            EffectType::RainRipples { .. } => {}
            EffectType::Fisheye { .. } => {}
            EffectType::LensCorrection { .. } => {}
            EffectType::GlitchDisplacement { .. } => {}
            EffectType::MatteChokeSpread { .. } => {}
            EffectType::AlphaFeather { .. } => {}
            EffectType::AlphaFromLuminance { .. } => {}
            EffectType::NightVision { .. } => {}
            EffectType::IrisWipe { .. } => {}
            EffectType::RadialWipe { .. } => {}
            EffectType::FilmEmulation { .. } => {}
            EffectType::GodRays { .. } => {}
            EffectType::RadialBlurZoom { amount } => push!("Zoom Amount", amount, Scalar),
            // Catch-all keeps this future-proof: new variants compile fine
            // and just show no rows until registered above.
            _ => {}
        }
        out
    }
}
