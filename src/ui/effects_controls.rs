use eframe::egui;
use crate::core::timeline::{Effect, EffectType, ColorConversionMode};
use crate::core::property::Animatable;
use crate::ui::inspector_property::draw_property_ui;

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
        EffectType::ColorBalance { .. } | EffectType::ChannelMixer { .. } => {
            ui.label("Parameter UI deferred");
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
