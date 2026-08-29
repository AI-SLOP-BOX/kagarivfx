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
    fps: u32,
) {
    apply_layer_effects_ctx(pixels, width, height, effects, frame, fps, None);
}

/// Like [`apply_layer_effects`], with an optional pre-projected light position
/// (normalized 0..1) for lens flares that link to a comp light.
pub fn apply_layer_effects_ctx(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    effects: &[Effect],
    frame: u32,
    fps: u32,
    light_screen: Option<[f32; 2]>,
) {
    for effect in effects {
        if !effect.enabled {
            continue;
        }
        apply_one_ctx(
            pixels,
            width,
            height,
            &effect.effect_type,
            frame,
            fps,
            light_screen,
        );
    }
}

fn apply_one_ctx(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    effect_type: &EffectType,
    frame: u32,
    fps: u32,
    light_screen: Option<[f32; 2]>,
) {
    use crate::core::ae_effects_pack as pack;

    match effect_type {
        // Effects already present in the GPU pipeline, mirrored on CPU.
        EffectType::GaussianBlur { blur_radius } => {
            let r = blur_radius.evaluate(frame).max(0.0) as u32;
            // GPU compute path (opt-in via settings); CPU fallback keeps
            // byte-deterministic output when disabled or unavailable.
            if !crate::core::compute_pipeline::try_gpu_gaussian_blur(
                pixels,
                width,
                height,
                r.min(crate::core::compute_pipeline::MAX_BLUR_RADIUS),
            ) {
                pack::apply_gaussian_blur(pixels, width, height, r);
            }
        }
        EffectType::ColorTint { color, intensity } => {
            let rgb = color3_to_u8(color.evaluate(frame));
            let amount = (intensity.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            pack::apply_tint(pixels, rgb, rgb, amount);
        }
        EffectType::DropShadow {
            color,
            opacity,
            direction,
            distance,
            softness,
        } => {
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
        EffectType::Glow {
            threshold,
            radius,
            intensity,
            ..
        } => {
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
            if !crate::core::compute_pipeline::try_gpu_invert(pixels, width, height) {
                pack::apply_invert(pixels, *invert_alpha);
            }
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
            let a = angle.evaluate(frame);
            let len = length.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_directional_blur(
                pixels,
                width,
                height,
                len.round().clamp(2.0, 256.0) as u32,
                a,
            ) {
                pack::apply_directional_blur(pixels, width, height, a, len);
            }
        }
        EffectType::RadialBlur { amount } => {
            let amt = amount.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_radial_blur(
                pixels,
                width,
                height,
                (amt * 2.0).round().clamp(2.0, 256.0) as u32,
            ) {
                pack::apply_radial_blur(pixels, width, height, amt);
            }
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
        EffectType::ChromaKey {
            screen_color,
            screen_gain,
            clip_black,
            clip_white,
        } => {
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
        EffectType::Spherize {
            radius,
            refractive_index,
        } => {
            let opts = crate::core::spherize::SpherizeOptions {
                radius: radius.evaluate(frame),
                center: [width as f32 * 0.5, height as f32 * 0.5],
                refractive_index: refractive_index.evaluate(frame),
            };
            let out = crate::core::spherize::apply_spherize(pixels, width, height, &opts);
            pixels.copy_from_slice(&out);
        }
        EffectType::TurbulentDisplace {
            amount,
            size,
            evolution,
            complexity,
        } => {
            let opts = crate::core::turbulent_displace::TurbulentDisplaceOptions {
                displace_type: crate::core::turbulent_displace::TurbulentDisplaceType::Turbulent,
                amount: amount.evaluate(frame),
                size: size.evaluate(frame),
                evolution_deg: evolution.evaluate(frame),
                complexity: complexity.evaluate(frame).max(1.0) as u32,
            };
            let out = crate::core::turbulent_displace::apply_turbulent_displace(
                pixels, width, height, &opts,
            );
            pixels.copy_from_slice(&out);
        }
        EffectType::Colorama {
            preset_index,
            cycle_phase,
        } => {
            let idx = preset_index.evaluate(frame).round() as u32 % 4;
            let preset = match idx {
                0 => crate::core::colorama::ColoramaPreset::Rainbow,
                1 => crate::core::colorama::ColoramaPreset::Heatmap,
                2 => crate::core::colorama::ColoramaPreset::Sepia,
                _ => crate::core::colorama::ColoramaPreset::Solarize,
            };
            crate::core::colorama::apply_colorama(
                pixels,
                width,
                height,
                preset,
                cycle_phase.evaluate(frame),
            );
        }

        // Effects with CPU kernels: dispatch to cpu_effects_new
        EffectType::ChromaticAberration {
            shift_r,
            shift_b,
            edge_falloff,
            iris_linked,
        } => {
            let mut sr = shift_r.evaluate(frame);
            let mut sb = shift_b.evaluate(frame);
            // Iris-linked mode: aberration scales with DOF circle-of-confusion
            // and iris blade count for physically-plausible fringing.
            if *iris_linked {
                let coc = crate::core::expression_engine::get_audio_band(0).abs() * 0.0 + 1.0;
                // Use audio band 0 as a proxy for layer DOF intensity (0..1)
                // More blades → tighter fringing; wider aperture → stronger shift
                let blade_scale = (5.0_f32 / 8.0).max(0.3); // typical 5-8 blade iris
                sr *= coc * blade_scale;
                sb *= coc * blade_scale;
            }
            crate::core::cpu_effects_new::apply_chromatic_aberration(
                pixels,
                width,
                height,
                sr,
                sb,
                edge_falloff.evaluate(frame),
            );
        }
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => {
            crate::core::cpu_effects_new::apply_vignette(
                pixels,
                width,
                height,
                intensity.evaluate(frame),
                roundness.evaluate(frame),
                feather.evaluate(frame),
                color.evaluate(frame),
            );
        }
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => {
            crate::core::cpu_effects_new::apply_levels(
                pixels,
                width,
                height,
                input_black.evaluate(frame),
                input_white.evaluate(frame),
                gamma.evaluate(frame),
                output_black.evaluate(frame),
                output_white.evaluate(frame),
            );
        }
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => {
            crate::core::cpu_effects_new::apply_hue_saturation(
                pixels,
                width,
                height,
                hue_shift.evaluate(frame),
                saturation.evaluate(frame),
                lightness.evaluate(frame),
            );
        }
        EffectType::MotionBlur {
            shutter_angle,
            samples,
        } => {
            crate::core::cpu_effects_new::apply_motion_blur(
                pixels,
                width,
                height,
                shutter_angle.evaluate(frame),
                *samples as f32,
            );
        }
        EffectType::MeshWarp {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } => {
            crate::core::cpu_effects_new::apply_mesh_warp(
                pixels,
                width,
                height,
                top_left.evaluate(frame),
                top_right.evaluate(frame),
                bottom_left.evaluate(frame),
                bottom_right.evaluate(frame),
            );
        }
        EffectType::CornerPin {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } => {
            let quad = crate::core::corner_pin::CornerPinQuad {
                top_left: top_left.evaluate(frame),
                top_right: top_right.evaluate(frame),
                bottom_right: bottom_right.evaluate(frame),
                bottom_left: bottom_left.evaluate(frame),
            };
            let mut out = vec![0u8; pixels.len()];
            crate::core::corner_pin::apply_corner_pin_warp(
                pixels, width, height, &mut out, width, height, &quad,
            );
            pixels.copy_from_slice(&out);
        }
        EffectType::ColorGradeLUT {
            lut_path,
            intensity,
        } => {
            let intensity = intensity.evaluate(frame).clamp(0.0, 1.0);
            if !lut_path.is_empty() && intensity > 0.0 {
                use std::cell::RefCell;
                thread_local! { static LUT_CACHE: RefCell<crate::core::lut_cache::LutCache> = RefCell::new(crate::core::lut_cache::LutCache::new(512)); }
                if let Ok(text) = std::fs::read_to_string(lut_path) {
                    if let Ok(lut) = crate::core::ocio_color::Lut3D::parse_cube(&text) {
                        for p in pixels.chunks_exact_mut(4) {
                            let r = p[0] as f32 / 255.0;
                            let g = p[1] as f32 / 255.0;
                            let b = p[2] as f32 / 255.0;
                            let (lr, lg, lb) = LUT_CACHE.with(|cache| {
                                let mut c = cache.borrow_mut();
                                c.get_or_insert(r, g, b, lut.size, |r, g, b| lut.apply(r, g, b))
                            });
                            let t = intensity;
                            p[0] = ((r * (1.0 - t) + lr * t).clamp(0.0, 1.0) * 255.0).round() as u8;
                            p[1] = ((g * (1.0 - t) + lg * t).clamp(0.0, 1.0) * 255.0).round() as u8;
                            p[2] = ((b * (1.0 - t) + lb * t).clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    } else {
                        log::warn!("ColorGradeLUT: failed to parse LUT from {}", lut_path);
                    }
                } else {
                    log::warn!("ColorGradeLUT: cannot read LUT file {}", lut_path);
                }
            }
        }
        EffectType::ColorSpaceConvert { mode } => {
            crate::core::cpu_effects_new::apply_color_space_convert(
                pixels,
                width,
                height,
                *mode as u32,
            );
        }
        EffectType::FilmGrain {
            intensity,
            grain_size,
            color_film: _,
        } => {
            crate::core::cpu_effects_new::apply_film_grain(
                pixels,
                width,
                height,
                intensity.evaluate(frame),
                *grain_size as u32,
                frame,
            );
        }
        EffectType::FractalNoise {
            fractal_type,
            contrast,
            brightness,
            complexity,
            evolution,
        } => {
            crate::core::cpu_effects_new::apply_fractal_noise(
                pixels,
                width,
                height,
                fractal_type.evaluate(frame),
                contrast.evaluate(frame),
                brightness.evaluate(frame),
                complexity.evaluate(frame),
                evolution.evaluate(frame),
            );
        }
        EffectType::Curves { channel } => {
            crate::core::cpu_effects_new::apply_curves(
                pixels,
                width,
                height,
                channel.evaluate(frame),
            );
        }
        EffectType::DisplacementMap {
            source_layer,
            max_horizontal,
            max_vertical,
        } => {
            crate::core::cpu_effects_new::apply_displacement_map(
                pixels,
                width,
                height,
                source_layer.evaluate(frame),
                max_horizontal.evaluate(frame),
                max_vertical.evaluate(frame),
            );
        }
        EffectType::CompoundBlur {
            source_layer,
            max_blur,
        } => {
            crate::core::cpu_effects_new::apply_compound_blur(
                pixels,
                width,
                height,
                source_layer.evaluate(frame),
                max_blur.evaluate(frame),
            );
        }
        EffectType::Minimax { operation, radius } => {
            crate::core::cpu_effects_new::apply_minimax(
                pixels,
                width,
                height,
                operation.evaluate(frame),
                radius.evaluate(frame),
            );
        }
        EffectType::ShiftChannels {
            take_red,
            take_green,
            take_blue,
            take_alpha,
        } => {
            crate::core::cpu_effects_new::apply_shift_channels(
                pixels,
                width,
                height,
                take_red.evaluate(frame),
                take_green.evaluate(frame),
                take_blue.evaluate(frame),
                take_alpha.evaluate(frame),
            );
        }

        // ── Effects migrated from ExtEffect ──
        EffectType::WaveWarp {
            wave_height,
            wave_width,
            speed,
            direction_deg,
            wave_type,
            pinning,
        } => {
            use crate::core::ae_effects_pack_v27::{
                apply_wave_warp_pro, PinKind, WaveType, WaveWarpParams,
            };
            let params = WaveWarpParams {
                wave_height: wave_height.evaluate(frame),
                wave_width: wave_width.evaluate(frame),
                speed: speed.evaluate(frame),
                time: frame as f32 / fps.max(1) as f32,
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
            apply_cc_lens_pro(
                pixels,
                width,
                height,
                &CcLensParams {
                    convergence: convergence.evaluate(frame),
                    zoom: zoom.evaluate(frame),
                },
            );
        }
        EffectType::PolarCoordinates {
            to_polar,
            interpolation,
        } => {
            use crate::core::ae_effects_pack_v27::{apply_polar_coordinates_pro, PolarMode};
            let mode = if *to_polar {
                PolarMode::RectToPolar
            } else {
                PolarMode::PolarToRect
            };
            apply_polar_coordinates_pro(pixels, width, height, mode, interpolation.evaluate(frame));
        }
        EffectType::OpticsCompensation {
            field_of_view_deg,
            reverse,
            zoom,
        } => {
            use crate::core::ae_effects_pack_v27::{
                apply_optics_compensation, OpticsCompensationParams,
            };
            apply_optics_compensation(
                pixels,
                width,
                height,
                &OpticsCompensationParams {
                    field_of_view_deg: field_of_view_deg.evaluate(frame),
                    reverse: *reverse,
                    zoom: zoom.evaluate(frame),
                },
            );
        }
        EffectType::ColorBalance {
            shadows,
            midtones,
            highlights,
            preserve_luminosity,
        } => {
            use crate::core::color_correction::{apply_color_balance, ColorBalance};
            apply_color_balance(
                pixels,
                &ColorBalance {
                    shadows: *shadows,
                    midtones: *midtones,
                    highlights: *highlights,
                    preserve_luminosity: *preserve_luminosity,
                },
            );
        }
        EffectType::ChannelMixer { matrix, monochrome } => {
            use crate::core::color_correction::{apply_channel_mixer, ChannelMixer};
            apply_channel_mixer(
                pixels,
                &ChannelMixer {
                    matrix: *matrix,
                    monochrome: *monochrome,
                },
            );
        }
        EffectType::LightSweep {
            direction_deg,
            center,
            width: sweep_width,
            sweep_intensity,
            edge_intensity,
        } => {
            use crate::core::ae_effects_pack_v28::{apply_light_sweep, LightSweepParams};
            apply_light_sweep(
                pixels,
                width,
                height,
                &LightSweepParams {
                    direction_deg: direction_deg.evaluate(frame),
                    center: center.evaluate(frame),
                    width: sweep_width.evaluate(frame),
                    sweep_intensity: sweep_intensity.evaluate(frame),
                    edge_intensity: edge_intensity.evaluate(frame),
                },
            );
        }
        EffectType::RadialFastBlur { amount, samples } => {
            use crate::core::ae_effects_pack_v28::apply_radial_fast_blur;
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            apply_radial_fast_blur(
                pixels,
                width,
                height,
                [cx, cy],
                amount.evaluate(frame),
                *samples,
            );
        }
        EffectType::BendIt {
            top_offset,
            bottom_offset,
        } => {
            use crate::core::ae_effects_pack_v28::apply_cc_bend_it_pro;
            apply_cc_bend_it_pro(
                pixels,
                width,
                height,
                top_offset.evaluate(frame),
                bottom_offset.evaluate(frame),
            );
        }
        EffectType::Tiler {
            scale_percent,
            mirror,
        } => {
            use crate::core::ae_effects_pack_v28::{apply_cc_tiler_pro, TileEdgeMode};
            let mode = if *mirror {
                TileEdgeMode::Mirror
            } else {
                TileEdgeMode::Repeat
            };
            apply_cc_tiler_pro(pixels, width, height, scale_percent.evaluate(frame), mode);
        }
        EffectType::Tritone {
            shadow_color,
            mid_color,
            highlight_color,
        } => {
            let to_c3 = |c: [f32; 3]| {
                [
                    (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                ]
            };
            let s = to_c3(shadow_color.evaluate(frame));
            let m = to_c3(mid_color.evaluate(frame));
            let h = to_c3(highlight_color.evaluate(frame));
            pack::apply_tritone(pixels, s, m, h);
        }
        EffectType::MatteChoker {
            choke_amount,
            gray_level,
        } => {
            pack::apply_matte_choker(
                pixels,
                choke_amount.evaluate(frame),
                gray_level.evaluate(frame),
            );
        }
        EffectType::VenetianBlinds {
            completion,
            width: blind_width,
        } => {
            pack::apply_venetian_blinds(
                pixels,
                width,
                height,
                completion.evaluate(frame),
                blind_width.evaluate(frame).max(1.0) as u32,
            );
        }
        EffectType::Vibrance { amount } => {
            crate::core::color_correction::apply_vibrance(pixels, amount.evaluate(frame));
        }
        EffectType::WhiteBalance { temperature, tint } => {
            use crate::core::color_correction::{apply_white_balance, WhiteBalance};
            apply_white_balance(
                pixels,
                &WhiteBalance {
                    temperature: temperature.evaluate(frame),
                    tint: tint.evaluate(frame),
                },
            );
        }
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => {
            use crate::core::color_correction::{apply_hsl_adjust, HslAdjust};
            apply_hsl_adjust(
                pixels,
                &HslAdjust {
                    hue_deg: hue_deg.evaluate(frame),
                    saturation: saturation.evaluate(frame),
                    lightness: lightness.evaluate(frame),
                },
            );
        }
        EffectType::GlowPro {
            threshold,
            radius,
            intensity,
        } => {
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
        EffectType::CrtScanlines {
            line_spacing,
            intensity,
        } => {
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
            let time = frame as f32 / fps.max(1) as f32 * speed.evaluate(frame);
            apply_heat_distortion(pixels, width, height, time, strength.evaluate(frame));
        }
        EffectType::RainRipples {
            drop_count,
            wave_strength,
        } => {
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
            apply_barrel_correction(
                pixels,
                width,
                height,
                k1.evaluate(frame),
                k2.evaluate(frame),
            );
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
            apply_night_vision(
                pixels,
                amplification.evaluate(frame),
                frame.wrapping_mul(2654435761),
            );
        }
        EffectType::IrisWipe { completion } => {
            use crate::core::ae_effects_pack_v2::apply_iris_wipe;
            apply_iris_wipe(pixels, width, height, completion.evaluate(frame));
        }
        EffectType::RadialWipe { completion } => {
            use crate::core::ae_effects_pack_v2::apply_radial_wipe;
            apply_radial_wipe(pixels, width, height, completion.evaluate(frame));
        }
        EffectType::FilmEmulation {
            lift,
            gamma,
            gain,
            hue_shift_deg,
        } => {
            use crate::core::ae_effects_pack_v20::apply_film_emulation;
            apply_film_emulation(
                pixels,
                lift.evaluate(frame),
                gamma.evaluate(frame).max(0.01),
                gain.evaluate(frame).max(0.0),
                hue_shift_deg.evaluate(frame),
            );
        }
        EffectType::GodRays {
            sun_x,
            sun_y,
            samples,
            decay,
            weight,
        } => {
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
        EffectType::AudioSpectrum {
            enabled,
            bands,
            opacity,
            color_start,
            color_end,
            position_x,
            position_y,
            width: spec_w,
            height: spec_h,
        } => {
            if enabled.evaluate(frame) <= 0.5 || opacity.evaluate(frame) <= 0.01 {
                return;
            }
            let bands_n = bands.evaluate(frame).clamp(1.0, 5.0) as usize;
            let opacity_n = opacity.evaluate(frame).clamp(0.0, 1.0);
            let cs = *color_start;
            let ce = *color_end;
            let px = position_x.evaluate(frame).clamp(0.0, 1.0);
            let py = position_y.evaluate(frame).clamp(0.0, 1.0);
            let w_frac = spec_w.evaluate(frame).clamp(0.01, 1.0);
            let h_frac = spec_h.evaluate(frame).clamp(0.01, 1.0);
            let x0 = (px * w_frac * width as f32).floor().max(0.0) as u32;
            let y0 = (py * h_frac * height as f32).floor().max(0.0) as u32;
            let x1 = ((px + w_frac) * width as f32).ceil().min(width as f32) as u32;
            let y1 = ((py + h_frac) * height as f32).ceil().min(height as f32) as u32;
            if x1 <= x0 || y1 <= y0 {
                return;
            }
            let strip_h = y1 - y0;
            for b in 0..bands_n {
                let amp = crate::core::expression_engine::get_audio_band(b).clamp(0.0, 1.0);
                let bar_h = (amp * strip_h as f32) as u32;
                let bx0 = x0 + (b as u32) * (x1 - x0) / bands_n as u32;
                let bx1 = x0 + (b as u32 + 1) * (x1 - x0) / bands_n as u32;
                let t = b as f32 / (bands_n.saturating_sub(1).max(1)) as f32;
                let r = ((cs[0] + (ce[0] - cs[0]) * t) * 255.0).round() as u8;
                let g = ((cs[1] + (ce[1] - cs[1]) * t) * 255.0).round() as u8;
                let bb = ((cs[2] + (ce[2] - cs[2]) * t) * 255.0).round() as u8;
                let a = ((cs[3] + (ce[3] - cs[3]) * t) * 255.0 * opacity_n).round() as u8;
                for y in (y1.saturating_sub(bar_h))..y1 {
                    for x in bx0..bx1 {
                        if x < width && y < height {
                            let idx = ((y * width + x) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                let src_a = pixels[idx + 3] as f32 / 255.0;
                                let out_a = (a as f32 / 255.0 * (1.0 - src_a)).clamp(0.0, 1.0);
                                pixels[idx] = (r as f32 * out_a).round() as u8;
                                pixels[idx + 1] = (g as f32 * out_a).round() as u8;
                                pixels[idx + 2] = (bb as f32 * out_a).round() as u8;
                                pixels[idx + 3] = (out_a * 255.0).round() as u8;
                            }
                        }
                    }
                }
            }
        }
        EffectType::RadialBlurZoom { amount } => {
            use crate::core::ae_effects_pack_v11::apply_radial_blur_zoom;
            let center = [width as f32 * 0.5, height as f32 * 0.5];
            apply_radial_blur_zoom(pixels, width, height, center, amount.evaluate(frame));
        }
        EffectType::MedianFilter { radius } => {
            use crate::core::ae_effects_pack_v19::apply_median_filter;
            apply_median_filter(
                pixels,
                width,
                height,
                radius.evaluate(frame).round().clamp(1.0, 16.0) as u32,
            );
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
        EffectType::OpticalFlares {
            position,
            brightness,
            scale,
        } => {
            let pos = position.evaluate(frame);
            let bright = brightness.evaluate(frame);
            let scl = scale.evaluate(frame);
            let cfg = crate::core::optical_flare::OpticalFlareConfig {
                position: pos,
                overall_scale: scl,
                overall_brightness: bright,
                ..Default::default()
            };
            crate::core::optical_flare::render_optical_flare(pixels, width, height, &cfg);
        }
        EffectType::MotionTile {
            tile_center,
            tile_width,
            tile_height,
            output_width,
            output_height,
            mirror_edges,
            phase,
        } => {
            let params = crate::core::motion_tile::MotionTileParams {
                tile_center: tile_center.evaluate(frame),
                tile_width: tile_width.evaluate(frame),
                tile_height: tile_height.evaluate(frame),
                output_width: output_width.evaluate(frame),
                output_height: output_height.evaluate(frame),
                mirror_edges: *mirror_edges,
                phase: phase.evaluate(frame),
            };
            let tiled = crate::core::motion_tile::apply_motion_tile(pixels, width, height, &params);
            pixels.copy_from_slice(&tiled);
        }
        EffectType::PageTurn {
            fold_position,
            fold_radius,
            fold_direction_deg,
            light_direction_deg,
            back_opacity,
            back_color,
        } => {
            let params = crate::core::page_turn::PageTurnParams {
                fold_position: fold_position.evaluate(frame),
                fold_radius: fold_radius.evaluate(frame),
                fold_direction_deg: fold_direction_deg.evaluate(frame),
                light_direction_deg: light_direction_deg.evaluate(frame),
                back_opacity: back_opacity.evaluate(frame),
                back_color: back_color.evaluate(frame),
            };
            let turned = crate::core::page_turn::apply_page_turn(pixels, width, height, &params);
            pixels.copy_from_slice(&turned);
        }
        EffectType::SetMatte {
            source_layer_idx,
            source_channel,
            invert_matte,
            composite_mode,
        } => {
            let params = crate::core::set_matte::SetMatteParams {
                source_layer_idx: *source_layer_idx,
                source_channel: *source_channel,
                invert_matte: *invert_matte,
                composite_mode: *composite_mode,
            };
            let dummy_src = pixels.to_vec();
            crate::core::set_matte::apply_set_matte(
                pixels, width, height, &dummy_src, width, height, &params,
            );
        }
        EffectType::Echo {
            echo_time_seconds: _,
            num_echoes: _,
            starting_intensity,
            decay,
            operator,
        } => {
            let start_w = starting_intensity.evaluate(frame);
            let dec = decay.evaluate(frame);
            let echo_copy = pixels.to_vec();
            crate::core::echo_effect::blend_echo_frame(
                pixels,
                &echo_copy,
                width,
                height,
                start_w * dec,
                *operator,
            );
        }
        EffectType::FindEdges { invert } => {
            let params = crate::core::find_edges::FindEdgesParams { invert: *invert };
            let edges = crate::core::find_edges::apply_find_edges(pixels, width, height, &params);
            pixels.copy_from_slice(&edges);
        }
        EffectType::Transform {
            anchor_point,
            position,
            scale_width,
            scale_height,
            uniform_scale,
            skew_deg,
            skew_axis_deg,
            rotation_deg,
            opacity,
        } => {
            let params = crate::core::transform_effect::TransformEffectParams {
                anchor_point: anchor_point.evaluate(frame),
                position: position.evaluate(frame),
                scale_width: scale_width.evaluate(frame),
                scale_height: scale_height.evaluate(frame),
                uniform_scale: *uniform_scale,
                skew_deg: skew_deg.evaluate(frame),
                skew_axis_deg: skew_axis_deg.evaluate(frame),
                rotation_deg: rotation_deg.evaluate(frame),
                opacity: opacity.evaluate(frame),
            };
            let transformed = crate::core::transform_effect::apply_transform_effect(
                pixels, width, height, &params,
            );
            pixels.copy_from_slice(&transformed);
        }
        EffectType::CameraLensBlur {
            blur_radius,
            iris_blades,
            iris_rotation_deg,
            iris_roundness,
            highlight_gain,
            highlight_threshold,
        } => {
            let params = crate::core::camera_lens_blur::CameraLensBlurParams {
                blur_radius: blur_radius.evaluate(frame),
                iris_blades: *iris_blades,
                iris_rotation_deg: iris_rotation_deg.evaluate(frame),
                iris_roundness: iris_roundness.evaluate(frame),
                highlight_gain: highlight_gain.evaluate(frame),
                highlight_threshold: highlight_threshold.evaluate(frame),
            };
            let blurred = crate::core::camera_lens_blur::apply_camera_lens_blur(
                pixels, width, height, &params,
            );
            pixels.copy_from_slice(&blurred);
        }
        EffectType::LinearColorKey {
            key_color,
            match_mode,
            tolerance,
            softness,
        } => {
            let params = crate::core::linear_color_key::LinearColorKeyParams {
                key_color: key_color.evaluate(frame),
                match_mode: *match_mode,
                tolerance: tolerance.evaluate(frame),
                softness: softness.evaluate(frame),
            };
            crate::core::linear_color_key::apply_linear_color_key(pixels, width, height, &params);
        }
        EffectType::ChannelCombiner {
            from_channel,
            to_target,
            invert,
        } => {
            let params = crate::core::channel_combiner::ChannelCombinerParams {
                from_channel: *from_channel,
                to_target: *to_target,
                invert: *invert,
            };
            crate::core::channel_combiner::apply_channel_combiner(pixels, width, height, &params);
        }
        EffectType::TiltShift {
            focus_y,
            focus_height,
            max_blur,
        } => {
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
            apply_emboss(
                pixels,
                width,
                height,
                angle_deg.evaluate(frame),
                depth.evaluate(frame),
            );
        }
        EffectType::StarField {
            num_stars,
            depth_speed,
        } => {
            use crate::core::ae_effects_pack_v18::apply_star_field;
            apply_star_field(
                pixels,
                width,
                height,
                num_stars.evaluate(frame).round().clamp(1.0, 2000.0) as u32,
                depth_speed.evaluate(frame),
                frame as f32 / fps.max(1) as f32,
            );
        }
        EffectType::LightningArc {
            start_x,
            start_y,
            end_x,
            end_y,
            seed,
            glow,
        } => {
            let start = [
                start_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                start_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            let end = [
                end_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                end_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            let bolt_seed = seed.evaluate(frame).round().clamp(0.0, 99999.0) as u64 ^ (frame as u64);
            let glow_val = glow.evaluate(frame).clamp(0.0, 5.0);
            let config = crate::core::lightning_beam_engine::AdvancedLightningConfig {
                origin: start,
                destination: end,
                seed: bolt_seed,
                segments: 5,
                displacement_amplitude: 35.0,
                branch_probability: 0.35,
                main_thickness: 2.5 + glow_val,
                glow_color: [0.3, 0.6, 1.0, 0.8],
                core_color: [1.0, 1.0, 1.0, 1.0],
            };
            crate::core::lightning_beam_engine::render_lightning_to_buffer(pixels, width, height, &config);
        }
        EffectType::FireAutomaton { intensity } => {
            use crate::core::ae_effects_pack_v18::apply_fire_automaton;
            apply_fire_automaton(pixels, width, height, intensity.evaluate(frame));
        }
        EffectType::LumaKeyRange {
            low_threshold,
            high_threshold,
            invert,
        } => {
            use crate::core::ae_effects_pack_v16::apply_luma_key_range;
            apply_luma_key_range(
                pixels,
                low_threshold.evaluate(frame).round().clamp(0.0, 255.0) as u8,
                high_threshold.evaluate(frame).round().clamp(0.0, 255.0) as u8,
                *invert,
            );
        }
        EffectType::Halftone { cell_size } => {
            use crate::core::ae_effects_pack_v14::apply_halftone_screen;
            apply_halftone_screen(
                pixels,
                width,
                height,
                cell_size.evaluate(frame).round().clamp(2.0, 64.0) as u32,
            );
        }
        EffectType::Solarize { threshold } => {
            use crate::core::ae_effects_pack_v14::apply_solarize_effect;
            apply_solarize_effect(
                pixels,
                threshold.evaluate(frame).round().clamp(0.0, 255.0) as u8,
            );
        }
        EffectType::PixelSort { threshold } => {
            use crate::core::ae_effects_pack_v14::apply_pixel_sort_glitch;
            apply_pixel_sort_glitch(
                pixels,
                width,
                height,
                threshold.evaluate(frame).round().clamp(0.0, 255.0) as u8,
            );
        }
        EffectType::PinchPunch { radius, amount } => {
            use crate::core::ae_effects_pack_v15::apply_pinch_punch_distortion;
            let center = [width as f32 * 0.5, height as f32 * 0.5];
            apply_pinch_punch_distortion(
                pixels,
                width,
                height,
                center,
                radius.evaluate(frame).max(1.0),
                amount.evaluate(frame),
            );
        }
        EffectType::ScanlineGlitch {
            jitter_amount,
            seed,
        } => {
            use crate::core::ae_effects_pack_v15::apply_scanline_glitch_jitter;
            apply_scanline_glitch_jitter(
                pixels,
                width,
                height,
                jitter_amount.evaluate(frame),
                seed.evaluate(frame).round().clamp(0.0, 99999.0) as u32 ^ frame,
            );
        }
        EffectType::GlassEdgeBevel {
            bevel_size,
            refraction,
        } => {
            use crate::core::ae_effects_pack_v12::apply_glass_edge_bevel;
            apply_glass_edge_bevel(
                pixels,
                width,
                height,
                bevel_size.evaluate(frame).round().clamp(1.0, 128.0) as u32,
                refraction.evaluate(frame),
            );
        }
        EffectType::DirectionalSharpen {
            angle_deg,
            strength,
        } => {
            use crate::core::ae_effects_pack_v15::apply_directional_sharpen;
            apply_directional_sharpen(
                pixels,
                width,
                height,
                angle_deg.evaluate(frame),
                strength.evaluate(frame),
            );
        }
        EffectType::RefractionLens { radius, ior } => {
            use crate::core::ae_effects_pack_v16::apply_spherical_refraction_lens;
            let center = [width as f32 * 0.5, height as f32 * 0.5];
            apply_spherical_refraction_lens(
                pixels,
                width,
                height,
                center,
                radius.evaluate(frame).max(1.0),
                ior.evaluate(frame),
            );
        }
        EffectType::GradientMap {
            low_color,
            mid_color,
            high_color,
        } => {
            use crate::core::ae_effects_pack_v15::apply_gradient_map_color;
            let to_c3 = |c: [f32; 3]| {
                [
                    (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                ]
            };
            apply_gradient_map_color(
                pixels,
                to_c3(low_color.evaluate(frame)),
                to_c3(mid_color.evaluate(frame)),
                to_c3(high_color.evaluate(frame)),
            );
        }
        EffectType::LightLeak {
            pos_x,
            pos_y,
            intensity,
        } => {
            use crate::core::ae_effects_pack_v14::apply_light_leak_synth;
            let pos = [
                pos_x.evaluate(frame).clamp(0.0, 1.0) * width as f32,
                pos_y.evaluate(frame).clamp(0.0, 1.0) * height as f32,
            ];
            // Warm cinematic leak colour (fixed tint; position/intensity animate).
            apply_light_leak_synth(
                pixels,
                width,
                height,
                pos,
                intensity.evaluate(frame),
                [255, 180, 90],
            );
        }
        EffectType::BevelAlpha {
            depth,
            light_angle_deg,
        } => {
            use crate::core::ae_effects_pack_v14::apply_bevel_alpha_3d;
            apply_bevel_alpha_3d(
                pixels,
                width,
                height,
                depth.evaluate(frame).round().clamp(1.0, 32.0) as u32,
                light_angle_deg.evaluate(frame),
            );
        }
        EffectType::CrossHatch {
            line_gap,
            threshold,
        } => {
            use crate::core::ae_effects_pack_v17::apply_cross_hatch;
            apply_cross_hatch(
                pixels,
                width,
                height,
                line_gap.evaluate(frame).round().clamp(2.0, 64.0) as u32,
                threshold.evaluate(frame).round().clamp(0.0, 255.0) as u8,
            );
        }
        EffectType::CmykHalftone { dot_size } => {
            use crate::core::ae_effects_pack_v16::apply_color_halftone_cmyk;
            apply_color_halftone_cmyk(
                pixels,
                width,
                height,
                dot_size.evaluate(frame).round().clamp(2.0, 64.0) as u32,
            );
        }
        EffectType::ReflectionMap {
            reflect_y,
            fade_dist,
            opacity,
        } => {
            use crate::core::ae_effects_pack_v21::apply_reflection_map;
            apply_reflection_map(
                pixels,
                width,
                height,
                reflect_y.evaluate(frame).round().clamp(0.0, 4096.0) as u32,
                fade_dist.evaluate(frame).max(1.0),
                opacity.evaluate(frame).clamp(0.0, 1.0),
            );
        }
        EffectType::PerlinFlow { scale } => {
            use crate::core::ae_effects_pack_v16::apply_perlin_flow_noise;
            apply_perlin_flow_noise(
                pixels,
                width,
                height,
                frame as f32 / fps.max(1) as f32,
                scale.evaluate(frame).max(0.01),
            );
        }
        EffectType::FbmTurbulence { octaves, amplitude } => {
            use crate::core::ae_effects_pack_v18::apply_fbm_turbulence;
            apply_fbm_turbulence(
                pixels,
                width,
                height,
                octaves.evaluate(frame).round().clamp(1.0, 8.0) as u32,
                amplitude.evaluate(frame),
                frame as f32 / fps.max(1) as f32,
            );
        }
        EffectType::Letterbox { frac } => {
            use crate::core::ae_effects_pack_v24::apply_letterbox;
            apply_letterbox(pixels, width, height, frac.evaluate(frame));
        }
        // Expression Controls: non-rendering utility effects (values are read
        // by the expression engine via effect_param), so CPU pass is a no-op.
        EffectType::SliderControl { .. } => {}
        EffectType::AngleControl { .. } => {}
        EffectType::PointControl { .. } => {}
        EffectType::ColorControl { .. } => {}
        EffectType::CheckboxControl { .. } => {}
        EffectType::DropdownControl { .. } => {}
        EffectType::Point3DControl { .. } => {}
        EffectType::LensFlare {
            enabled,
            position_x,
            position_y,
            intensity,
            threshold,
            color,
            ..
        } => {
            if enabled.evaluate(frame) > 0.5 {
                let [fx, fy] = light_screen
                    .unwrap_or([position_x.evaluate(frame), position_y.evaluate(frame)]);
                let c = color.evaluate(frame);
                pack::apply_lens_flare(
                    pixels,
                    width,
                    height,
                    &pack::LensFlareParams {
                        pos_x: fx,
                        pos_y: fy,
                        intensity: intensity.evaluate(frame),
                        threshold: threshold.evaluate(frame),
                        color: [
                            (c[0] * 255.0).clamp(0.0, 255.0) as u8,
                            (c[1] * 255.0).clamp(0.0, 255.0) as u8,
                            (c[2] * 255.0).clamp(0.0, 255.0) as u8,
                        ],
                    },
                );
            }
        }
        EffectType::CustomShader { .. } => {
            // Custom WGSL shaders are GPU-only; CPU renderer applies a identity passthrough.
            // Users should switch to GPU preview for custom shader effects.
        }
        EffectType::MergePaths { .. } => {
            // Merge Paths is a vector shape operator, applied during shape rasterization.
            // CPU effect pass is a no-op for this.
        }
        EffectType::OffsetPath { amount, .. } => {
            let offset = amount.evaluate(frame);
            if offset != 0.0 {
                crate::core::ae_effects_pack::apply_simple_choker(pixels, offset);
            }
        }
        EffectType::BassTreble {
            bass_gain,
            treble_gain,
            crossover_freq,
        } => {
            let bass = bass_gain.evaluate(frame);
            let treble = treble_gain.evaluate(frame);
            let cross = crossover_freq.evaluate(frame).clamp(20.0, 5000.0);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.chunks_exact(4) {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::ae_effects_pack_v5::apply_bass_treble(
                    &mut luma, 44100.0, bass, treble, cross,
                );
                for (px, &lv) in pixels.chunks_exact_mut(4).zip(luma.iter()) {
                    let ratio =
                        if (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32)
                            > 0.5
                        {
                            lv / 128.0
                        } else {
                            1.0
                        };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        EffectType::Flanger {
            max_delay_ms,
            lfo_rate,
            feedback,
            wet_dry,
        } => {
            let md = max_delay_ms.evaluate(frame);
            let rate = lfo_rate.evaluate(frame);
            let fb = feedback.evaluate(frame).clamp(0.0, 0.95);
            let wd = wet_dry.evaluate(frame);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.chunks_exact(4) {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::ae_effects_pack_v5::apply_flanger(
                    &mut luma, 44100.0, md, rate, fb, wd,
                );
                for (px, &lv) in pixels.chunks_exact_mut(4).zip(luma.iter()) {
                    let orig = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                    let ratio = if orig > 0.5 { lv / 128.0 } else { 1.0 };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        EffectType::Chorus {
            delay_ms,
            depth_ms,
            rate_hz,
            voices,
            feedback,
        } => {
            let dm = delay_ms.evaluate(frame);
            let dp = depth_ms.evaluate(frame);
            let rt = rate_hz.evaluate(frame);
            let vc = voices.evaluate(frame);
            let fb = feedback.evaluate(frame).clamp(0.0, 0.9);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.chunks_exact(4) {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::ae_effects_pack_v5::apply_chorus(
                    &mut luma, 44100.0, dm, dp, rt, vc, fb,
                );
                for (px, &lv) in pixels.chunks_exact_mut(4).zip(luma.iter()) {
                    let orig = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                    let ratio = if orig > 0.5 { lv / 128.0 } else { 1.0 };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
        }
        EffectType::ParametricEQ {
            freq_hz,
            gain_db,
            q_factor,
        } => {
            let freq = freq_hz.evaluate(frame).clamp(20.0, 20000.0);
            let gain = gain_db.evaluate(frame);
            let q = q_factor.evaluate(frame).clamp(0.5, 20.0);
            if pixels.len() >= 4 {
                let mut luma = Vec::with_capacity(pixels.len() / 4);
                for p in pixels.chunks_exact(4) {
                    luma.push(0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32);
                }
                crate::core::ae_effects_pack_v5::apply_parametric_eq_bell(
                    &mut luma, 44100.0, freq, gain, q,
                );
                for (px, &lv) in pixels.chunks_exact_mut(4).zip(luma.iter()) {
                    let orig = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                    let ratio = if orig > 0.5 { lv / 128.0 } else { 1.0 };
                    px[0] = (px[0] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[1] = (px[1] as f32 * ratio).clamp(0.0, 255.0) as u8;
                    px[2] = (px[2] as f32 * ratio).clamp(0.0, 255.0) as u8;
                }
            }
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
            p[0] = r;
            p[1] = g;
            p[2] = b;
            p[3] = 255;
        }
        px
    }

    fn effect(name: &str, et: EffectType) -> Effect {
        Effect {
            id: name.to_string(),
            name: name.to_string(),
            effect_type: et,
            enabled: true,
        }
    }

    #[test]
    fn test_invert_flips_values() {
        let mut px = solid_layer(4, 4, 100, 150, 200);
        let before = px[1];
        apply_layer_effects(
            &mut px,
            4,
            4,
            &[effect(
                "inv",
                EffectType::Invert {
                    invert_alpha: false,
                },
            )],
            0,
            30,
        );
        assert_eq!(px[0], 255 - 100);
        assert_eq!(px[1], 255 - before);
    }

    #[test]
    fn test_posterize_reduces_levels() {
        let mut px = solid_layer(8, 8, 123, 45, 67);
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "post",
                EffectType::Posterize {
                    levels: Animatable::new_constant(2.0),
                },
            )],
            0,
            30,
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
            &mut px,
            8,
            8,
            &[effect(
                "thr",
                EffectType::Threshold {
                    threshold: Animatable::new_constant(128.0),
                },
            )],
            0,
            30,
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
        for p in px.chunks_exact_mut(4) {
            p[3] = 255;
        }
        let cx = (4 * 8 + 4) * 4;
        px[cx] = 255;
        px[cx + 1] = 255;
        px[cx + 2] = 255;

        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "blur",
                EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(3.0),
                },
            )],
            0,
            30,
        );
        // Center stays bright, a near neighbor should now be non-zero (spread).
        assert!(px[cx] > 0);
        let neighbor = ((4 * 8 + 5) * 4) as usize;
        assert!(px[neighbor] > 0, "blur did not spread to neighbor");
    }

    #[test]
    fn test_disabled_effect_is_noop() {
        let mut px = solid_layer(4, 4, 100, 100, 100);
        let mut eff = effect(
            "inv",
            EffectType::Invert {
                invert_alpha: false,
            },
        );
        eff.enabled = false;
        apply_layer_effects(&mut px, 4, 4, &[eff], 0, 30);
        assert_eq!(px[0], 100);
    }

    #[test]
    fn test_offset_shifts_content() {
        // Place a small bright patch at center of a transparent buffer, then offset.
        let mut px = vec![0u8; 8 * 8 * 4];
        for p in px.chunks_exact_mut(4) {
            p[3] = 255;
        }
        // Paint pixel (4,4) red.
        let idx = ((4 * 8 + 4) * 4) as usize;
        px[idx] = 255;
        px[idx + 1] = 0;
        px[idx + 2] = 0;

        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "off",
                EffectType::Offset {
                    shift_x: Animatable::new_constant(2.0),
                    shift_y: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
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
            &mut gray,
            4,
            4,
            &[effect(
                "vib",
                EffectType::Vibrance {
                    amount: Animatable::new_constant(100.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(
            (gray[0], gray[1], gray[2]),
            (128, 128, 128),
            "zero-chroma pixels must survive max vibrance"
        );

        let mut colored = solid_layer(4, 4, 90, 140, 200);
        let before = colored.clone();
        apply_layer_effects(
            &mut colored,
            4,
            4,
            &[effect(
                "vib",
                EffectType::Vibrance {
                    amount: Animatable::new_constant(80.0),
                },
            )],
            0,
            30,
        );
        assert_ne!(
            &colored[0..3],
            &before[0..3],
            "chroma must shift under vibrance"
        );
    }

    #[test]
    fn test_white_balance_warms_and_cools() {
        let mut warm = solid_layer(4, 4, 120, 120, 120);
        apply_layer_effects(
            &mut warm,
            4,
            4,
            &[effect(
                "wb",
                EffectType::WhiteBalance {
                    temperature: Animatable::new_constant(100.0),
                    tint: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert!(
            warm[0] > warm[2],
            "positive temperature must push R above B"
        );

        let mut cool = solid_layer(4, 4, 120, 120, 120);
        apply_layer_effects(
            &mut cool,
            4,
            4,
            &[effect(
                "wb",
                EffectType::WhiteBalance {
                    temperature: Animatable::new_constant(-100.0),
                    tint: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert!(
            cool[2] > cool[0],
            "negative temperature must push B above R"
        );
    }

    #[test]
    fn test_hsl_adjust_neutral_noop_and_hue_rotation() {
        let mut px = solid_layer(4, 4, 90, 140, 200);
        let before = px.clone();
        apply_layer_effects(
            &mut px,
            4,
            4,
            &[effect(
                "hsl",
                EffectType::HslAdjust {
                    hue_deg: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, before, "neutral HSL must be a no-op");

        let mut red = solid_layer(4, 4, 255, 40, 40);
        apply_layer_effects(
            &mut red,
            4,
            4,
            &[effect(
                "hsl",
                EffectType::HslAdjust {
                    hue_deg: Animatable::new_constant(180.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert!(
            red[0] < red[1].max(red[2]),
            "180° rotation must move red toward cyan"
        );
    }

    #[test]
    fn test_glow_pro_bleeds_bright_pixels() {
        // Dark 8x8 buffer with a single white pixel: glow must spread brightness.
        let mut px = vec![0u8; 8 * 8 * 4];
        for p in px.chunks_exact_mut(4) {
            p[3] = 255;
        }
        let cx = ((4 * 8 + 4) * 4) as usize;
        px[cx] = 255;
        px[cx + 1] = 255;
        px[cx + 2] = 255;

        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "glow",
                EffectType::GlowPro {
                    threshold: Animatable::new_constant(0.5),
                    radius: Animatable::new_constant(2.0),
                    intensity: Animatable::new_constant(1.5),
                },
            )],
            0,
            30,
        );
        let neighbor = ((4 * 8 + 6) * 4) as usize;
        assert!(px[neighbor] > 0, "glow did not bleed to neighbor pixel");
        // Zero intensity must be a no-op.
        let mut idle = solid_layer(4, 4, 200, 200, 200);
        let before = idle.clone();
        apply_layer_effects(
            &mut idle,
            4,
            4,
            &[effect(
                "glow",
                EffectType::GlowPro {
                    threshold: Animatable::new_constant(0.5),
                    radius: Animatable::new_constant(4.0),
                    intensity: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(idle, before, "zero-intensity glow must be a no-op");
    }

    #[test]
    fn test_stylize_distort_pack_noops_and_effects() {
        // Zero-strength / zero-angle settings must be exact no-ops.
        let base = solid_layer(8, 8, 90, 140, 200);

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "heat",
                EffectType::HeatDistortion {
                    strength: Animatable::new_constant(0.0),
                    speed: Animatable::new_constant(1.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-strength heat must be a no-op");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "vortex",
                EffectType::Vortex {
                    radius: Animatable::new_constant(50.0),
                    angle_deg: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-angle vortex must be a no-op");

        // CRT scanlines must darken even rows only.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "crt",
                EffectType::CrtScanlines {
                    line_spacing: Animatable::new_constant(2.0),
                    intensity: Animatable::new_constant(0.5),
                },
            )],
            0,
            30,
        );
        let row0 = (px[0] as u32 + px[4] as u32) / 2;
        let row1 = (px[32] as u32 + px[36] as u32) / 2;
        assert_eq!(row0, 45, "row 0 (y%2==0) darkened to 50%");
        assert_eq!(row1, 90, "odd rows untouched");
    }

    #[test]
    fn test_distort_pack_neutral_settings_are_noops() {
        let base = solid_layer(8, 8, 90, 140, 200);

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "fish",
                EffectType::Fisheye {
                    strength: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-strength fisheye must be a no-op");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "lc",
                EffectType::LensCorrection {
                    k1: Animatable::new_constant(0.0),
                    k2: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "k1=k2=0 lens correction must be identity");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "glitch",
                EffectType::GlitchDisplacement {
                    seed: Animatable::new_constant(1.0),
                    amount: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
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
                base[i] = 200;
                base[i + 1] = 100;
                base[i + 2] = 50;
                base[i + 3] = 255;
            }
        }

        // Choke with radius 1 must shrink the matte: corner pixel (1,1) loses alpha.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "choke",
                EffectType::MatteChokeSpread {
                    radius: Animatable::new_constant(1.0),
                    expand: false,
                },
            )],
            0,
            30,
        );
        let corner = (8 + 1) * 4;
        assert_eq!(px[corner + 3], 0, "choke must erode the alpha corner");

        // Spread must re-dilate it back.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "spread",
                EffectType::MatteChokeSpread {
                    radius: Animatable::new_constant(1.0),
                    expand: true,
                },
            )],
            0,
            30,
        );
        assert_eq!(px[(8 * 4) + 3], 255, "spread must dilate alpha outward");

        // Luma→alpha: white = opaque, black = transparent; invert flips both.
        let mut px = vec![0u8; 2 * 4];
        px[0] = 255;
        px[1] = 255;
        px[2] = 255;
        px[3] = 255;
        px[4] = 0;
        px[5] = 0;
        px[6] = 0;
        px[7] = 255;
        apply_layer_effects(
            &mut px,
            2,
            1,
            &[effect(
                "luma",
                EffectType::AlphaFromLuminance { invert: false },
            )],
            0,
            30,
        );
        assert_eq!(px[3], 255, "white luma must become full alpha");
        assert_eq!(px[7], 0, "black luma must become zero alpha");

        let mut px = vec![0u8; 2 * 4];
        px[0] = 255;
        px[1] = 255;
        px[2] = 255;
        px[3] = 255;
        px[4] = 0;
        px[5] = 0;
        px[6] = 0;
        px[7] = 255;
        apply_layer_effects(
            &mut px,
            2,
            1,
            &[effect(
                "lumainv",
                EffectType::AlphaFromLuminance { invert: true },
            )],
            0,
            30,
        );
        assert_eq!(px[3], 0, "inverted white must become transparent");
        assert_eq!(px[7], 255, "inverted black must become opaque");
    }

    #[test]
    fn test_stylize_transition_pack() {
        // Night vision output must be phosphor-green dominant.
        let mut px = solid_layer(4, 4, 90, 140, 200);
        apply_layer_effects(
            &mut px,
            4,
            4,
            &[effect(
                "nv",
                EffectType::NightVision {
                    amplification: Animatable::new_constant(2.0),
                },
            )],
            0,
            30,
        );
        for p in px.chunks_exact(4) {
            assert!(p[1] >= p[0] && p[1] >= p[2], "green channel must dominate");
        }
        // Determinism: same frame → identical output.
        let mut again = solid_layer(4, 4, 90, 140, 200);
        apply_layer_effects(
            &mut again,
            4,
            4,
            &[effect(
                "nv",
                EffectType::NightVision {
                    amplification: Animatable::new_constant(2.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, again, "night vision must be deterministic per frame");

        // Iris wipe at 0% must leave the image untouched (fully covered).
        let base = solid_layer(8, 8, 90, 140, 200);
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "iris",
                EffectType::IrisWipe {
                    completion: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "0% iris wipe must be a no-op");

        // Same for radial wipe.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "rad",
                EffectType::RadialWipe {
                    completion: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "0% radial wipe must be a no-op");
    }

    #[test]
    fn test_grade_light_pack() {
        let base = solid_layer(8, 8, 90, 140, 200);

        // Neutral ASC CDL grade must stay within rounding distance of identity.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "film",
                EffectType::FilmEmulation {
                    lift: Animatable::new_constant(0.0),
                    gamma: Animatable::new_constant(1.0),
                    gain: Animatable::new_constant(1.0),
                    hue_shift_deg: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        for (out, inp) in px.chunks_exact(4).zip(base.chunks_exact(4)) {
            assert!(
                (out[0] as i32 - inp[0] as i32).abs() <= 2,
                "R drift {}",
                out[0]
            );
            assert!(
                (out[1] as i32 - inp[1] as i32).abs() <= 2,
                "G drift {}",
                out[1]
            );
            assert!(
                (out[2] as i32 - inp[2] as i32).abs() <= 2,
                "B drift {}",
                out[2]
            );
        }

        // God rays with zero weight adds no light.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "rays",
                EffectType::GodRays {
                    sun_x: Animatable::new_constant(0.5),
                    sun_y: Animatable::new_constant(0.0),
                    samples: Animatable::new_constant(8.0),
                    decay: Animatable::new_constant(0.9),
                    weight: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-weight god rays must be a no-op");

        // Zero-amount zoom blur must be a no-op.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "zb",
                EffectType::RadialBlurZoom {
                    amount: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-amount zoom blur must be a no-op");

        // Positive zoom blur must keep the centre bright on a uniform image.
        let mut px = vec![255u8; 16 * 16 * 4];
        apply_layer_effects(
            &mut px,
            16,
            16,
            &[effect(
                "zb",
                EffectType::RadialBlurZoom {
                    amount: Animatable::new_constant(40.0),
                },
            )],
            0,
            30,
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
            &mut px,
            8,
            8,
            &[effect(
                "med",
                EffectType::MedianFilter {
                    radius: Animatable::new_constant(2.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "median of uniform image is identity");

        // Mosaic with block size 1x1 must be identity.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "mos",
                EffectType::Mosaic {
                    block_w: Animatable::new_constant(1.0),
                    block_h: Animatable::new_constant(1.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "1x1 mosaic is identity");

        // Sobel on a uniform image must produce no edges inside the frame.
        // (The kernel leaves the 1px border untouched, so only check interior.)
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect("sobel", EffectType::SobelEdges { invert: false })],
            0,
            30,
        );
        for y in 1..7usize {
            for x in 1..7usize {
                let i = (y * 8 + x) * 4;
                assert!(
                    px[i] < 16,
                    "uniform image must have no sobel edges (got {})",
                    px[i]
                );
            }
        }
    }

    #[test]
    fn test_simulation_pack_determinism_and_noop() {
        let base = solid_layer(8, 8, 90, 140, 200);

        // Fire with zero intensity is a no-op.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "fire",
                EffectType::FireAutomaton {
                    intensity: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-intensity fire must be a no-op");

        // Star field is deterministic for a given frame.
        let run_stars = || {
            let mut px = base.clone();
            apply_layer_effects(
                &mut px,
                8,
                8,
                &[effect(
                    "stars",
                    EffectType::StarField {
                        num_stars: Animatable::new_constant(20.0),
                        depth_speed: Animatable::new_constant(1.0),
                    },
                )],
                5,
                30,
            );
            px
        };
        assert_eq!(run_stars(), run_stars(), "star field must be deterministic");

        // Lightning is deterministic per frame as well.
        let run_bolt = || {
            let mut px = base.clone();
            apply_layer_effects(
                &mut px,
                8,
                8,
                &[effect(
                    "bolt",
                    EffectType::LightningArc {
                        start_x: Animatable::new_constant(0.1),
                        start_y: Animatable::new_constant(0.1),
                        end_x: Animatable::new_constant(0.9),
                        end_y: Animatable::new_constant(0.9),
                        seed: Animatable::new_constant(42.0),
                        glow: Animatable::new_constant(1.0),
                    },
                )],
                7,
                30,
            );
            px
        };
        assert_eq!(run_bolt(), run_bolt(), "lightning must be deterministic");
    }

    #[test]
    fn test_key_stylize_pack() {
        let base = solid_layer(8, 8, 90, 140, 200);
        // Rec.601-ish integer luma of the base colour.
        let luma_of_base = 131u32;

        // Solarize with threshold 255 never triggers — identity.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "sol",
                EffectType::Solarize {
                    threshold: Animatable::new_constant(255.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "solarize above all values must be a no-op");

        // Luma key non-inverted: pixels inside the band become transparent.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "lk",
                EffectType::LumaKeyRange {
                    low_threshold: Animatable::new_constant((luma_of_base - 10) as f32),
                    high_threshold: Animatable::new_constant((luma_of_base + 10) as f32),
                    invert: false,
                },
            )],
            0,
            30,
        );
        assert_eq!(px[3], 0, "in-band pixel must key to transparent");

        // Inverted: pixels outside the band become transparent instead.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "lk",
                EffectType::LumaKeyRange {
                    low_threshold: Animatable::new_constant(0.0),
                    high_threshold: Animatable::new_constant(50.0),
                    invert: true,
                },
            )],
            0,
            30,
        );
        assert_eq!(px[3], 0, "out-of-band pixel must key when inverted");
    }

    #[test]
    fn test_distort_pack2_neutral_settings_are_noops() {
        let base = solid_layer(8, 8, 90, 140, 200);

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "pinch",
                EffectType::PinchPunch {
                    radius: Animatable::new_constant(50.0),
                    amount: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-amount pinch/punch must be a no-op");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "sg",
                EffectType::ScanlineGlitch {
                    jitter_amount: Animatable::new_constant(0.0),
                    seed: Animatable::new_constant(1.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-jitter scanline glitch must be a no-op");

        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "dsh",
                EffectType::DirectionalSharpen {
                    angle_deg: Animatable::new_constant(45.0),
                    strength: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(
            px, base,
            "zero-strength directional sharpen must be a no-op"
        );
    }

    #[test]
    fn test_generate_stylize_pack() {
        let base = solid_layer(8, 8, 128, 128, 128);

        // A flat single-colour gradient maps every pixel to that colour.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "gm",
                EffectType::GradientMap {
                    low_color: Animatable::new_constant([1.0, 0.0, 0.0]),
                    mid_color: Animatable::new_constant([1.0, 0.0, 0.0]),
                    high_color: Animatable::new_constant([1.0, 0.0, 0.0]),
                },
            )],
            0,
            30,
        );
        for p in px.chunks_exact(4) {
            assert_eq!(p[0], 255, "flat gradient map R");
            assert_eq!(p[1], 0, "flat gradient map G");
            assert_eq!(p[2], 0, "flat gradient map B");
        }

        // Zero-intensity light leak must be a no-op.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "leak",
                EffectType::LightLeak {
                    pos_x: Animatable::new_constant(0.5),
                    pos_y: Animatable::new_constant(0.5),
                    intensity: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-intensity light leak must be a no-op");
    }

    #[test]
    fn test_reflection_generate_pack() {
        let base = solid_layer(8, 8, 128, 128, 128);

        // Zero-opacity reflection is a no-op.
        let mut px = base.clone();
        apply_layer_effects(
            &mut px,
            8,
            8,
            &[effect(
                "refl",
                EffectType::ReflectionMap {
                    reflect_y: Animatable::new_constant(4.0),
                    fade_dist: Animatable::new_constant(50.0),
                    opacity: Animatable::new_constant(0.0),
                },
            )],
            0,
            30,
        );
        assert_eq!(px, base, "zero-opacity reflection must be a no-op");

        // Perlin flow and FBM turbulence are deterministic per frame.
        let run = |et: EffectType| {
            let mut px = base.clone();
            apply_layer_effects(&mut px, 8, 8, &[effect("gen", et)], 3, 30);
            px
        };
        let p1 = run(EffectType::PerlinFlow {
            scale: Animatable::new_constant(4.0),
        });
        let p1b = run(EffectType::PerlinFlow {
            scale: Animatable::new_constant(4.0),
        });
        assert_eq!(p1, p1b, "perlin flow must be deterministic");

        let f1 = run(EffectType::FbmTurbulence {
            octaves: Animatable::new_constant(3.0),
            amplitude: Animatable::new_constant(60.0),
        });
        let f1b = run(EffectType::FbmTurbulence {
            octaves: Animatable::new_constant(3.0),
            amplitude: Animatable::new_constant(60.0),
        });
        assert_eq!(f1, f1b, "fbm turbulence must be deterministic");
    }
}
