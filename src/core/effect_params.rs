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
    Vec3(&'a mut Animatable<[f32; 3]>),
    Vec4Color(&'a mut Animatable<[f32; 4]>),
}

/// Immutable borrow of one keyframeable parameter track.
pub enum ParamRefRef<'a> {
    Scalar(&'a Animatable<f32>),
    Vec2(&'a Animatable<[f32; 2]>),
    Vec3(&'a Animatable<[f32; 3]>),
    Vec4Color(&'a Animatable<[f32; 4]>),
}

impl EffectType {
    /// All keyframeable parameters of this effect, in display order.
    pub fn animatable_params(&mut self) -> Vec<(&'static str, ParamRef<'_>)> {
        let mut out: Vec<(&'static str, ParamRef<'_>)> = Vec::new();
        macro_rules! push {
            ($label:expr, $field:expr, Scalar) => {
                out.push(($label, ParamRef::Scalar($field)))
            };
            ($label:expr, $field:expr, Vec2) => {
                out.push(($label, ParamRef::Vec2($field)))
            };
            ($label:expr, $field:expr, Vec3) => {
                out.push(($label, ParamRef::Vec3($field)))
            };
            ($label:expr, $field:expr, Color) => {
                out.push(($label, ParamRef::Vec4Color($field)))
            };
        }
        match self {
            EffectType::GaussianBlur { blur_radius } => push!("Blur Radius", blur_radius, Scalar),
            EffectType::ColorTint { color, intensity } => {
                push!("Tint Color", color, Color);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::DropShadow {
                color,
                opacity,
                direction,
                distance,
                softness,
            } => {
                push!("Shadow Color", color, Color);
                push!("Opacity", opacity, Scalar);
                push!("Direction", direction, Scalar);
                push!("Distance", distance, Scalar);
                push!("Softness", softness, Scalar);
            }
            EffectType::ChromaticAberration {
                shift_r,
                shift_b,
                edge_falloff,
                iris_linked: _,
            } => {
                push!("Red Shift", shift_r, Scalar);
                push!("Blue Shift", shift_b, Scalar);
                push!("Edge Falloff", edge_falloff, Scalar);
            }
            EffectType::Vignette {
                intensity,
                roundness,
                feather,
                color,
            } => {
                push!("Vignette Color", color, Color);
                push!("Intensity", intensity, Scalar);
                push!("Roundness", roundness, Scalar);
                push!("Feather", feather, Scalar);
            }
            EffectType::Levels {
                input_black,
                input_white,
                gamma,
                output_black,
                output_white,
            } => {
                push!("Input Black", input_black, Scalar);
                push!("Input White", input_white, Scalar);
                push!("Gamma", gamma, Scalar);
                push!("Output Black", output_black, Scalar);
                push!("Output White", output_white, Scalar);
            }
            EffectType::HueSaturation {
                hue_shift,
                saturation,
                lightness,
            } => {
                push!("Hue Shift", hue_shift, Scalar);
                push!("Saturation", saturation, Scalar);
                push!("Lightness", lightness, Scalar);
            }
            EffectType::Glow {
                threshold,
                radius,
                intensity,
                color,
            } => {
                push!("Glow Color", color, Color);
                push!("Threshold", threshold, Scalar);
                push!("Radius", radius, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::LensFlare {
                enabled,
                position_x,
                position_y,
                intensity,
                threshold,
                color,
                ..
            } => {
                push!("Flare Enabled", enabled, Scalar);
                push!("Position X", position_x, Scalar);
                push!("Position Y", position_y, Scalar);
                push!("Intensity", intensity, Scalar);
                push!("Threshold", threshold, Scalar);
                push!("Flare Color", color, Color);
            }
            EffectType::MotionBlur { shutter_angle, .. } => {
                push!("Shutter Angle", shutter_angle, Scalar)
            }
            EffectType::MeshWarp {
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => {
                push!("Top Left", top_left, Vec2);
                push!("Top Right", top_right, Vec2);
                push!("Bottom Left", bottom_left, Vec2);
                push!("Bottom Right", bottom_right, Vec2);
            }
            EffectType::CornerPin {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => {
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
            EffectType::SimpleChoker { choke_amount } => {
                push!("Choke Amount", choke_amount, Scalar)
            }
            EffectType::ChromaKey {
                screen_gain,
                clip_black,
                clip_white,
                ..
            } => {
                push!("Screen Gain", screen_gain, Scalar);
                push!("Clip Black", clip_black, Scalar);
                push!("Clip White", clip_white, Scalar);
            }
            EffectType::Spherize {
                radius,
                refractive_index,
            } => {
                push!("Radius", radius, Scalar);
                push!("Refractive Index", refractive_index, Scalar);
            }
            EffectType::TurbulentDisplace {
                amount,
                size,
                evolution,
                complexity,
            } => {
                push!("Amount", amount, Scalar);
                push!("Size", size, Scalar);
                push!("Evolution", evolution, Scalar);
                push!("Complexity", complexity, Scalar);
            }
            EffectType::DisplacementMap {
                source_layer,
                max_horizontal,
                max_vertical,
            } => {
                push!("Source Layer Idx", source_layer, Scalar);
                push!("Max Horizontal", max_horizontal, Scalar);
                push!("Max Vertical", max_vertical, Scalar);
            }
            EffectType::CompoundBlur {
                source_layer,
                max_blur,
            } => {
                push!("Source Layer Idx", source_layer, Scalar);
                push!("Max Blur", max_blur, Scalar);
            }
            EffectType::Minimax { operation, radius } => {
                push!("Operation", operation, Scalar);
                push!("Radius", radius, Scalar);
            }
            EffectType::ShiftChannels {
                take_red,
                take_green,
                take_blue,
                take_alpha,
            } => {
                push!("Take Red", take_red, Scalar);
                push!("Take Green", take_green, Scalar);
                push!("Take Blue", take_blue, Scalar);
                push!("Take Alpha", take_alpha, Scalar);
            }
            EffectType::WaveWarp {
                wave_height,
                wave_width,
                speed,
                direction_deg,
                ..
            } => {
                push!("Wave Height", wave_height, Scalar);
                push!("Wave Width", wave_width, Scalar);
                push!("Speed", speed, Scalar);
                push!("Direction °", direction_deg, Scalar);
            }
            EffectType::CcLens { convergence, zoom } => {
                push!("Convergence", convergence, Scalar);
                push!("Zoom", zoom, Scalar);
            }
            EffectType::PolarCoordinates { interpolation, .. } => {
                push!("Interpolation", interpolation, Scalar)
            }
            EffectType::OpticsCompensation {
                field_of_view_deg,
                zoom,
                ..
            } => {
                push!("Field of View", field_of_view_deg, Scalar);
                push!("Zoom", zoom, Scalar);
            }
            EffectType::ColorBalance { .. } => {}
            EffectType::ChannelMixer { .. } => {}
            EffectType::LightSweep {
                direction_deg,
                center,
                width,
                sweep_intensity,
                edge_intensity,
            } => {
                push!("Direction °", direction_deg, Scalar);
                push!("Center", center, Scalar);
                push!("Width", width, Scalar);
                push!("Sweep Intensity", sweep_intensity, Scalar);
                push!("Edge Intensity", edge_intensity, Scalar);
            }
            EffectType::BendIt {
                top_offset,
                bottom_offset,
            } => {
                push!("Top Offset", top_offset, Scalar);
                push!("Bottom Offset", bottom_offset, Scalar);
            }
            EffectType::Tiler { scale_percent, .. } => push!("Scale %", scale_percent, Scalar),
            EffectType::Tritone { .. } => {}
            EffectType::MatteChoker {
                choke_amount,
                gray_level,
            } => {
                push!("Choke Amount", choke_amount, Scalar);
                push!("Gray Level", gray_level, Scalar);
            }
            EffectType::VenetianBlinds { completion, width } => {
                push!("Completion", completion, Scalar);
                push!("Stripe Width", width, Scalar);
            }
            EffectType::Vibrance { amount } => push!("Amount", amount, Scalar),
            EffectType::WhiteBalance { temperature, tint } => {
                push!("Temperature", temperature, Scalar);
                push!("Tint", tint, Scalar);
            }
            EffectType::HslAdjust {
                hue_deg,
                saturation,
                lightness,
            } => {
                push!("Hue °", hue_deg, Scalar);
                push!("Saturation", saturation, Scalar);
                push!("Lightness", lightness, Scalar);
            }
            EffectType::GlowPro {
                threshold,
                radius,
                intensity,
                ..
            } => {
                push!("Threshold", threshold, Scalar);
                push!("Radius", radius, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::CrtScanlines {
                line_spacing,
                intensity,
                ..
            } => {
                push!("Line Spacing", line_spacing, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::Vortex {
                radius, angle_deg, ..
            } => {
                push!("Radius", radius, Scalar);
                push!("Angle", angle_deg, Scalar);
            }
            EffectType::HeatDistortion {
                strength, speed, ..
            } => {
                push!("Strength", strength, Scalar);
                push!("Speed", speed, Scalar);
            }
            EffectType::RainRipples {
                drop_count,
                wave_strength,
                ..
            } => {
                push!("Drop Count", drop_count, Scalar);
                push!("Wave Strength", wave_strength, Scalar);
            }
            EffectType::Fisheye { strength, .. } => push!("Strength", strength, Scalar),
            EffectType::LensCorrection { k1, k2, .. } => {
                push!("K1", k1, Scalar);
                push!("K2", k2, Scalar);
            }
            EffectType::GlitchDisplacement { seed, amount, .. } => {
                push!("Seed", seed, Scalar);
                push!("Amount", amount, Scalar);
            }
            EffectType::MatteChokeSpread { radius, .. } => push!("Radius", radius, Scalar),
            EffectType::AlphaFeather { radius, .. } => push!("Radius", radius, Scalar),
            EffectType::AlphaFromLuminance { .. } => {}
            EffectType::NightVision { amplification, .. } => {
                push!("Amplification", amplification, Scalar)
            }
            EffectType::IrisWipe { completion, .. } => push!("Completion", completion, Scalar),
            EffectType::RadialWipe { completion, .. } => push!("Completion", completion, Scalar),
            EffectType::FilmEmulation {
                lift,
                gamma,
                gain,
                hue_shift_deg,
                ..
            } => {
                push!("Lift", lift, Scalar);
                push!("Gamma", gamma, Scalar);
                push!("Gain", gain, Scalar);
                push!("Hue Shift", hue_shift_deg, Scalar);
            }
            EffectType::GodRays {
                sun_x,
                sun_y,
                samples,
                decay,
                weight,
                ..
            } => {
                push!("Sun X", sun_x, Scalar);
                push!("Sun Y", sun_y, Scalar);
                push!("Samples", samples, Scalar);
                push!("Decay", decay, Scalar);
                push!("Weight", weight, Scalar);
            }
            EffectType::RadialBlurZoom { amount } => push!("Zoom Amount", amount, Scalar),
            EffectType::OpticalFlares {
                position,
                brightness,
                scale,
            } => {
                push!("Position", position, Vec2);
                push!("Brightness", brightness, Scalar);
                push!("Scale", scale, Scalar);
            }
            EffectType::MotionTile {
                tile_center,
                tile_width,
                tile_height,
                output_width,
                output_height,
                phase,
                ..
            } => {
                push!("Tile Center", tile_center, Vec2);
                push!("Tile Width", tile_width, Scalar);
                push!("Tile Height", tile_height, Scalar);
                push!("Output Width", output_width, Scalar);
                push!("Output Height", output_height, Scalar);
                push!("Phase", phase, Scalar);
            }
            EffectType::PageTurn {
                fold_position,
                fold_radius,
                fold_direction_deg,
                light_direction_deg,
                back_opacity,
                ..
            } => {
                push!("Fold Position", fold_position, Vec2);
                push!("Fold Radius", fold_radius, Scalar);
                push!("Fold Direction", fold_direction_deg, Scalar);
                push!("Light Direction", light_direction_deg, Scalar);
                push!("Back Opacity", back_opacity, Scalar);
            }
            EffectType::SetMatte { .. } => {}
            EffectType::Echo {
                echo_time_seconds,
                starting_intensity,
                decay,
                ..
            } => {
                push!("Echo Time (s)", echo_time_seconds, Scalar);
                push!("Starting Intensity", starting_intensity, Scalar);
                push!("Decay", decay, Scalar);
            }
            EffectType::FindEdges { .. } => {}
            EffectType::Transform {
                anchor_point,
                position,
                scale_width,
                scale_height,
                skew_deg,
                skew_axis_deg,
                rotation_deg,
                opacity,
                ..
            } => {
                push!("Anchor Point", anchor_point, Vec2);
                push!("Position", position, Vec2);
                push!("Scale Width", scale_width, Scalar);
                push!("Scale Height", scale_height, Scalar);
                push!("Skew", skew_deg, Scalar);
                push!("Skew Axis", skew_axis_deg, Scalar);
                push!("Rotation", rotation_deg, Scalar);
                push!("Opacity", opacity, Scalar);
            }
            EffectType::CameraLensBlur {
                blur_radius,
                iris_rotation_deg,
                iris_roundness,
                highlight_gain,
                highlight_threshold,
                ..
            } => {
                push!("Blur Radius", blur_radius, Scalar);
                push!("Iris Rotation", iris_rotation_deg, Scalar);
                push!("Iris Roundness", iris_roundness, Scalar);
                push!("Highlight Gain", highlight_gain, Scalar);
                push!("Highlight Threshold", highlight_threshold, Scalar);
            }
            EffectType::LinearColorKey {
                key_color,
                tolerance,
                softness,
                ..
            } => {
                push!("Key Color", key_color, Vec3);
                push!("Tolerance", tolerance, Scalar);
                push!("Softness", softness, Scalar);
            }
            EffectType::ChannelCombiner { .. } => {}
            // ── Newly registered effects (session 3) ──
            EffectType::BevelAlpha {
                depth,
                light_angle_deg,
            } => {
                push!("Depth", depth, Scalar);
                push!("Light Angle", light_angle_deg, Scalar);
            }
            EffectType::CrossHatch {
                line_gap,
                threshold,
            } => {
                push!("Line Gap", line_gap, Scalar);
                push!("Threshold", threshold, Scalar);
            }
            EffectType::CmykHalftone { dot_size } => push!("Dot Size", dot_size, Scalar),
            EffectType::ColorGradeLUT { intensity, .. } => {
                push!("LUT Intensity", intensity, Scalar)
            }
            EffectType::Colorama {
                preset_index,
                cycle_phase,
            } => {
                push!("Preset", preset_index, Scalar);
                push!("Cycle Phase", cycle_phase, Scalar);
            }
            EffectType::ColorSpaceConvert { .. } => {}
            EffectType::Curves { channel } => push!("Channel", channel, Scalar),
            EffectType::DirectionalSharpen {
                angle_deg,
                strength,
            } => {
                push!("Angle", angle_deg, Scalar);
                push!("Strength", strength, Scalar);
            }
            EffectType::Emboss { angle_deg, depth } => {
                push!("Angle", angle_deg, Scalar);
                push!("Depth", depth, Scalar);
            }
            EffectType::FbmTurbulence { octaves, amplitude } => {
                push!("Octaves", octaves, Scalar);
                push!("Amplitude", amplitude, Scalar);
            }
            EffectType::FireAutomaton { intensity } => push!("Intensity", intensity, Scalar),
            EffectType::FractalNoise {
                fractal_type,
                contrast,
                brightness,
                complexity,
                evolution,
            } => {
                push!("Fractal Type", fractal_type, Scalar);
                push!("Contrast", contrast, Scalar);
                push!("Brightness", brightness, Scalar);
                push!("Complexity", complexity, Scalar);
                push!("Evolution", evolution, Scalar);
            }
            EffectType::GradientMap { .. } => {} // [f32;3] not yet keyframeable
            EffectType::SliderControl { value } => push!("Value", value, Scalar),
            EffectType::AngleControl { angle_degrees } => push!("Angle", angle_degrees, Scalar),
            EffectType::PointControl { point } => push!("Point", point, Vec2),
            EffectType::ColorControl { color } => push!("Color", color, Color),
            EffectType::CheckboxControl { .. } => {}
            EffectType::DropdownControl { .. } => {}
            EffectType::Point3DControl { point } => push!("Point 3D", point, Vec3),
            EffectType::Letterbox { frac } => push!("Bars", frac, Scalar),
            EffectType::Halftone { cell_size } => push!("Cell Size", cell_size, Scalar),
            EffectType::Invert { .. } => {}
            EffectType::LightLeak {
                pos_x,
                pos_y,
                intensity,
            } => {
                push!("Position X", pos_x, Scalar);
                push!("Position Y", pos_y, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::LightningArc {
                start_x,
                start_y,
                end_x,
                end_y,
                seed,
                glow,
            } => {
                push!("Start X", start_x, Scalar);
                push!("Start Y", start_y, Scalar);
                push!("End X", end_x, Scalar);
                push!("End Y", end_y, Scalar);
                push!("Seed", seed, Scalar);
                push!("Glow", glow, Scalar);
            }
            EffectType::LaserBeam {
                start_x,
                start_y,
                end_x,
                end_y,
                progress,
                length,
                starting_thickness,
                ending_thickness,
                core_color,
                glow_color,
            } => {
                push!("Start X", start_x, Scalar);
                push!("Start Y", start_y, Scalar);
                push!("End X", end_x, Scalar);
                push!("End Y", end_y, Scalar);
                push!("Progress", progress, Scalar);
                push!("Length %", length, Scalar);
                push!("Start Thickness", starting_thickness, Scalar);
                push!("End Thickness", ending_thickness, Scalar);
                push!("Core Color", core_color, Color);
                push!("Glow Color", glow_color, Color);
            }
            EffectType::LumaKeyRange {
                low_threshold,
                high_threshold,
                ..
            } => {
                push!("Low Threshold", low_threshold, Scalar);
                push!("High Threshold", high_threshold, Scalar);
            }
            EffectType::MedianFilter { radius } => push!("Radius", radius, Scalar),
            EffectType::Mosaic { block_w, block_h } => {
                push!("Block Width", block_w, Scalar);
                push!("Block Height", block_h, Scalar);
            }
            EffectType::PerlinFlow { scale } => push!("Scale", scale, Scalar),
            EffectType::PinchPunch { radius, amount } => {
                push!("Radius", radius, Scalar);
                push!("Amount", amount, Scalar);
            }
            EffectType::PixelSort { threshold } => push!("Threshold", threshold, Scalar),
            EffectType::ReflectionMap {
                reflect_y,
                fade_dist,
                opacity,
            } => {
                push!("Reflect Y", reflect_y, Scalar);
                push!("Fade Distance", fade_dist, Scalar);
                push!("Opacity", opacity, Scalar);
            }
            EffectType::RefractionLens { radius, ior } => {
                push!("Radius", radius, Scalar);
                push!("IOR", ior, Scalar);
            }
            EffectType::ScanlineGlitch {
                jitter_amount,
                seed,
            } => {
                push!("Jitter", jitter_amount, Scalar);
                push!("Seed", seed, Scalar);
            }
            EffectType::SobelEdges { .. } => {}
            EffectType::Solarize { threshold } => push!("Threshold", threshold, Scalar),
            EffectType::TiltShift {
                focus_y,
                focus_height,
                max_blur,
            } => {
                push!("Focus Y", focus_y, Scalar);
                push!("Focus Height", focus_height, Scalar);
                push!("Max Blur", max_blur, Scalar);
            }
            EffectType::GlassEdgeBevel {
                bevel_size,
                refraction,
            } => {
                push!("Bevel Size", bevel_size, Scalar);
                push!("Refraction", refraction, Scalar);
            }
            // Catch-all keeps this future-proof: new variants compile fine
            // and just show no rows until registered above.
            EffectType::StarField {
                num_stars,
                depth_speed,
            } => {
                push!("Star Count", num_stars, Scalar);
                push!("Depth Speed", depth_speed, Scalar);
            }
            EffectType::MergePaths { operation } => {
                push!("Operation", operation, Scalar);
            }
            EffectType::OffsetPath {
                amount,
                line_join,
                miter_limit,
            } => {
                push!("Amount", amount, Scalar);
                push!("Line Join", line_join, Scalar);
                push!("Miter Limit", miter_limit, Scalar);
            }
            EffectType::CustomShader { .. } => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
        out
    }

    /// Apply one interpolation mode to every existing keyframe in this effect.
    /// Returns whether at least one keyframe changed.
    pub fn set_keyframe_interpolation(
        &mut self,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        self.set_parameter_keyframe_interpolation(None, interpolation)
    }

    pub fn set_parameter_keyframe_interpolation(
        &mut self,
        parameter_name: Option<&str>,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        fn apply<T: Clone>(
            track: &mut Animatable<T>,
            interpolation: crate::core::keyframe::InterpolationType,
        ) -> bool {
            let Some(keyframes) = track.keyframes_mut() else {
                return false;
            };
            let mut changed = false;
            for keyframe in keyframes {
                if keyframe.interpolation != interpolation {
                    keyframe.interpolation = interpolation;
                    changed = true;
                }
            }
            changed
        }

        let mut changed = false;
        for (name, parameter) in self.animatable_params() {
            if parameter_name.is_some_and(|wanted| wanted != name) {
                continue;
            }
            changed |= match parameter {
                ParamRef::Scalar(track) => apply(track, interpolation),
                ParamRef::Vec2(track) => apply(track, interpolation),
                ParamRef::Vec3(track) => apply(track, interpolation),
                ParamRef::Vec4Color(track) => apply(track, interpolation),
            };
        }
        changed
    }

    pub fn set_scalar_parameter_keyframe(
        &mut self,
        parameter_name: &str,
        frame: u32,
        value: f32,
    ) -> bool {
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            if let ParamRef::Scalar(track) = parameter {
                let before = track.evaluate(frame);
                let had_key = track
                    .keyframes()
                    .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                if let Some(key) = track
                    .keyframes_mut()
                    .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                {
                    key.value = value;
                } else {
                    track.add_keyframe(crate::core::keyframe::Keyframe::new(
                        frame,
                        value,
                        crate::core::keyframe::InterpolationType::Linear,
                    ));
                }
                return !had_key || before != value;
            }
        }
        false
    }

    pub fn remove_scalar_parameter_keyframe(&mut self, parameter_name: &str, frame: u32) -> bool {
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            if let ParamRef::Scalar(track) = parameter {
                let fallback = track.evaluate(frame);
                if track.keyframes().is_some() {
                    let (removed, became_constant) = {
                        let keyframes = track.keyframes_mut().expect("checked above");
                        let before = keyframes.len();
                        keyframes.retain(|keyframe| keyframe.frame != frame);
                        (
                            keyframes.len() != before,
                            keyframes.is_empty() && keyframes.len() != before,
                        )
                    };
                    if became_constant {
                        *track = Animatable::Constant(fallback);
                    }
                    return removed;
                }
            }
        }
        false
    }

    pub fn set_parameter_component_keyframe(
        &mut self,
        parameter_name: &str,
        component: usize,
        frame: u32,
        value: f32,
    ) -> bool {
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            match parameter {
                ParamRef::Vec2(track) => {
                    if component >= 2 {
                        return false;
                    }
                    let mut next = track.evaluate(frame);
                    let changed = next[component] != value;
                    let had_key = track
                        .keyframes()
                        .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                    next[component] = value;
                    let interpolation = track
                        .keyframes()
                        .and_then(|keys| keys.iter().find(|key| key.frame == frame))
                        .map(|key| key.interpolation)
                        .unwrap_or_default();
                    if let Some(key) = track
                        .keyframes_mut()
                        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                    {
                        key.value = next;
                    } else {
                        track.add_keyframe(crate::core::keyframe::Keyframe::new(
                            frame,
                            next,
                            interpolation,
                        ));
                    }
                    return !had_key || changed;
                }
                ParamRef::Vec3(track) => {
                    if component >= 3 {
                        return false;
                    }
                    let mut next = track.evaluate(frame);
                    let changed = next[component] != value;
                    let had_key = track
                        .keyframes()
                        .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                    next[component] = value;
                    let interpolation = track
                        .keyframes()
                        .and_then(|keys| keys.iter().find(|key| key.frame == frame))
                        .map(|key| key.interpolation)
                        .unwrap_or_default();
                    if let Some(key) = track
                        .keyframes_mut()
                        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                    {
                        key.value = next;
                    } else {
                        track.add_keyframe(crate::core::keyframe::Keyframe::new(
                            frame,
                            next,
                            interpolation,
                        ));
                    }
                    return !had_key || changed;
                }
                ParamRef::Vec4Color(track) => {
                    if component >= 4 {
                        return false;
                    }
                    let mut next = track.evaluate(frame);
                    let changed = next[component] != value;
                    let had_key = track
                        .keyframes()
                        .is_some_and(|keys| keys.iter().any(|key| key.frame == frame));
                    next[component] = value;
                    let interpolation = track
                        .keyframes()
                        .and_then(|keys| keys.iter().find(|key| key.frame == frame))
                        .map(|key| key.interpolation)
                        .unwrap_or_default();
                    if let Some(key) = track
                        .keyframes_mut()
                        .and_then(|keys| keys.iter_mut().find(|key| key.frame == frame))
                    {
                        key.value = next;
                    } else {
                        track.add_keyframe(crate::core::keyframe::Keyframe::new(
                            frame,
                            next,
                            interpolation,
                        ));
                    }
                    return !had_key || changed;
                }
                ParamRef::Scalar(_) => return false,
            }
        }
        false
    }

    pub fn move_scalar_parameter_keyframe(
        &mut self,
        parameter_name: &str,
        from_frame: u32,
        to_frame: u32,
    ) -> bool {
        if from_frame == to_frame {
            return false;
        }
        let Some((value, interpolation)) =
            self.animatable_params_ref()
                .into_iter()
                .find_map(|(name, parameter)| {
                    if name != parameter_name {
                        return None;
                    }
                    match parameter {
                        ParamRefRef::Scalar(track) => track
                            .keyframes()?
                            .iter()
                            .find(|k| k.frame == from_frame)
                            .map(|k| (k.value, k.interpolation)),
                        _ => None,
                    }
                })
        else {
            return false;
        };
        if !self.remove_scalar_parameter_keyframe(parameter_name, from_frame) {
            return false;
        }
        if !self.set_scalar_parameter_keyframe(parameter_name, to_frame, value) {
            return false;
        }
        self.set_parameter_keyframe_interpolation_at_frame(parameter_name, to_frame, interpolation);
        true
    }

    pub fn move_parameter_component_keyframe(
        &mut self,
        parameter_name: &str,
        component: usize,
        from_frame: u32,
        to_frame: u32,
    ) -> bool {
        if from_frame == to_frame {
            return false;
        }
        let Some((value, interpolation)) =
            self.animatable_params_ref()
                .into_iter()
                .find_map(|(name, parameter)| {
                    if name != parameter_name {
                        return None;
                    }
                    match parameter {
                        ParamRefRef::Vec2(track) if component < 2 => track
                            .keyframes()?
                            .iter()
                            .find(|k| k.frame == from_frame)
                            .map(|k| (k.value[component], k.interpolation)),
                        ParamRefRef::Vec3(track) if component < 3 => track
                            .keyframes()?
                            .iter()
                            .find(|k| k.frame == from_frame)
                            .map(|k| (k.value[component], k.interpolation)),
                        ParamRefRef::Vec4Color(track) if component < 4 => track
                            .keyframes()?
                            .iter()
                            .find(|k| k.frame == from_frame)
                            .map(|k| (k.value[component], k.interpolation)),
                        _ => None,
                    }
                })
        else {
            return false;
        };
        if !self.remove_parameter_component_keyframe(parameter_name, from_frame) {
            return false;
        }
        if !self.set_parameter_component_keyframe(parameter_name, component, to_frame, value) {
            return false;
        }
        self.set_parameter_keyframe_interpolation_at_frame(parameter_name, to_frame, interpolation);
        true
    }

    pub fn set_parameter_keyframe_interpolation_at_frame(
        &mut self,
        parameter_name: &str,
        frame: u32,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        fn apply<T: Clone>(
            track: &mut Animatable<T>,
            frame: u32,
            interpolation: crate::core::keyframe::InterpolationType,
        ) -> bool {
            let Some(keys) = track.keyframes_mut() else {
                return false;
            };
            let Some(key) = keys.iter_mut().find(|key| key.frame == frame) else {
                return false;
            };
            let changed = key.interpolation != interpolation;
            key.interpolation = interpolation;
            changed
        }
        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            return match parameter {
                ParamRef::Scalar(track) => apply(track, frame, interpolation),
                ParamRef::Vec2(track) => apply(track, frame, interpolation),
                ParamRef::Vec3(track) => apply(track, frame, interpolation),
                ParamRef::Vec4Color(track) => apply(track, frame, interpolation),
            };
        }
        false
    }

    pub fn set_parameter_keyframe_bezier_at_frame(
        &mut self,
        parameter_name: &str,
        frame: u32,
        points: [f32; 4],
    ) -> bool {
        let interpolation = crate::core::keyframe::InterpolationType::Bezier {
            outgoing: Default::default(),
            incoming: Default::default(),
            custom_bezier: Some(points),
        };
        self.set_parameter_keyframe_interpolation_at_frame(parameter_name, frame, interpolation)
    }

    pub fn remove_parameter_component_keyframe(
        &mut self,
        parameter_name: &str,
        frame: u32,
    ) -> bool {
        fn remove_key_and_restore_constant<T: Clone + crate::core::property::Interpolate>(
            track: &mut Animatable<T>,
            frame: u32,
        ) -> bool {
            let fallback = track.evaluate(frame);
            let (removed, became_empty) = {
                let Some(keys) = track.keyframes_mut() else {
                    return false;
                };
                let before = keys.len();
                keys.retain(|key| key.frame != frame);
                let removed = keys.len() != before;
                (removed, keys.is_empty() && removed)
            };
            if became_empty {
                *track = Animatable::Constant(fallback);
            }
            removed
        }

        for (name, parameter) in self.animatable_params() {
            if name != parameter_name {
                continue;
            }
            let removed = match parameter {
                ParamRef::Vec2(track) => remove_key_and_restore_constant(track, frame),
                ParamRef::Vec3(track) => remove_key_and_restore_constant(track, frame),
                ParamRef::Vec4Color(track) => remove_key_and_restore_constant(track, frame),
                ParamRef::Scalar(_) => false,
            };
            return removed;
        }
        false
    }

    pub fn animatable_params_ref(&self) -> Vec<(&'static str, ParamRefRef<'_>)> {
        let mut out: Vec<(&'static str, ParamRefRef<'_>)> = Vec::new();
        macro_rules! push {
            ($label:expr, $field:expr, Scalar) => {
                out.push(($label, ParamRefRef::Scalar($field)))
            };
            ($label:expr, $field:expr, Vec2) => {
                out.push(($label, ParamRefRef::Vec2($field)))
            };
            ($label:expr, $field:expr, Vec3) => {
                out.push(($label, ParamRefRef::Vec3($field)))
            };
            ($label:expr, $field:expr, Color) => {
                out.push(($label, ParamRefRef::Vec4Color($field)))
            };
        }
        match self {
            EffectType::GaussianBlur { blur_radius } => push!("Blur Radius", blur_radius, Scalar),
            EffectType::ColorTint { color, intensity } => {
                push!("Tint Color", color, Color);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::DropShadow {
                color,
                opacity,
                direction,
                distance,
                softness,
            } => {
                push!("Shadow Color", color, Color);
                push!("Opacity", opacity, Scalar);
                push!("Direction", direction, Scalar);
                push!("Distance", distance, Scalar);
                push!("Softness", softness, Scalar);
            }
            EffectType::ChromaticAberration {
                shift_r,
                shift_b,
                edge_falloff,
                iris_linked: _,
            } => {
                push!("Red Shift", shift_r, Scalar);
                push!("Blue Shift", shift_b, Scalar);
                push!("Edge Falloff", edge_falloff, Scalar);
            }
            EffectType::Vignette {
                intensity,
                roundness,
                feather,
                color,
            } => {
                push!("Vignette Color", color, Color);
                push!("Intensity", intensity, Scalar);
                push!("Roundness", roundness, Scalar);
                push!("Feather", feather, Scalar);
            }
            EffectType::Levels {
                input_black,
                input_white,
                gamma,
                output_black,
                output_white,
            } => {
                push!("Input Black", input_black, Scalar);
                push!("Input White", input_white, Scalar);
                push!("Gamma", gamma, Scalar);
                push!("Output Black", output_black, Scalar);
                push!("Output White", output_white, Scalar);
            }
            EffectType::HueSaturation {
                hue_shift,
                saturation,
                lightness,
            } => {
                push!("Hue Shift", hue_shift, Scalar);
                push!("Saturation", saturation, Scalar);
                push!("Lightness", lightness, Scalar);
            }
            EffectType::Glow {
                threshold,
                radius,
                intensity,
                color,
            } => {
                push!("Glow Color", color, Color);
                push!("Threshold", threshold, Scalar);
                push!("Radius", radius, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::LensFlare {
                enabled,
                position_x,
                position_y,
                intensity,
                threshold,
                color,
                ..
            } => {
                push!("Flare Enabled", enabled, Scalar);
                push!("Position X", position_x, Scalar);
                push!("Position Y", position_y, Scalar);
                push!("Intensity", intensity, Scalar);
                push!("Threshold", threshold, Scalar);
                push!("Flare Color", color, Color);
            }
            EffectType::MotionBlur { shutter_angle, .. } => {
                push!("Shutter Angle", shutter_angle, Scalar)
            }
            EffectType::MeshWarp {
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => {
                push!("Top Left", top_left, Vec2);
                push!("Top Right", top_right, Vec2);
                push!("Bottom Left", bottom_left, Vec2);
                push!("Bottom Right", bottom_right, Vec2);
            }
            EffectType::CornerPin {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => {
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
            EffectType::SimpleChoker { choke_amount } => {
                push!("Choke Amount", choke_amount, Scalar)
            }
            EffectType::ChromaKey {
                screen_gain,
                clip_black,
                clip_white,
                ..
            } => {
                push!("Screen Gain", screen_gain, Scalar);
                push!("Clip Black", clip_black, Scalar);
                push!("Clip White", clip_white, Scalar);
            }
            EffectType::Spherize {
                radius,
                refractive_index,
            } => {
                push!("Radius", radius, Scalar);
                push!("Refractive Index", refractive_index, Scalar);
            }
            EffectType::TurbulentDisplace {
                amount,
                size,
                evolution,
                complexity,
            } => {
                push!("Amount", amount, Scalar);
                push!("Size", size, Scalar);
                push!("Evolution", evolution, Scalar);
                push!("Complexity", complexity, Scalar);
            }
            EffectType::DisplacementMap {
                source_layer,
                max_horizontal,
                max_vertical,
            } => {
                push!("Source Layer Idx", source_layer, Scalar);
                push!("Max Horizontal", max_horizontal, Scalar);
                push!("Max Vertical", max_vertical, Scalar);
            }
            EffectType::CompoundBlur {
                source_layer,
                max_blur,
            } => {
                push!("Source Layer Idx", source_layer, Scalar);
                push!("Max Blur", max_blur, Scalar);
            }
            EffectType::Minimax { operation, radius } => {
                push!("Operation", operation, Scalar);
                push!("Radius", radius, Scalar);
            }
            EffectType::ShiftChannels {
                take_red,
                take_green,
                take_blue,
                take_alpha,
            } => {
                push!("Take Red", take_red, Scalar);
                push!("Take Green", take_green, Scalar);
                push!("Take Blue", take_blue, Scalar);
                push!("Take Alpha", take_alpha, Scalar);
            }
            EffectType::WaveWarp {
                wave_height,
                wave_width,
                speed,
                direction_deg,
                ..
            } => {
                push!("Wave Height", wave_height, Scalar);
                push!("Wave Width", wave_width, Scalar);
                push!("Speed", speed, Scalar);
                push!("Direction °", direction_deg, Scalar);
            }
            EffectType::CcLens { convergence, zoom } => {
                push!("Convergence", convergence, Scalar);
                push!("Zoom", zoom, Scalar);
            }
            EffectType::PolarCoordinates { interpolation, .. } => {
                push!("Interpolation", interpolation, Scalar)
            }
            EffectType::OpticsCompensation {
                field_of_view_deg,
                zoom,
                ..
            } => {
                push!("Field of View", field_of_view_deg, Scalar);
                push!("Zoom", zoom, Scalar);
            }
            EffectType::ColorBalance { .. } => {}
            EffectType::ChannelMixer { .. } => {}
            EffectType::LightSweep {
                direction_deg,
                center,
                width,
                sweep_intensity,
                edge_intensity,
            } => {
                push!("Direction °", direction_deg, Scalar);
                push!("Center", center, Scalar);
                push!("Width", width, Scalar);
                push!("Sweep Intensity", sweep_intensity, Scalar);
                push!("Edge Intensity", edge_intensity, Scalar);
            }
            EffectType::BendIt {
                top_offset,
                bottom_offset,
            } => {
                push!("Top Offset", top_offset, Scalar);
                push!("Bottom Offset", bottom_offset, Scalar);
            }
            EffectType::Tiler { scale_percent, .. } => push!("Scale %", scale_percent, Scalar),
            EffectType::Tritone { .. } => {}
            EffectType::MatteChoker {
                choke_amount,
                gray_level,
            } => {
                push!("Choke Amount", choke_amount, Scalar);
                push!("Gray Level", gray_level, Scalar);
            }
            EffectType::VenetianBlinds { completion, width } => {
                push!("Completion", completion, Scalar);
                push!("Stripe Width", width, Scalar);
            }
            EffectType::Vibrance { amount } => push!("Amount", amount, Scalar),
            EffectType::WhiteBalance { temperature, tint } => {
                push!("Temperature", temperature, Scalar);
                push!("Tint", tint, Scalar);
            }
            EffectType::HslAdjust {
                hue_deg,
                saturation,
                lightness,
            } => {
                push!("Hue °", hue_deg, Scalar);
                push!("Saturation", saturation, Scalar);
                push!("Lightness", lightness, Scalar);
            }
            EffectType::GlowPro {
                threshold,
                radius,
                intensity,
                ..
            } => {
                push!("Threshold", threshold, Scalar);
                push!("Radius", radius, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::CrtScanlines {
                line_spacing,
                intensity,
                ..
            } => {
                push!("Line Spacing", line_spacing, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::Vortex {
                radius, angle_deg, ..
            } => {
                push!("Radius", radius, Scalar);
                push!("Angle", angle_deg, Scalar);
            }
            EffectType::HeatDistortion {
                strength, speed, ..
            } => {
                push!("Strength", strength, Scalar);
                push!("Speed", speed, Scalar);
            }
            EffectType::RainRipples {
                drop_count,
                wave_strength,
                ..
            } => {
                push!("Drop Count", drop_count, Scalar);
                push!("Wave Strength", wave_strength, Scalar);
            }
            EffectType::Fisheye { strength, .. } => push!("Strength", strength, Scalar),
            EffectType::LensCorrection { k1, k2, .. } => {
                push!("K1", k1, Scalar);
                push!("K2", k2, Scalar);
            }
            EffectType::GlitchDisplacement { seed, amount, .. } => {
                push!("Seed", seed, Scalar);
                push!("Amount", amount, Scalar);
            }
            EffectType::MatteChokeSpread { radius, .. } => push!("Radius", radius, Scalar),
            EffectType::AlphaFeather { radius, .. } => push!("Radius", radius, Scalar),
            EffectType::AlphaFromLuminance { .. } => {}
            EffectType::NightVision { amplification, .. } => {
                push!("Amplification", amplification, Scalar)
            }
            EffectType::IrisWipe { completion, .. } => push!("Completion", completion, Scalar),
            EffectType::RadialWipe { completion, .. } => push!("Completion", completion, Scalar),
            EffectType::FilmEmulation {
                lift,
                gamma,
                gain,
                hue_shift_deg,
                ..
            } => {
                push!("Lift", lift, Scalar);
                push!("Gamma", gamma, Scalar);
                push!("Gain", gain, Scalar);
                push!("Hue Shift", hue_shift_deg, Scalar);
            }
            EffectType::GodRays {
                sun_x,
                sun_y,
                samples,
                decay,
                weight,
                ..
            } => {
                push!("Sun X", sun_x, Scalar);
                push!("Sun Y", sun_y, Scalar);
                push!("Samples", samples, Scalar);
                push!("Decay", decay, Scalar);
                push!("Weight", weight, Scalar);
            }
            EffectType::RadialBlurZoom { amount } => push!("Zoom Amount", amount, Scalar),
            EffectType::BevelAlpha {
                depth,
                light_angle_deg,
            } => {
                push!("Depth", depth, Scalar);
                push!("Light Angle", light_angle_deg, Scalar);
            }
            EffectType::CrossHatch {
                line_gap,
                threshold,
            } => {
                push!("Line Gap", line_gap, Scalar);
                push!("Threshold", threshold, Scalar);
            }
            EffectType::CmykHalftone { dot_size } => push!("Dot Size", dot_size, Scalar),
            EffectType::ColorGradeLUT { intensity, .. } => {
                push!("LUT Intensity", intensity, Scalar)
            }
            EffectType::Colorama {
                preset_index,
                cycle_phase,
            } => {
                push!("Preset", preset_index, Scalar);
                push!("Cycle Phase", cycle_phase, Scalar);
            }
            EffectType::ColorSpaceConvert { .. } => {}
            EffectType::Curves { channel } => push!("Channel", channel, Scalar),
            EffectType::DirectionalSharpen {
                angle_deg,
                strength,
            } => {
                push!("Angle", angle_deg, Scalar);
                push!("Strength", strength, Scalar);
            }
            EffectType::Emboss { angle_deg, depth } => {
                push!("Angle", angle_deg, Scalar);
                push!("Depth", depth, Scalar);
            }
            EffectType::FbmTurbulence { octaves, amplitude } => {
                push!("Octaves", octaves, Scalar);
                push!("Amplitude", amplitude, Scalar);
            }
            EffectType::FireAutomaton { intensity } => push!("Intensity", intensity, Scalar),
            EffectType::FractalNoise {
                fractal_type,
                contrast,
                brightness,
                complexity,
                evolution,
            } => {
                push!("Fractal Type", fractal_type, Scalar);
                push!("Contrast", contrast, Scalar);
                push!("Brightness", brightness, Scalar);
                push!("Complexity", complexity, Scalar);
                push!("Evolution", evolution, Scalar);
            }
            EffectType::GradientMap { .. } => {}
            EffectType::SliderControl { value } => push!("Value", value, Scalar),
            EffectType::AngleControl { angle_degrees } => push!("Angle", angle_degrees, Scalar),
            EffectType::PointControl { point } => push!("Point", point, Vec2),
            EffectType::ColorControl { color } => push!("Color", color, Color),
            EffectType::CheckboxControl { .. } => {}
            EffectType::DropdownControl { .. } => {}
            EffectType::Point3DControl { point } => push!("Point 3D", point, Vec3),
            EffectType::Letterbox { frac } => push!("Bars", frac, Scalar),
            EffectType::Halftone { cell_size } => push!("Cell Size", cell_size, Scalar),
            EffectType::Invert { .. } => {}
            EffectType::LightLeak {
                pos_x,
                pos_y,
                intensity,
            } => {
                push!("Position X", pos_x, Scalar);
                push!("Position Y", pos_y, Scalar);
                push!("Intensity", intensity, Scalar);
            }
            EffectType::LightningArc {
                start_x,
                start_y,
                end_x,
                end_y,
                seed,
                glow,
            } => {
                push!("Start X", start_x, Scalar);
                push!("Start Y", start_y, Scalar);
                push!("End X", end_x, Scalar);
                push!("End Y", end_y, Scalar);
                push!("Seed", seed, Scalar);
                push!("Glow", glow, Scalar);
            }
            EffectType::LaserBeam {
                start_x,
                start_y,
                end_x,
                end_y,
                progress,
                length,
                starting_thickness,
                ending_thickness,
                core_color,
                glow_color,
            } => {
                push!("Start X", start_x, Scalar);
                push!("Start Y", start_y, Scalar);
                push!("End X", end_x, Scalar);
                push!("End Y", end_y, Scalar);
                push!("Progress", progress, Scalar);
                push!("Length %", length, Scalar);
                push!("Start Thickness", starting_thickness, Scalar);
                push!("End Thickness", ending_thickness, Scalar);
                push!("Core Color", core_color, Color);
                push!("Glow Color", glow_color, Color);
            }
            EffectType::LumaKeyRange {
                low_threshold,
                high_threshold,
                ..
            } => {
                push!("Low Threshold", low_threshold, Scalar);
                push!("High Threshold", high_threshold, Scalar);
            }
            EffectType::MedianFilter { radius } => push!("Radius", radius, Scalar),
            EffectType::Mosaic { block_w, block_h } => {
                push!("Block Width", block_w, Scalar);
                push!("Block Height", block_h, Scalar);
            }
            EffectType::PerlinFlow { scale } => push!("Scale", scale, Scalar),
            EffectType::PinchPunch { radius, amount } => {
                push!("Radius", radius, Scalar);
                push!("Amount", amount, Scalar);
            }
            EffectType::PixelSort { threshold } => push!("Threshold", threshold, Scalar),
            EffectType::ReflectionMap {
                reflect_y,
                fade_dist,
                opacity,
            } => {
                push!("Reflect Y", reflect_y, Scalar);
                push!("Fade Distance", fade_dist, Scalar);
                push!("Opacity", opacity, Scalar);
            }
            EffectType::RefractionLens { radius, ior } => {
                push!("Radius", radius, Scalar);
                push!("IOR", ior, Scalar);
            }
            EffectType::ScanlineGlitch {
                jitter_amount,
                seed,
            } => {
                push!("Jitter", jitter_amount, Scalar);
                push!("Seed", seed, Scalar);
            }
            EffectType::SobelEdges { .. } => {}
            EffectType::Solarize { threshold } => push!("Threshold", threshold, Scalar),
            EffectType::TiltShift {
                focus_y,
                focus_height,
                max_blur,
            } => {
                push!("Focus Y", focus_y, Scalar);
                push!("Focus Height", focus_height, Scalar);
                push!("Max Blur", max_blur, Scalar);
            }
            EffectType::GlassEdgeBevel {
                bevel_size,
                refraction,
            } => {
                push!("Bevel Size", bevel_size, Scalar);
                push!("Refraction", refraction, Scalar);
            }
            EffectType::StarField {
                num_stars,
                depth_speed,
            } => {
                push!("Star Count", num_stars, Scalar);
                push!("Depth Speed", depth_speed, Scalar);
            }
            // Exhaustive today (all 97 variants registered); keep the
            // catch-all so future variants still compile without rows.
            EffectType::MergePaths { operation } => {
                push!("Operation", operation, Scalar);
            }
            EffectType::OffsetPath {
                amount,
                line_join,
                miter_limit,
            } => {
                push!("Amount", amount, Scalar);
                push!("Line Join", line_join, Scalar);
                push!("Miter Limit", miter_limit, Scalar);
            }
            EffectType::CustomShader { .. } => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
        out
    }
}

#[cfg(test)]
mod registration_tests {
    use super::*;
    use crate::core::effect_plugin::RenderEffectPlugin;
    use crate::core::property::Animatable;

    fn c() -> Animatable<f32> {
        Animatable::new_constant(0.0)
    }

    #[test]
    fn formerly_unregistered_variants_expose_keyframe_rows() {
        let mut cases: Vec<EffectType> = vec![
            EffectType::StarField {
                num_stars: c(),
                depth_speed: c(),
            },
            EffectType::WaveWarp {
                wave_height: c(),
                wave_width: c(),
                speed: c(),
                direction_deg: c(),
                wave_type: 0,
                pinning: 0,
            },
            EffectType::Minimax {
                operation: c(),
                radius: c(),
            },
            EffectType::ShiftChannels {
                take_red: c(),
                take_green: c(),
                take_blue: c(),
                take_alpha: c(),
            },
            EffectType::LightSweep {
                direction_deg: c(),
                center: c(),
                width: c(),
                sweep_intensity: c(),
                edge_intensity: c(),
            },
            EffectType::WhiteBalance {
                temperature: c(),
                tint: c(),
            },
            EffectType::HslAdjust {
                hue_deg: c(),
                saturation: c(),
                lightness: c(),
            },
            EffectType::CompoundBlur {
                source_layer: c(),
                max_blur: c(),
            },
        ];
        for effect in &mut cases {
            let rows = effect.animatable_params().len();
            assert!(rows > 0, "{} exposes no timeline rows", effect_name(effect));
        }
    }

    fn effect_name(e: &EffectType) -> String {
        crate::core::effect_plugin::EnumEffectPlugin {
            effect_type: e.clone(),
        }
        .name()
        .to_string()
    }

    #[test]
    fn animatable_params_ref_matches_mutable_counts() {
        let mut mutable = EffectType::LightSweep {
            direction_deg: c(),
            center: c(),
            width: c(),
            sweep_intensity: c(),
            edge_intensity: c(),
        };
        let read_only = EffectType::LightSweep {
            direction_deg: c(),
            center: c(),
            width: c(),
            sweep_intensity: c(),
            edge_intensity: c(),
        };
        assert_eq!(
            mutable.animatable_params().len(),
            read_only.animatable_params_ref().len(),
        );
    }

    #[test]
    fn parameter_interpolation_changes_only_selected_effect_track() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_constant([1.0, 0.0, 0.0, 1.0]),
            intensity: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                0,
                0.5,
                crate::core::keyframe::InterpolationType::Linear,
            )]),
        };

        assert!(effect.set_parameter_keyframe_interpolation(
            Some("Intensity"),
            crate::core::keyframe::InterpolationType::Hold,
        ));
        let params = effect.animatable_params_ref();
        let intensity = params
            .into_iter()
            .find_map(|(name, parameter)| (name == "Intensity").then_some(parameter));
        match intensity.expect("intensity parameter should be registered") {
            ParamRefRef::Scalar(track) => assert!(matches!(
                track.keyframes().unwrap()[0].interpolation,
                crate::core::keyframe::InterpolationType::Hold
            )),
            _ => panic!("intensity must be scalar"),
        }
    }

    #[test]
    fn removing_last_scalar_key_restores_constant_value() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                12,
                7.5,
                crate::core::keyframe::InterpolationType::Linear,
            )]),
        };

        assert!(effect.remove_scalar_parameter_keyframe("Blur Radius", 12));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => {
                assert!(matches!(track, Animatable::Constant(value) if *value == 7.5));
            }
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn component_keyframe_api_supports_color_and_rejects_invalid_channel() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            intensity: c(),
        };

        assert!(effect.set_parameter_component_keyframe("Tint Color", 1, 24, 0.75));
        assert!(!effect.set_parameter_component_keyframe("Tint Color", 4, 24, 0.5));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert_eq!(track.evaluate(24)[1], 0.75),
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn component_keyframe_api_preserves_unedited_vector_channels() {
        let mut effect = EffectType::DropShadow {
            color: Animatable::new_constant([0.1, 0.2, 0.3, 1.0]),
            opacity: c(),
            direction: c(),
            distance: Animatable::new_constant(10.0),
            softness: c(),
        };

        assert!(effect.set_parameter_component_keyframe("Shadow Color", 2, 30, 0.9));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert_eq!(track.evaluate(30), [0.1, 0.2, 0.9, 1.0]),
            _ => panic!("shadow color must be a color track"),
        }
        assert!(effect.set_parameter_keyframe_interpolation(
            Some("Shadow Color"),
            crate::core::keyframe::InterpolationType::Hold,
        ));
        assert!(effect.set_parameter_component_keyframe("Shadow Color", 0, 30, 0.8));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                assert_eq!(track.evaluate(30), [0.8, 0.2, 0.9, 1.0]);
                assert_eq!(
                    track
                        .keyframes()
                        .unwrap()
                        .iter()
                        .filter(|key| key.frame == 30)
                        .count(),
                    1
                );
            }
            _ => panic!("shadow color must be a color track"),
        }
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert!(matches!(
                track.keyframes().unwrap()[0].interpolation,
                crate::core::keyframe::InterpolationType::Hold
            )),
            _ => panic!("shadow color must be a color track"),
        }
    }

    #[test]
    fn component_interpolation_can_be_restored_at_one_frame() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                8,
                [0.2, 0.3, 0.4, 1.0],
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: Default::default(),
                    incoming: Default::default(),
                    custom_bezier: Some([0.25, 0.1, 0.25, 1.0]),
                },
            )]),
            intensity: c(),
        };

        assert!(effect.set_parameter_keyframe_interpolation_at_frame(
            "Tint Color",
            8,
            crate::core::keyframe::InterpolationType::Hold,
        ));
        assert!(!effect.set_parameter_keyframe_interpolation_at_frame(
            "Tint Color",
            99,
            crate::core::keyframe::InterpolationType::Linear,
        ));
    }

    #[test]
    fn scalar_keyframe_addition_is_idempotent_for_same_value() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(4.0),
        };

        assert!(effect.set_scalar_parameter_keyframe("Blur Radius", 5, 4.0));
        assert!(!effect.set_scalar_parameter_keyframe("Blur Radius", 5, 4.0));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => assert_eq!(
                track
                    .keyframes()
                    .unwrap()
                    .iter()
                    .filter(|key| key.frame == 5)
                    .count(),
                1
            ),
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn component_keyframe_move_preserves_interpolation_and_other_channels() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                10,
                [0.1, 0.2, 0.3, 1.0],
                crate::core::keyframe::InterpolationType::Bezier {
                    outgoing: Default::default(),
                    incoming: Default::default(),
                    custom_bezier: Some([0.2, 0.1, 0.8, 0.9]),
                },
            )]),
            intensity: c(),
        };
        assert!(effect.move_parameter_component_keyframe("Tint Color", 2, 10, 20));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                assert_eq!(track.evaluate(20), [0.1, 0.2, 0.3, 1.0]);
                assert!(track.keyframes().unwrap().iter().all(|key| key.frame != 10));
                assert!(matches!(
                    track
                        .keyframes()
                        .unwrap()
                        .iter()
                        .find(|key| key.frame == 20)
                        .unwrap()
                        .interpolation,
                    crate::core::keyframe::InterpolationType::Bezier { .. }
                ));
            }
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn component_keyframe_move_rejects_missing_or_same_frame_without_mutation() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_constant([0.1, 0.2, 0.3, 1.0]),
            intensity: c(),
        };
        assert!(!effect.move_parameter_component_keyframe("Tint Color", 0, 4, 4));
        assert!(!effect.move_parameter_component_keyframe("Tint Color", 0, 4, 8));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => assert!(track.keyframes().is_none()),
            _ => panic!("tint color must be a color track"),
        }
    }

    #[test]
    fn scalar_keyframe_move_preserves_value_and_interpolation() {
        let mut effect = EffectType::GaussianBlur {
            blur_radius: Animatable::new_animated(vec![crate::core::keyframe::Keyframe::new(
                12,
                18.0,
                crate::core::keyframe::InterpolationType::Hold,
            )]),
        };
        assert!(effect.move_scalar_parameter_keyframe("Blur Radius", 12, 24));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Scalar(track) => {
                assert_eq!(track.evaluate(24), 18.0);
                let key = track
                    .keyframes()
                    .unwrap()
                    .iter()
                    .find(|key| key.frame == 24)
                    .unwrap();
                assert!(matches!(
                    key.interpolation,
                    crate::core::keyframe::InterpolationType::Hold
                ));
            }
            _ => panic!("blur radius must be scalar"),
        }
    }

    #[test]
    fn component_keyframe_move_merges_into_existing_destination_key() {
        let mut effect = EffectType::ColorTint {
            color: Animatable::new_animated(vec![
                crate::core::keyframe::Keyframe::new(
                    5,
                    [0.9, 0.2, 0.3, 1.0],
                    crate::core::keyframe::InterpolationType::Linear,
                ),
                crate::core::keyframe::Keyframe::new(
                    15,
                    [0.1, 0.8, 0.7, 1.0],
                    crate::core::keyframe::InterpolationType::Hold,
                ),
            ]),
            intensity: c(),
        };
        assert!(effect.move_parameter_component_keyframe("Tint Color", 0, 5, 15));
        match effect.animatable_params_ref()[0].1 {
            ParamRefRef::Vec4Color(track) => {
                assert_eq!(track.evaluate(15), [0.9, 0.8, 0.7, 1.0]);
                assert_eq!(
                    track
                        .keyframes()
                        .unwrap()
                        .iter()
                        .filter(|key| key.frame == 15)
                        .count(),
                    1
                );
            }
            _ => panic!("tint color must be a color track"),
        }
    }
}
