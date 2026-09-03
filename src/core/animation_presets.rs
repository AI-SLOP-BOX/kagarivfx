use serde::{Deserialize, Serialize};

use crate::core::keyframe::{EasePreset, InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationPreset {
    pub name: String,
    pub category: PresetCategory,
    pub description: String,
    pub property_type: PresetPropertyType,
    pub keyframes: Vec<PresetKeyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetCategory {
    Position,
    Opacity,
    Scale,
    Rotation,
    Focus,
    Color,
    Text,
    Combo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetPropertyType {
    Position,
    Opacity,
    Scale,
    Rotation,
    Blur,
    ColorRGB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetKeyframe {
    pub time: f32,
    pub value: f32,
    #[serde(default)]
    pub ease: f32,
    #[serde(default)]
    pub value_y: Option<f32>,
}

pub fn all_presets() -> Vec<AnimationPreset> {
    let mut p = Vec::new();
    p.extend(position_presets());
    p.extend(opacity_presets());
    p.extend(scale_presets());
    p.extend(rotation_presets());
    p.extend(blur_presets());
    p.extend(text_presets());
    p.extend(combo_presets());
    p
}

pub fn presets_by_category(cat: PresetCategory) -> Vec<AnimationPreset> {
    all_presets()
        .into_iter()
        .filter(|p| p.category == cat)
        .collect()
}

pub fn find_preset(name: &str) -> Option<AnimationPreset> {
    all_presets().into_iter().find(|p| p.name == name)
}

fn eased_kf(frame: u32, v: f32, ease: f32) -> Keyframe<f32> {
    let mut kf = Keyframe::new(frame, v, InterpolationType::Linear);
    if ease > 0.001 {
        let coords = EasePreset::Standard.control_points();
        kf.interpolation = InterpolationType::Bezier {
            outgoing: crate::core::keyframe::BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: crate::core::keyframe::BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some(coords),
        };
    }
    kf
}

fn eased_kfv2(frame: u32, v: [f32; 2], ease: f32) -> Keyframe<[f32; 2]> {
    let mut kf = Keyframe::new(frame, v, InterpolationType::Linear);
    if ease > 0.001 {
        let coords = EasePreset::Standard.control_points();
        kf.interpolation = InterpolationType::Bezier {
            outgoing: crate::core::keyframe::BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            incoming: crate::core::keyframe::BezierControlPoint {
                influence: 0.333,
                speed: 0.0,
            },
            custom_bezier: Some(coords),
        };
    }
    kf
}

/// Apply a preset's keyframes to a layer at the given start frame.
/// Position/Scale presets preserve the layer's current value as the rest point.
pub fn apply_preset_to_layer(
    preset: &AnimationPreset,
    layer: &mut Layer,
    start_frame: u32,
) -> bool {
    let to_frame = |t: f32| start_frame + (t * 30.0).round() as u32;

    match preset.property_type {
        PresetPropertyType::Opacity | PresetPropertyType::Blur => {
            let kfs: Vec<Keyframe<f32>> = preset
                .keyframes
                .iter()
                .map(|k| eased_kf(to_frame(k.time), k.value, k.ease))
                .collect();
            if kfs.is_empty() {
                return false;
            }
            if preset.property_type == PresetPropertyType::Opacity {
                layer.transform.opacity = Animatable::Animated(kfs);
            }
            true
        }
        PresetPropertyType::Scale => {
            let rest = layer.transform.scale.evaluate(start_frame);
            let kfs: Vec<Keyframe<[f32; 2]>> = preset
                .keyframes
                .iter()
                .map(|k| {
                    let v = k.value.max(0.0) / 100.0;
                    let vx = rest[0] * v;
                    let vy = k
                        .value_y
                        .map(|y| rest[1] * (y.max(0.0) / 100.0))
                        .unwrap_or(vx);
                    eased_kfv2(to_frame(k.time), [vx, vy], k.ease)
                })
                .collect();
            if kfs.is_empty() {
                return false;
            }
            layer.transform.scale = Animatable::Animated(kfs);
            true
        }
        PresetPropertyType::Position => {
            let rest = layer.transform.position.evaluate(start_frame);
            let kfs: Vec<Keyframe<[f32; 2]>> = preset
                .keyframes
                .iter()
                .map(|k| {
                    let x = rest[0] + k.value;
                    let y = rest[1] + k.value_y.unwrap_or(0.0);
                    eased_kfv2(to_frame(k.time), [x, y], k.ease)
                })
                .collect();
            if kfs.is_empty() {
                return false;
            }
            layer.transform.position = Animatable::Animated(kfs);
            true
        }
        PresetPropertyType::Rotation => {
            let rest = layer.transform.rotation.evaluate(start_frame);
            let kfs: Vec<Keyframe<f32>> = preset
                .keyframes
                .iter()
                .map(|k| eased_kf(to_frame(k.time), rest + k.value, k.ease))
                .collect();
            if kfs.is_empty() {
                return false;
            }
            layer.transform.rotation = Animatable::Animated(kfs);
            true
        }
        PresetPropertyType::ColorRGB => false,
    }
}

/// Apply a preset by name; returns false if not found or not applicable.
pub fn apply_by_name(name: &str, layer: &mut Layer, start_frame: u32) -> bool {
    match find_preset(name) {
        Some(p) => apply_preset_to_layer(&p, layer, start_frame),
        None => false,
    }
}

fn position_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Slide In from Left".into(),
            category: PresetCategory::Position,
            description: "Slides in from the left with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: -100.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 5.0,
                    ease: 0.8,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide In from Right".into(),
            category: PresetCategory::Position,
            description: "Slides in from the right with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: -5.0,
                    ease: 0.8,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide In from Top".into(),
            category: PresetCategory::Position,
            description: "Slides in from above with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.8,
                    value_y: Some(5.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide In from Bottom".into(),
            category: PresetCategory::Position,
            description: "Slides in from below with overshoot".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.8,
                    value_y: Some(-5.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.6,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Bounce In".into(),
            category: PresetCategory::Position,
            description: "Bounces in from below with decreasing amplitude".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(80.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-15.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(8.0),
                },
                PresetKeyframe {
                    time: 0.8,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-3.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Elastic Pop".into(),
            category: PresetCategory::Position,
            description: "Elastic overshoot with rapid oscillation".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(60.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-20.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(10.0),
                },
                PresetKeyframe {
                    time: 0.65,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-5.0),
                },
                PresetKeyframe {
                    time: 0.8,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(2.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
            ],
        },
    ]
}

fn opacity_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Fade In".into(),
            category: PresetCategory::Opacity,
            description: "Simple fade from transparent to opaque".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Fade Out".into(),
            category: PresetCategory::Opacity,
            description: "Simple fade from opaque to transparent".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Fade In + Hold + Fade Out".into(),
            category: PresetCategory::Opacity,
            description: "Fade in, hold, then fade out".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 100.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Flash".into(),
            category: PresetCategory::Opacity,
            description: "Quick flash: appear, hold, disappear".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.05,
                    value: 100.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.15,
                    value: 100.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.2,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
            ],
        },
    ]
}

fn scale_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Pop In".into(),
            category: PresetCategory::Scale,
            description: "Scale from 0% to 110% then settle to 100%".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: 110.0,
                    ease: 0.7,
                    value_y: Some(110.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Scale Down".into(),
            category: PresetCategory::Scale,
            description: "Scale from 100% to 0%".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: -5.0,
                    ease: 0.7,
                    value_y: Some(-5.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Breathing".into(),
            category: PresetCategory::Scale,
            description: "Continuous scale oscillation between 95% and 105%".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 105.0,
                    ease: 0.5,
                    value_y: Some(105.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Squash and Stretch".into(),
            category: PresetCategory::Scale,
            description: "Classic cartoon squash-stretch".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.0,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.15,
                    value: 120.0,
                    ease: 0.0,
                    value_y: Some(80.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 85.0,
                    ease: 0.0,
                    value_y: Some(115.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
    ]
}

fn rotation_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Spin Clockwise".into(),
            category: PresetCategory::Rotation,
            description: "360 degree clockwise rotation".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 360.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Spin Counter-Clockwise".into(),
            category: PresetCategory::Rotation,
            description: "360 degree counter-clockwise rotation".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: -360.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Wobble".into(),
            category: PresetCategory::Rotation,
            description: "Decaying wobble oscillation".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.15,
                    value: 15.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.35,
                    value: -10.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.55,
                    value: 5.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.75,
                    value: -2.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
    ]
}

fn blur_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Focus In".into(),
            category: PresetCategory::Focus,
            description: "Blur from 20px to 0px (rack focus)".into(),
            property_type: PresetPropertyType::Blur,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 20.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Blur Out".into(),
            category: PresetCategory::Focus,
            description: "Blur from 0px to 20px".into(),
            property_type: PresetPropertyType::Blur,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 20.0,
                    ease: 0.5,
                    value_y: None,
                },
            ],
        },
    ]
}

fn combo_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Pop and Fade In".into(),
            category: PresetCategory::Combo,
            description: "Scale pop + fade in combined".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 115.0,
                    ease: 0.7,
                    value_y: Some(115.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Shrink and Fade Out".into(),
            category: PresetCategory::Combo,
            description: "Shrink to 0% while fading out".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.7,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Spin and Scale In".into(),
            category: PresetCategory::Combo,
            description: "Spin 360 + scale from 0% combined".into(),
            property_type: PresetPropertyType::Rotation,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 360.0,
                    ease: 0.7,
                    value_y: Some(110.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 360.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
        AnimationPreset {
            name: "Drop and Bounce".into(),
            category: PresetCategory::Combo,
            description: "Drop from above + bounce scale".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-80.0),
                },
                PresetKeyframe {
                    time: 0.4,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(5.0),
                },
                PresetKeyframe {
                    time: 0.6,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(-2.0),
                },
                PresetKeyframe {
                    time: 0.8,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
            ],
        },
        AnimationPreset {
            name: "Slide and Rotate In".into(),
            category: PresetCategory::Combo,
            description: "Slide from left + rotate 90 degrees".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: -100.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.7,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.7,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(0.0),
                },
            ],
        },
    ]
}

fn text_presets() -> Vec<AnimationPreset> {
    vec![
        AnimationPreset {
            name: "Typewriter".into(),
            category: PresetCategory::Text,
            description: "Character-by-character reveal".into(),
            property_type: PresetPropertyType::Opacity,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: None,
                },
                PresetKeyframe {
                    time: 0.05,
                    value: 100.0,
                    ease: 0.0,
                    value_y: None,
                },
            ],
        },
        AnimationPreset {
            name: "Text Wave".into(),
            category: PresetCategory::Text,
            description: "Per-character wave animation".into(),
            property_type: PresetPropertyType::Position,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(10.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(-10.0),
                },
                PresetKeyframe {
                    time: 1.0,
                    value: 0.0,
                    ease: 0.5,
                    value_y: Some(10.0),
                },
            ],
        },
        AnimationPreset {
            name: "Text Scale Pop".into(),
            category: PresetCategory::Text,
            description: "Per-character scale pop".into(),
            property_type: PresetPropertyType::Scale,
            keyframes: vec![
                PresetKeyframe {
                    time: 0.0,
                    value: 0.0,
                    ease: 0.0,
                    value_y: Some(0.0),
                },
                PresetKeyframe {
                    time: 0.3,
                    value: 120.0,
                    ease: 0.7,
                    value_y: Some(120.0),
                },
                PresetKeyframe {
                    time: 0.5,
                    value: 100.0,
                    ease: 0.5,
                    value_y: Some(100.0),
                },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_non_empty() {
        let presets = all_presets();
        assert!(
            presets.len() >= 20,
            "Expected at least 20 presets, got {}",
            presets.len()
        );
    }

    #[test]
    fn test_presets_by_category() {
        let pos = presets_by_category(PresetCategory::Position);
        assert!(pos.len() >= 4, "Expected at least 4 position presets");
        for p in &pos {
            assert_eq!(p.category, PresetCategory::Position);
        }
    }

    #[test]
    fn test_find_preset() {
        let p = find_preset("Fade In");
        assert!(p.is_some());
        assert_eq!(p.unwrap().category, PresetCategory::Opacity);
    }

    #[test]
    fn test_find_preset_not_found() {
        assert!(find_preset("Nonexistent").is_none());
    }

    #[test]
    fn test_preset_keyframe_serialization() {
        let kf = PresetKeyframe {
            time: 0.5,
            value: 100.0,
            ease: 0.8,
            value_y: None,
        };
        let json = serde_json::to_string(&kf).unwrap();
        let decoded: PresetKeyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.time, 0.5);
        assert_eq!(decoded.value, 100.0);
    }

    #[test]
    fn test_preset_keyframe_with_y() {
        let kf = PresetKeyframe {
            time: 0.0,
            value: 50.0,
            ease: 0.0,
            value_y: Some(75.0),
        };
        let json = serde_json::to_string(&kf).unwrap();
        let decoded: PresetKeyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.value_y, Some(75.0));
    }

    fn test_layer() -> Layer {
        Layer::new(
            "t1".into(),
            "Test".into(),
            crate::core::timeline::LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            100,
        )
    }

    #[test]
    fn test_apply_fade_in() {
        let mut layer = test_layer();
        assert!(apply_by_name("Fade In", &mut layer, 10));
        match &layer.transform.opacity {
            Animatable::Animated(kfs) => {
                assert_eq!(kfs.len(), 2);
                assert_eq!(kfs[0].frame, 10);
                assert_eq!(kfs[1].frame, 25);
            }
            _ => panic!("expected animated opacity"),
        }
    }

    #[test]
    fn test_apply_pop_in_preserves_rest_scale() {
        let mut layer = test_layer();
        layer.transform.scale = Animatable::Constant([50.0, 50.0]);
        assert!(apply_by_name("Pop In", &mut layer, 0));
        match &layer.transform.scale {
            Animatable::Animated(kfs) => {
                // Final keyframe settles at rest scale (50%)
                let last = kfs.last().unwrap();
                assert!((last.value[0] - 50.0).abs() < 0.01);
            }
            _ => panic!("expected animated scale"),
        }
    }

    #[test]
    fn test_apply_position_preset_offsets_from_rest() {
        let mut layer = test_layer();
        layer.transform.position = Animatable::Constant([500.0, 300.0]);
        assert!(apply_by_name("Slide In from Left", &mut layer, 5));
        match &layer.transform.position {
            Animatable::Animated(kfs) => {
                // First keyframe is 100px left of rest position
                assert!((kfs[0].value[0] - 400.0).abs() < 0.01);
                assert!((kfs[0].value[1] - 300.0).abs() < 0.01);
            }
            _ => panic!("expected animated position"),
        }
    }

    #[test]
    fn test_apply_unknown_returns_false() {
        let mut layer = test_layer();
        assert!(!apply_by_name("No Such Preset", &mut layer, 0));
    }

    #[test]
    fn test_apply_rotation_preset() {
        let mut layer = test_layer();
        layer.transform.rotation = Animatable::Constant(45.0);
        assert!(apply_by_name("Spin Clockwise", &mut layer, 0));
        match &layer.transform.rotation {
            Animatable::Animated(kfs) => {
                let last = kfs.last().unwrap();
                assert!((last.value - 405.0).abs() < 0.01);
            }
            _ => panic!("expected animated rotation"),
        }
    }
}
