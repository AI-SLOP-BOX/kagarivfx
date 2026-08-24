use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Effect, EffectType, ColorConversionMode};
use crate::core::property::Animatable;
use crate::ui::inspector_property::draw_property_ui;
use crate::ui::theme::colors;

#[allow(dead_code)]
pub struct EffectPreset {
    pub name: &'static str,
    pub button_label: &'static str,
    pub search_key: &'static str,
    pub id_prefix: &'static str,
    pub create_fn: fn(idx: usize) -> Effect,
}

pub fn get_all_effect_presets() -> &'static [EffectPreset] {
    &[
        EffectPreset {
            name: "Gaussian Blur",
            button_label: "+ Gaussian Blur",
            search_key: "gaussian blur",
            id_prefix: "blur",
            create_fn: |idx| Effect {
                id: format!("blur_{}", idx),
                name: "Gaussian Blur".to_string(),
                effect_type: EffectType::GaussianBlur { blur_radius: Animatable::new_constant(10.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Color Tint",
            button_label: "+ Color Tint",
            search_key: "color tint",
            id_prefix: "tint",
            create_fn: |idx| Effect {
                id: format!("tint_{}", idx),
                name: "Color Tint".to_string(),
                effect_type: EffectType::ColorTint {
                    color: Animatable::new_constant([1.0, 0.2, 0.2, 1.0]),
                    intensity: Animatable::new_constant(50.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Drop Shadow",
            button_label: "+ Drop Shadow",
            search_key: "drop shadow",
            id_prefix: "shadow",
            create_fn: |idx| Effect {
                id: format!("shadow_{}", idx),
                name: "Drop Shadow".to_string(),
                effect_type: EffectType::DropShadow {
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                    opacity: Animatable::new_constant(50.0),
                    direction: Animatable::new_constant(135.0),
                    distance: Animatable::new_constant(5.0),
                    softness: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chromatic Aberration",
            button_label: "+ Chromatic Aberration",
            search_key: "chromatic aberration",
            id_prefix: "ca",
            create_fn: |idx| Effect {
                id: format!("ca_{}", idx),
                name: "Chromatic Aberration".to_string(),
                effect_type: EffectType::ChromaticAberration {
                    shift_r: Animatable::new_constant(3.0),
                    shift_b: Animatable::new_constant(3.0),
                    edge_falloff: Animatable::new_constant(0.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Vignette",
            button_label: "+ Vignette",
            search_key: "vignette",
            id_prefix: "vignette",
            create_fn: |idx| Effect {
                id: format!("vignette_{}", idx),
                name: "Vignette".to_string(),
                effect_type: EffectType::Vignette {
                    intensity: Animatable::new_constant(60.0),
                    roundness: Animatable::new_constant(1.0),
                    feather: Animatable::new_constant(50.0),
                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Levels",
            button_label: "+ Levels (Gamma/Crush)",
            search_key: "levels",
            id_prefix: "levels",
            create_fn: |idx| Effect {
                id: format!("levels_{}", idx),
                name: "Levels".to_string(),
                effect_type: EffectType::Levels {
                    input_black: Animatable::new_constant(0.0),
                    input_white: Animatable::new_constant(1.0),
                    gamma: Animatable::new_constant(1.0),
                    output_black: Animatable::new_constant(0.0),
                    output_white: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Hue / Saturation",
            button_label: "+ Hue / Saturation",
            search_key: "hue / saturation",
            id_prefix: "huesat",
            create_fn: |idx| Effect {
                id: format!("huesat_{}", idx),
                name: "Hue / Saturation".to_string(),
                effect_type: EffectType::HueSaturation {
                    hue_shift: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glow",
            button_label: "+ Glow",
            search_key: "glow",
            id_prefix: "glow",
            create_fn: |idx| Effect {
                id: format!("glow_{}", idx),
                name: "Glow".to_string(),
                effect_type: EffectType::Glow {
                    threshold: Animatable::new_constant(50.0),
                    radius: Animatable::new_constant(20.0),
                    intensity: Animatable::new_constant(50.0),
                    color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Mesh Warp",
            button_label: "+ Mesh Warp (Grid)",
            search_key: "mesh warp",
            id_prefix: "meshwarp",
            create_fn: |idx| Effect {
                id: format!("meshwarp_{}", idx),
                name: "Mesh Warp".to_string(),
                effect_type: EffectType::MeshWarp {
                    top_left: Animatable::new_constant([0.0, 0.0]),
                    top_right: Animatable::new_constant([1920.0, 0.0]),
                    bottom_left: Animatable::new_constant([0.0, 1080.0]),
                    bottom_right: Animatable::new_constant([1920.0, 1080.0]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Cinematic 3D LUT",
            button_label: "+ Cinematic 3D LUT",
            search_key: "lut",
            id_prefix: "lut",
            create_fn: |idx| Effect {
                id: format!("lut_{}", idx),
                name: "Cinematic 3D LUT".to_string(),
                effect_type: EffectType::ColorGradeLUT {
                    lut_path: "alexa_logc_to_rec709.cube".to_string(),
                    intensity: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Color Space Converter",
            button_label: "+ Log Space Converter",
            search_key: "log space converter",
            id_prefix: "convert",
            create_fn: |idx| Effect {
                id: format!("convert_{}", idx),
                name: "Color Space Converter".to_string(),
                effect_type: EffectType::ColorSpaceConvert {
                    mode: ColorConversionMode::LogCToLinear,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Physical Film Grain",
            button_label: "+ Physical Film Grain",
            search_key: "film grain",
            id_prefix: "grain",
            create_fn: |idx| Effect {
                id: format!("grain_{}", idx),
                name: "Physical Film Grain".to_string(),
                effect_type: EffectType::FilmGrain {
                    intensity: Animatable::new_constant(15.0),
                    grain_size: 1.5,
                    color_film: true,
                },
                enabled: true,
            },
        },
        // ── CPU pixel-effect kernels (core::cpu_effects) ──
        EffectPreset {
            name: "Twirl",
            button_label: "+ Twirl",
            search_key: "twirl swirl rotate distort",
            id_prefix: "twirl",
            create_fn: |idx| Effect {
                id: format!("twirl_{}", idx),
                name: "Twirl".to_string(),
                effect_type: EffectType::Twirl {
                    angle: Animatable::new_constant(90.0),
                    radius: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Bulge",
            button_label: "+ Bulge",
            search_key: "bulge magnify lens distort",
            id_prefix: "bulge",
            create_fn: |idx| Effect {
                id: format!("bulge_{}", idx),
                name: "Bulge".to_string(),
                effect_type: EffectType::Bulge {
                    amount: Animatable::new_constant(30.0),
                    radius: Animatable::new_constant(100.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Posterize",
            button_label: "+ Posterize",
            search_key: "posterize levels poster levels",
            id_prefix: "posterize",
            create_fn: |idx| Effect {
                id: format!("posterize_{}", idx),
                name: "Posterize".to_string(),
                effect_type: EffectType::Posterize { levels: Animatable::new_constant(4.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Invert",
            button_label: "+ Invert",
            search_key: "invert negative reverse",
            id_prefix: "invert",
            create_fn: |idx| Effect {
                id: format!("invert_{}", idx),
                name: "Invert".to_string(),
                effect_type: EffectType::Invert { invert_alpha: false },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Offset",
            button_label: "+ Offset",
            search_key: "offset shift move translate",
            id_prefix: "offset",
            create_fn: |idx| Effect {
                id: format!("offset_{}", idx),
                name: "Offset".to_string(),
                effect_type: EffectType::Offset {
                    shift_x: Animatable::new_constant(0.0),
                    shift_y: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Directional Blur",
            button_label: "+ Directional Blur",
            search_key: "directional blur motion",
            id_prefix: "dirblur",
            create_fn: |idx| Effect {
                id: format!("dirblur_{}", idx),
                name: "Directional Blur".to_string(),
                effect_type: EffectType::DirectionalBlur {
                    angle: Animatable::new_constant(0.0),
                    length: Animatable::new_constant(10.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Radial Blur",
            button_label: "+ Radial Blur",
            search_key: "radial blur zoom spin",
            id_prefix: "radblur",
            create_fn: |idx| Effect {
                id: format!("radblur_{}", idx),
                name: "Radial Blur".to_string(),
                effect_type: EffectType::RadialBlur { amount: Animatable::new_constant(20.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Sharpen",
            button_label: "+ Sharpen",
            search_key: "sharpen unsharp contrast",
            id_prefix: "sharpen",
            create_fn: |idx| Effect {
                id: format!("sharpen_{}", idx),
                name: "Sharpen".to_string(),
                effect_type: EffectType::Sharpen { amount: Animatable::new_constant(50.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Threshold",
            button_label: "+ Threshold",
            search_key: "threshold cutoff binary",
            id_prefix: "threshold",
            create_fn: |idx| Effect {
                id: format!("threshold_{}", idx),
                name: "Threshold".to_string(),
                effect_type: EffectType::Threshold { threshold: Animatable::new_constant(128.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Linear Wipe",
            button_label: "+ Linear Wipe",
            search_key: "linear wipe transition reveal",
            id_prefix: "linwipe",
            create_fn: |idx| Effect {
                id: format!("linwipe_{}", idx),
                name: "Linear Wipe".to_string(),
                effect_type: EffectType::LinearWipe {
                    completion: Animatable::new_constant(0.0),
                    angle: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Simple Choker",
            button_label: "+ Simple Choker",
            search_key: "simple choker matte shrink grow",
            id_prefix: "choker",
            create_fn: |idx| Effect {
                id: format!("choker_{}", idx),
                name: "Simple Choker".to_string(),
                effect_type: EffectType::SimpleChoker { choke_amount: Animatable::new_constant(0.0) },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chroma Key",
            button_label: "+ Chroma Key",
            search_key: "chroma key green screen keying",
            id_prefix: "chroma",
            create_fn: |idx| Effect {
                id: format!("chroma_{}", idx),
                name: "Chroma Key".to_string(),
                effect_type: EffectType::ChromaKey {
                    screen_color: Animatable::new_constant([0.0, 1.0, 0.0]),
                    screen_gain: Animatable::new_constant(1.0),
                    clip_black: Animatable::new_constant(0.0),
                    clip_white: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Spherize",
            button_label: "+ Spherize",
            search_key: "spherize sphere lens distortion cc sphere",
            id_prefix: "spherize",
            create_fn: |idx| Effect {
                id: format!("spherize_{}", idx),
                name: "Spherize".to_string(),
                effect_type: EffectType::Spherize {
                    radius: Animatable::new_constant(100.0),
                    refractive_index: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Turbulent Displace",
            button_label: "+ Turbulent Displace",
            search_key: "turbulent displace noise turbulence warp",
            id_prefix: "turbdisp",
            create_fn: |idx| Effect {
                id: format!("turbdisp_{}", idx),
                name: "Turbulent Displace".to_string(),
                effect_type: EffectType::TurbulentDisplace {
                    amount: Animatable::new_constant(25.0),
                    size: Animatable::new_constant(100.0),
                    evolution: Animatable::new_constant(0.0),
                    complexity: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Colorama",
            button_label: "+ Colorama",
            search_key: "colorama color cycle gradient rainbow",
            id_prefix: "colorama",
            create_fn: |idx| Effect {
                id: format!("colorama_{}", idx),
                name: "Colorama".to_string(),
                effect_type: EffectType::Colorama {
                    preset_index: Animatable::new_constant(0.0),
                    cycle_phase: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        // ── New AE-standard effects ──
        EffectPreset {
            name: "Fractal Noise",
            button_label: "+ Fractal Noise",
            search_key: "fractal noise turbulence procedural texture",
            id_prefix: "fn",
            create_fn: |idx| Effect {
                id: format!("fn_{}", idx),
                name: "Fractal Noise".to_string(),
                effect_type: EffectType::FractalNoise {
                    fractal_type: Animatable::new_constant(0.0),
                    contrast: Animatable::new_constant(100.0),
                    brightness: Animatable::new_constant(0.0),
                    complexity: Animatable::new_constant(5.0),
                    evolution: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Curves",
            button_label: "+ Curves",
            search_key: "curves color correction tone",
            id_prefix: "curves",
            create_fn: |idx| Effect {
                id: format!("curves_{}", idx),
                name: "Curves".to_string(),
                effect_type: EffectType::Curves {
                    channel: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Displacement Map",
            button_label: "+ Displacement Map",
            search_key: "displacement map distortion warp",
            id_prefix: "dispmap",
            create_fn: |idx| Effect {
                id: format!("dispmap_{}", idx),
                name: "Displacement Map".to_string(),
                effect_type: EffectType::DisplacementMap {
                    source_layer: Animatable::new_constant(0.0),
                    max_horizontal: Animatable::new_constant(50.0),
                    max_vertical: Animatable::new_constant(50.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Compound Blur",
            button_label: "+ Compound Blur",
            search_key: "compound blur variable map",
            id_prefix: "cblur",
            create_fn: |idx| Effect {
                id: format!("cblur_{}", idx),
                name: "Compound Blur".to_string(),
                effect_type: EffectType::CompoundBlur {
                    source_layer: Animatable::new_constant(0.0),
                    max_blur: Animatable::new_constant(20.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Minimax",
            button_label: "+ Minimax",
            search_key: "minimax dilate erode matte",
            id_prefix: "mmx",
            create_fn: |idx| Effect {
                id: format!("mmx_{}", idx),
                name: "Minimax".to_string(),
                effect_type: EffectType::Minimax {
                    operation: Animatable::new_constant(0.0),
                    radius: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Shift Channels",
            button_label: "+ Shift Channels",
            search_key: "shift channels swap remap rgba",
            id_prefix: "shiftch",
            create_fn: |idx| Effect {
                id: format!("shiftch_{}", idx),
                name: "Shift Channels".to_string(),
                effect_type: EffectType::ShiftChannels {
                    take_red: Animatable::new_constant(0.0),
                    take_green: Animatable::new_constant(1.0),
                    take_blue: Animatable::new_constant(2.0),
                    take_alpha: Animatable::new_constant(3.0),
                },
                enabled: true,
            },
        },
        // ── Lumetri Basic Correction ──
        // NOTE: names must match the constants in src/ui/lumetri_color.rs so the
        // live sliders in that panel keep driving these same effects.
        EffectPreset {
            name: "Vibrance",
            button_label: "+ Vibrance",
            search_key: "vibrance saturation skin tone lumetri basic correction",
            id_prefix: "lum_vib",
            create_fn: |idx| Effect {
                id: format!("lum_vib_{}", idx),
                name: "Lumetri Vibrance".to_string(),
                effect_type: EffectType::Vibrance {
                    amount: Animatable::new_constant(25.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "White Balance",
            button_label: "+ White Balance",
            search_key: "white balance temperature tint kelvin lumetri basic correction",
            id_prefix: "lum_wb",
            create_fn: |idx| Effect {
                id: format!("lum_wb_{}", idx),
                name: "Lumetri White Balance".to_string(),
                effect_type: EffectType::WhiteBalance {
                    temperature: Animatable::new_constant(0.0),
                    tint: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "HSL Adjust",
            button_label: "+ HSL Adjust",
            search_key: "hsl hue saturation lightness secondary lumetri basic correction",
            id_prefix: "lum_hsl",
            create_fn: |idx| Effect {
                id: format!("lum_hsl_{}", idx),
                name: "Lumetri HSL Adjust".to_string(),
                effect_type: EffectType::HslAdjust {
                    hue_deg: Animatable::new_constant(0.0),
                    saturation: Animatable::new_constant(0.0),
                    lightness: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glow",
            button_label: "+ Glow (Pro)",
            search_key: "glow bloom threshold bleed stylize light",
            id_prefix: "glowpro",
            create_fn: |idx| Effect {
                id: format!("glowpro_{}", idx),
                name: "Glow".to_string(),
                effect_type: EffectType::GlowPro {
                    threshold: Animatable::new_constant(0.7),
                    radius: Animatable::new_constant(4.0),
                    intensity: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "CRT Scanlines",
            button_label: "+ CRT Scanlines",
            search_key: "crt scanlines tv retro vhs screen",
            id_prefix: "crt",
            create_fn: |idx| Effect {
                id: format!("crt_{}", idx),
                name: "CRT Scanlines".to_string(),
                effect_type: EffectType::CrtScanlines {
                    line_spacing: Animatable::new_constant(3.0),
                    intensity: Animatable::new_constant(0.4),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Vortex Distortion",
            button_label: "+ Vortex",
            search_key: "vortex spiral swirl twist distort",
            id_prefix: "vortex",
            create_fn: |idx| Effect {
                id: format!("vortex_{}", idx),
                name: "Vortex Distortion".to_string(),
                effect_type: EffectType::Vortex {
                    radius: Animatable::new_constant(300.0),
                    angle_deg: Animatable::new_constant(120.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Heat Distortion",
            button_label: "+ Heat Distortion",
            search_key: "heat haze shimmer thermal turbulence fire",
            id_prefix: "heat",
            create_fn: |idx| Effect {
                id: format!("heat_{}", idx),
                name: "Heat Distortion".to_string(),
                effect_type: EffectType::HeatDistortion {
                    strength: Animatable::new_constant(6.0),
                    speed: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Rain Ripples",
            button_label: "+ Rain Ripples",
            search_key: "rain water drop ripple wave puddle",
            id_prefix: "rainrip",
            create_fn: |idx| Effect {
                id: format!("rainrip_{}", idx),
                name: "Rain Ripples".to_string(),
                effect_type: EffectType::RainRipples {
                    drop_count: Animatable::new_constant(12.0),
                    wave_strength: Animatable::new_constant(3.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Fisheye",
            button_label: "+ Fisheye",
            search_key: "fisheye lens bulge round gopro distort",
            id_prefix: "fisheye",
            create_fn: |idx| Effect {
                id: format!("fisheye_{}", idx),
                name: "Fisheye".to_string(),
                effect_type: EffectType::Fisheye {
                    strength: Animatable::new_constant(0.35),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Lens Correction",
            button_label: "+ Lens Correction",
            search_key: "lens correction barrel pincushion camera fix k1 k2",
            id_prefix: "lenscorr",
            create_fn: |idx| Effect {
                id: format!("lenscorr_{}", idx),
                name: "Lens Correction".to_string(),
                effect_type: EffectType::LensCorrection {
                    k1: Animatable::new_constant(0.0),
                    k2: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glitch Displacement",
            button_label: "+ Glitch",
            search_key: "glitch digital block displacement datamosh vhs error",
            id_prefix: "glitch",
            create_fn: |idx| Effect {
                id: format!("glitch_{}", idx),
                name: "Glitch Displacement".to_string(),
                effect_type: EffectType::GlitchDisplacement {
                    seed: Animatable::new_constant(7.0),
                    amount: Animatable::new_constant(2.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Matte Choke / Spread",
            button_label: "+ Matte Choke",
            search_key: "matte choke spread alpha erode dilate mask edge shrink grow",
            id_prefix: "mchoke",
            create_fn: |idx| Effect {
                id: format!("mchoke_{}", idx),
                name: "Matte Choke / Spread".to_string(),
                effect_type: EffectType::MatteChokeSpread {
                    radius: Animatable::new_constant(3.0),
                    expand: false,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Alpha Feather",
            button_label: "+ Alpha Feather",
            search_key: "alpha feather soft edge blur mask smooth",
            id_prefix: "afeather",
            create_fn: |idx| Effect {
                id: format!("afeather_{}", idx),
                name: "Alpha Feather".to_string(),
                effect_type: EffectType::AlphaFeather {
                    radius: Animatable::new_constant(4.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Alpha From Luminance",
            button_label: "+ Alpha From Luma",
            search_key: "alpha from luminance luma matte transparency set",
            id_prefix: "aluma",
            create_fn: |idx| Effect {
                id: format!("aluma_{}", idx),
                name: "Alpha From Luminance".to_string(),
                effect_type: EffectType::AlphaFromLuminance { invert: false },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Night Vision",
            button_label: "+ Night Vision",
            search_key: "night vision green phosphor goggles surveillance",
            id_prefix: "nv",
            create_fn: |idx| Effect {
                id: format!("nv_{}", idx),
                name: "Night Vision".to_string(),
                effect_type: EffectType::NightVision {
                    amplification: Animatable::new_constant(2.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Iris Wipe",
            button_label: "+ Iris Wipe",
            search_key: "iris circle wipe transition reveal round",
            id_prefix: "irisw",
            create_fn: |idx| Effect {
                id: format!("irisw_{}", idx),
                name: "Iris Wipe".to_string(),
                effect_type: EffectType::IrisWipe {
                    completion: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Radial Wipe",
            button_label: "+ Radial Wipe",
            search_key: "radial sweep wipe transition clock reveal",
            id_prefix: "radw",
            create_fn: |idx| Effect {
                id: format!("radw_{}", idx),
                name: "Radial Wipe".to_string(),
                effect_type: EffectType::RadialWipe {
                    completion: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Film Emulation",
            button_label: "+ Film Emulation",
            search_key: "film emulation kodak fuji cdl lift gamma gain grade look",
            id_prefix: "filmem",
            create_fn: |idx| Effect {
                id: format!("filmem_{}", idx),
                name: "Film Emulation".to_string(),
                effect_type: EffectType::FilmEmulation {
                    lift: Animatable::new_constant(0.0),
                    gamma: Animatable::new_constant(1.0),
                    gain: Animatable::new_constant(1.0),
                    hue_shift_deg: Animatable::new_constant(0.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "God Rays",
            button_label: "+ God Rays",
            search_key: "god rays volumetric light sun scattering beams",
            id_prefix: "godrays",
            create_fn: |idx| Effect {
                id: format!("godrays_{}", idx),
                name: "God Rays".to_string(),
                effect_type: EffectType::GodRays {
                    sun_x: Animatable::new_constant(0.5),
                    sun_y: Animatable::new_constant(0.0),
                    samples: Animatable::new_constant(24.0),
                    decay: Animatable::new_constant(0.95),
                    weight: Animatable::new_constant(0.6),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Zoom Blur",
            button_label: "+ Zoom Blur",
            search_key: "zoom blur radial motion speed warp center",
            id_prefix: "zblur",
            create_fn: |idx| Effect {
                id: format!("zblur_{}", idx),
                name: "Zoom Blur".to_string(),
                effect_type: EffectType::RadialBlurZoom {
                    amount: Animatable::new_constant(20.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Median Filter",
            button_label: "+ Median Filter",
            search_key: "median filter noise removal salt pepper denoise",
            id_prefix: "medf",
            create_fn: |idx| Effect {
                id: format!("medf_{}", idx),
                name: "Median Filter".to_string(),
                effect_type: EffectType::MedianFilter {
                    radius: Animatable::new_constant(2.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Sobel Edges",
            button_label: "+ Sobel Edges",
            search_key: "sobel edge detection outline sketch line",
            id_prefix: "sobel",
            create_fn: |idx| Effect {
                id: format!("sobel_{}", idx),
                name: "Sobel Edges".to_string(),
                effect_type: EffectType::SobelEdges { invert: false },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Mosaic",
            button_label: "+ Mosaic",
            search_key: "mosaic pixelate block censor blur squares",
            id_prefix: "mosaic",
            create_fn: |idx| Effect {
                id: format!("mosaic_{}", idx),
                name: "Mosaic".to_string(),
                effect_type: EffectType::Mosaic {
                    block_w: Animatable::new_constant(10.0),
                    block_h: Animatable::new_constant(10.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Tilt Shift",
            button_label: "+ Tilt Shift",
            search_key: "tilt shift miniature focus depth of field diorama",
            id_prefix: "tiltsh",
            create_fn: |idx| Effect {
                id: format!("tiltsh_{}", idx),
                name: "Tilt Shift".to_string(),
                effect_type: EffectType::TiltShift {
                    focus_y: Animatable::new_constant(0.5),
                    focus_height: Animatable::new_constant(0.3),
                    max_blur: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Emboss",
            button_label: "+ Emboss",
            search_key: "emboss relief 3d surface engrave",
            id_prefix: "emboss",
            create_fn: |idx| Effect {
                id: format!("emboss_{}", idx),
                name: "Emboss".to_string(),
                effect_type: EffectType::Emboss {
                    angle_deg: Animatable::new_constant(45.0),
                    depth: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Star Field",
            button_label: "+ Star Field",
            search_key: "star field space stars parallax night sky generate",
            id_prefix: "stars",
            create_fn: |idx| Effect {
                id: format!("stars_{}", idx),
                name: "Star Field".to_string(),
                effect_type: EffectType::StarField {
                    num_stars: Animatable::new_constant(150.0),
                    depth_speed: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Lightning",
            button_label: "+ Lightning",
            search_key: "lightning bolt electric storm arc thunder",
            id_prefix: "bolt",
            create_fn: |idx| Effect {
                id: format!("bolt_{}", idx),
                name: "Lightning".to_string(),
                effect_type: EffectType::LightningArc {
                    start_x: Animatable::new_constant(0.2),
                    start_y: Animatable::new_constant(0.0),
                    end_x: Animatable::new_constant(0.7),
                    end_y: Animatable::new_constant(1.0),
                    seed: Animatable::new_constant(3.0),
                    glow: Animatable::new_constant(1.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Fire",
            button_label: "+ Fire",
            search_key: "fire flame burn cellular combustion heat",
            id_prefix: "firefx",
            create_fn: |idx| Effect {
                id: format!("firefx_{}", idx),
                name: "Fire".to_string(),
                effect_type: EffectType::FireAutomaton {
                    intensity: Animatable::new_constant(2.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Luma Key Range",
            button_label: "+ Luma Key Range",
            search_key: "luma key range luminance matte extract transparency",
            id_prefix: "lumakey",
            create_fn: |idx| Effect {
                id: format!("lumakey_{}", idx),
                name: "Luma Key Range".to_string(),
                effect_type: EffectType::LumaKeyRange {
                    low_threshold: Animatable::new_constant(40.0),
                    high_threshold: Animatable::new_constant(220.0),
                    invert: false,
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Halftone",
            button_label: "+ Halftone",
            search_key: "halftone dot screen print newspaper comic",
            id_prefix: "half",
            create_fn: |idx| Effect {
                id: format!("half_{}", idx),
                name: "Halftone".to_string(),
                effect_type: EffectType::Halftone {
                    cell_size: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Solarize",
            button_label: "+ Solarize",
            search_key: "solarize invert threshold sabattier negative",
            id_prefix: "sol",
            create_fn: |idx| Effect {
                id: format!("sol_{}", idx),
                name: "Solarize".to_string(),
                effect_type: EffectType::Solarize {
                    threshold: Animatable::new_constant(128.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Pixel Sort",
            button_label: "+ Pixel Sort",
            search_key: "pixel sort glitch columns datamosh aesthetic",
            id_prefix: "pixsort",
            create_fn: |idx| Effect {
                id: format!("pixsort_{}", idx),
                name: "Pixel Sort".to_string(),
                effect_type: EffectType::PixelSort {
                    threshold: Animatable::new_constant(140.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Pinch / Punch",
            button_label: "+ Pinch / Punch",
            search_key: "pinch punch polar distort squeeze bubble",
            id_prefix: "pinch",
            create_fn: |idx| Effect {
                id: format!("pinch_{}", idx),
                name: "Pinch / Punch".to_string(),
                effect_type: EffectType::PinchPunch {
                    radius: Animatable::new_constant(300.0),
                    amount: Animatable::new_constant(0.8),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Scanline Glitch",
            button_label: "+ Scanline Glitch",
            search_key: "scanline glitch jitter vhs signal noise rows",
            id_prefix: "sglitch",
            create_fn: |idx| Effect {
                id: format!("sglitch_{}", idx),
                name: "Scanline Glitch".to_string(),
                effect_type: EffectType::ScanlineGlitch {
                    jitter_amount: Animatable::new_constant(8.0),
                    seed: Animatable::new_constant(5.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Glass Edge Bevel",
            button_label: "+ Glass Edge Bevel",
            search_key: "glass edge bevel refraction specular frame border",
            id_prefix: "gbevel",
            create_fn: |idx| Effect {
                id: format!("gbevel_{}", idx),
                name: "Glass Edge Bevel".to_string(),
                effect_type: EffectType::GlassEdgeBevel {
                    bevel_size: Animatable::new_constant(12.0),
                    refraction: Animatable::new_constant(0.6),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Directional Sharpen",
            button_label: "+ Directional Sharpen",
            search_key: "directional sharpen angle motion enhance detail",
            id_prefix: "dsharp",
            create_fn: |idx| Effect {
                id: format!("dsharp_{}", idx),
                name: "Directional Sharpen".to_string(),
                effect_type: EffectType::DirectionalSharpen {
                    angle_deg: Animatable::new_constant(45.0),
                    strength: Animatable::new_constant(1.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Refraction Lens",
            button_label: "+ Refraction Lens",
            search_key: "refraction lens glass ball sphere ior crystal ball",
            id_prefix: "refrac",
            create_fn: |idx| Effect {
                id: format!("refrac_{}", idx),
                name: "Refraction Lens".to_string(),
                effect_type: EffectType::RefractionLens {
                    radius: Animatable::new_constant(150.0),
                    ior: Animatable::new_constant(1.4),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Gradient Map",
            button_label: "+ Gradient Map",
            search_key: "gradient map shadow mid high ramp duotone tritone colorize",
            id_prefix: "gradmap",
            create_fn: |idx| Effect {
                id: format!("gradmap_{}", idx),
                name: "Gradient Map".to_string(),
                effect_type: EffectType::GradientMap {
                    low_color: Animatable::new_constant([0.1, 0.1, 0.3]),
                    mid_color: Animatable::new_constant([0.6, 0.3, 0.4]),
                    high_color: Animatable::new_constant([1.0, 0.9, 0.7]),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Light Leak",
            button_label: "+ Light Leak",
            search_key: "light leak flare warm cinematic vintage overlay glow",
            id_prefix: "leak",
            create_fn: |idx| Effect {
                id: format!("leak_{}", idx),
                name: "Light Leak".to_string(),
                effect_type: EffectType::LightLeak {
                    pos_x: Animatable::new_constant(0.85),
                    pos_y: Animatable::new_constant(0.15),
                    intensity: Animatable::new_constant(1.2),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Bevel Alpha 3D",
            button_label: "+ Bevel Alpha 3D",
            search_key: "bevel alpha 3d inner contour highlight depth emboss edge",
            id_prefix: "balpha",
            create_fn: |idx| Effect {
                id: format!("balpha_{}", idx),
                name: "Bevel Alpha 3D".to_string(),
                effect_type: EffectType::BevelAlpha {
                    depth: Animatable::new_constant(6.0),
                    light_angle_deg: Animatable::new_constant(135.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Cross Hatch",
            button_label: "+ Cross Hatch",
            search_key: "cross hatch ink sketch drawing pen lines comic",
            id_prefix: "xhatch",
            create_fn: |idx| Effect {
                id: format!("xhatch_{}", idx),
                name: "Cross Hatch".to_string(),
                effect_type: EffectType::CrossHatch {
                    line_gap: Animatable::new_constant(8.0),
                    threshold: Animatable::new_constant(140.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "CMYK Halftone",
            button_label: "+ CMYK Halftone",
            search_key: "cmyk halftone print newspaper dots offset press",
            id_prefix: "cmhk",
            create_fn: |idx| Effect {
                id: format!("cmhk_{}", idx),
                name: "CMYK Halftone".to_string(),
                effect_type: EffectType::CmykHalftone {
                    dot_size: Animatable::new_constant(6.0),
                },
                enabled: true,
            },
        },
    ]
}

pub fn draw_effect_type_ui(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::GaussianBlur { blur_radius } => {
            let val_before = blur_radius.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Blur Radius", blur_radius, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if val_before != *blur_radius {
                *project_changed = true;
            }
        }
        EffectType::ColorTint { color, intensity } => {
            let color_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Tint Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) {
                *next_frame = Some(nf);
            }
            if color_before != *color {
                *project_changed = true;
            }

            let intensity_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if intensity_before != *intensity {
                *project_changed = true;
            }
        }
        EffectType::DropShadow { color, opacity, direction, distance, softness } => {
            let color_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Shadow Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) {
                *next_frame = Some(nf);
            }
            if color_before != *color {
                *project_changed = true;
            }

            let opacity_before = opacity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Opacity", opacity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if opacity_before != *opacity {
                *project_changed = true;
            }

            let direction_before = direction.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Direction", direction, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
            }) {
                *next_frame = Some(nf);
            }
            if direction_before != *direction {
                *project_changed = true;
            }

            let distance_before = distance.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Distance", distance, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix(" px"));
            }) {
                *next_frame = Some(nf);
            }
            if distance_before != *distance {
                *project_changed = true;
            }

            let softness_before = softness.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Softness", softness, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) {
                *next_frame = Some(nf);
            }
            if softness_before != *softness {
                *project_changed = true;
            }
        }
        EffectType::ChromaticAberration { shift_r, shift_b, edge_falloff } => {
            let shift_r_before = shift_r.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Red Shift", shift_r, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
            }) { *next_frame = Some(nf); }
            if shift_r_before != *shift_r { *project_changed = true; }

            let shift_b_before = shift_b.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Blue Shift", shift_b, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=20.0).suffix(" px"));
            }) { *next_frame = Some(nf); }
            if shift_b_before != *shift_b { *project_changed = true; }

            let ef_before = edge_falloff.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Edge Falloff", edge_falloff, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ef_before != *edge_falloff { *project_changed = true; }
        }
        EffectType::Vignette { intensity, roundness, feather, color } => {
            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }

            let r_before = roundness.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Roundness", roundness, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if r_before != *roundness { *project_changed = true; }

            let f_before = feather.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Feather", feather, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if f_before != *feather { *project_changed = true; }

            let c_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) { *next_frame = Some(nf); }
            if c_before != *color { *project_changed = true; }
        }
        EffectType::Levels { input_black, input_white, gamma, output_black, output_white } => {
            let ib_before = input_black.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Input Black", input_black, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ib_before != *input_black { *project_changed = true; }

            let iw_before = input_white.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Input White", input_white, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if iw_before != *input_white { *project_changed = true; }

            let g_before = gamma.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Gamma", gamma, |ui, val| {
                ui.add(egui::Slider::new(val, 0.1..=10.0));
            }) { *next_frame = Some(nf); }
            if g_before != *gamma { *project_changed = true; }

            let ob_before = output_black.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Output Black", output_black, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ob_before != *output_black { *project_changed = true; }

            let ow_before = output_white.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Output White", output_white, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=1.0));
            }) { *next_frame = Some(nf); }
            if ow_before != *output_white { *project_changed = true; }
        }
        EffectType::HueSaturation { hue_shift, saturation, lightness } => {
            let h_before = hue_shift.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Hue Shift", hue_shift, |ui, val| {
                ui.add(egui::Slider::new(val, -180.0..=180.0).suffix("°"));
            }) { *next_frame = Some(nf); }
            if h_before != *hue_shift { *project_changed = true; }

            let s_before = saturation.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Saturation", saturation, |ui, val| {
                ui.add(egui::Slider::new(val, -100.0..=100.0));
            }) { *next_frame = Some(nf); }
            if s_before != *saturation { *project_changed = true; }

            let l_before = lightness.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Lightness", lightness, |ui, val| {
                ui.add(egui::Slider::new(val, -100.0..=100.0));
            }) { *next_frame = Some(nf); }
            if l_before != *lightness { *project_changed = true; }
        }
        EffectType::Glow { threshold, radius, intensity, color } => {
            let t_before = threshold.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Threshold", threshold, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if t_before != *threshold { *project_changed = true; }

            let r_before = radius.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Radius", radius, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=200.0).suffix(" px"));
            }) { *next_frame = Some(nf); }
            if r_before != *radius { *project_changed = true; }

            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }

            let c_before = color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Glow Color", color, |ui, val| {
                ui.color_edit_button_rgba_unmultiplied(val);
            }) { *next_frame = Some(nf); }
            if c_before != *color { *project_changed = true; }
        }
        EffectType::MotionBlur { shutter_angle, samples } => {
            let sa_before = shutter_angle.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Shutter Angle", shutter_angle, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=360.0).suffix("°"));
            }) { *next_frame = Some(nf); }
            if sa_before != *shutter_angle { *project_changed = true; }

            ui.horizontal(|ui| {
                ui.label("Samples:");
                let before_s = *samples;
                ui.add(egui::DragValue::new(samples).range(2..=16));
                if before_s != *samples { *project_changed = true; }
            });
        }
        EffectType::MeshWarp { top_left, top_right, bottom_left, bottom_right } => {
            let tl_before = top_left.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Top Left Corner", top_left, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if tl_before != *top_left { *project_changed = true; }

            let tr_before = top_right.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Top Right Corner", top_right, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if tr_before != *top_right { *project_changed = true; }

            let bl_before = bottom_left.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Bottom Left Corner", bottom_left, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if bl_before != *bottom_left { *project_changed = true; }

            let br_before = bottom_right.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Bottom Right Corner", bottom_right, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            if br_before != *bottom_right { *project_changed = true; }
        }
        EffectType::ColorGradeLUT { lut_path, intensity } => {
            ui.horizontal(|ui| {
                ui.label("LUT Path:");
                let path_before = lut_path.clone();
                ui.text_edit_singleline(lut_path);
                if path_before != *lut_path { *project_changed = true; }
            });

            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }
        }
        EffectType::ColorSpaceConvert { mode } => {
            let mode_before = *mode;
            egui::ComboBox::from_id_salt(format!("convert_combo_{:?}", ui.next_auto_id()))
                .selected_text(format!("{:?}", mode))
                .show_ui(ui, |ui| {
                    for m in [
                        ColorConversionMode::LogCToLinear,
                        ColorConversionMode::LinearToLogC,
                        ColorConversionMode::SLog3ToLinear,
                        ColorConversionMode::LinearToSLog3,
                    ] {
                        ui.selectable_value(mode, m, format!("{:?}", m));
                    }
                });
            if mode_before != *mode { *project_changed = true; }
        }
        EffectType::FilmGrain { intensity, grain_size, color_film } => {
            let i_before = intensity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Grain Intensity", intensity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
            }) { *next_frame = Some(nf); }
            if i_before != *intensity { *project_changed = true; }

            ui.horizontal(|ui| {
                ui.label("Grain Size:");
                let size_before = *grain_size;
                ui.add(egui::Slider::new(grain_size, 1.0..=5.0));
                if size_before != *grain_size { *project_changed = true; }
            });

            ui.horizontal(|ui| {
                let c_before = *color_film;
                ui.checkbox(color_film, "Color Film Grain");
                if c_before != *color_film { *project_changed = true; }
            });
        }

        // ── CPU pixel-effect kernels (core::cpu_effects) ──
        EffectType::Twirl { angle, radius } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Twirl Angle", angle, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Twirl Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=300.0)); });
        }
        EffectType::Bulge { amount, radius } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Bulge Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Bulge Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=300.0)); });
        }
        EffectType::Posterize { levels } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Posterize Levels", levels, |ui, v| { ui.add(egui::Slider::new(v, 2.0..=32.0)); });
        }
        EffectType::Invert { invert_alpha } => {
            ui.horizontal(|ui| {
                let b_before = *invert_alpha;
                ui.checkbox(invert_alpha, "Invert Alpha");
                if b_before != *invert_alpha { *project_changed = true; }
            });
        }
        EffectType::Offset { shift_x, shift_y } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Offset X", shift_x, |ui, v| { ui.add(egui::Slider::new(v, -300.0..=300.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Offset Y", shift_y, |ui, v| { ui.add(egui::Slider::new(v, -300.0..=300.0)); });
        }
        EffectType::DirectionalBlur { angle, length } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Direction", angle, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Length", length, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0)); });
        }
        EffectType::RadialBlur { amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Radial Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
        }
        EffectType::Sharpen { amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Sharpen Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0)); });
        }
        EffectType::Threshold { threshold } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Threshold", threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=255.0)); });
        }
        EffectType::LinearWipe { completion, angle } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Completion", completion, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%")); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Wipe Angle", angle, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
        }
        EffectType::SimpleChoker { choke_amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Choke Amount", choke_amount, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
        }
        EffectType::ChromaKey { screen_color, screen_gain, clip_black, clip_white } => {
            let c_before = screen_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Key Color", screen_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if c_before != *screen_color { *project_changed = true; }
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Screen Gain", screen_gain, |ui, v| { ui.add(egui::Slider::new(v, 0.5..=2.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Clip Black", clip_black, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Clip White", clip_white, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
        }
        EffectType::Spherize { radius, refractive_index } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=500.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Refractive Index", refractive_index, |ui, v| { ui.add(egui::Slider::new(v, 0.5..=2.0)); });
        }
        EffectType::TurbulentDisplace { amount, size, evolution, complexity } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, -200.0..=200.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Size", size, |ui, v| { ui.add(egui::Slider::new(v, 2.0..=500.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Evolution", evolution, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Complexity", complexity, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=10.0)); });
        }
        EffectType::Colorama { preset_index, cycle_phase } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Preset (0=Rainbow,1=Heat,2=Sepia,3=Solar)", preset_index, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=3.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Cycle Phase", cycle_phase, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
        }
        // ── New AE-standard effects ──
        EffectType::FractalNoise { fractal_type, contrast, brightness, complexity, evolution } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Type (0=Fbm,1=Turb,2=Dyn,3=Ridge)", fractal_type, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=3.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Contrast", contrast, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=200.0).suffix("%")); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Brightness", brightness, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0).suffix("%")); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Complexity", complexity, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=10.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Evolution", evolution, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
        }
        EffectType::Curves { channel } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Channel (0=Master,1=R,2=G,3=B)", channel, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=3.0)); });
            ui.label("Use S-curve preset (5-point catmull-rom)");
        }
        EffectType::DisplacementMap { source_layer, max_horizontal, max_vertical } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Source Layer ID", source_layer, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=10.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Max Horizontal", max_horizontal, |ui, v| { ui.add(egui::Slider::new(v, -200.0..=200.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Max Vertical", max_vertical, |ui, v| { ui.add(egui::Slider::new(v, -200.0..=200.0)); });
        }
        EffectType::CompoundBlur { source_layer, max_blur } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Source Layer ID", source_layer, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=10.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Max Blur", max_blur, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0)); });
        }
        EffectType::Minimax { operation, radius } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Operation (0=Min,1=Max)", operation, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=50.0)); });
        }
        EffectType::ShiftChannels { take_red, take_green, take_blue, take_alpha } => {
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Take Red (0=R,1=G,2=B,3=A,4=Off,5=On)", take_red, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Take Green", take_green, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Take Blue", take_blue, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame,
                "Take Alpha", take_alpha, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
        }
        // ── Effects migrated from ExtEffect (UI deferred) ──
        EffectType::WaveWarp { wave_height, wave_width, speed, direction_deg, .. } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Wave Height", wave_height, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=200.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Wave Width", wave_width, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=500.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Speed", speed, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=20.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Direction", direction_deg, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
        }
        EffectType::CcLens { convergence, zoom } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Convergence", convergence, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=200.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Zoom", zoom, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
        }
        EffectType::PolarCoordinates { interpolation, .. } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Interpolation", interpolation, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%")); });
        }
        EffectType::OpticsCompensation { field_of_view_deg, zoom, .. } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "FOV", field_of_view_deg, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=180.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Zoom", zoom, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
        }
        EffectType::LightSweep { direction_deg, sweep_intensity, edge_intensity, .. } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Direction", direction_deg, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Sweep Intensity", sweep_intensity, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Edge Intensity", edge_intensity, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
        }
        EffectType::RadialFastBlur { amount, .. } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
        }
        EffectType::BendIt { top_offset, bottom_offset } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Top Offset", top_offset, |ui, v| { ui.add(egui::Slider::new(v, -50.0..=50.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Bottom Offset", bottom_offset, |ui, v| { ui.add(egui::Slider::new(v, -50.0..=50.0).suffix(" px")); });
        }
        EffectType::Tiler { scale_percent, .. } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Scale", scale_percent, |ui, v| { ui.add(egui::Slider::new(v, 10.0..=500.0).suffix("%")); });
        }
        EffectType::ColorBalance { shadows, midtones, highlights, preserve_luminosity } => {
            // Plain (non-Animatable) fields: edit in place, flag project change.
            for (label, band) in [("Shadows", shadows), ("Midtones", midtones), ("Highlights", highlights)] {
                ui.small(label);
                ui.horizontal(|ui| {
                    for (i, cname) in ["R", "G", "B"].iter().enumerate() {
                        ui.label(*cname);
                        if ui
                            .add(egui::DragValue::new(&mut band[i]).speed(1.0).range(-100.0..=100.0))
                            .changed()
                        {
                            *project_changed = true;
                        }
                    }
                });
            }
            if ui.checkbox(preserve_luminosity, "Preserve Luminosity").changed() {
                *project_changed = true;
            }
        }
        EffectType::ChannelMixer { matrix, monochrome } => {
            ui.label("Output ← Input (%)");
            egui::Grid::new("channel_mixer_grid").num_columns(4).show(ui, |ui| {
                let names = ["R", "G", "B"];
                for (r, row) in matrix.iter_mut().enumerate() {
                    ui.label(format!("←{}", names[r]));
                    for v in row.iter_mut() {
                        if ui
                            .add(egui::DragValue::new(v).speed(1.0).range(-200.0..=200.0))
                            .changed()
                        {
                            *project_changed = true;
                        }
                    }
                    ui.end_row();
                }
            });
            if ui.checkbox(monochrome, "Monochrome").changed() {
                *project_changed = true;
            }
        }
        EffectType::Tritone { shadow_color, mid_color, highlight_color } => {
            // Color pickers for 3-tone mapping
            let sc_before = shadow_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Shadow Color", shadow_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if sc_before != *shadow_color { *project_changed = true; }

            let mc_before = mid_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Mid Color", mid_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if mc_before != *mid_color { *project_changed = true; }

            let hc_before = highlight_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Highlight Color", highlight_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if hc_before != *highlight_color { *project_changed = true; }
        }
        EffectType::MatteChoker { choke_amount, gray_level } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Choke Amount", choke_amount, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=50.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Gray Level", gray_level, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
        }
        EffectType::VenetianBlinds { completion, width } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Completion", completion, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Width", width, |ui, v| { ui.add(egui::Slider::new(v, 2.0..=100.0).suffix(" px")); });
        }
        // ── Lumetri Basic Correction ──
        EffectType::Vibrance { amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
        }
        EffectType::WhiteBalance { temperature, tint } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Temperature", temperature, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Tint", tint, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
        }
        EffectType::HslAdjust { hue_deg, saturation, lightness } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Hue", hue_deg, |ui, v| { ui.add(egui::Slider::new(v, -180.0..=180.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Saturation", saturation, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Lightness", lightness, |ui, v| { ui.add(egui::Slider::new(v, -100.0..=100.0)); });
        }
        EffectType::GlowPro { threshold, radius, intensity } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Threshold", threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=128.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Intensity", intensity, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=4.0)); });
        }
        EffectType::CrtScanlines { line_spacing, intensity } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Line Spacing", line_spacing, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=50.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Intensity", intensity, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
        }
        EffectType::Vortex { radius, angle_deg } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=2000.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Angle", angle_deg, |ui, v| { ui.add(egui::Slider::new(v, -720.0..=720.0).suffix("°")); });
        }
        EffectType::HeatDistortion { strength, speed } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Strength", strength, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=30.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Speed", speed, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0).suffix("×")); });
        }
        EffectType::RainRipples { drop_count, wave_strength } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Drop Count", drop_count, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Wave Strength", wave_strength, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=20.0)); });
        }
        EffectType::Fisheye { strength } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Strength (− = pincushion)", strength, |ui, v| { ui.add(egui::Slider::new(v, -1.0..=1.0)); });
        }
        EffectType::LensCorrection { k1, k2 } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "K1 (Barrel + / Pincushion −)", k1, |ui, v| { ui.add(egui::Slider::new(v, -0.5..=0.5)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "K2", k2, |ui, v| { ui.add(egui::Slider::new(v, -0.5..=0.5)); });
        }
        EffectType::GlitchDisplacement { seed, amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Seed", seed, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=9999.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=10.0)); });
        }
        EffectType::MatteChokeSpread { radius, expand } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=64.0).suffix(" px")); });
            if ui.checkbox(expand, "Expand (spread instead of choke)").changed() {
                *project_changed = true;
            }
        }
        EffectType::AlphaFeather { radius } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=64.0).suffix(" px")); });
        }
        EffectType::AlphaFromLuminance { invert } => {
            if ui.checkbox(invert, "Invert (dark = opaque)").changed() {
                *project_changed = true;
            }
        }
        EffectType::NightVision { amplification } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Amplification", amplification, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=8.0).suffix("×")); });
        }
        EffectType::IrisWipe { completion } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Completion", completion, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%")); });
        }
        EffectType::RadialWipe { completion } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Completion", completion, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0).suffix("%")); });
        }
        EffectType::FilmEmulation { lift, gamma, gain, hue_shift_deg } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Lift", lift, |ui, v| { ui.add(egui::Slider::new(v, -0.5..=0.5)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Gamma", gamma, |ui, v| { ui.add(egui::Slider::new(v, 0.1..=3.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Gain", gain, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=3.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Hue Shift", hue_shift_deg, |ui, v| { ui.add(egui::Slider::new(v, -180.0..=180.0).suffix("°")); });
        }
        EffectType::GodRays { sun_x, sun_y, samples, decay, weight } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Sun X", sun_x, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Sun Y", sun_y, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Samples", samples, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=64.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Decay", decay, |ui, v| { ui.add(egui::Slider::new(v, 0.5..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Weight", weight, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=2.0)); });
        }
        EffectType::RadialBlurZoom { amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Amount", amount, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=100.0)); });
        }
        EffectType::MedianFilter { radius } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=16.0).suffix(" px")); });
        }
        EffectType::SobelEdges { invert } => {
            if ui.checkbox(invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        EffectType::Mosaic { block_w, block_h } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Block Width", block_w, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=128.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Block Height", block_h, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=128.0).suffix(" px")); });
        }
        EffectType::TiltShift { focus_y, focus_height, max_blur } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Focus Y", focus_y, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Band Height", focus_height, |ui, v| { ui.add(egui::Slider::new(v, 0.02..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Max Blur", max_blur, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=32.0).suffix(" px")); });
        }
        EffectType::Emboss { angle_deg, depth } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Angle", angle_deg, |ui, v| { ui.add(egui::Slider::new(v, -180.0..=180.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Depth", depth, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=10.0)); });
        }
        EffectType::StarField { num_stars, depth_speed } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Stars", num_stars, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=2000.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Depth Speed", depth_speed, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=10.0)); });
        }
        EffectType::LightningArc { start_x, start_y, end_x, end_y, seed, glow } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Start X", start_x, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Start Y", start_y, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "End X", end_x, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "End Y", end_y, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Seed", seed, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=9999.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Glow", glow, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
        }
        EffectType::FireAutomaton { intensity } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Intensity", intensity, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=10.0)); });
        }
        EffectType::LumaKeyRange { low_threshold, high_threshold, invert } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Low Threshold", low_threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=255.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "High Threshold", high_threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=255.0)); });
            if ui.checkbox(invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        EffectType::Halftone { cell_size } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Cell Size", cell_size, |ui, v| { ui.add(egui::Slider::new(v, 2.0..=64.0).suffix(" px")); });
        }
        EffectType::Solarize { threshold } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Threshold", threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=255.0)); });
        }
        EffectType::PixelSort { threshold } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Threshold", threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=255.0)); });
        }
        EffectType::PinchPunch { radius, amount } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=2000.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Amount (+pinch / −punch)", amount, |ui, v| { ui.add(egui::Slider::new(v, -2.0..=2.0)); });
        }
        EffectType::ScanlineGlitch { jitter_amount, seed } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Jitter", jitter_amount, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=50.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Seed", seed, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=9999.0)); });
        }
        EffectType::GlassEdgeBevel { bevel_size, refraction } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Bevel Size", bevel_size, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=64.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Refraction", refraction, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=3.0)); });
        }
        EffectType::DirectionalSharpen { angle_deg, strength } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Angle", angle_deg, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Strength", strength, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=5.0)); });
        }
        EffectType::RefractionLens { radius, ior } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Radius", radius, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=2000.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "IOR", ior, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=3.0)); });
        }
        EffectType::GradientMap { low_color, mid_color, high_color } => {
            let lc_before = low_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Shadow Color", low_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if lc_before != *low_color { *project_changed = true; }

            let mc_before = mid_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Mid Color", mid_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if mc_before != *mid_color { *project_changed = true; }

            let hc_before = high_color.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Highlight Color", high_color, |ui, val| {
                ui.color_edit_button_rgb(val);
            }) { *next_frame = Some(nf); }
            if hc_before != *high_color { *project_changed = true; }
        }
        EffectType::LightLeak { pos_x, pos_y, intensity } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Position X", pos_x, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Position Y", pos_y, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=1.0)); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Intensity", intensity, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=3.0)); });
        }
        EffectType::BevelAlpha { depth, light_angle_deg } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Depth", depth, |ui, v| { ui.add(egui::Slider::new(v, 1.0..=32.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Light Angle", light_angle_deg, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=360.0).suffix("°")); });
        }
        EffectType::CrossHatch { line_gap, threshold } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Line Gap", line_gap, |ui, v| { ui.add(egui::Slider::new(v, 2.0..=32.0).suffix(" px")); });
            draw_prop(ui, current_frame, project_changed, next_frame, "Threshold", threshold, |ui, v| { ui.add(egui::Slider::new(v, 0.0..=255.0)); });
        }
        EffectType::CmykHalftone { dot_size } => {
            draw_prop(ui, current_frame, project_changed, next_frame, "Dot Size", dot_size, |ui, v| { ui.add(egui::Slider::new(v, 2.0..=32.0).suffix(" px")); });
        }
    }
}

/// Helper: draw a single animatable `f32` property row for an `EffectType` arm.
///
/// `field` is a mutable borrow of the matched `EffectType` field; edits (and
/// keyframe reassignments inside `draw_property_ui`) persist directly back to
/// the caller's `EffectType` through that borrow.
fn draw_prop(
    ui: &mut egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
    label: &str,
    field: &mut Animatable<f32>,
    draw_value: impl FnOnce(&mut egui::Ui, &mut f32),
) {
    let before = field.clone();
    if let Some(nf) = draw_property_ui(current_frame, ui, label, field, draw_value) {
        *next_frame = Some(nf);
    }
    if before != *field {
        *project_changed = true;
    }
}

// ── Particle Emitter Inspector ─────────────────────────────────────────────
//
// Exposes the full emitter configuration for a selected Particle layer:
// emission, forces, shape and the collision set (bounds + particle-vs-
// particle). Every edit writes straight into the layer's emitter via
// `modify_project`, so changes are undoable and render immediately.

/// Draws emitter controls when the selected layer is a Particle layer.
pub fn draw_particle_emitter_controls(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    use crate::core::particle_system::{EmitterShape, ParticleEmitter};
    use crate::core::timeline::LayerType;

    let Some(idx) = app.selected_layer_idx else { return };
    let is_particle = matches!(
        app.history.current().active_composition().layers.get(idx),
        Some(l) if matches!(l.layer_type, LayerType::Particle { .. })
    );
    if !is_particle {
        return;
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(
        egui::RichText::new("Particle Emitter")
            .strong()
            .color(colors::ACCENT_CYAN),
    );

    // Local working copy so sliders feel responsive; committed on change.
    let mut em: ParticleEmitter = match app.history.current().active_composition().layers.get(idx) {
        Some(l) => match &l.layer_type {
            LayerType::Particle { emitter } => emitter.clone(),
            _ => return,
        },
        None => return,
    };
    let mut changed = false;

    ui.collapsing("Emission", |ui| {
        changed |= row_f32(ui, "Rate (p/s)", &mut em.rate, 0.0..=500.0, 1.0);
        changed |= row_u32(ui, "Max particles", &mut em.max_particles, 1..=20000);
        changed |= row_f32(ui, "Lifetime (s)", &mut em.lifetime, 0.1..=30.0, 0.1);
        changed |= row_f32(ui, "Lifetime var", &mut em.lifetime_variance, 0.0..=1.0, 0.01);
        changed |= row_f32(ui, "Speed", &mut em.speed, 0.0..=1000.0, 1.0);
        changed |= row_f32(ui, "Speed var", &mut em.speed_variance, 0.0..=2.0, 0.01);
        changed |= row_f32(ui, "Spread (°)", &mut em.spread_degrees, 0.0..=360.0, 1.0);
    });

    ui.collapsing("Shape", |ui| {
        let shapes = [
            ("Point", EmitterShape::Point),
            ("Box", EmitterShape::Box),
            ("Circle", EmitterShape::Circle),
            ("Line", EmitterShape::Line),
            ("Ring", EmitterShape::Ring),
        ];
        ui.horizontal(|ui| {
            for (label, shape) in shapes {
                let selected = em.shape == shape;
                if ui
                    .selectable_label(selected, label)
                    .clicked()
                {
                    em.shape = shape;
                    changed = true;
                }
            }
        });
        changed |= row_f32(ui, "Size X / Ø", &mut em.emitter_size[0], 1.0..=4000.0, 1.0);
        if em.shape == EmitterShape::Box {
            changed |= row_f32(ui, "Size Y", &mut em.emitter_size[1], 1.0..=4000.0, 1.0);
        }
    });

    ui.collapsing("Forces", |ui| {
        ui.horizontal(|ui| {
            ui.label("Gravity:");
            changed |= drag2(ui, &mut em.gravity);
            ui.label("Wind:");
            changed |= drag2(ui, &mut em.wind);
        });
        changed |= row_f32(ui, "Gust strength", &mut em.wind_gust_strength, 0.0..=300.0, 1.0);
        changed |= row_f32(ui, "Gust freq (Hz)", &mut em.wind_gust_frequency, 0.0..=10.0, 0.05);
        changed |= row_f32(ui, "Turbulence", &mut em.turbulence, 0.0..=300.0, 1.0);
        changed |= row_f32(ui, "Air drag", &mut em.drag, 0.0..=20.0, 0.05);
    });

    ui.collapsing("Look", |ui| {
        changed |= row_f32(ui, "Size start", &mut em.size_start, 0.0..=200.0, 0.5);
        changed |= row_f32(ui, "Size end", &mut em.size_end, 0.0..=200.0, 0.5);
    });

    ui.collapsing("Collisions", |ui| {
        if ui.checkbox(&mut em.collision_enabled, "Boundary collisions").changed() {
            changed = true;
        }
        if em.collision_enabled {
            ui.horizontal(|ui| {
                ui.label("Bounds x0/y0/x1/y1:");
                for v in &mut em.collision_bounds {
                    if ui.add(egui::DragValue::new(v).speed(1.0)).changed() {
                        changed = true;
                    }
                }
            });
            changed |= row_f32(ui, "Restitution", &mut em.restitution, 0.0..=1.0, 0.01);
            changed |= row_f32(ui, "Friction", &mut em.surface_friction, 0.0..=1.0, 0.01);
        }
        if ui.checkbox(&mut em.particle_collisions, "Particle ↔ particle").changed() {
            changed = true;
        }
        if em.particle_collisions {
            changed |= row_f32(ui, "Contact Ø (px)", &mut em.particle_diameter, 0.5..=200.0, 0.5);
        }
    });

    if changed {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if let Some(layer) = comp.layers.get_mut(idx) {
                if let LayerType::Particle { emitter } = &mut layer.layer_type {
                    *emitter = em.clone();
                }
            }
        });
        crate::core::frame_cache::bump_version();
    }
}

fn row_f32(ui: &mut egui::Ui, label: &str, val: &mut f32, range: std::ops::RangeInclusive<f32>, speed: f32) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val).speed(speed).range(range)).changed()
    }).inner
}

fn row_u32(ui: &mut egui::Ui, label: &str, val: &mut u32, range: std::ops::RangeInclusive<u32>) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val).range(range)).changed()
    }).inner
}

fn drag2(ui: &mut egui::Ui, v: &mut [f32; 2]) -> bool {
    let mut ch = false;
    for c in v.iter_mut() {
        if ui.add(egui::DragValue::new(c).speed(1.0)).changed() {
            ch = true;
        }
    }
    ch
}
