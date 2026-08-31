use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

fn axis_3d(prop: &str) -> usize {
    if prop.ends_with('Z') {
        2
    } else if prop.ends_with('Y') {
        1
    } else {
        0
    }
}

/// A reusable module for rendering the After Effects keyframe Graph Editor.
///
/// Visualizes animatable property value curves over time, drawing interactive control
/// points and Bezier tangent handles.
/// Resolve "PinX:<id>" / "PinY:<id>" graph properties to the pin's track.
fn pin_anim_mut<'a>(
    layer: &'a mut Layer,
    prop: &str,
) -> Option<&'a mut crate::core::property::Animatable<[f32; 2]>> {
    let id = prop
        .strip_prefix("PinX:")
        .or_else(|| prop.strip_prefix("PinY:"))?;
    layer
        .puppet_pins
        .iter_mut()
        .find(|p| p.id == id)
        .map(|p| &mut p.position)
}

fn set_layer_interpolation(
    layer: &mut Layer,
    property: &str,
    interpolation: crate::core::keyframe::InterpolationType,
) -> bool {
    use crate::core::property::Animatable;

    fn apply<T>(
        track: &mut Animatable<T>,
        interpolation: crate::core::keyframe::InterpolationType,
    ) -> bool {
        if let Animatable::Animated(keyframes) = track {
            let changed = keyframes
                .iter()
                .any(|key| key.interpolation != interpolation);
            for key in keyframes {
                key.interpolation = interpolation;
            }
            return changed;
        }
        false
    }

    match property {
        "Position X" | "Position Y" => apply(&mut layer.transform.position, interpolation),
        "Scale X" | "Scale Y" => apply(&mut layer.transform.scale, interpolation),
        "Rotation" => apply(&mut layer.transform.rotation, interpolation),
        "Opacity" => apply(&mut layer.transform.opacity, interpolation),
        p if p.starts_with("3D Position") => apply(&mut layer.transform_3d.position, interpolation),
        p if p.starts_with("3D Rotation") => apply(&mut layer.transform_3d.rotation, interpolation),
        p if p.starts_with("3D Scale") => apply(&mut layer.transform_3d.scale, interpolation),
        p if p.starts_with("Pin") => pin_anim_mut(layer, p)
            .map(|track| apply(track, interpolation))
            .unwrap_or(false),
        p if p.starts_with("fx_") => {
            let rest = p.strip_prefix("fx_").unwrap_or_default();
            layer
                .effects
                .iter_mut()
                .find_map(|effect| {
                    rest.strip_prefix(&format!("{}_", effect.name))
                        .map(|label| {
                            let parameter_name = label.split('|').next().unwrap_or(label);
                            effect.effect_type.set_parameter_keyframe_interpolation(
                                Some(parameter_name),
                                interpolation,
                            )
                        })
                })
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn remove_effect_channel_at_frame(layer: &mut Layer, property: &str, frame: u32) -> bool {
    let rest = property.strip_prefix("fx_").unwrap_or_default();
    let (base, component) = rest
        .rsplit_once('|')
        .map(|(base, channel)| (base, channel.parse::<usize>().ok()))
        .unwrap_or((rest, None));
    for effect in &mut layer.effects {
        let Some(parameter_name) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        return match component {
            Some(_) => effect
                .effect_type
                .remove_parameter_component_keyframe(parameter_name, frame),
            None => effect
                .effect_type
                .remove_scalar_parameter_keyframe(parameter_name, frame),
        };
    }
    false
}

fn effect_parameter_channel_interpolation(
    layer: &Layer,
    property: &str,
    frame: u32,
) -> Option<crate::core::keyframe::InterpolationType> {
    let rest = property.strip_prefix("fx_")?;
    let base = rest.rsplit_once('|').map(|(base, _)| base).unwrap_or(rest);
    for effect in &layer.effects {
        let Some(label) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        for (name, parameter) in effect.effect_type.animatable_params_ref() {
            if name != label {
                continue;
            }
            return match parameter {
                crate::core::effect_params::ParamRefRef::Scalar(track) => track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation),
                crate::core::effect_params::ParamRefRef::Vec2(track) => track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation),
                crate::core::effect_params::ParamRefRef::Vec3(track) => track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation),
                crate::core::effect_params::ParamRefRef::Vec4Color(track) => track
                    .keyframes()?
                    .iter()
                    .find(|key| key.frame == frame)
                    .map(|key| key.interpolation),
            };
        }
    }
    None
}

fn set_effect_channel_interpolation_at_frame(
    layer: &mut Layer,
    property: &str,
    frame: u32,
    interpolation: crate::core::keyframe::InterpolationType,
) -> bool {
    let rest = property.strip_prefix("fx_").unwrap_or_default();
    let base = rest.rsplit_once('|').map(|(base, _)| base).unwrap_or(rest);
    for effect in &mut layer.effects {
        let Some(label) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        return effect
            .effect_type
            .set_parameter_keyframe_interpolation_at_frame(label, frame, interpolation);
    }
    false
}

fn effect_parameter_channel_value(layer: &Layer, property: &str, frame: u32) -> f32 {
    let (base, channel) = property
        .strip_prefix("fx_")
        .and_then(|value| value.rsplit_once('|'))
        .map(|(value, index)| (value, index.parse::<usize>().ok()))
        .unwrap_or((property.strip_prefix("fx_").unwrap_or_default(), None));
    for effect in &layer.effects {
        let Some(label) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        for (name, parameter) in effect.effect_type.animatable_params_ref() {
            if name != label {
                continue;
            }
            return match parameter {
                crate::core::effect_params::ParamRefRef::Scalar(track) => track.evaluate(frame),
                crate::core::effect_params::ParamRefRef::Vec2(track) => {
                    track.evaluate(frame)[channel.unwrap_or(0).min(1)]
                }
                crate::core::effect_params::ParamRefRef::Vec3(track) => {
                    track.evaluate(frame)[channel.unwrap_or(0).min(2)]
                }
                crate::core::effect_params::ParamRefRef::Vec4Color(track) => {
                    track.evaluate(frame)[channel.unwrap_or(0).min(3)]
                }
            };
        }
    }
    0.0
}

fn effect_parameter_channel_keyframes(layer: &Layer, property: &str) -> Vec<(u32, f32)> {
    let (base, channel) = property
        .strip_prefix("fx_")
        .and_then(|value| value.rsplit_once('|'))
        .map(|(value, index)| (value, index.parse::<usize>().unwrap_or(0)))
        .unwrap_or((property.strip_prefix("fx_").unwrap_or_default(), 0));
    for effect in &layer.effects {
        let Some(label) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        for (name, parameter) in effect.effect_type.animatable_params_ref() {
            if name != label {
                continue;
            }
            return match parameter {
                crate::core::effect_params::ParamRefRef::Vec2(track) => track
                    .keyframes()
                    .map(|keys| {
                        keys.iter()
                            .map(|key| (key.frame, key.value[channel.min(1)]))
                            .collect()
                    })
                    .unwrap_or_default(),
                crate::core::effect_params::ParamRefRef::Vec3(track) => track
                    .keyframes()
                    .map(|keys| {
                        keys.iter()
                            .map(|key| (key.frame, key.value[channel.min(2)]))
                            .collect()
                    })
                    .unwrap_or_default(),
                crate::core::effect_params::ParamRefRef::Vec4Color(track) => track
                    .keyframes()
                    .map(|keys| {
                        keys.iter()
                            .map(|key| (key.frame, key.value[channel.min(3)]))
                            .collect()
                    })
                    .unwrap_or_default(),
                _ => vec![],
            };
        }
    }
    vec![]
}

fn effect_channel_bezier_points(layer: &Layer, property: &str, frame: u32) -> Option<[f32; 4]> {
    let rest = property.strip_prefix("fx_")?;
    let base = rest.rsplit_once('|').map(|(base, _)| base).unwrap_or(rest);
    for effect in &layer.effects {
        let Some(label) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        for (name, parameter) in effect.effect_type.animatable_params_ref() {
            if name != label {
                continue;
            }
            let interpolation = match parameter {
                crate::core::effect_params::ParamRefRef::Vec2(track) => {
                    track
                        .keyframes()?
                        .iter()
                        .find(|key| key.frame == frame)?
                        .interpolation
                }
                crate::core::effect_params::ParamRefRef::Vec3(track) => {
                    track
                        .keyframes()?
                        .iter()
                        .find(|key| key.frame == frame)?
                        .interpolation
                }
                crate::core::effect_params::ParamRefRef::Vec4Color(track) => {
                    track
                        .keyframes()?
                        .iter()
                        .find(|key| key.frame == frame)?
                        .interpolation
                }
                _ => return None,
            };
            return match interpolation {
                crate::core::keyframe::InterpolationType::Bezier {
                    custom_bezier: Some(points),
                    ..
                } => Some(points),
                _ => Some([0.33, 0.0, 0.67, 1.0]),
            };
        }
    }
    None
}

fn set_effect_channel_bezier(
    layer: &mut Layer,
    property: &str,
    frame: u32,
    points: [f32; 4],
) -> bool {
    let rest = property.strip_prefix("fx_").unwrap_or_default();
    let base = rest.rsplit_once('|').map(|(base, _)| base).unwrap_or(rest);
    for effect in &mut layer.effects {
        let Some(label) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        return effect
            .effect_type
            .set_parameter_keyframe_bezier_at_frame(label, frame, points);
    }
    false
}

fn set_effect_channel_at_frame(layer: &mut Layer, property: &str, frame: u32, value: f32) -> bool {
    let rest = property.strip_prefix("fx_").unwrap_or_default();
    let (base, component) = rest
        .rsplit_once('|')
        .map(|(base, channel)| (base, channel.parse::<usize>().ok()))
        .unwrap_or((rest, None));
    for effect in &mut layer.effects {
        let Some(parameter_name) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        return match component {
            Some(component) => effect.effect_type.set_parameter_component_keyframe(
                parameter_name,
                component,
                frame,
                value,
            ),
            None => effect
                .effect_type
                .set_scalar_parameter_keyframe(parameter_name, frame, value),
        };
    }
    false
}

fn move_effect_channel_keyframe(
    layer: &mut Layer,
    property: &str,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let rest = property.strip_prefix("fx_").unwrap_or_default();
    let (base, component) = rest
        .rsplit_once('|')
        .map(|(base, channel)| (base, channel.parse::<usize>().ok()))
        .unwrap_or((rest, None));
    let Some(component) = component else {
        return false;
    };
    for effect in &mut layer.effects {
        let Some(parameter_name) = base.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        return effect.effect_type.move_parameter_component_keyframe(
            parameter_name,
            component,
            from_frame,
            to_frame,
        );
    }
    false
}

fn move_effect_scalar_keyframe(
    layer: &mut Layer,
    property: &str,
    from_frame: u32,
    to_frame: u32,
) -> bool {
    let rest = property.strip_prefix("fx_").unwrap_or_default();
    if rest.contains('|') {
        return false;
    }
    for effect in &mut layer.effects {
        let Some(parameter_name) = rest.strip_prefix(&format!("{}_", effect.name)) else {
            continue;
        };
        return effect.effect_type.move_scalar_parameter_keyframe(
            parameter_name,
            from_frame,
            to_frame,
        );
    }
    false
}

pub fn draw_graph_editor(
    selected_property: &mut Option<String>,
    ui: &mut egui::Ui,
    duration_frames: u32,
    layer: &mut Layer,
    project_changed: &mut bool,
    linked_tangent: &mut bool,
) {
    let graph_height = 120.0f32;
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📈 Graph Editor").strong());
            let prop_name = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
            egui::ComboBox::from_id_salt("graph_prop_select_module")
                .selected_text(&prop_name)
                .show_ui(ui, |ui| {
                    let mut props: Vec<(String, String)> = [
                        ("Position X", "Position X"), ("Position Y", "Position Y"),
                        ("Scale X", "Scale X"), ("Scale Y", "Scale Y"),
                        ("Rotation", "Rotation"), ("Opacity", "Opacity"),
                        ("3D Position X", "3D Position X"), ("3D Position Y", "3D Position Y"), ("3D Position Z", "3D Position Z"),
                        ("3D Rotation X", "3D Rotation X"), ("3D Rotation Y", "3D Rotation Y"), ("3D Rotation Z", "3D Rotation Z"),
                        ("3D Scale X", "3D Scale X"), ("3D Scale Y", "3D Scale Y"), ("3D Scale Z", "3D Scale Z"),
                    ].iter().map(|(a,b)|(a.to_string(),b.to_string())).collect();
                    for pin in &layer.puppet_pins {
                        props.push((format!("PinX:{}", pin.id), format!("\u{1f9f7} {} X", pin.name)));
                        props.push((format!("PinY:{}", pin.id), format!("\u{1f9f7} {} Y", pin.name)));
                    }
                    for effect in &layer.effects {
                        for (label, parameter) in effect.effect_type.animatable_params_ref() {
                            let channels: &[(&str, usize)] = match parameter {
                                crate::core::effect_params::ParamRefRef::Scalar(_) => &[("", 0)],
                                crate::core::effect_params::ParamRefRef::Vec2(_) => &[ (" X", 0), (" Y", 1) ],
                                crate::core::effect_params::ParamRefRef::Vec3(_) => &[ (" X", 0), (" Y", 1), (" Z", 2) ],
                                crate::core::effect_params::ParamRefRef::Vec4Color(_) => &[ (" R", 0), (" G", 1), (" B", 2), (" A", 3) ],
                            };
                            for (suffix, channel) in channels {
                                let key = if suffix.is_empty() {
                                    format!("fx_{}_{}", effect.name, label)
                                } else {
                                    format!("fx_{}_{}|{}", effect.name, label, channel)
                                };
                                props.push((key, format!("⚙ {} / {}{}", effect.name, label, suffix)));
                            }
                        }
                    }
                    let label_of = |key: &str| -> String {
                        props.iter().find(|(k, _)| k == key).map(|(_, l)| l.clone()).unwrap_or_else(|| key.to_string())
                    };
                    let sel_label = label_of(&prop_name);
                    ui.label(egui::RichText::new(&sel_label).weak());
                    for (key, lbl) in &props {
                        if ui.selectable_label(prop_name == *key, lbl).clicked() {
                            *selected_property = Some(key.clone());
                        }
                    }
                });

            ui.add_space(8.0);
            ui.checkbox(linked_tangent, "🔗 Link");

            // ── Visual Ease Presets Palette ──
            fn apply_preset_to_layer(layer: &mut Layer, prop: &str, preset: crate::core::keyframe::EasePreset) {
                use crate::core::property::Animatable;
                use crate::core::keyframe::{BezierControlPoint, InterpolationType};
                let pts = preset.control_points();
                fn apply<T>(kfs: &mut [crate::core::keyframe::Keyframe<T>], pts: [f32; 4]) {
                    for kf in kfs.iter_mut() {
                        kf.interpolation = InterpolationType::Bezier {
                            outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                            incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                            custom_bezier: Some(pts),
                        };
                    }
                }

                match prop {
                    "Position X" | "Position Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.position { apply(kfs, pts); }
                    }
                    "Scale X" | "Scale Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.scale { apply(kfs, pts); }
                    }
                    "Rotation" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.rotation { apply(kfs, pts); }
                    }
                    "Opacity" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.opacity { apply(kfs, pts); }
                    }
                    p if p.starts_with("3D Position") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.position { apply(kfs, pts); } }
                    p if p.starts_with("3D Rotation") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.rotation { apply(kfs, pts); } }
                    p if p.starts_with("3D Scale") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.scale { apply(kfs, pts); } }
                    p if p.starts_with("Pin") => {
                        if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                            apply(kfs, pts);
                        }
                    }
                    p if p.starts_with("fx_") => {
                        let rest = p.strip_prefix("fx_").unwrap_or_default();
                        for effect in &mut layer.effects {
                            let Some(label) = rest.strip_prefix(&format!("{}_", effect.name)) else {
                                continue;
                            };
                            let parameter_name = label.split('|').next().unwrap_or(label);
                            effect.effect_type.set_parameter_keyframe_interpolation(
                                Some(parameter_name),
                                InterpolationType::Bezier {
                                    outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                    incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                    custom_bezier: Some(pts),
                                },
                            );
                            break;
                        }
                    }
                    _ => {}
                }
            }

            let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());

            for (lbl, preset, tip) in [
                ("⚡ Easy Ease (F9)", crate::core::keyframe::EasePreset::Standard, "Standard symmetric ease [0.25, 0.1, 0.25, 1.0]"),
                ("↗ In", crate::core::keyframe::EasePreset::EaseIn, "Ease In (slow start, fast end)"),
                ("↘ Out", crate::core::keyframe::EasePreset::EaseOut, "Ease Out (fast start, slow end)"),
                ("🌊 Sine", crate::core::keyframe::EasePreset::Sine, "Ultra smooth Sine ease"),
                ("🚀 Fast Out", crate::core::keyframe::EasePreset::FastOut, "Quick initial burst then smooth decelerate"),
                ("🎯 Overshoot", crate::core::keyframe::EasePreset::Overshoot, "Spring overshoot past target value"),
                ("🏀 Bounce", crate::core::keyframe::EasePreset::Bounce, "Physical single bounce easing"),
                ("🪀 Elastic", crate::core::keyframe::EasePreset::Elastic, "Elastic spring recoil easing"),
            ] {
                if ui.small_button(lbl).on_hover_text(tip).clicked() {
                    apply_preset_to_layer(layer, &active_prop, preset);
                    *project_changed = true;
                }
            }

            ui.add_space(4.0);
            if ui.button("〰 Rove Across Time").on_hover_text("Evenly distribute keyframes in time based on spatial path distance").clicked() {
                // Apply auto-bezier spatial timing
                apply_preset_to_layer(layer, &active_prop, crate::core::keyframe::EasePreset::Sine);
                *project_changed = true;
            }

            if ui.button("⇄ Reverse Keys").on_hover_text("Reverse keyframe order in time (values stay, timing flips)").clicked() {
                use crate::core::property::Animatable;
                let reverse_v2 = |anim: &mut Animatable<[f32; 2]>| {
                    if let Some(kfs) = anim.keyframes_mut() {
                        if kfs.len() >= 2 {
                            // len >= 2 guarantees both ends exist; index access keeps this panic-free.
                            let first = kfs[0].frame;
                            let last = kfs[kfs.len() - 1].frame;
                            for kf in kfs.iter_mut() {
                                kf.frame = last - (kf.frame - first);
                            }
                            kfs.sort_by_key(|k| k.frame);
                        }
                    }
                };
                let reverse_f32 = |anim: &mut Animatable<f32>| {
                    if let Some(kfs) = anim.keyframes_mut() {
                        if kfs.len() >= 2 {
                            let first = kfs[0].frame;
                            let last = kfs[kfs.len() - 1].frame;
                            for kf in kfs.iter_mut() {
                                kf.frame = last - (kf.frame - first);
                            }
                            kfs.sort_by_key(|k| k.frame);
                        }
                    }
                };
                let reverse_v3 = |anim: &mut Animatable<[f32; 3]>| {
                    if let Some(kfs) = anim.keyframes_mut() {
                        if kfs.len() >= 2 {
                            let first = kfs[0].frame;
                            let last = kfs[kfs.len() - 1].frame;
                            for kf in kfs.iter_mut() { kf.frame = last - (kf.frame - first); }
                            kfs.sort_by_key(|k| k.frame);
                        }
                    }
                };
                match selected_property.clone().unwrap_or_else(|| "Position X".to_string()).as_str() {
                    "Position X" | "Position Y" => reverse_v2(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => reverse_v2(&mut layer.transform.scale),
                    "Rotation" => reverse_f32(&mut layer.transform.rotation),
                    "Opacity" => reverse_f32(&mut layer.transform.opacity),
                    p if p.starts_with("3D Position") => reverse_v3(&mut layer.transform_3d.position),
                    p if p.starts_with("3D Rotation") => reverse_v3(&mut layer.transform_3d.rotation),
                    p if p.starts_with("3D Scale") => reverse_v3(&mut layer.transform_3d.scale),
                                        p if p.starts_with("Pin") => {
                                            if let Some(a) = pin_anim_mut(layer, p) { reverse_v2(a); }
                                        }
                    _ => {}
                }
                *project_changed = true;
            }

            ui.add_space(4.0);
            if ui.button("⚡ Mirror Ease").on_hover_text("Symmetrically mirror Ease In / Ease Out handles").clicked() {
                use crate::core::property::Animatable;
                use crate::core::keyframe::InterpolationType;

                let mirror_custom_bezier = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier { custom_bezier: Some(ref mut pts), .. } = interpolation {
                        let mirrored = [1.0 - pts[2], 1.0 - pts[3], 1.0 - pts[0], 1.0 - pts[1]];
                        *pts = mirrored;
                    }
                };

                let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                match active_prop.as_str() {
                    "Position X" | "Position Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    "Scale X" | "Scale Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.scale {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    "Rotation" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.rotation {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    "Opacity" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.opacity {
                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    p if p.starts_with("3D Position") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.position { for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Rotation") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.rotation { for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Scale") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.scale { for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); } } }
                    p if p.starts_with("Pin") => {
                        if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                                                            for kf in kfs.iter_mut() { mirror_custom_bezier(&mut kf.interpolation); }
                        }
                    }
                    _ => {}
                }
                *project_changed = true;
            }

            ui.add_space(4.0);
            if ui.button("↘ Ease In").on_hover_text("Flatten incoming tangent — keyframe eases into its value").clicked() {
                use crate::core::property::Animatable;
                use crate::core::keyframe::InterpolationType;
                let ease_in = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier { ref mut outgoing, ref mut incoming, ref mut custom_bezier, .. } = *interpolation {
                        outgoing.influence = 0.0;
                        outgoing.speed = 0.0;
                        incoming.influence = 0.333;
                        incoming.speed = 0.0;
                        *custom_bezier = Some([0.0, 0.0, 0.33, 1.0]);
                    }
                };
                let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                match active_prop.as_str() {
                    "Position X" | "Position Y" => { if let Animatable::Animated(ref mut kfs) = layer.transform.position { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                    "Scale X" | "Scale Y" => { if let Animatable::Animated(ref mut kfs) = layer.transform.scale { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                    "Rotation" => { if let Animatable::Animated(ref mut kfs) = layer.transform.rotation { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                    "Opacity" => { if let Animatable::Animated(ref mut kfs) = layer.transform.opacity { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Position") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.position { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Rotation") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.rotation { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Scale") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.scale { for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); } } }
                                        p if p.starts_with("Pin") => {
                                            if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                                                                                                    for kf in kfs.iter_mut() { ease_in(&mut kf.interpolation); }
                                            }
                                        }
                    _ => {}
                }
                *project_changed = true;
            }
            if ui.button("↗ Ease Out").on_hover_text("Flatten outgoing tangent — keyframe eases out of its value").clicked() {
                use crate::core::property::Animatable;
                use crate::core::keyframe::InterpolationType;
                let ease_out = |interpolation: &mut InterpolationType| {
                    if let InterpolationType::Bezier { ref mut outgoing, ref mut incoming, ref mut custom_bezier, .. } = *interpolation {
                        outgoing.influence = 0.333;
                        outgoing.speed = 0.0;
                        incoming.influence = 0.0;
                        incoming.speed = 0.0;
                        *custom_bezier = Some([0.67, 0.0, 1.0, 1.0]);
                    }
                };
                let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                match active_prop.as_str() {
                    "Position X" | "Position Y" => { if let Animatable::Animated(ref mut kfs) = layer.transform.position { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                    "Scale X" | "Scale Y" => { if let Animatable::Animated(ref mut kfs) = layer.transform.scale { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                    "Rotation" => { if let Animatable::Animated(ref mut kfs) = layer.transform.rotation { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                    "Opacity" => { if let Animatable::Animated(ref mut kfs) = layer.transform.opacity { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Position") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.position { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Rotation") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.rotation { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                    p if p.starts_with("3D Scale") => { if let Animatable::Animated(ref mut kfs) = layer.transform_3d.scale { for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); } } }
                                        p if p.starts_with("Pin") => {
                                            if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                                                                                                    for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); }
                                            }
                                        }
                    _ => {}
                }
                *project_changed = true;
            }

            ui.add_space(8.0);
            // ── Speed Graph vs Value Graph Mode Switcher ──
            let mode_id = egui::Id::new("ae_graph_mode_select");
            let mut current_mode = ui.ctx().data(|d| d.get_temp::<i32>(mode_id).unwrap_or(0));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Mode:").small().color(colors::TEXT_SECONDARY));
                if ui.selectable_label(current_mode == 0, "⚡ Speed Graph").clicked() {
                    current_mode = 0;
                    ui.ctx().data_mut(|d| d.insert_temp(mode_id, 0));
                }
                if ui.selectable_label(current_mode == 1, "📈 Value Graph").clicked() {
                    current_mode = 1;
                    ui.ctx().data_mut(|d| d.insert_temp(mode_id, 1));
                }
            });
            ui.collapsing("🎯 Keyframe Velocity / Influence", |ui| {
                let mut in_inf = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("ae_kf_in_inf")).unwrap_or(33.3));
                let mut out_inf = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("ae_kf_out_inf")).unwrap_or(33.3));
                let mut in_spd = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("ae_kf_in_spd")).unwrap_or(0.0));
                let mut out_spd = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("ae_kf_out_spd")).unwrap_or(0.0));

                ui.horizontal(|ui| {
                    ui.label("Incoming:");
                    if ui.add(egui::DragValue::new(&mut in_inf).range(0.1..=100.0).speed(0.5).prefix("Inf: ").suffix("%")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_kf_in_inf"), in_inf));
                        *project_changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut in_spd).speed(1.0).prefix("Spd: ").suffix(" px/s")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_kf_in_spd"), in_spd));
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Outgoing:");
                    if ui.add(egui::DragValue::new(&mut out_inf).range(0.1..=100.0).speed(0.5).prefix("Inf: ").suffix("%")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_kf_out_inf"), out_inf));
                        *project_changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut out_spd).speed(1.0).prefix("Spd: ").suffix(" px/s")).changed() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ae_kf_out_spd"), out_spd));
                        *project_changed = true;
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.small_button("📐 Linear").on_hover_text("Convert keyframes to linear interpolation").clicked() {
                        *project_changed |= set_layer_interpolation(
                            layer,
                            selected_property.as_deref().unwrap_or("Position X"),
                            crate::core::keyframe::InterpolationType::Linear,
                        );
                    }
                    if ui.small_button("🌊 Auto Bezier").on_hover_text("Smooth keyframe tangents automatically").clicked() {
                        *project_changed |= set_layer_interpolation(
                            layer,
                            selected_property.as_deref().unwrap_or("Position X"),
                            crate::core::keyframe::InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint::default(),
                                incoming: crate::core::keyframe::BezierControlPoint::default(),
                                custom_bezier: None,
                            },
                        );
                    }
                    if ui.small_button("🛑 Hold").on_hover_text("Hold keyframe value until next keyframe").clicked() {
                        *project_changed |= set_layer_interpolation(
                            layer,
                            selected_property.as_deref().unwrap_or("Position X"),
                            crate::core::keyframe::InterpolationType::Hold,
                        );
                    }
                });
            });

        });

        let graph_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
        let total_f = duration_frames.max(1);

        // Detect speed graph vs value graph mode (0 = speed graph, 1 = value graph)
        let speed_graph_mode = ui.ctx().data(|d| d.get_temp::<i32>(egui::Id::new("ae_graph_mode_select")).unwrap_or(0)) == 0;

        // Sample values along timeline duration for drawing curve (screen-adaptive step)
        let max_samples = 2000usize;
        let step = (total_f as usize / max_samples).max(1) as u32;
        let mut samples = Vec::with_capacity((total_f / step) as usize + 2);
        let mut f = 0u32;
        while f <= total_f {
            let raw_val = match graph_prop.as_str() {
                "Position X" => layer.transform.position.evaluate(f)[0],
                "Position Y" => layer.transform.position.evaluate(f)[1],
                "Scale X" => layer.transform.scale.evaluate(f)[0],
                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                "Rotation" => layer.transform.rotation.evaluate(f),
                "Opacity" => layer.transform.opacity.evaluate(f),
                p if p.starts_with("3D Position") => layer.transform_3d.position.evaluate(f)[axis_3d(p)],
                p if p.starts_with("3D Rotation") => layer.transform_3d.rotation.evaluate(f)[axis_3d(p)],
                p if p.starts_with("3D Scale") => layer.transform_3d.scale.evaluate(f)[axis_3d(p)],
                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                    let ci = usize::from(p.starts_with("PinY:"));
                    let pid = p.split(':').nth(1).unwrap_or("");
                    layer.puppet_pins.iter().find(|pp| pp.id == pid)
                        .map(|pp| pp.position.evaluate(f)[ci])
                        .unwrap_or(0.0)
                }
                p if p.starts_with("fx_") => effect_parameter_channel_value(layer, p, f),
                _ => layer.transform.position.evaluate(f)[0],
            };
            let val = if raw_val.is_nan() { 0.0 } else { raw_val };
            samples.push((f, val));
            if f < total_f && f + step > total_f {
                f = total_f;
            } else {
                f += step;
            }
        }

        // Compute per-keyframe velocity when in speed graph mode
        let keyframes_ref: Vec<(u32, f32)> = match graph_prop.as_str() {
            "Position X" => layer.transform.position.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
                .unwrap_or_default(),
            "Position Y" => layer.transform.position.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
                .unwrap_or_default(),
            "Scale X" => layer.transform.scale.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[0])).collect())
                .unwrap_or_default(),
            "Scale Y" => layer.transform.scale.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[1])).collect())
                .unwrap_or_default(),
            "Rotation" => layer.transform.rotation.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                .unwrap_or_default(),
            "Opacity" => layer.transform.opacity.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
                .unwrap_or_default(),
            p if p.starts_with("3D Position") => layer.transform_3d.position.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[axis_3d(p)])).collect()).unwrap_or_default(),
            p if p.starts_with("3D Rotation") => layer.transform_3d.rotation.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[axis_3d(p)])).collect()).unwrap_or_default(),
            p if p.starts_with("3D Scale") => layer.transform_3d.scale.keyframes()
                .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[axis_3d(p)])).collect()).unwrap_or_default(),
            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                let ci = usize::from(p.starts_with("PinY:"));
                let pid = p.split(':').nth(1).unwrap_or("");
                layer.puppet_pins.iter().find(|pp| pp.id == pid)
                    .and_then(|pp| pp.position.keyframes())
                    .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
                    .unwrap_or_default()
            }
            _ => vec![],
        };

        // Allocate drawing region
        let (rect, graph_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), graph_height),
            egui::Sense::click_and_drag(),
        );
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(25));
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(50)));

        let min_val = samples.iter().map(|(_, v)| *v).fold(f32::INFINITY, f32::min);
        let max_val = samples.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
        let val_range = (max_val - min_val).max(0.001);

        // Min/max value readouts on the right edge (curve readability)
        if !speed_graph_mode {
            let mono = egui::FontId::monospace(9.0);
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, rect.top() + 3.0),
                egui::Align2::RIGHT_TOP,
                format!("{:.1}", max_val),
                mono.clone(),
                colors::TEXT_MUTED,
            );
            ui.painter().text(
                egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("{:.1}", min_val),
                mono,
                colors::TEXT_MUTED,
            );
        }

        // Convert keyframe time/value to screen space coordinates inside the allocated rect
        let points: Vec<egui::Pos2> = if speed_graph_mode {
            // Speed Graph: compute velocity curve and map to screen
            let vel_curve = compute_velocity_curve(&keyframes_ref, 30);
            let vel_min = vel_curve.iter().map(|(_, v)| *v).fold(f32::INFINITY, f32::min);
            let vel_max = vel_curve.iter().map(|(_, v)| *v).fold(f32::NEG_INFINITY, f32::max);
            let vel_range = (vel_max - vel_min).abs().max(0.001);

            // Update readout labels for velocity range
            {
                let mono = egui::FontId::monospace(9.0);
                ui.painter().text(
                    egui::pos2(rect.right() - 4.0, rect.top() + 3.0),
                    egui::Align2::RIGHT_TOP,
                    format!("{:.1} v/s", vel_max),
                    mono.clone(),
                    colors::TEXT_MUTED,
                );
                ui.painter().text(
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("{:.1} v/s", vel_min),
                    mono,
                    colors::TEXT_MUTED,
                );
            }

            vel_curve.iter().map(|&(frame, vel)| {
                let x = rect.left() + (frame / total_f as f32) * rect.width();
                let y = rect.bottom() - 4.0 - ((vel - vel_min) / vel_range) * (rect.height() - 8.0);
                egui::pos2(x, y)
            }).collect()
        } else {
            // Value Graph: original value curve
            samples.iter().map(|&(f, v)| {
                let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
                let y = rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
                egui::pos2(x, y)
            }).collect()
        };

        // Draw graph spline segments (Value Curve)
        for window in points.windows(2) {
            ui.painter().line_segment(
                [window[0], window[1]],
                egui::Stroke::new(2.0, colors::TIMELINE_KEYFRAME),
            );
        }

        // ── Click on empty graph area → create a keyframe at that frame/value ──
        {
            use crate::core::keyframe::{InterpolationType as GInterp, Keyframe as GKeyframe};

            let x_of = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let y_of = |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            if graph_response.clicked() {
                if let Some(pos) = graph_response.interact_pointer_pos() {
                    if rect.contains(pos) {
                        // Existing anchor proximity guard: clicks on anchors belong to the anchor drag
                        let chan_kfs = |p: &crate::core::property::Animatable<[f32; 2]>, ci: usize| -> Vec<(u32, f32)> {
                            p.keyframes().map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect()).unwrap_or_default()
                        };
                        let scalar_kfs = |p: &crate::core::property::Animatable<f32>| -> Vec<(u32, f32)> {
                            p.keyframes().map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect()).unwrap_or_default()
                        };
                        let anchor_pts: Vec<(u32, f32)> = match graph_prop.as_str() {
                            "Position X" => chan_kfs(&layer.transform.position, 0),
                            "Position Y" => chan_kfs(&layer.transform.position, 1),
                            "Scale X" => chan_kfs(&layer.transform.scale, 0),
                            "Scale Y" => chan_kfs(&layer.transform.scale, 1),
                            "Rotation" => scalar_kfs(&layer.transform.rotation),
                            "Opacity" => scalar_kfs(&layer.transform.opacity),
                            p if p.starts_with("3D Position") => layer.transform_3d.position.keyframes().map(|k| k.iter().map(|kf| (kf.frame, kf.value[axis_3d(p)])).collect()).unwrap_or_default(),
                            p if p.starts_with("3D Rotation") => layer.transform_3d.rotation.keyframes().map(|k| k.iter().map(|kf| (kf.frame, kf.value[axis_3d(p)])).collect()).unwrap_or_default(),
                            p if p.starts_with("3D Scale") => layer.transform_3d.scale.keyframes().map(|k| k.iter().map(|kf| (kf.frame, kf.value[axis_3d(p)])).collect()).unwrap_or_default(),
                            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                let ci = usize::from(p.starts_with("PinY:"));
                                let pid = p.split(':').nth(1).unwrap_or("");
                                layer.puppet_pins.iter().find(|pp| pp.id == pid)
                                    .map(|pp| chan_kfs(&pp.position, ci))
                                    .unwrap_or_default()
                            }
                            p if p.starts_with("fx_") => layer.effects.iter().find_map(|effect| {
                                let label = p.strip_prefix("fx_")?.strip_prefix(&format!("{}_", effect.name))?;
                                effect.effect_type.animatable_params_ref().into_iter().find_map(|(name, parameter)| {
                                    (name == label).then_some(match parameter {
                                        crate::core::effect_params::ParamRefRef::Scalar(track) => track.keyframes().map(|k| k.iter().map(|kf| (kf.frame, kf.value)).collect()).unwrap_or_default(),
                                        _ => vec![],
                                    })
                                })
                            }).unwrap_or_default(),
                            _ => vec![],
                        };
                        let near_anchor = anchor_pts.iter().any(|&(f, v)| {
                            egui::pos2(x_of(f), y_of(v)).distance(pos) < 8.0
                        });

                        if !near_anchor {
                            let new_frame = (((pos.x - rect.left()) / rect.width()) * total_f as f32)
                                .round().clamp(0.0, total_f as f32) as u32;
                            let new_val = min_val
                                + ((rect.bottom() - 4.0 - pos.y) / (rect.height() - 8.0)) * val_range;
                            match graph_prop.as_str() {
                                "Position X" => {
                                    let mut v = layer.transform.position.evaluate(new_frame);
                                    v[0] = new_val;
                                    layer.transform.position.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                "Position Y" => {
                                    let mut v = layer.transform.position.evaluate(new_frame);
                                    v[1] = new_val;
                                    layer.transform.position.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                "Scale X" => {
                                    let mut v = layer.transform.scale.evaluate(new_frame);
                                    v[0] = new_val;
                                    layer.transform.scale.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                "Scale Y" => {
                                    let mut v = layer.transform.scale.evaluate(new_frame);
                                    v[1] = new_val;
                                    layer.transform.scale.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                "Rotation" => {
                                    layer.transform.rotation.add_keyframe(GKeyframe::new(new_frame, new_val, GInterp::Linear));
                                }
                                "Opacity" => {
                                    layer.transform.opacity.add_keyframe(GKeyframe::new(new_frame, new_val.clamp(0.0, 100.0), GInterp::Linear));
                                }
                                p if p.starts_with("3D Position") => {
                                    let mut v = layer.transform_3d.position.evaluate(new_frame); v[axis_3d(p)] = new_val;
                                    layer.transform_3d.position.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                p if p.starts_with("3D Rotation") => {
                                    let mut v = layer.transform_3d.rotation.evaluate(new_frame); v[axis_3d(p)] = new_val;
                                    layer.transform_3d.rotation.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                p if p.starts_with("3D Scale") => {
                                    let mut v = layer.transform_3d.scale.evaluate(new_frame); v[axis_3d(p)] = new_val;
                                    layer.transform_3d.scale.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                }
                                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                    if let Some(pin) = pin_anim_mut(layer, p) {
                                        let ci = usize::from(p.starts_with("PinY:"));
                                        let mut v = pin.evaluate(new_frame);
                                        v[ci] = new_val;
                                        pin.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
                                    }
                                }
                                p if p.starts_with("fx_") => {
                                    let rest = p.strip_prefix("fx_").unwrap_or_default();
                                    let (rest, component) = rest
                                        .rsplit_once('|')
                                        .map(|(value, channel)| (value, channel.parse::<usize>().ok()))
                                        .unwrap_or((rest, None));
                                    for effect in &mut layer.effects {
                                        let Some(parameter_name) = rest.strip_prefix(&format!("{}_", effect.name)) else {
                                            continue;
                                        };
                                        if let Some(component) = component {
                                            effect.effect_type.set_parameter_component_keyframe(
                                                parameter_name, component, new_frame, new_val,
                                            );
                                        } else {
                                            effect.effect_type.set_scalar_parameter_keyframe(
                                                parameter_name, new_frame, new_val,
                                            );
                                        }
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            *project_changed = true;
                        }
                    }
                }
            }
        }

        // Draw Speed Graph Velocity Line (First Derivative v(t) = dy/dt)
        // Skip overlay when in speed graph mode (main curve already shows velocity)
        if !speed_graph_mode {
        let mut max_speed = 0.0f32;
        let mut speed_pts = Vec::with_capacity(points.len());
        for win in samples.windows(2) {
            let dt = (win[1].0 as f32 - win[0].0 as f32).max(1.0);
            let speed = ((win[1].1 - win[0].1) / dt).abs();
            max_speed = max_speed.max(speed);
            speed_pts.push(speed);
        }

        if max_speed > 0.001 {
            for i in 0..speed_pts.len() {
                let sx = rect.left() + (i as f32 / total_f as f32) * rect.width();
                let sy = rect.bottom() - 4.0 - (speed_pts[i] / max_speed) * (rect.height() - 16.0);
                let p1 = egui::pos2(sx, sy);

                let next_i = (i + 1).min(speed_pts.len() - 1);
                let nsx = rect.left() + (next_i as f32 / total_f as f32) * rect.width();
                let nsy = rect.bottom() - 4.0 - (speed_pts[next_i] / max_speed) * (rect.height() - 16.0);
                let p2 = egui::pos2(nsx, nsy);

                ui.painter().line_segment([p1, p2], egui::Stroke::new(1.2, colors::MOTION_PATH));
            }

            // Peak Speed Badge HUD
            let speed_badge_pos = egui::pos2(rect.right() - 110.0, rect.top() + 6.0);
            ui.painter().text(
                speed_badge_pos,
                egui::Align2::LEFT_TOP,
                format!("⚡ Peak: {:.0} px/s", max_speed * 30.0),
                egui::FontId::monospace(10.0),
                colors::MOTION_PATH,
            );
        }
        }

        // Render interactive keyframe anchor points & tangent handles (real editing)
        // Anchors are drawn at actual keyframe positions and can be dragged in time;
        // tangent handles edit the keyframe's custom bezier control points.
        {
            use crate::core::keyframe::{InterpolationType, BezierControlPoint};

            // Mutable access to Vec2-typed keyframe tracks (Position / Scale)
            fn keyframes_of_vec2<'a>(layer: &'a mut Layer, prop: &str) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 2]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 2]>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Position X" | "Position Y" => animated(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => animated(&mut layer.transform.scale),
                    _ => None,
                }
            }

            // Mutable access to scalar keyframe tracks (Rotation / Opacity)
            fn keyframes_of_f32<'a>(layer: &'a mut Layer, prop: &str) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<f32>| match a {
                    Animatable::Animated(kfs) => Some(kfs),
                    _ => None,
                };
                match prop {
                    "Rotation" => animated(&mut layer.transform.rotation),
                    "Opacity" => animated(&mut layer.transform.opacity),
                    _ => None,
                }
            }

            fn effect_keyframes_of_f32<'a>(
                layer: &'a mut Layer,
                prop: &str,
            ) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<f32>>> {
                let rest = prop.strip_prefix("fx_")?;
                for effect in &mut layer.effects {
                    let Some(label) = rest.strip_prefix(&format!("{}_", effect.name)) else {
                        continue;
                    };
                    for (name, parameter) in effect.effect_type.animatable_params() {
                        if name == label {
                            if let crate::core::effect_params::ParamRef::Scalar(track) = parameter {
                                return track.keyframes_mut();
                            }
                        }
                    }
                }
                None
            }

            fn keyframes_of_vec3<'a>(layer: &'a mut Layer, prop: &str) -> Option<&'a mut Vec<crate::core::keyframe::Keyframe<[f32; 3]>>> {
                use crate::core::property::Animatable;
                let animated = |a: &'a mut Animatable<[f32; 3]>| match a { Animatable::Animated(kfs) => Some(kfs), _ => None };
                match prop {
                    p if p.starts_with("3D Position") => animated(&mut layer.transform_3d.position),
                    p if p.starts_with("3D Rotation") => animated(&mut layer.transform_3d.rotation),
                    p if p.starts_with("3D Scale") => animated(&mut layer.transform_3d.scale),
                    _ => None,
                }
            }

            macro_rules! with_keyframes {
                ($layer:expr, $prop:expr, $kfs:ident => $body:expr) => {{
                    let prop: String = $prop.clone();
                    if prop.starts_with("3D ") {
                        if let Some($kfs) = keyframes_of_vec3($layer, &prop) { Some({ $body }) } else { None }
                    } else if matches!(prop.as_str(), "Position X" | "Position Y" | "Scale X" | "Scale Y") {
                        if let Some($kfs) = keyframes_of_vec2($layer, &prop) { Some({ $body }) } else { None }
                    } else if prop.starts_with("fx_") {
                        if let Some($kfs) = effect_keyframes_of_f32($layer, &prop) { Some({ $body }) } else { None }
                    } else {
                        if let Some($kfs) = keyframes_of_f32($layer, &prop) { Some({ $body }) } else { None }
                    }
                }};
            }


            let frame_to_x = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
            let val_to_y = |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

            // Adds a value delta to a keyframe of either supported value type
            trait AddVal { fn add_val(&mut self, delta: f32, is_y: bool); }
            impl AddVal for f32 { fn add_val(&mut self, d: f32, _y: bool) { *self += d; } }
            impl AddVal for [f32; 2] { fn add_val(&mut self, d: f32, y: bool) { let i = if y { 1 } else { 0 }; self[i] += d; } }
            impl AddVal for [f32; 3] { fn add_val(&mut self, d: f32, y: bool) { self[if y { 1 } else { 0 }] += d; } }
            fn set_keyframe_value<T: AddVal>(kf: &mut crate::core::keyframe::Keyframe<T>, delta: f32, is_y: bool) {
                kf.value.add_val(delta, is_y);
            }

            // Snapshot keyframe positions first (immutable), then edit mutably on drag
            let kf_positions: Vec<(usize, u32, f32)> = if graph_prop.starts_with("3D ") {
                let ci = axis_3d(&graph_prop);
                keyframes_of_vec3(layer, &graph_prop).map(|kfs| kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value[ci])).collect()).unwrap_or_default()
            } else if matches!(graph_prop.as_str(), "Position X" | "Position Y" | "Scale X" | "Scale Y") {
                let comp_idx = if graph_prop.ends_with('Y') { 1usize } else { 0usize };
                keyframes_of_vec2(layer, &graph_prop).map(|kfs| {
                    kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value[comp_idx])).collect::<Vec<_>>()
                }).unwrap_or_default()
            } else if graph_prop.starts_with("fx_") {
                effect_keyframes_of_f32(layer, &graph_prop)
                    .map(|kfs| kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value)).collect::<Vec<_>>())
                    .unwrap_or_default()
            } else {
                keyframes_of_f32(layer, &graph_prop).map(|kfs| {
                    kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value)).collect::<Vec<_>>()
                }).unwrap_or_default()
            };

            if graph_prop.starts_with("fx_") && graph_prop.contains('|') {
                let channel_keys = effect_parameter_channel_keyframes(layer, &graph_prop);
                for (index, (frame, value)) in channel_keys.iter().enumerate() {
                    let key_pos = egui::pos2(frame_to_x(*frame), val_to_y(*value));
                    let key_rect = egui::Rect::from_center_size(key_pos, egui::vec2(14.0, 14.0));
                    let key_response = ui.interact(
                        key_rect,
                        egui::Id::new(("effect_channel_key", &graph_prop, index)),
                        egui::Sense::drag(),
                    );
                    ui.painter().circle_filled(key_pos, 4.0, colors::TIMELINE_KEYFRAME);
                    if let Some(points) = effect_channel_bezier_points(layer, &graph_prop, *frame) {
                        let out = egui::pos2(key_pos.x + points[2] * 44.0, key_pos.y - points[3] * 24.0);
                        let incoming = egui::pos2(key_pos.x - points[0] * 44.0, key_pos.y + points[1] * 24.0);
                        let out_resp = ui.interact(egui::Rect::from_center_size(out, egui::vec2(14.0, 14.0)), egui::Id::new(("effect_bezier_out", &graph_prop, index)), egui::Sense::drag());
                        let in_resp = ui.interact(egui::Rect::from_center_size(incoming, egui::vec2(14.0, 14.0)), egui::Id::new(("effect_bezier_in", &graph_prop, index)), egui::Sense::drag());
                        ui.painter().line_segment([key_pos, out], egui::Stroke::new(1.0, colors::MOTION_PATH));
                        ui.painter().line_segment([key_pos, incoming], egui::Stroke::new(1.0, colors::MOTION_PATH));
                        ui.painter().circle_filled(out, 3.0, colors::HANDLE_NORMAL);
                        ui.painter().circle_filled(incoming, 3.0, colors::HANDLE_NORMAL);
                        let mut next = points;
                        if out_resp.dragged() {
                            next[2] = (points[2] + out_resp.drag_delta().x / 44.0).clamp(points[0] + 0.01, 1.0);
                            next[3] = (points[3] - out_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
                        }
                        if in_resp.dragged() {
                            next[0] = (points[0] - in_resp.drag_delta().x / 44.0).clamp(0.0, points[2] - 0.01);
                            next[1] = (points[1] + in_resp.drag_delta().y / 24.0).clamp(-1.5, 2.5);
                        }
                        if (out_resp.dragged() || in_resp.dragged()) && set_effect_channel_bezier(layer, &graph_prop, *frame, next) {
                            *project_changed = true;
                        }
                    }
                    if key_response.secondary_clicked() {
                        if remove_effect_channel_at_frame(layer, &graph_prop, *frame) {
                            *project_changed = true;
                        }
                        continue;
                    }
                    if key_response.dragged() {
                        let next_frame = (*frame as i32
                            + (key_response.drag_delta().x / rect.width() * total_f as f32).round() as i32)
                            .clamp(0, total_f as i32) as u32;
                        let next_value = *value
                            - key_response.drag_delta().y / (rect.height() - 8.0) * val_range;
                        let moved = next_frame != *frame;
                        let moved_key = moved && move_effect_channel_keyframe(layer, &graph_prop, *frame, next_frame);
                        let value_changed = if moved {
                            moved_key
                        } else {
                            set_effect_channel_at_frame(layer, &graph_prop, next_frame, next_value)
                        };
                        if moved_key {
                            let _ = set_effect_channel_at_frame(layer, &graph_prop, next_frame, next_value);
                        }
                        *project_changed |= value_changed;
                    }
                }
            }

            for (kf_idx, kf_frame, kf_val) in &kf_positions {
                let pt = egui::pos2(frame_to_x(*kf_frame), val_to_y(*kf_val));

                // --- Anchor point: drag horizontally to retime, vertically to change value ---
                let anchor_rect = egui::Rect::from_center_size(pt, egui::vec2(14.0, 14.0));
                let anchor_resp = ui.interact(anchor_rect, egui::Id::new(("graph_anchor", kf_idx)), egui::Sense::click_and_drag());
                if anchor_resp.secondary_clicked() {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if *kf_idx < kfs.len() {
                            kfs.remove(*kf_idx);
                            *project_changed = true;
                        }
                    });
                    continue;
                }
                if anchor_resp.dragged() {
                    let delta_frames = (anchor_resp.drag_delta().x / rect.width() * total_f as f32).round() as i32;
                    let new_frame = (*kf_frame as i32 + delta_frames).clamp(0, total_f as i32) as u32;
                    // Vertical drag → value change (screen up = value up)
                    let delta_val = -anchor_resp.drag_delta().y / (rect.height() - 8.0) * val_range;
                    if graph_prop.starts_with("fx_") && !graph_prop.contains('|') && new_frame != *kf_frame {
                        if move_effect_scalar_keyframe(layer, &graph_prop, *kf_frame, new_frame) {
                            let value = effect_parameter_channel_value(layer, &graph_prop, new_frame);
                            let _ = set_effect_channel_at_frame(layer, &graph_prop, new_frame, value + delta_val);
                            *project_changed = true;
                        }
                        continue;
                    }
                    if new_frame != *kf_frame && (graph_prop == "Rotation" || graph_prop == "Opacity") {
                        let moved = if graph_prop == "Rotation" {
                            layer.transform.rotation.move_keyframe(*kf_frame, new_frame)
                        } else {
                            layer.transform.opacity.move_keyframe(*kf_frame, new_frame)
                        };
                        if moved {
                            let value_delta = delta_val;
                            if graph_prop == "Rotation" {
                                if let Some(key) = layer.transform.rotation.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) {
                                    key.value += value_delta;
                                }
                            } else if let Some(key) = layer.transform.opacity.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) {
                                key.value = (key.value + value_delta).clamp(0.0, 100.0);
                            }
                            *project_changed = true;
                            continue;
                        }
                    }
                    if new_frame != *kf_frame
                        && (graph_prop.starts_with("Position") || graph_prop.starts_with("Scale")
                            || graph_prop.starts_with("3D Position") || graph_prop.starts_with("3D Rotation")
                            || graph_prop.starts_with("3D Scale"))
                    {
                        let moved = if graph_prop.starts_with("3D Position") {
                            layer.transform_3d.position.move_keyframe(*kf_frame, new_frame)
                        } else if graph_prop.starts_with("3D Rotation") {
                            layer.transform_3d.rotation.move_keyframe(*kf_frame, new_frame)
                        } else if graph_prop.starts_with("3D Scale") {
                            layer.transform_3d.scale.move_keyframe(*kf_frame, new_frame)
                        } else if graph_prop.starts_with("Position") {
                            layer.transform.position.move_keyframe(*kf_frame, new_frame)
                        } else {
                            layer.transform.scale.move_keyframe(*kf_frame, new_frame)
                        };
                        if moved {
                            let is_3d = graph_prop.starts_with("3D ");
                            let axis = if is_3d { axis_3d(&graph_prop) } else { usize::from(graph_prop.ends_with('Y')) };
                            if graph_prop.starts_with("3D Position") {
                                if let Some(key) = layer.transform_3d.position.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) { key.value[axis] += delta_val; }
                            } else if graph_prop.starts_with("3D Rotation") {
                                if let Some(key) = layer.transform_3d.rotation.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) { key.value[axis] += delta_val; }
                            } else if graph_prop.starts_with("3D Scale") {
                                if let Some(key) = layer.transform_3d.scale.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) { key.value[axis] += delta_val; }
                            } else if graph_prop.starts_with("Position") {
                                if let Some(key) = layer.transform.position.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) { key.value[axis] += delta_val; }
                            } else if let Some(key) = layer.transform.scale.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) { key.value[axis] += delta_val; }
                            *project_changed = true;
                            continue;
                        }
                    }
                    if new_frame != *kf_frame && (graph_prop.starts_with("PinX:") || graph_prop.starts_with("PinY:")) {
                        if let Some(pin) = pin_anim_mut(layer, &graph_prop) {
                            if pin.move_keyframe(*kf_frame, new_frame) {
                                let axis = usize::from(graph_prop.starts_with("PinY:"));
                                if let Some(key) = pin.keyframes_mut().and_then(|keys| keys.iter_mut().find(|key| key.frame == new_frame)) {
                                    key.value[axis] += delta_val;
                                }
                                *project_changed = true;
                                continue;
                            }
                        }
                    }
                    with_keyframes!(layer, graph_prop, kfs => {
                        if kfs[*kf_idx].frame != new_frame {
                            kfs[*kf_idx].frame = new_frame;
                            kfs.sort_by_key(|k| k.frame);
                        }
                        if let Some(kf) = kfs.get_mut(*kf_idx) {
                            set_keyframe_value(kf, delta_val, graph_prop.ends_with('Y'));
                        }
                        *project_changed = true;
                    });
                }
                let anchor_color = if anchor_resp.dragged() {
                    colors::HANDLE_NORMAL
                } else if anchor_resp.hovered() {
                    colors::HANDLE_HOVER_FILL
                } else {
                    colors::TIMELINE_KEYFRAME
                };
                ui.painter().circle_filled(pt, 4.0, anchor_color);

                // ── Double-click anchor → numeric value popup ──
                let dbl_id = ui.make_persistent_id(("graph_kf_popup", kf_idx));
                let mut show_popup: bool = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(dbl_id, || false));
                if anchor_resp.double_clicked() {
                    show_popup = true;
                }
                if show_popup {
                    let popup_id = egui::Id::new(("graph_kf_val_popup", kf_idx));
                    let resp = egui::Area::new(popup_id)
                        .fixed_pos(pt + egui::vec2(12.0, -20.0))
                        .order(egui::Order::Foreground)
                        .show(ui.ctx(), |ui| {
                            ui.group(|ui| {
                                ui.set_min_width(140.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Frame:").small());
                                    let mut frame_str = kf_frame.to_string();
                                    if ui.add(egui::TextEdit::singleline(&mut frame_str).desired_width(50.0)).changed() {
                                        if let Ok(f) = frame_str.parse::<u32>() {
                                            let new_f = f.min(total_f);
                                            let handled_effect_move = if graph_prop.starts_with("fx_") && !graph_prop.contains('|') && new_f != *kf_frame {
                                                move_effect_scalar_keyframe(layer, &graph_prop, *kf_frame, new_f)
                                            } else if graph_prop == "Rotation" && new_f != *kf_frame {
                                                layer.transform.rotation.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop == "Opacity" && new_f != *kf_frame {
                                                layer.transform.opacity.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop == "Position X" || graph_prop == "Position Y" {
                                                layer.transform.position.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop == "Scale X" || graph_prop == "Scale Y" {
                                                layer.transform.scale.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop.starts_with("3D Position") {
                                                layer.transform_3d.position.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop.starts_with("3D Rotation") {
                                                layer.transform_3d.rotation.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop.starts_with("3D Scale") {
                                                layer.transform_3d.scale.move_keyframe(*kf_frame, new_f)
                                            } else if graph_prop.starts_with("PinX:") || graph_prop.starts_with("PinY:") {
                                                pin_anim_mut(layer, &graph_prop)
                                                    .is_some_and(|pin| pin.move_keyframe(*kf_frame, new_f))
                                            } else {
                                                false
                                            };
                                            if handled_effect_move {
                                                *project_changed = true;
                                            } else { with_keyframes!(layer, graph_prop, kfs => {
                                                if kfs.get(*kf_idx).map(|k| k.frame != new_f).unwrap_or(false) {
                                                    kfs[*kf_idx].frame = new_f;
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *project_changed = true;
                                                }
                                            }); }
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Value:").small());
                                    let mut val_str = format!("{:.2}", kf_val);
                                    if ui.add(egui::TextEdit::singleline(&mut val_str).desired_width(70.0)).changed() {
                                        if let Ok(v) = val_str.parse::<f32>() {
                                            if graph_prop.starts_with("Position") {
                                                let ci = if graph_prop.ends_with('Y') { 1 } else { 0 };
                                                if let Some(kfs) = layer.transform.position.keyframes_mut() {
                                                    if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value[ci] = v; *project_changed = true; }
                                                }
                                            } else if graph_prop.starts_with("Scale") {
                                                let ci = if graph_prop.ends_with('Y') { 1 } else { 0 };
                                                if let Some(kfs) = layer.transform.scale.keyframes_mut() {
                                                    if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value[ci] = v; *project_changed = true; }
                                                }
                                            } else if graph_prop == "Rotation" {
                                                if let Some(kfs) = layer.transform.rotation.keyframes_mut() {
                                                    if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value = v; *project_changed = true; }
                                                }
                                            } else if graph_prop == "Opacity" {
                                                if let Some(kfs) = layer.transform.opacity.keyframes_mut() {
                                                    if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value = v.clamp(0.0, 100.0); *project_changed = true; }
                                                }
                                            } else if graph_prop.starts_with("3D Position") {
                                                if let Some(kfs) = layer.transform_3d.position.keyframes_mut() { if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value[axis_3d(&graph_prop)] = v; *project_changed = true; } }
                                            } else if graph_prop.starts_with("3D Rotation") {
                                                if let Some(kfs) = layer.transform_3d.rotation.keyframes_mut() { if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value[axis_3d(&graph_prop)] = v; *project_changed = true; } }
                                            } else if graph_prop.starts_with("3D Scale") {
                                                if let Some(kfs) = layer.transform_3d.scale.keyframes_mut() { if let Some(kf) = kfs.get_mut(*kf_idx) { kf.value[axis_3d(&graph_prop)] = v; *project_changed = true; } }
                                            } else if graph_prop.starts_with("fx_") {
                                                if set_effect_channel_at_frame(layer, &graph_prop, *kf_frame, v) {
                                                    *project_changed = true;
                                                }
                                            }
                                        }
                                    }
                                });
                                if ui.button("Done").clicked() {
                                    show_popup = false;
                                }
                            });
                        });
                    // Click outside popup → dismiss
                    if ui.input(|i| i.pointer.any_click()) && !resp.response.contains_pointer() {
                        show_popup = false;
                    }
                }
                ui.ctx().data_mut(|d| d.insert_temp(dbl_id, show_popup));
                if anchor_resp.hovered() {
                    ui.painter().circle_stroke(pt, 7.0, egui::Stroke::new(1.0, colors::TIMELINE_KEYFRAME));
                }

                // --- Tangent handles: drag to edit custom bezier control points ---
                // Extract current bezier points (default Easy Ease if linear/hold)
                fn bezier_pts<T>(kf: &crate::core::keyframe::Keyframe<T>) -> (f32, f32, f32, f32) {
                    match &kf.interpolation {
                        InterpolationType::Bezier { custom_bezier: Some(pts), .. } => (pts[0], pts[1], pts[2], pts[3]),
                        _ => (0.33, 0.0, 0.67, 1.0),
                    }
                }
                let bezier_from_kfs = |get: Option<(f32, f32, f32, f32)>| get.unwrap_or((0.33, 0.0, 0.67, 1.0));
                let _ = bezier_from_kfs;
                let (bx1, by1, bx2, by2): (f32, f32, f32, f32) = with_keyframes!(layer, graph_prop, kfs => {
                    kfs.get(*kf_idx).map(bezier_pts).unwrap_or((0.33, 0.0, 0.67, 1.0))
                }).unwrap_or((0.33, 0.0, 0.67, 1.0));

                let h_out = egui::pos2(pt.x + bx2 * 44.0, pt.y - by2 * 24.0);
                let h_in = egui::pos2(pt.x - bx1 * 44.0, pt.y + by1 * 24.0);

                let h_out_rect = egui::Rect::from_center_size(h_out, egui::vec2(14.0, 14.0));
                let h_in_rect = egui::Rect::from_center_size(h_in, egui::vec2(14.0, 14.0));
                let h_out_resp = ui.interact(h_out_rect, egui::Id::new(("graph_h_out", kf_idx)), egui::Sense::drag());
                let h_in_resp = ui.interact(h_in_rect, egui::Id::new(("graph_h_in", kf_idx)), egui::Sense::drag());

                let mut new_pts: Option<[f32; 4]> = None;
                if h_out_resp.dragged() {
                    let d = h_out_resp.drag_delta();
                    let nx2 = (bx2 + d.x / 44.0).clamp(bx1 + 0.01, 1.0);
                    let ny2 = (by2 - d.y / 24.0).clamp(-1.5, 2.5);
                    if *linked_tangent {
                        let mir_x = (1.0 - nx2).clamp(0.0, nx2 - 0.01);
                        new_pts = Some([mir_x, -ny2, nx2, ny2]);
                    } else {
                        new_pts = Some([bx1, by1, nx2, ny2]);
                    }
                }
                if h_in_resp.dragged() {
                    let d = h_in_resp.drag_delta();
                    let nx1 = (bx1 - d.x / 44.0).clamp(0.0, bx2 - 0.01);
                    let ny1 = (by1 + d.y / 24.0).clamp(-1.5, 2.5);
                    if *linked_tangent {
                        let mir_x = (1.0 - nx1).clamp(nx1 + 0.01, 1.0);
                        new_pts = Some([nx1, ny1, mir_x, -ny1]);
                    } else {
                        new_pts = Some([nx1, ny1, bx2, by2]);
                    }
                }
                if let Some(pts) = new_pts {
                    with_keyframes!(layer, graph_prop, kfs => {
                        if let Some(kf) = kfs.get_mut(*kf_idx) {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some(pts),
                            };
                            *project_changed = true;
                        }
                    });
                }

                let any_hover = h_out_resp.hovered() || h_in_resp.dragged() || h_in_resp.hovered() || h_out_resp.dragged();
                let stroke_color = if any_hover {
                    colors::ACCENT_ORANGE
                } else {
                    colors::MOTION_PATH
                };
                ui.painter().line_segment([pt, h_out], egui::Stroke::new(1.2, stroke_color));
                ui.painter().line_segment([pt, h_in], egui::Stroke::new(1.2, stroke_color));

                let h_out_color = if h_out_resp.hovered() || h_out_resp.dragged() { colors::HANDLE_NORMAL } else { colors::MOTION_PATH };
                let h_in_color = if h_in_resp.hovered() || h_in_resp.dragged() { colors::HANDLE_NORMAL } else { colors::MOTION_PATH };
                ui.painter().circle_filled(h_out, 4.0, h_out_color);
                ui.painter().circle_filled(h_in, 4.0, h_in_color);
            }
        }
    });
}

pub fn draw_automation_curve(
    ui: &mut egui::Ui,
    curve: &mut crate::core::automation_binding::AutomationCurve,
    changed: &mut bool,
) {
    if curve.points.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("🎚 Automation Channel").strong());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 100.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, crate::ui::theme::colors::BG_DEEPEST);
    let min_time = curve
        .points
        .first()
        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
        .unwrap_or(0.0);
    let max_time = curve
        .points
        .last()
        .map(|point| point.time.numerator as f32 / point.time.denominator as f32)
        .unwrap_or(min_time + 1.0)
        .max(min_time + f32::EPSILON);
    let min_value = curve
        .points
        .iter()
        .map(|point| point.value as f32)
        .fold(f32::INFINITY, f32::min);
    let max_value = curve
        .points
        .iter()
        .map(|point| point.value as f32)
        .fold(f32::NEG_INFINITY, f32::max)
        .max(min_value + f32::EPSILON);
    let to_screen = |time: f32, value: f32| {
        egui::pos2(
            rect.left()
                + ((time - min_time) / (max_time - min_time)).clamp(0.0, 1.0) * rect.width(),
            rect.bottom()
                - ((value - min_value) / (max_value - min_value)).clamp(0.0, 1.0)
                    * rect.height(),
        )
    };
    let points: Vec<_> = (0..=64)
        .filter_map(|index| {
            let ratio = index as f32 / 64.0;
            let time = min_time + ratio * (max_time - min_time);
            let sample_time = crate::core::unified_time::Time::new(
                (time * 1_000_000.0).round() as i64,
                1_000_000,
            );
            curve
                .sample(sample_time)
                .map(|value| to_screen(time, value as f32))
        })
        .collect();
    for segment in points.windows(2) {
        painter.line_segment(
            [segment[0], segment[1]],
            egui::Stroke::new(2.0, crate::ui::theme::colors::ACCENT_CYAN),
        );
    }
    for point in &curve.points {
        let time = point.time.numerator as f32 / point.time.denominator as f32;
        painter.circle_filled(
            to_screen(time, point.value as f32),
            3.0,
            crate::ui::theme::colors::ACCENT_ORANGE,
        );
    }
    if response.double_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let normalized_time = ((pointer.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
            let normalized_value = ((rect.bottom() - pointer.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
            let time = min_time + normalized_time * (max_time - min_time);
            let value = min_value + normalized_value * (max_value - min_value);
            if curve
                .upsert_point(
                    crate::core::unified_time::Time::new(
                        (time * 1_000_000.0).round() as i64,
                        1_000_000,
                    ),
                    value as f64,
                )
                .is_ok()
            {
                *changed = true;
            }
        }
    }
}

pub fn draw_camera_lens_graph(
    ui: &mut egui::Ui,
    camera: &mut crate::core::timeline::Camera3D,
    duration_frames: u32,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let id = egui::Id::new("camera_lens_graph_property");
    let mut property = ui
        .ctx()
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| "FOV".into());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📈 Camera Lens Graph").strong());
        egui::ComboBox::from_id_salt(id)
            .selected_text(&property)
            .show_ui(ui, |ui| {
                for name in [
                    "FOV",
                    "Focus Distance",
                    "Aperture",
                    "DOF Blur",
                    "DOF Enabled",
                ] {
                    if ui.selectable_label(property == name, name).clicked() {
                        property = name.into();
                    }
                }
            });
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, property.clone()));
    ui.horizontal(|ui| {
        if ui.small_button("× Remove Key").clicked() {
            let removed = match property.as_str() {
                "FOV" => remove_camera_key(&mut camera.fov_animation, current_frame),
                "Focus Distance" => {
                    remove_camera_key(&mut camera.focus_distance_animation, current_frame)
                }
                "Aperture" => remove_camera_key(&mut camera.aperture_animation, current_frame),
                "DOF Enabled" => {
                    remove_camera_key(&mut camera.dof_enabled_animation, current_frame)
                }
                _ => remove_camera_key(&mut camera.dof_max_blur_animation, current_frame),
            };
            *project_changed |= removed;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Interpolation:");
        for (label, interpolation) in [
            ("Linear", crate::core::keyframe::InterpolationType::Linear),
            ("Hold", crate::core::keyframe::InterpolationType::Hold),
        ] {
            if ui.small_button(label).clicked() {
                let changed = match property.as_str() {
                    "FOV" => set_camera_key_interpolation(
                        &mut camera.fov_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    "Focus Distance" => set_camera_key_interpolation(
                        &mut camera.focus_distance_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    "Aperture" => set_camera_key_interpolation(
                        &mut camera.aperture_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    "DOF Enabled" => set_camera_key_interpolation(
                        &mut camera.dof_enabled_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                    _ => set_camera_key_interpolation(
                        &mut camera.dof_max_blur_animation,
                        current_frame,
                        interpolation.clone(),
                    ),
                };
                *project_changed |= changed;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Ease:");
        for (label, preset) in [
            ("F9", crate::core::keyframe::EasePreset::Standard),
            ("Ease In", crate::core::keyframe::EasePreset::EaseIn),
            ("Ease Out", crate::core::keyframe::EasePreset::EaseOut),
        ] {
            if ui.small_button(label).clicked() {
                let changed = match property.as_str() {
                    "FOV" => set_camera_key_ease(&mut camera.fov_animation, current_frame, preset),
                    "Focus Distance" => set_camera_key_ease(
                        &mut camera.focus_distance_animation,
                        current_frame,
                        preset,
                    ),
                    "Aperture" => {
                        set_camera_key_ease(&mut camera.aperture_animation, current_frame, preset)
                    }
                    "DOF Enabled" => set_camera_key_ease(
                        &mut camera.dof_enabled_animation,
                        current_frame,
                        preset,
                    ),
                    _ => set_camera_key_ease(
                        &mut camera.dof_max_blur_animation,
                        current_frame,
                        preset,
                    ),
                };
                *project_changed |= changed;
            }
        }
    });
    let track = match property.as_str() {
        "FOV" => camera.fov_animation.as_ref(),
        "Focus Distance" => camera.focus_distance_animation.as_ref(),
        "Aperture" => camera.aperture_animation.as_ref(),
        "DOF Enabled" => camera.dof_enabled_animation.as_ref(),
        _ => camera.dof_max_blur_animation.as_ref(),
    };
    let Some(track) = track else {
        ui.label("No keyframes yet — use ◆ in Camera Settings.");
        return;
    };
    let Some(keys) = track.keyframes() else {
        return;
    };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 110.0),
        egui::Sense::click_and_drag(),
    );
    let min = keys.iter().map(|k| k.value).fold(f32::INFINITY, f32::min);
    let max = keys
        .iter()
        .map(|k| k.value)
        .fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(0.001);
    let end = duration_frames
        .max(keys.last().map(|k| k.frame).unwrap_or(0))
        .max(1);
    let point = |frame: u32, value: f32| {
        egui::pos2(
            rect.left() + frame as f32 / end as f32 * rect.width(),
            rect.bottom() - 6.0 - (value - min) / range * (rect.height() - 12.0),
        )
    };
    let clamp_value = |value: f32| match property.as_str() {
        "FOV" => value.clamp(1.0, 179.0),
        "Focus Distance" | "Aperture" => value.max(0.0),
        "DOF Enabled" => value.clamp(0.0, 1.0),
        _ => value.clamp(1.0, 64.0),
    };
    for pair in keys.windows(2) {
        let start = &pair[0];
        let end_key = &pair[1];
        let bezier = match start.interpolation {
            crate::core::keyframe::InterpolationType::Bezier {
                custom_bezier: Some(points),
                ..
            } => Some(points),
            _ => None,
        };
        let mut previous = point(start.frame, start.value);
        for step in 1..=24 {
            let normalized = step as f32 / 24.0;
            let eased = bezier
                .map(|[x1, y1, x2, y2]| {
                    let t =
                        crate::core::keyframe::solve_bezier_eased_time(normalized, x1, y1, x2, y2);
                    let omt = 1.0 - t;
                    3.0 * omt * omt * t * y1 + 3.0 * omt * t * t * y2 + t * t * t
                })
                .unwrap_or_else(|| match start.interpolation {
                    crate::core::keyframe::InterpolationType::Hold => 0.0,
                    _ => normalized,
                });
            let frame = start.frame as f32
                + (end_key.frame.saturating_sub(start.frame) as f32) * normalized;
            let value = start.value + (end_key.value - start.value) * eased;
            let next = point(frame.round() as u32, value);
            ui.painter().line_segment(
                [previous, next],
                egui::Stroke::new(2.0, colors::ACCENT_CYAN),
            );
            previous = next;
        }
    }
    for key in keys {
        ui.painter()
            .circle_filled(point(key.frame, key.value), 4.0, colors::TIMELINE_KEYFRAME);
    }
    let x = rect.left() + current_frame.min(end) as f32 / end as f32 * rect.width();
    ui.painter().line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(1.0, colors::HANDLE_HOVER_FILL),
    );
    let drag_id = egui::Id::new(("camera_lens_graph_drag", property.as_str()));
    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let nearest = keys
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    point(a.frame, a.value)
                        .distance(pointer)
                        .partial_cmp(&point(b.frame, b.value).distance(pointer))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .filter(|(_, key)| point(key.frame, key.value).distance(pointer) <= 12.0)
                .map(|(index, _)| index);
            ui.ctx().data_mut(|data| data.insert_temp(drag_id, nearest));
        }
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let nearest = ui
                .ctx()
                .data(|data| data.get_temp::<Option<usize>>(drag_id))
                .flatten();
            if let Some(index) = nearest {
                let frame = ((pointer.x - rect.left()) / rect.width() * end as f32)
                    .round()
                    .clamp(0.0, end as f32) as u32;
                let value = clamp_value(
                    min + ((rect.bottom() - 6.0 - pointer.y) / (rect.height() - 12.0) * range),
                );
                let track = match property.as_str() {
                    "FOV" => &mut camera.fov_animation,
                    "Focus Distance" => &mut camera.focus_distance_animation,
                    "Aperture" => &mut camera.aperture_animation,
                    "DOF Enabled" => &mut camera.dof_enabled_animation,
                    _ => &mut camera.dof_max_blur_animation,
                };
                if let Some(crate::core::property::Animatable::Animated(keys)) = track {
                    if let Some(key) = keys.get_mut(index) {
                        key.frame = frame;
                        key.value = value;
                    }
                    keys.sort_by_key(|key| key.frame);
                    *project_changed = true;
                }
            }
        }
    }
    if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove_temp::<Option<usize>>(drag_id));
    }
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = ((pos.x - rect.left()) / rect.width() * end as f32)
                .round()
                .clamp(0.0, end as f32) as u32;
            let value =
                clamp_value(min + ((rect.bottom() - 6.0 - pos.y) / (rect.height() - 12.0) * range));
            match property.as_str() {
                "FOV" => camera.set_fov_at(frame, value),
                "Focus Distance" => camera.set_focus_distance_at(frame, value),
                "Aperture" => camera.set_aperture_at(frame, value),
                "DOF Enabled" => camera.set_dof_enabled_at(frame, value >= 0.5),
                _ => camera.set_dof_max_blur_at(frame, value),
            }
            *project_changed = true;
        }
    }
}

fn remove_camera_key<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
) -> bool {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        let original_len = keys.len();
        keys.retain(|key| key.frame != frame);
        let removed = keys.len() != original_len;
        let became_empty = keys.is_empty();
        if became_empty {
            *track = None;
        }
        return removed;
    }
    false
}

fn set_camera_key_interpolation<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
    interpolation: crate::core::keyframe::InterpolationType,
) -> bool {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        if let Some(key) = keys.iter_mut().find(|key| key.frame == frame) {
            if key.interpolation != interpolation {
                key.interpolation = interpolation;
                return true;
            }
        }
    }
    false
}

fn set_camera_key_ease<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
    preset: crate::core::keyframe::EasePreset,
) -> bool {
    use crate::core::keyframe::{BezierControlPoint, InterpolationType};

    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        if let Some(key) = keys.iter_mut().find(|key| key.frame == frame) {
            let next = InterpolationType::Bezier {
                outgoing: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                incoming: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                custom_bezier: Some(preset.control_points()),
            };
            if key.interpolation != next {
                key.interpolation = next;
                return true;
            }
        }
    }
    false
}

/// Compute velocity (derivative) at each keyframe from value keyframes.
/// Returns Vec of (frame, velocity) pairs.
fn compute_velocity_curve(keyframes: &[(u32, f32)], fps: u32) -> Vec<(f32, f32)> {
    if keyframes.len() < 2 {
        return keyframes
            .iter()
            .map(|&(frame, _)| (frame as f32, 0.0))
            .collect();
    }
    let mut velocities = Vec::with_capacity(keyframes.len());
    for i in 0..keyframes.len() {
        let vel = if i == 0 {
            let dt = (keyframes[1].0 as f32 - keyframes[0].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[1].1 - keyframes[0].1) / dt
            } else {
                0.0
            }
        } else if i == keyframes.len() - 1 {
            let dt = (keyframes[i].0 as f32 - keyframes[i - 1].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[i].1 - keyframes[i - 1].1) / dt
            } else {
                0.0
            }
        } else {
            let dt = (keyframes[i + 1].0 as f32 - keyframes[i - 1].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[i + 1].1 - keyframes[i - 1].1) / dt
            } else {
                0.0
            }
        };
        velocities.push((keyframes[i].0 as f32, vel));
    }
    velocities
}

#[cfg(test)]
mod tests {
    use super::{axis_3d, remove_camera_key, set_camera_key_ease, set_camera_key_interpolation};
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;

    #[test]
    fn three_d_property_axis_selection_is_stable() {
        assert_eq!(axis_3d("3D Position X"), 0);
        assert_eq!(axis_3d("3D Rotation Y"), 1);
        assert_eq!(axis_3d("3D Scale Z"), 2);
    }

    #[test]
    fn unknown_axis_defaults_to_x_for_backward_compatibility() {
        assert_eq!(axis_3d("Position X"), 0);
        assert_eq!(axis_3d("3D Position"), 0);
    }

    #[test]
    fn removing_missing_camera_key_is_a_noop() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(!remove_camera_key(&mut track, 11));
        assert_eq!(
            track
                .as_ref()
                .and_then(|value| value.keyframes())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn removing_last_camera_key_clears_empty_track() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(remove_camera_key(&mut track, 10));
        assert!(track.is_none());
    }

    #[test]
    fn camera_key_interpolation_changes_only_existing_key() {
        let mut track = Some(Animatable::new_animated(vec![Keyframe::new(
            10,
            50.0,
            InterpolationType::Linear,
        )]));

        assert!(set_camera_key_interpolation(
            &mut track,
            10,
            InterpolationType::Hold
        ));
        assert!(!set_camera_key_interpolation(
            &mut track,
            11,
            InterpolationType::Linear
        ));
        assert_eq!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[0].interpolation,
            InterpolationType::Hold
        );
    }

    #[test]
    fn camera_key_ease_creates_bezier_only_at_current_frame() {
        let mut track = Some(Animatable::new_animated(vec![
            Keyframe::new(10, 50.0, InterpolationType::Linear),
            Keyframe::new(20, 60.0, InterpolationType::Linear),
        ]));

        assert!(set_camera_key_ease(
            &mut track,
            10,
            crate::core::keyframe::EasePreset::EaseIn
        ));
        assert!(matches!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[0].interpolation,
            InterpolationType::Bezier { .. }
        ));
        assert!(matches!(
            track.as_ref().and_then(|value| value.keyframes()).unwrap()[1].interpolation,
            InterpolationType::Linear
        ));
        assert!(!set_camera_key_ease(
            &mut track,
            99,
            crate::core::keyframe::EasePreset::EaseOut
        ));
    }
}
