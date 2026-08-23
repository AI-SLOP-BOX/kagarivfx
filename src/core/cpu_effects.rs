//! CPU pixel-effect pipeline.
//!
//! This module is the bridge that finally wires the large library of orphaned
//! `ae_effects_pack*` image kernels into the render pipeline. The GPU path
//! (`effect_plugin` -> `EffectParams` -> WGSL) handles the live viewport; this
//! CPU path applies the same logical effects to a layer's rasterized RGBA
//! buffer so effects are also visible in the software renderer used for frame
//! export and CPU preview, and so the effect kernels are actually used.
//!
//! Each `EffectType` maps to one or more `ae_effects_pack::apply_*` kernels.

use crate::core::timeline::{Effect, EffectType};

/// Convert a color (0..1 floats) into an `[u8; 4]` (0..255).
fn color_to_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// Convert a color (0..1 floats) into a 3-channel `[u8; 3]`.
fn color3_to_u8(c: [f32; 4]) -> [u8; 3] {
    let c = color_to_u8(c);
    [c[0], c[1], c[2]]
}

/// Apply every enabled effect on a layer to its already-rasterized RGBA buffer.
///
/// `pixels` is a straight (non-premultiplied) RGBA8 buffer of size
/// `width*height*4`. Effects are applied in order, mirroring After Effects'
/// top-to-bottom effect stack within a layer.
pub fn apply_layer_effects(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    effects: &[Effect],
    frame: u32,
) {
    for effect in effects {
        if !effect.enabled {
            continue;
        }
        apply_one(pixels, width, height, &effect.effect_type, frame);
    }
}

fn apply_one(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    effect_type: &EffectType,
    frame: u32,
) {
    use crate::core::ae_effects_pack as pack;

    match effect_type {
        // Effects already present in the GPU pipeline, mirrored on CPU.
        EffectType::GaussianBlur { blur_radius } => {
            let r = blur_radius.evaluate(frame).max(0.0) as u32;
            pack::apply_gaussian_blur(pixels, width, height, r);
        }
        EffectType::ColorTint { color, intensity } => {
            let rgb = color3_to_u8(color.evaluate(frame));
            let amount = (intensity.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            pack::apply_tint(pixels, rgb, rgb, amount);
        }
        EffectType::DropShadow { color, opacity, direction, distance, softness } => {
            let mut sc = color_to_u8(color.evaluate(frame));
            sc[3] = (sc[3] as f32 * (opacity.evaluate(frame) / 100.0)).clamp(0.0, 255.0) as u8;
            pack::apply_drop_shadow(
                pixels,
                width,
                height,
                distance.evaluate(frame),
                direction.evaluate(frame),
                softness.evaluate(frame).max(0.0) as u32,
                sc,
            );
        }
        EffectType::Glow { threshold, radius, intensity, .. } => {
            pack::apply_glow(
                pixels,
                width,
                height,
                threshold.evaluate(frame) / 100.0,
                radius.evaluate(frame).max(0.0) as u32,
                intensity.evaluate(frame) / 100.0,
            );
        }

        // New CPU-only kernels (no GPU equivalent yet).
        EffectType::Twirl { angle, radius } => {
            pack::apply_twirl(
                pixels,
                width,
                height,
                angle.evaluate(frame),
                radius.evaluate(frame).max(1.0),
            );
        }
        EffectType::Bulge { amount, radius } => {
            pack::apply_bulge(
                pixels,
                width,
                height,
                amount.evaluate(frame),
                radius.evaluate(frame).max(1.0),
            );
        }
        EffectType::Posterize { levels } => {
            pack::apply_posterize(pixels, levels.evaluate(frame).max(2.0) as u32);
        }
        EffectType::Invert { invert_alpha } => {
            pack::apply_invert(pixels, *invert_alpha);
        }
        EffectType::Offset { shift_x, shift_y } => {
            pack::apply_offset(
                pixels,
                width,
                height,
                shift_x.evaluate(frame) as i32,
                shift_y.evaluate(frame) as i32,
            );
        }
        EffectType::DirectionalBlur { angle, length } => {
            pack::apply_directional_blur(
                pixels,
                width,
                height,
                angle.evaluate(frame),
                length.evaluate(frame),
            );
        }
        EffectType::RadialBlur { amount } => {
            pack::apply_radial_blur(pixels, width, height, amount.evaluate(frame));
        }
        EffectType::Sharpen { amount } => {
            pack::apply_sharpen(pixels, width, height, amount.evaluate(frame));
        }
        EffectType::Threshold { threshold } => {
            pack::apply_threshold(pixels, threshold.evaluate(frame).clamp(0.0, 255.0) as u8);
        }
        EffectType::LinearWipe { completion, angle } => {
            pack::apply_linear_wipe(
                pixels,
                width,
                height,
                completion.evaluate(frame).clamp(0.0, 100.0),
                angle.evaluate(frame),
            );
        }
        EffectType::SimpleChoker { choke_amount } => {
            pack::apply_simple_choker(pixels, choke_amount.evaluate(frame));
        }

        // Effects from standalone core modules.
        EffectType::ChromaKey { screen_color, screen_gain, clip_black, clip_white } => {
            let sc = screen_color.evaluate(frame);
            let opts = crate::core::chroma_key::ChromaKeyOptions {
                screen_color: sc,
                screen_gain: screen_gain.evaluate(frame),
                screen_balance: 0.5,
                despill_strength: 0.8,
                clip_black: clip_black.evaluate(frame),
                clip_white: clip_white.evaluate(frame),
            };
            crate::core::chroma_key::apply_chroma_key(pixels, width, height, &opts);
        }
        EffectType::Spherize { radius, refractive_index } => {
            let opts = crate::core::spherize::SpherizeOptions {
                radius: radius.evaluate(frame),
                center: [width as f32 * 0.5, height as f32 * 0.5],
                refractive_index: refractive_index.evaluate(frame),
            };
            let out = crate::core::spherize::apply_spherize(pixels, width, height, &opts);
            pixels.copy_from_slice(&out);
        }
        EffectType::TurbulentDisplace { amount, size, evolution, complexity } => {
            let opts = crate::core::turbulent_displace::TurbulentDisplaceOptions {
                displace_type: crate::core::turbulent_displace::TurbulentDisplaceType::Turbulent,
                amount: amount.evaluate(frame),
                size: size.evaluate(frame),
                evolution_deg: evolution.evaluate(frame),
                complexity: complexity.evaluate(frame).max(1.0) as u32,
            };
            let out = crate::core::turbulent_displace::apply_turbulent_displace(pixels, width, height, &opts);
            pixels.copy_from_slice(&out);
        }
        EffectType::Colorama { preset_index, cycle_phase } => {
            let idx = preset_index.evaluate(frame).round() as u32 % 4;
            let preset = match idx {
                0 => crate::core::colorama::ColoramaPreset::Rainbow,
                1 => crate::core::colorama::ColoramaPreset::Heatmap,
                2 => crate::core::colorama::ColoramaPreset::Sepia,
                _ => crate::core::colorama::ColoramaPreset::Solarize,
            };
            crate::core::colorama::apply_colorama(pixels, width, height, preset, cycle_phase.evaluate(frame));
        }

        // Effects with CPU kernels: dispatch to cpu_effects_new
        EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
            crate::core::cpu_effects_new::apply_chromatic_aberration(
                pixels, width, height,
                shift_r.evaluate(frame), shift_b.evaluate(frame), edge_falloff.evaluate(frame),
            );
        }
        EffectType::Vignette { intensity, roundness, feather, color } => {
            crate::core::cpu_effects_new::apply_vignette(
                pixels, width, height,
                intensity.evaluate(frame), roundness.evaluate(frame),
                feather.evaluate(frame), color.evaluate(frame),
            );
        }
        EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
            crate::core::cpu_effects_new::apply_levels(
                pixels, width, height,
                input_black.evaluate(frame), input_white.evaluate(frame),
                gamma.evaluate(frame), output_black.evaluate(frame),
                output_white.evaluate(frame),
            );
        }
        EffectType::HueSaturation { hue_shift, saturation, lightness } => {
            crate::core::cpu_effects_new::apply_hue_saturation(
                pixels, width, height,
                hue_shift.evaluate(frame), saturation.evaluate(frame),
                lightness.evaluate(frame),
            );
        }
        EffectType::MotionBlur { shutter_angle, samples } => {
            crate::core::cpu_effects_new::apply_motion_blur(
                pixels, width, height,
                shutter_angle.evaluate(frame), *samples as f32,
            );
        }
        EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
            crate::core::cpu_effects_new::apply_mesh_warp(
                pixels, width, height,
                top_left.evaluate(frame), top_right.evaluate(frame),
                bottom_left.evaluate(frame), bottom_right.evaluate(frame),
            );
        }
        EffectType::ColorGradeLUT { lut_path, intensity } => {
            let _ = intensity.evaluate(frame);
            if !lut_path.is_empty() {
                log::debug!("ColorGradeLUT: LUT file support pending (path={})", lut_path);
            }
        }
        EffectType::ColorSpaceConvert { mode } => {
            crate::core::cpu_effects_new::apply_color_space_convert(
                pixels, width, height, *mode as u32,
            );
        }
        EffectType::FilmGrain { intensity, grain_size, color_film: _ } => {
            crate::core::cpu_effects_new::apply_film_grain(
                pixels, width, height,
                intensity.evaluate(frame), *grain_size as u32, frame,
            );
        }
        EffectType::FractalNoise { fractal_type, contrast, brightness, complexity, evolution } => {
            crate::core::cpu_effects_new::apply_fractal_noise(
                pixels, width, height,
                fractal_type.evaluate(frame), contrast.evaluate(frame),
                brightness.evaluate(frame), complexity.evaluate(frame),
                evolution.evaluate(frame),
            );
        }
        EffectType::Curves { channel } => {
            crate::core::cpu_effects_new::apply_curves(
                pixels, width, height, channel.evaluate(frame),
            );
        }
        EffectType::DisplacementMap { source_layer, max_horizontal, max_vertical } => {
            crate::core::cpu_effects_new::apply_displacement_map(
                pixels, width, height,
                source_layer.evaluate(frame), max_horizontal.evaluate(frame),
                max_vertical.evaluate(frame),
            );
        }
        EffectType::CompoundBlur { source_layer, max_blur } => {
            crate::core::cpu_effects_new::apply_compound_blur(
                pixels, width, height,
                source_layer.evaluate(frame), max_blur.evaluate(frame),
            );
        }
        EffectType::Minimax { operation, radius } => {
            crate::core::cpu_effects_new::apply_minimax(
                pixels, width, height,
                operation.evaluate(frame), radius.evaluate(frame),
            );
        }
        EffectType::ShiftChannels { take_red, take_green, take_blue, take_alpha } => {
            crate::core::cpu_effects_new::apply_shift_channels(
                pixels, width, height,
                take_red.evaluate(frame), take_green.evaluate(frame),
                take_blue.evaluate(frame), take_alpha.evaluate(frame),
            );
        }

        // ── Effects migrated from ExtEffect ──
        EffectType::WaveWarp { wave_height, wave_width, speed, direction_deg, wave_type, pinning } => {
            use crate::core::ae_effects_pack_v27::{apply_wave_warp_pro, WaveWarpParams, WaveType, PinKind};
            let params = WaveWarpParams {
                wave_height: wave_height.evaluate(frame),
                wave_width: wave_width.evaluate(frame),
                speed: speed.evaluate(frame),
                time: frame as f32 / 30.0,
                direction_deg: direction_deg.evaluate(frame),
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
        EffectType::CcLens { convergence, zoom } => {
            use crate::core::ae_effects_pack_v27::{apply_cc_lens_pro, CcLensParams};
            apply_cc_lens_pro(pixels, width, height, &CcLensParams {
                convergence: convergence.evaluate(frame),
                zoom: zoom.evaluate(frame),
            });
        }
        EffectType::PolarCoordinates { to_polar, interpolation } => {
            use crate::core::ae_effects_pack_v27::{apply_polar_coordinates_pro, PolarMode};
            let mode = if *to_polar { PolarMode::RectToPolar } else { PolarMode::PolarToRect };
            apply_polar_coordinates_pro(pixels, width, height, mode, interpolation.evaluate(frame));
        }
        EffectType::OpticsCompensation { field_of_view_deg, reverse, zoom } => {
            use crate::core::ae_effects_pack_v27::{apply_optics_compensation, OpticsCompensationParams};
            apply_optics_compensation(pixels, width, height, &OpticsCompensationParams {
                field_of_view_deg: field_of_view_deg.evaluate(frame),
                reverse: *reverse,
                zoom: zoom.evaluate(frame),
            });
        }
        EffectType::ColorBalance { shadows, midtones, highlights, preserve_luminosity } => {
            use crate::core::color_correction::{apply_color_balance, ColorBalance};
            apply_color_balance(pixels, &ColorBalance {
                shadows: *shadows,
                midtones: *midtones,
                highlights: *highlights,
                preserve_luminosity: *preserve_luminosity,
            });
        }
        EffectType::ChannelMixer { matrix, monochrome } => {
            use crate::core::color_correction::{apply_channel_mixer, ChannelMixer};
            apply_channel_mixer(pixels, &ChannelMixer {
                matrix: *matrix,
                monochrome: *monochrome,
            });
        }
        EffectType::LightSweep { direction_deg, center, width: sweep_width, sweep_intensity, edge_intensity } => {
            use crate::core::ae_effects_pack_v28::{apply_light_sweep, LightSweepParams};
            apply_light_sweep(pixels, width, height, &LightSweepParams {
                direction_deg: direction_deg.evaluate(frame),
                center: center.evaluate(frame),
                width: sweep_width.evaluate(frame),
                sweep_intensity: sweep_intensity.evaluate(frame),
                edge_intensity: edge_intensity.evaluate(frame),
            });
        }
        EffectType::RadialFastBlur { amount, samples } => {
            use crate::core::ae_effects_pack_v28::apply_radial_fast_blur;
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            apply_radial_fast_blur(pixels, width, height, [cx, cy], amount.evaluate(frame), *samples);
        }
        EffectType::BendIt { top_offset, bottom_offset } => {
            use crate::core::ae_effects_pack_v28::apply_cc_bend_it_pro;
            apply_cc_bend_it_pro(pixels, width, height, top_offset.evaluate(frame), bottom_offset.evaluate(frame));
        }
        EffectType::Tiler { scale_percent, mirror } => {
            use crate::core::ae_effects_pack_v28::{apply_cc_tiler_pro, TileEdgeMode};
            let mode = if *mirror { TileEdgeMode::Mirror } else { TileEdgeMode::Repeat };
            apply_cc_tiler_pro(pixels, width, height, scale_percent.evaluate(frame), mode);
        }
        EffectType::Tritone { shadow_color, mid_color, highlight_color } => {
            let to_c3 = |c: [f32; 3]| [(c[0].clamp(0.0, 1.0) * 255.0).round() as u8, (c[1].clamp(0.0, 1.0) * 255.0).round() as u8, (c[2].clamp(0.0, 1.0) * 255.0).round() as u8];
            let s = to_c3(shadow_color.evaluate(frame));
            let m = to_c3(mid_color.evaluate(frame));
            let h = to_c3(highlight_color.evaluate(frame));
            pack::apply_tritone(pixels, s, m, h);
        }
        EffectType::MatteChoker { choke_amount, gray_level } => {
            pack::apply_matte_choker(pixels, choke_amount.evaluate(frame), gray_level.evaluate(frame));
        }
        EffectType::VenetianBlinds { completion, width: blind_width } => {
            pack::apply_venetian_blinds(pixels, width, height, completion.evaluate(frame), blind_width.evaluate(frame).max(1.0) as u32);
        }
        EffectType::Vibrance { amount } => {
            crate::core::color_correction::apply_vibrance(pixels, amount.evaluate(frame));
        }
        EffectType::WhiteBalance { temperature, tint } => {
            use crate::core::color_correction::{apply_white_balance, WhiteBalance};
            apply_white_balance(pixels, &WhiteBalance {
                temperature: temperature.evaluate(frame),
                tint: tint.evaluate(frame),
            });
        }
        EffectType::HslAdjust { hue_deg, saturation, lightness } => {
            use crate::core::color_correction::{apply_hsl_adjust, HslAdjust};
            apply_hsl_adjust(pixels, &HslAdjust {
                hue_deg: hue_deg.evaluate(frame),
                saturation: saturation.evaluate(frame),
                lightness: lightness.evaluate(frame),
            });
        }
        EffectType::GlowPro { threshold, radius, intensity } => {
            use crate::core::ae_effects_pack_v28::apply_glow_pro;
            apply_glow_pro(
                pixels,
                width,
                height,
                threshold.evaluate(frame),
                radius.evaluate(frame).round().clamp(0.0, 128.0) as u32,
                intensity.evaluate(frame),
            );
        }
        EffectType::CrtScanlines { line_spacing, intensity } => {
            use crate::core::ae_effects_pack_v12::apply_crt_scanlines;
            apply_crt_scanlines(
                pixels,
                width,
                height,
                line_spacing.evaluate(frame).round().clamp(1.0, 200.0) as u32,
                intensity.evaluate(frame),
            );
        }
        EffectType::Vortex { radius, angle_deg } => {
            use crate::core::ae_effects_pack_v13::apply_vortex_distortion;
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            apply_vortex_distortion(
                pixels,
                width,
                height,
                [cx, cy],
                radius.evaluate(frame).max(1.0),
                angle_deg.evaluate(frame),
            );
        }
        EffectType::HeatDistortion { strength, speed } => {
            use crate::core::ae_effects_pack_v13::apply_heat_distortion;
            let time = frame as f32 / 30.0 * speed.evaluate(frame);
            apply_heat_distortion(pixels, width, height, time, strength.evaluate(frame));
        }
        EffectType::RainRipples { drop_count, wave_strength } => {
            use crate::core::ae_effects_pack_v12::apply_rain_ripples;
            apply_rain_ripples(
                pixels,
                width,
                height,
                frame,
                drop_count.evaluate(frame).round().clamp(0.0, 100.0) as u32,
                wave_strength.evaluate(frame),
            );
        }
        EffectType::Fisheye { strength } => {
            use crate::core::ae_effects_pack_v21::apply_fisheye;
            apply_fisheye(pixels, width, height, strength.evaluate(frame));
        }
        EffectType::LensCorrection { k1, k2 } => {
            use crate::core::ae_effects_pack_v21::apply_barrel_correction;
            apply_barrel_correction(pixels, width, height, k1.evaluate(frame), k2.evaluate(frame));
        }
        EffectType::GlitchDisplacement { seed, amount } => {
            use crate::core::ae_effects_pack_v13::apply_glitch_displacement;
            apply_glitch_displacement(
                pixels,
                width,
                height,
                seed.evaluate(frame).round().clamp(0.0, 99999.0) as u32,
                amount.evaluate(frame),
            );
        }
        EffectType::MatteChokeSpread { radius, expand } => {
            use crate::core::ae_effects_pack_v22::apply_matte_choke;
            apply_matte_choke(
                pixels,
                width,
                height,
                radius.evaluate(frame).round().clamp(1.0, 64.0) as u32,
                *expand,
            );
        }
        EffectType::AlphaFeather { radius } => {
            use crate::core::ae_effects_pack_v22::apply_alpha_feather;
            apply_alpha_feather(
                pixels,
                width,
                height,
                radius.evaluate(frame).round().clamp(1.0, 64.0) as u32,
            );
        }
        EffectType::AlphaFromLuminance { invert } => {
            use crate::core::ae_effects_pack_v22::apply_alpha_from_luminance;
            apply_alpha_from_luminance(pixels, *invert);
        }
        EffectType::NightVision { amplification } => {
            use crate::core::ae_effects_pack_v20::apply_night_vision;
            // Seed advances with the frame: deterministic per frame, animated over time.
            apply_night_vision(pixels, amplification.evaluate(frame), frame.wrapping_mul(2654435761));
        }
        EffectType::IrisWipe { completion } => {
            use crate::core::ae_effects_pack_v2::apply_iris_wipe;
            apply_iris_wipe(pixels, width, height, completion.evaluate(frame));
        }
        EffectType::RadialWipe { completion } => {
            use crate::core::ae_effects_pack_v2::apply_radial_wipe;
            apply_radial_wipe(pixels, width, height, completion.evaluate(frame));
        }
        EffectType::FilmEmulation { lift, gamma, gain, hue_shift_deg } => {
            use crate::core::ae_effects_pack_v20::apply_film_emulation;
            apply_film_emulation(
                pixels,
                lift.evaluate(frame),
                gamma.evaluate(frame).max(0.01),
                gain.evaluate(frame).max(0.0),
                hue_shift_deg.evaluate(frame),
            );
        }
        EffectType::GodRays { sun_x, sun_y, samples, decay, weight } => {
            use crate::core::ae_effects_pack_v25::apply_god_rays;
            let sun = [
                sun_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                sun_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            apply_god_rays(
                pixels,
                width,
                height,
                sun,
                samples.evaluate(frame).round().clamp(1.0, 64.0) as u32,
                decay.evaluate(frame).clamp(0.5, 1.0),
                weight.evaluate(frame).max(0.0),
            );
        }
        EffectType::RadialBlurZoom { amount } => {
            use crate::core::ae_effects_pack_v11::apply_radial_blur_zoom;
            let center = [width as f32 * 0.5, height as f32 * 0.5];
            apply_radial_blur_zoom(pixels, width, height, center, amount.evaluate(frame));
        }
        EffectType::MedianFilter { radius } => {
            use crate::core::ae_effects_pack_v19::apply_median_filter;
            apply_median_filter(pixels, width, height, radius.evaluate(frame).round().clamp(1.0, 16.0) as u32);
        }
        EffectType::SobelEdges { invert } => {
            use crate::core::ae_effects_pack_v19::apply_sobel_edges;
            apply_sobel_edges(pixels, width, height, *invert);
        }
        EffectType::Mosaic { block_w, block_h } => {
            use crate::core::ae_effects_pack_v19::apply_mosaic;
            apply_mosaic(
                pixels,
                width,
                height,
                block_w.evaluate(frame).round().clamp(1.0, 256.0) as u32,
                block_h.evaluate(frame).round().clamp(1.0, 256.0) as u32,
            );
        }
        EffectType::TiltShift { focus_y, focus_height, max_blur } => {
            use crate::core::ae_effects_pack_v19::apply_tilt_shift;
            apply_tilt_shift(
                pixels,
                width,
                height,
                focus_y.evaluate(frame),
                focus_height.evaluate(frame).max(1.0),
                max_blur.evaluate(frame).round().clamp(0.0, 32.0) as u32,
            );
        }
        EffectType::Emboss { angle_deg, depth } => {
            use crate::core::ae_effects_pack_v17::apply_emboss;
            apply_emboss(pixels, width, height, angle_deg.evaluate(frame), depth.evaluate(frame));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Effect, EffectType};

    fn solid_layer(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut px = vec![0u8; (w * h * 4) as usize];
        for p in px.chunks_exact_mut(4) {
            p[0] = r; p[1] = g; p[2] = b; p[3] = 255;
        }
        px
    }

    fn effect(name: &str, et: EffectType) -> Effect {
        Effect { id: name.to_string(), name: name.to_string(), effect_type: et, enabled: true }
    }

    #[test]
    fn test_invert_flips_values() {
        let mut px = solid_layer(4, 4, 100, 150, 200);
        let before = px[1];
        apply_layer_effects(
            &mut px, 4, 4,
            &[effect("inv", EffectType::Invert { invert_alpha: false })],
            0,
        );
        assert_eq!(px[0], 255 - 100);
        assert_eq!(px[1], 255 - before);
    }

    #[test]
    fn test_posterize_reduces_levels() {
        let mut px = solid_layer(8, 8, 123, 45, 67);
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("post", EffectType::Posterize { levels: Animatable::new_constant(2.0) })],
            0,
        );
        // With 2 levels, each channel becomes either 0 or 255.
        for &c in &[px[0], px[1], px[2]] {
            assert!(c == 0 || c == 255, "channel {} not posterized", c);
        }
    }

    #[test]
    fn test_threshold_binaries() {
        let mut px = solid_layer(8, 8, 123, 200, 10);
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("thr", EffectType::Threshold { threshold: Animatable::new_constant(128.0) })],
            0,
        );
        for p in px.chunks_exact(4) {
            let luma = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
            let expected = if luma >= 128 { 255 } else { 0 };
            assert_eq!(p[0], expected);
            assert_eq!(p[1], expected);
            assert_eq!(p[2], expected);
        }
    }

    #[test]
    fn test_blur_changes_pixels() {
        // A single bright pixel in an otherwise dark buffer should spread when blurred.
        let mut px = vec![0u8; 8 * 8 * 4];
        for p in px.chunks_exact_mut(4) { p[3] = 255; }
        let cx = (4 * 8 + 4) * 4;
        px[cx] = 255; px[cx + 1] = 255; px[cx + 2] = 255;

        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("blur", EffectType::GaussianBlur { blur_radius: Animatable::new_constant(3.0) })],
            0,
        );
        // Center stays bright, a near neighbor should now be non-zero (spread).
        assert!(px[cx] > 0);
        let neighbor = ((4 * 8 + 5) * 4) as usize;
        assert!(px[neighbor] > 0, "blur did not spread to neighbor");
    }

    #[test]
    fn test_disabled_effect_is_noop() {
        let mut px = solid_layer(4, 4, 100, 100, 100);
        let mut eff = effect("inv", EffectType::Invert { invert_alpha: false });
        eff.enabled = false;
        apply_layer_effects(&mut px, 4, 4, &[eff], 0);
        assert_eq!(px[0], 100);
    }

    #[test]
    fn test_offset_shifts_content() {
        // Place a small bright patch at center of a transparent buffer, then offset.
        let mut px = vec![0u8; 8 * 8 * 4];
        for p in px.chunks_exact_mut(4) { p[3] = 255; }
        // Paint pixel (4,4) red.
        let idx = ((4 * 8 + 4) * 4) as usize;
        px[idx] = 255; px[idx+1] = 0; px[idx+2] = 0;

        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("off", EffectType::Offset {
                shift_x: Animatable::new_constant(2.0),
                shift_y: Animatable::new_constant(0.0),
            })],
            0,
        );
        // After toroidal shift right by 2, red pixel moved from (4,4) to (6,4).
        let new_idx = ((4 * 8 + 6) * 4) as usize;
        assert_eq!(px[new_idx], 255, "red channel at shifted position");
        // Original position should now be transparent (or at least not red).
        assert_eq!(px[idx], 0, "original position cleared after offset");
    }

    #[test]
    fn test_vibrance_boosts_chroma_but_not_gray() {
        let mut gray = solid_layer(4, 4, 128, 128, 128);
        apply_layer_effects(
            &mut gray, 4, 4,
            &[effect("vib", EffectType::Vibrance { amount: Animatable::new_constant(100.0) })],
            0,
        );
        assert_eq!(
            (gray[0], gray[1], gray[2]),
            (128, 128, 128),
            "zero-chroma pixels must survive max vibrance"
        );

        let mut colored = solid_layer(4, 4, 90, 140, 200);
        let before = colored.clone();
        apply_layer_effects(
            &mut colored, 4, 4,
            &[effect("vib", EffectType::Vibrance { amount: Animatable::new_constant(80.0) })],
            0,
        );
        assert_ne!(&colored[0..3], &before[0..3], "chroma must shift under vibrance");
    }

    #[test]
    fn test_white_balance_warms_and_cools() {
        let mut warm = solid_layer(4, 4, 120, 120, 120);
        apply_layer_effects(
            &mut warm, 4, 4,
            &[effect("wb", EffectType::WhiteBalance {
                temperature: Animatable::new_constant(100.0),
                tint: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert!(warm[0] > warm[2], "positive temperature must push R above B");

        let mut cool = solid_layer(4, 4, 120, 120, 120);
        apply_layer_effects(
            &mut cool, 4, 4,
            &[effect("wb", EffectType::WhiteBalance {
                temperature: Animatable::new_constant(-100.0),
                tint: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert!(cool[2] > cool[0], "negative temperature must push B above R");
    }

    #[test]
    fn test_hsl_adjust_neutral_noop_and_hue_rotation() {
        let mut px = solid_layer(4, 4, 90, 140, 200);
        let before = px.clone();
        apply_layer_effects(
            &mut px, 4, 4,
            &[effect("hsl", EffectType::HslAdjust {
                hue_deg: Animatable::new_constant(0.0),
                saturation: Animatable::new_constant(0.0),
                lightness: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert_eq!(px, before, "neutral HSL must be a no-op");

        let mut red = solid_layer(4, 4, 255, 40, 40);
        apply_layer_effects(
            &mut red, 4, 4,
            &[effect("hsl", EffectType::HslAdjust {
                hue_deg: Animatable::new_constant(180.0),
                saturation: Animatable::new_constant(0.0),
                lightness: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert!(red[0] < red[1].max(red[2]), "180° rotation must move red toward cyan");
    }

    #[test]
    fn test_glow_pro_bleeds_bright_pixels() {
        // Dark 8x8 buffer with a single white pixel: glow must spread brightness.
        let mut px = vec![0u8; 8 * 8 * 4];
        for p in px.chunks_exact_mut(4) { p[3] = 255; }
        let cx = ((4 * 8 + 4) * 4) as usize;
        px[cx] = 255; px[cx + 1] = 255; px[cx + 2] = 255;

        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("glow", EffectType::GlowPro {
                threshold: Animatable::new_constant(0.5),
                radius: Animatable::new_constant(2.0),
                intensity: Animatable::new_constant(1.5),
            })],
            0,
        );
        let neighbor = ((4 * 8 + 6) * 4) as usize;
        assert!(px[neighbor] > 0, "glow did not bleed to neighbor pixel");
        // Zero intensity must be a no-op.
        let mut idle = solid_layer(4, 4, 200, 200, 200);
        let before = idle.clone();
        apply_layer_effects(
            &mut idle, 4, 4,
            &[effect("glow", EffectType::GlowPro {
                threshold: Animatable::new_constant(0.5),
                radius: Animatable::new_constant(4.0),
                intensity: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert_eq!(idle, before, "zero-intensity glow must be a no-op");
    }

    #[test]
    fn test_stylize_distort_pack_noops_and_effects() {
        // Zero-strength / zero-angle settings must be exact no-ops.
        let base = solid_layer(8, 8, 90, 140, 200);

        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("heat", EffectType::HeatDistortion {
                strength: Animatable::new_constant(0.0),
                speed: Animatable::new_constant(1.0),
            })],
            0,
        );
        assert_eq!(px, base, "zero-strength heat must be a no-op");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("vortex", EffectType::Vortex {
                radius: Animatable::new_constant(50.0),
                angle_deg: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert_eq!(px, base, "zero-angle vortex must be a no-op");

        // CRT scanlines must darken even rows only.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("crt", EffectType::CrtScanlines {
                line_spacing: Animatable::new_constant(2.0),
                intensity: Animatable::new_constant(0.5),
            })],
            0,
        );
        let row0 = (px[0] as u32 + px[4] as u32) / 2;
        let row1 = ((px[(1 * 8 * 4)] as u32) + px[(1 * 8 * 4) + 4] as u32) / 2;
        assert_eq!(row0, 45, "row 0 (y%2==0) darkened to 50%");
        assert_eq!(row1, 90, "odd rows untouched");
    }

    #[test]
    fn test_distort_pack_neutral_settings_are_noops() {
        let base = solid_layer(8, 8, 90, 140, 200);

        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("fish", EffectType::Fisheye { strength: Animatable::new_constant(0.0) })],
            0,
        );
        assert_eq!(px, base, "zero-strength fisheye must be a no-op");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("lc", EffectType::LensCorrection {
                k1: Animatable::new_constant(0.0),
                k2: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert_eq!(px, base, "k1=k2=0 lens correction must be identity");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("glitch", EffectType::GlitchDisplacement {
                seed: Animatable::new_constant(1.0),
                amount: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert_eq!(px, base, "zero-amount glitch must be a no-op");
    }

    #[test]
    fn test_matte_alpha_pack_behaviour() {
        // Opaque square in a transparent 8x8 buffer (alpha edge at x=1..6).
        let mut base = vec![0u8; 8 * 8 * 4];
        for y in 1..7usize {
            for x in 1..7usize {
                let i = (y * 8 + x) * 4;
                base[i] = 200; base[i + 1] = 100; base[i + 2] = 50; base[i + 3] = 255;
            }
        }

        // Choke with radius 1 must shrink the matte: corner pixel (1,1) loses alpha.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("choke", EffectType::MatteChokeSpread {
                radius: Animatable::new_constant(1.0),
                expand: false,
            })],
            0,
        );
        let corner = (1 * 8 + 1) * 4;
        assert_eq!(px[corner + 3], 0, "choke must erode the alpha corner");

        // Spread must re-dilate it back.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("spread", EffectType::MatteChokeSpread {
                radius: Animatable::new_constant(1.0),
                expand: true,
            })],
            0,
        );
        assert_eq!(px[(1 * 8 + 0) * 4 + 3], 255, "spread must dilate alpha outward");

        // Luma→alpha: white = opaque, black = transparent; invert flips both.
        let mut px = vec![0u8; 2 * 1 * 4];
        px[0] = 255; px[1] = 255; px[2] = 255; px[3] = 255;
        px[4] = 0; px[5] = 0; px[6] = 0; px[7] = 255;
        apply_layer_effects(
            &mut px, 2, 1,
            &[effect("luma", EffectType::AlphaFromLuminance { invert: false })],
            0,
        );
        assert_eq!(px[3], 255, "white luma must become full alpha");
        assert_eq!(px[7], 0, "black luma must become zero alpha");

        let mut px = vec![0u8; 2 * 1 * 4];
        px[0] = 255; px[1] = 255; px[2] = 255; px[3] = 255;
        px[4] = 0; px[5] = 0; px[6] = 0; px[7] = 255;
        apply_layer_effects(
            &mut px, 2, 1,
            &[effect("lumainv", EffectType::AlphaFromLuminance { invert: true })],
            0,
        );
        assert_eq!(px[3], 0, "inverted white must become transparent");
        assert_eq!(px[7], 255, "inverted black must become opaque");
    }

    #[test]
    fn test_stylize_transition_pack() {
        // Night vision output must be phosphor-green dominant.
        let mut px = solid_layer(4, 4, 90, 140, 200);
        apply_layer_effects(
            &mut px, 4, 4,
            &[effect("nv", EffectType::NightVision { amplification: Animatable::new_constant(2.0) })],
            0,
        );
        for p in px.chunks_exact(4) {
            assert!(p[1] >= p[0] && p[1] >= p[2], "green channel must dominate");
        }
        // Determinism: same frame → identical output.
        let mut again = solid_layer(4, 4, 90, 140, 200);
        apply_layer_effects(
            &mut again, 4, 4,
            &[effect("nv", EffectType::NightVision { amplification: Animatable::new_constant(2.0) })],
            0,
        );
        assert_eq!(px, again, "night vision must be deterministic per frame");

        // Iris wipe at 0% must leave the image untouched (fully covered).
        let base = solid_layer(8, 8, 90, 140, 200);
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("iris", EffectType::IrisWipe { completion: Animatable::new_constant(0.0) })],
            0,
        );
        assert_eq!(px, base, "0% iris wipe must be a no-op");

        // Same for radial wipe.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("rad", EffectType::RadialWipe { completion: Animatable::new_constant(0.0) })],
            0,
        );
        assert_eq!(px, base, "0% radial wipe must be a no-op");
    }

    #[test]
    fn test_grade_light_pack() {
        let base = solid_layer(8, 8, 90, 140, 200);

        // Neutral ASC CDL grade must stay within rounding distance of identity.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("film", EffectType::FilmEmulation {
                lift: Animatable::new_constant(0.0),
                gamma: Animatable::new_constant(1.0),
                gain: Animatable::new_constant(1.0),
                hue_shift_deg: Animatable::new_constant(0.0),
            })],
            0,
        );
        for (out, inp) in px.chunks_exact(4).zip(base.chunks_exact(4)) {
            assert!((out[0] as i32 - inp[0] as i32).abs() <= 2, "R drift {}", out[0]);
            assert!((out[1] as i32 - inp[1] as i32).abs() <= 2, "G drift {}", out[1]);
            assert!((out[2] as i32 - inp[2] as i32).abs() <= 2, "B drift {}", out[2]);
        }

        // God rays with zero weight adds no light.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("rays", EffectType::GodRays {
                sun_x: Animatable::new_constant(0.5),
                sun_y: Animatable::new_constant(0.0),
                samples: Animatable::new_constant(8.0),
                decay: Animatable::new_constant(0.9),
                weight: Animatable::new_constant(0.0),
            })],
            0,
        );
        assert_eq!(px, base, "zero-weight god rays must be a no-op");

        // Zero-amount zoom blur must be a no-op.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("zb", EffectType::RadialBlurZoom { amount: Animatable::new_constant(0.0) })],
            0,
        );
        assert_eq!(px, base, "zero-amount zoom blur must be a no-op");

        // Positive zoom blur must keep the centre bright on a uniform image.
        let mut px = vec![255u8; 16 * 16 * 4];
        apply_layer_effects(
            &mut px, 16, 16,
            &[effect("zb", EffectType::RadialBlurZoom { amount: Animatable::new_constant(40.0) })],
            0,
        );
        let center = ((8 * 16 + 8) * 4) as usize;
        assert!(px[center] > 0, "zoom blur keeps centre bright");
    }

    #[test]
    fn test_sharpen_stylize_pack_uniform_safety() {
        // Median filter of a uniform image must be the image itself.
        let base = solid_layer(8, 8, 90, 140, 200);
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("med", EffectType::MedianFilter { radius: Animatable::new_constant(2.0) })],
            0,
        );
        assert_eq!(px, base, "median of uniform image is identity");

        // Mosaic with block size 1x1 must be identity.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("mos", EffectType::Mosaic {
                block_w: Animatable::new_constant(1.0),
                block_h: Animatable::new_constant(1.0),
            })],
            0,
        );
        assert_eq!(px, base, "1x1 mosaic is identity");

        // Sobel on a uniform image must produce no edges inside the frame.
        // (The kernel leaves the 1px border untouched, so only check interior.)
        let mut px = base.clone();
        apply_layer_effects(
            &mut px, 8, 8,
            &[effect("sobel", EffectType::SobelEdges { invert: false })],
            0,
        );
        for y in 1..7usize {
            for x in 1..7usize {
                let i = (y * 8 + x) * 4;
                assert!(px[i] < 16, "uniform image must have no sobel edges (got {})", px[i]);
            }
        }
    }
}
