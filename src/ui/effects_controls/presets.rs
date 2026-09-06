use crate::core::property::Animatable;
use crate::core::timeline::{ColorConversionMode, Effect, EffectType};

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
                effect_type: EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(10.0),
                },
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
                    iris_linked: true,
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
            name: "Corner Pin",
            button_label: "+ Corner Pin",
            search_key: "corner pin perspective homography screen insert cc power pin",
            id_prefix: "cornerpin",
            create_fn: |idx| Effect {
                id: format!("cornerpin_{}", idx),
                name: "Corner Pin".to_string(),
                effect_type: EffectType::CornerPin {
                    top_left: Animatable::new_constant([0.0, 0.0]),
                    top_right: Animatable::new_constant([1920.0, 0.0]),
                    bottom_right: Animatable::new_constant([1920.0, 1080.0]),
                    bottom_left: Animatable::new_constant([0.0, 1080.0]),
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
                effect_type: EffectType::Posterize {
                    levels: Animatable::new_constant(4.0),
                },
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
                effect_type: EffectType::Invert {
                    invert_alpha: false,
                },
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
                effect_type: EffectType::RadialBlur {
                    amount: Animatable::new_constant(20.0),
                },
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
                effect_type: EffectType::Sharpen {
                    amount: Animatable::new_constant(50.0),
                },
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
                effect_type: EffectType::Threshold {
                    threshold: Animatable::new_constant(128.0),
                },
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
                effect_type: EffectType::SimpleChoker {
                    choke_amount: Animatable::new_constant(0.0),
                },
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
            name: "Laser Beam",
            button_label: "+ Laser Beam",
            search_key: "laser beam ray energy projectile blaster shoot glow core",
            id_prefix: "laser",
            create_fn: |idx| Effect {
                id: format!("laser_{}", idx),
                name: "Laser Beam".to_string(),
                effect_type: EffectType::LaserBeam {
                    start_x: Animatable::new_constant(0.1),
                    start_y: Animatable::new_constant(0.5),
                    end_x: Animatable::new_constant(0.9),
                    end_y: Animatable::new_constant(0.5),
                    progress: Animatable::new_constant(0.5),
                    length: Animatable::new_constant(40.0),
                    starting_thickness: Animatable::new_constant(12.0),
                    ending_thickness: Animatable::new_constant(4.0),
                    core_color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                    glow_color: Animatable::new_constant([1.0, 0.2, 0.1, 0.8]),
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
            name: "Lens Flare (GPU)",
            button_label: "+ Lens Flare (GPU)",
            search_key:
                "lens flare optical anamorphic streak star rings light source gpu screen space",
            id_prefix: "flare",
            create_fn: |idx| Effect {
                id: format!("flare_{}", idx),
                name: "Lens Flare".to_string(),
                effect_type: EffectType::LensFlare {
                    enabled: Animatable::new_constant(1.0),
                    position_x: Animatable::new_constant(0.5),
                    position_y: Animatable::new_constant(0.35),
                    intensity: Animatable::new_constant(1.0),
                    threshold: Animatable::new_constant(0.8),
                    color: Animatable::new_constant([1.0, 0.95, 0.9, 1.0]),
                    link_to_light: None,
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
        EffectPreset {
            name: "Reflection Map",
            button_label: "+ Reflection Map",
            search_key: "reflection mirror water floor horizon fade",
            id_prefix: "reflmap",
            create_fn: |idx| Effect {
                id: format!("reflmap_{}", idx),
                name: "Reflection Map".to_string(),
                effect_type: EffectType::ReflectionMap {
                    reflect_y: Animatable::new_constant(200.0),
                    fade_dist: Animatable::new_constant(150.0),
                    opacity: Animatable::new_constant(0.6),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Perlin Flow Noise",
            button_label: "+ Perlin Flow",
            search_key: "perlin flow noise organic smoke fog generate procedural",
            id_prefix: "pflow",
            create_fn: |idx| Effect {
                id: format!("pflow_{}", idx),
                name: "Perlin Flow Noise".to_string(),
                effect_type: EffectType::PerlinFlow {
                    scale: Animatable::new_constant(4.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "FBM Turbulence",
            button_label: "+ FBM Turbulence",
            search_key: "fbm turbulence fractal brownian octaves displacement clouds",
            id_prefix: "fbmt",
            create_fn: |idx| Effect {
                id: format!("fbmt_{}", idx),
                name: "FBM Turbulence".to_string(),
                effect_type: EffectType::FbmTurbulence {
                    octaves: Animatable::new_constant(4.0),
                    amplitude: Animatable::new_constant(80.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Bass & Treble",
            button_label: "+ Bass & Treble",
            search_key: "bass treble eq equalizer bass boost treble cut crossover",
            id_prefix: "btreble",
            create_fn: |idx| Effect {
                id: format!("btreble_{}", idx),
                name: "Bass & Treble".to_string(),
                effect_type: EffectType::BassTreble {
                    bass_gain: Animatable::new_constant(0.0),
                    treble_gain: Animatable::new_constant(0.0),
                    crossover_freq: Animatable::new_constant(300.0),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Flanger",
            button_label: "+ Flanger",
            search_key: "flanger delay modulation lfo sweeping comb",
            id_prefix: "flanger",
            create_fn: |idx| Effect {
                id: format!("flanger_{}", idx),
                name: "Flanger".to_string(),
                effect_type: EffectType::Flanger {
                    max_delay_ms: Animatable::new_constant(5.0),
                    lfo_rate: Animatable::new_constant(0.5),
                    feedback: Animatable::new_constant(0.5),
                    wet_dry: Animatable::new_constant(0.5),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Chorus",
            button_label: "+ Chorus",
            search_key: "chorus detune multi-voice doubling modulation",
            id_prefix: "chorus",
            create_fn: |idx| Effect {
                id: format!("chorus_{}", idx),
                name: "Chorus".to_string(),
                effect_type: EffectType::Chorus {
                    delay_ms: Animatable::new_constant(15.0),
                    depth_ms: Animatable::new_constant(5.0),
                    rate_hz: Animatable::new_constant(1.0),
                    voices: Animatable::new_constant(3.0),
                    feedback: Animatable::new_constant(0.3),
                },
                enabled: true,
            },
        },
        EffectPreset {
            name: "Parametric EQ",
            button_label: "+ Parametric EQ",
            search_key: "parametric eq equalizer bell filter frequency resonance q",
            id_prefix: "peq",
            create_fn: |idx| Effect {
                id: format!("peq_{}", idx),
                name: "Parametric EQ".to_string(),
                effect_type: EffectType::ParametricEQ {
                    freq_hz: Animatable::new_constant(1000.0),
                    gain_db: Animatable::new_constant(0.0),
                    q_factor: Animatable::new_constant(1.0),
                },
                enabled: true,
            },
        },
    ]
}
