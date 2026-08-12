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
            EffectType::ColorGradeLUT { .. } => "3D LUT Color Grading",
            EffectType::ColorSpaceConvert { .. } => "Color Space Converter",
            EffectType::FilmGrain { .. } => "Physical Film Grain",
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
            EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
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
            EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
                params.meshwarp_enabled = 1;
                params.corner_top_left = top_left.evaluate(frame);
                params.corner_top_right = top_right.evaluate(frame);
                params.corner_bottom_left = bottom_left.evaluate(frame);
                params.corner_bottom_right = bottom_right.evaluate(frame);
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
