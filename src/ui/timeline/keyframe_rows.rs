//! Expanded-layer keyframe rows: transform props, effects, time remap,
//! masks, reveal-mode rows, and nested PreComp children rendering.
use super::layers::{draw_prop_row, draw_prop_row_ext};
use super::utils::get_kfs;
use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

#[allow(clippy::too_many_arguments)]
pub fn draw_expanded_rows(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    i: usize,
    selected_keyframes: &mut std::collections::HashSet<(usize, String, u32)>,
    selected_property: &mut Option<String>,
    current_frame: &mut u32,
    start_frame: u32,
    zoom_span: u32,
    left_pane_w: f32,
    total_frames: u32,
    all_kf_frames: &[u32],
    precomp_children: &[(usize, Vec<super::precomp_children::PreCompChild>, String)],
    open_comp_request: &mut Option<String>,
    project_changed: &mut bool,
) {
    // Keyframe drag-to-move: mutator maps a prop label to its Animatable
    fn move_kf<T: Clone + crate::core::property::Interpolate>(
        anim: &mut crate::core::property::Animatable<T>,
        old_f: u32,
        new_f: u32,
    ) {
        if old_f == new_f {
            return;
        }
        if let Some(kfs) = anim.keyframes_mut() {
            // If another keyframe already occupies new_f, remove it first to avoid duplicates
            kfs.retain(|k| k.frame != new_f || k.frame == old_f);
            if let Some(kf) = kfs.iter_mut().find(|k| k.frame == old_f) {
                kf.frame = new_f;
                kfs.sort_by_key(|k| k.frame);
            }
        }
    }
    let pos_kfs = get_kfs(&layer.transform.position);
    let scale_kfs = get_kfs(&layer.transform.scale);
    let rot_kfs = get_kfs(&layer.transform.rotation);
    let op_kfs = get_kfs(&layer.transform.opacity);

    // Keyframes selected for this layer: (prop_key, frame)
    let prop_sel: std::collections::HashSet<(String, u32)> = selected_keyframes
        .iter()
        .filter(|(li, _, _)| *li == i)
        .map(|(_, pk, f)| (pk.clone(), *f))
        .collect();

    // Collect selection toggles first; apply to app after the
    // row borrows end (app and layer cannot borrow together).
    let mut select_requests: Vec<(&'static str, u32, bool, bool)> = Vec::new();
    let mut select_all_reqs: Vec<&'static str> = Vec::new();
    // Right-click menu commands: (prop_key, frame, action)
    // 0=Linear 1=EasyEase 2=ToggleHold 3=TimeReverse 4=Delete
    let mut kf_menu_cmds: Vec<(&'static str, u32, u8)> = Vec::new();

    macro_rules! kf_menu_cb {
        () => {
            Some(&mut |_pk: &'static str, f: u32, resp: &egui::Response| {
                resp.context_menu(|ui| {
                    ui.set_min_width(210.0);
                    if ui.button("⬤ Linear Interpolation").clicked() {
                        kf_menu_cmds.push((_pk, f, 0));
                        ui.close_menu();
                    }
                    if ui.button("◆ Easy Ease (F9)").clicked() {
                        kf_menu_cmds.push((_pk, f, 1));
                        ui.close_menu();
                    }
                    if ui.button("↗ Ease In (Shift+F9)").clicked() {
                        kf_menu_cmds.push((_pk, f, 5));
                        ui.close_menu();
                    }
                    if ui.button("↘ Ease Out (Ctrl+Shift+F9)").clicked() {
                        kf_menu_cmds.push((_pk, f, 6));
                        ui.close_menu();
                    }
                    if ui.button("🎯 Overshoot / Spring").clicked() {
                        kf_menu_cmds.push((_pk, f, 7));
                        ui.close_menu();
                    }
                    if ui.button("🏀 Bounce").clicked() {
                        kf_menu_cmds.push((_pk, f, 8));
                        ui.close_menu();
                    }
                    if ui.button("🪀 Elastic").clicked() {
                        kf_menu_cmds.push((_pk, f, 9));
                        ui.close_menu();
                    }
                    if ui.button("⬛ Toggle Hold Keyframe").clicked() {
                        kf_menu_cmds.push((_pk, f, 2));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("⇄ Time-Reverse Keyframes").clicked() {
                        kf_menu_cmds.push((_pk, f, 3));
                        ui.close_menu();
                    }
                    if ui.button("🗑 Delete Keyframe (Del)").clicked() {
                        kf_menu_cmds.push((_pk, f, 4));
                        ui.close_menu();
                    }
                });
            })
        };
    }

    // Marquee box-select results: (prop_key, boxed frames)
    let mut box_selects: Vec<(&'static str, Vec<u32>)> = Vec::new();
    // Group keyframe moves: (prop_key, dragged_frame, delta)
    let mut group_moves: Vec<(&'static str, u32, i32)> = Vec::new();

    {
        let t = &mut layer.transform;
        let moved: &mut bool = project_changed;
        // ── U / UU / A reveal modes (AE parity) ──
        // "Keyframed"   -> only properties that have keyframes
        // "Anchor Point"-> anchor point track only
        // otherwise     -> all four transform rows
        let reveal_mode = selected_property.clone().unwrap_or_default();
        let show_transform_rows = reveal_mode != "Anchor Point";
        let kf_only = reveal_mode == "Keyframed";

        if show_transform_rows && (!kf_only || !pos_kfs.is_empty()) {
            draw_prop_row_ext(
                ui,
                "  ⏱ Position",
                &pos_kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                &prop_sel,
                "position",
                Some(&mut |old_f, new_f| {
                    move_kf(&mut t.position, old_f, new_f);
                    *moved = true;
                }),
                Some(&mut |pk, f, shift, cmd| select_requests.push((pk, f, shift, cmd))),
                kf_menu_cb!(),
                Some(&mut |pk, frames: Vec<u32>, _add: bool| box_selects.push((pk, frames))),
                Some(&mut |pk, dragged_f, delta| group_moves.push((pk, dragged_f, delta))),
                all_kf_frames,
                Some(&mut |pk| select_all_reqs.push(pk)),
            );
        }
        if show_transform_rows && (!kf_only || !scale_kfs.is_empty()) {
            draw_prop_row_ext(
                ui,
                "  ⏱ Scale",
                &scale_kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                &prop_sel,
                "scale",
                Some(&mut |old_f, new_f| {
                    move_kf(&mut t.scale, old_f, new_f);
                    *moved = true;
                }),
                Some(&mut |pk, f, shift, cmd| select_requests.push((pk, f, shift, cmd))),
                kf_menu_cb!(),
                Some(&mut |pk, frames: Vec<u32>, _add: bool| box_selects.push((pk, frames))),
                Some(&mut |pk, dragged_f, delta| group_moves.push((pk, dragged_f, delta))),
                all_kf_frames,
                Some(&mut |pk| select_all_reqs.push(pk)),
            );
        }
        if show_transform_rows && (!kf_only || !rot_kfs.is_empty()) {
            draw_prop_row_ext(
                ui,
                "  ⏱ Rotation",
                &rot_kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                &prop_sel,
                "rotation",
                Some(&mut |old_f, new_f| {
                    move_kf(&mut t.rotation, old_f, new_f);
                    *moved = true;
                }),
                Some(&mut |pk, f, shift, cmd| select_requests.push((pk, f, shift, cmd))),
                kf_menu_cb!(),
                Some(&mut |pk, frames: Vec<u32>, _add: bool| box_selects.push((pk, frames))),
                Some(&mut |pk, dragged_f, delta| group_moves.push((pk, dragged_f, delta))),
                all_kf_frames,
                Some(&mut |pk| select_all_reqs.push(pk)),
            );
        }
        if show_transform_rows && (!kf_only || !op_kfs.is_empty()) {
            draw_prop_row_ext(
                ui,
                "  ⏱ Opacity",
                &op_kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                &prop_sel,
                "opacity",
                Some(&mut |old_f, new_f| {
                    move_kf(&mut t.opacity, old_f, new_f);
                    *moved = true;
                }),
                Some(&mut |pk, f, shift, cmd| select_requests.push((pk, f, shift, cmd))),
                kf_menu_cb!(),
                Some(&mut |pk, frames: Vec<u32>, _add: bool| box_selects.push((pk, frames))),
                Some(&mut |pk, dragged_f, delta| group_moves.push((pk, dragged_f, delta))),
                all_kf_frames,
                Some(&mut |pk| select_all_reqs.push(pk)),
            );
        }
        if reveal_mode == "Anchor Point" {
            let ap_kfs = get_kfs(&t.anchor_point);
            draw_prop_row(
                ui,
                "  ⏱ Anchor Point",
                &ap_kfs,
                current_frame,
                start_frame,
                zoom_span,
                left_pane_w,
                Some(&mut |old_f, new_f| {
                    move_kf(&mut t.anchor_point, old_f, new_f);
                    *moved = true;
                }),
            );
        }
    }

    // Shape Repeater animation tracks are regular layer properties, so expose
    // them beside Transform instead of hiding them in the Inspector only.
    if let Some(repeater) = layer.shape_repeater.as_mut() {
        macro_rules! repeater_row {
            ($anim:expr, $label:expr, $key:expr) => {{
                let kfs = get_kfs($anim);
                draw_prop_row_ext(
                    ui,
                    $label,
                    &kfs,
                    current_frame,
                    start_frame,
                    zoom_span,
                    left_pane_w,
                    &prop_sel,
                    $key,
                    Some(&mut |old, new| {
                        move_kf($anim, old, new);
                        *project_changed = true;
                    }),
                    Some(&mut |pk, f, shift, cmd| select_requests.push((pk, f, shift, cmd))),
                    kf_menu_cb!(),
                    Some(&mut |pk, frames: Vec<u32>, _add: bool| box_selects.push((pk, frames))),
                    Some(&mut |pk, dragged, delta| group_moves.push((pk, dragged, delta))),
                    all_kf_frames,
                    Some(&mut |pk| select_all_reqs.push(pk)),
                );
            }};
        }
        if let Some(copies) = repeater.copies_animation.as_mut() {
            repeater_row!(copies, "  ⏱ Repeater Copies", "repeater_copies");
        }
        if let Some(position) = repeater.position_offset_animation.as_mut() {
            repeater_row!(position, "  ⏱ Repeater Position", "repeater_position");
        }
        if let Some(scale) = repeater.scale_offset_animation.as_mut() {
            repeater_row!(scale, "  ⏱ Repeater Scale", "repeater_scale");
        }
        if let Some(rotation) = repeater.rotation_offset_animation.as_mut() {
            repeater_row!(rotation, "  ⏱ Repeater Rotation", "repeater_rotation");
        }
        if let Some(opacity) = repeater.opacity_animation.as_mut() {
            repeater_row!(opacity, "  ⏱ Repeater Opacity", "repeater_opacity");
        }
    }

    // ── Effect keyframe rows: collapsible per-effect groups ──
    for effect in layer.effects.iter_mut() {
        let fx_name = effect.name.clone();
        let moved: &mut bool = project_changed;
        use crate::core::effect_params::ParamRef;
        let fx_enabled = effect.enabled;
        let hdr = egui::CollapsingHeader::new(
            egui::RichText::new(format!(
                "{} {}",
                if fx_enabled {
                    egui_phosphor::regular::POWER
                } else {
                    egui_phosphor::regular::PROHIBIT
                },
                fx_name
            ))
            .small()
            .color(if fx_enabled {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_MUTED
            }),
        )
        .id_salt(ui.make_persistent_id(format!("fxgrp_{}_{}", i, fx_name)))
        .default_open(false);
        hdr.show(ui, |ui| {
            for (label, param) in effect.effect_type.animatable_params() {
                let prop_key_str = format!("fx_{}_{}", fx_name, label);
                let prop_key: &'static str = Box::leak(prop_key_str.into_boxed_str());
                let row_label = format!("  [{}] {}", fx_name, label);
                match param {
                    ParamRef::Scalar(anim) => {
                        let kfs = get_kfs(anim);
                        draw_prop_row_ext(
                            ui,
                            &row_label,
                            &kfs,
                            current_frame,
                            start_frame,
                            zoom_span,
                            left_pane_w,
                            &prop_sel,
                            prop_key,
                            Some(&mut |old_f, new_f| {
                                move_kf(anim, old_f, new_f);
                                *moved = true;
                            }),
                            Some(&mut |pk, f, shift, cmd| {
                                select_requests.push((pk, f, shift, cmd))
                            }),
                            kf_menu_cb!(),
                            Some(&mut |pk, frames: Vec<u32>, _add: bool| {
                                box_selects.push((pk, frames))
                            }),
                            Some(&mut |pk, dragged_f, delta| {
                                group_moves.push((pk, dragged_f, delta))
                            }),
                            all_kf_frames,
                            Some(&mut |pk| select_all_reqs.push(pk)),
                        );
                    }
                    ParamRef::Vec2(anim) => {
                        let kfs = get_kfs(anim);
                        draw_prop_row_ext(
                            ui,
                            &row_label,
                            &kfs,
                            current_frame,
                            start_frame,
                            zoom_span,
                            left_pane_w,
                            &prop_sel,
                            prop_key,
                            Some(&mut |old_f, new_f| {
                                move_kf(anim, old_f, new_f);
                                *moved = true;
                            }),
                            Some(&mut |pk, f, shift, cmd| {
                                select_requests.push((pk, f, shift, cmd))
                            }),
                            kf_menu_cb!(),
                            Some(&mut |pk, frames: Vec<u32>, _add: bool| {
                                box_selects.push((pk, frames))
                            }),
                            Some(&mut |pk, dragged_f, delta| {
                                group_moves.push((pk, dragged_f, delta))
                            }),
                            all_kf_frames,
                            Some(&mut |pk| select_all_reqs.push(pk)),
                        );
                    }
                    ParamRef::Vec3(anim) => {
                        let kfs = get_kfs(anim);
                        draw_prop_row_ext(
                            ui,
                            &row_label,
                            &kfs,
                            current_frame,
                            start_frame,
                            zoom_span,
                            left_pane_w,
                            &prop_sel,
                            prop_key,
                            Some(&mut |old_f, new_f| {
                                move_kf(anim, old_f, new_f);
                                *moved = true;
                            }),
                            Some(&mut |pk, f, shift, cmd| {
                                select_requests.push((pk, f, shift, cmd))
                            }),
                            kf_menu_cb!(),
                            Some(&mut |pk, frames: Vec<u32>, _add: bool| {
                                box_selects.push((pk, frames))
                            }),
                            Some(&mut |pk, dragged_f, delta| {
                                group_moves.push((pk, dragged_f, delta))
                            }),
                            all_kf_frames,
                            Some(&mut |pk| select_all_reqs.push(pk)),
                        );
                    }
                    ParamRef::Vec4Color(anim) => {
                        let kfs = get_kfs(anim);
                        draw_prop_row_ext(
                            ui,
                            &row_label,
                            &kfs,
                            current_frame,
                            start_frame,
                            zoom_span,
                            left_pane_w,
                            &prop_sel,
                            prop_key,
                            Some(&mut |old_f, new_f| {
                                move_kf(anim, old_f, new_f);
                                *moved = true;
                            }),
                            Some(&mut |pk, f, shift, cmd| {
                                select_requests.push((pk, f, shift, cmd))
                            }),
                            kf_menu_cb!(),
                            Some(&mut |pk, frames: Vec<u32>, _add: bool| {
                                box_selects.push((pk, frames))
                            }),
                            Some(&mut |pk, dragged_f, delta| {
                                group_moves.push((pk, dragged_f, delta))
                            }),
                            all_kf_frames,
                            Some(&mut |pk| select_all_reqs.push(pk)),
                        );
                    }
                }
            }
        });
    }

    // ── Puppet Pin rows (keyframeable positions) ──
    for pin in layer.puppet_pins.iter_mut() {
        let pin_key_str = format!("pin_{}", pin.id);
        let pin_key: &'static str = Box::leak(pin_key_str.into_boxed_str());
        let moved_p: &mut bool = project_changed;
        let kfs = get_kfs(&pin.position);
        draw_prop_row_ext(
            ui,
            &format!("  🧷 {}", pin.name),
            &kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            &prop_sel,
            pin_key,
            Some(&mut |old_f, new_f| {
                move_kf(&mut pin.position, old_f, new_f);
                *moved_p = true;
            }),
            Some(&mut |pk, f, shift, cmd| select_requests.push((pk, f, shift, cmd))),
            None,
            Some(&mut |pk, frames: Vec<u32>, _add: bool| box_selects.push((pk, frames))),
            Some(&mut |pk, dragged_f, delta| group_moves.push((pk, dragged_f, delta))),
            all_kf_frames,
            Some(&mut |pk| select_all_reqs.push(pk)),
        );
    }

    // ── Apply keyframe context-menu commands ──
    fn set_kf_interp<T: Clone>(
        anim: &mut crate::core::property::Animatable<T>,
        frames: &[u32],
        mode: u8,
    ) {
        use crate::core::keyframe::{BezierControlPoint, InterpolationType};
        if let Some(kfs) = anim.keyframes_mut() {
            for kf in kfs.iter_mut() {
                if !frames.contains(&kf.frame) {
                    continue;
                }
                kf.interpolation = match mode {
                    0 => InterpolationType::Linear,
                    1 => InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(
                            crate::core::keyframe::EasePreset::Standard.control_points(),
                        ),
                    },
                    2 => {
                        if matches!(kf.interpolation, InterpolationType::Hold) {
                            InterpolationType::Linear
                        } else {
                            InterpolationType::Hold
                        }
                    }
                    5 => InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(
                            crate::core::keyframe::EasePreset::EaseIn.control_points(),
                        ),
                    },
                    6 => InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(
                            crate::core::keyframe::EasePreset::EaseOut.control_points(),
                        ),
                    },
                    7 => InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(
                            crate::core::keyframe::EasePreset::Overshoot.control_points(),
                        ),
                    },
                    8 => InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(
                            crate::core::keyframe::EasePreset::Bounce.control_points(),
                        ),
                    },
                    9 => InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(
                            crate::core::keyframe::EasePreset::Elastic.control_points(),
                        ),
                    },
                    _ => kf.interpolation,
                };
            }
        }
    }
    fn reverse_track<T: Clone>(anim: &mut crate::core::property::Animatable<T>) {
        if let Some(kfs) = anim.keyframes_mut() {
            if kfs.len() < 2 {
                return;
            }
            let first = kfs.first().map(|k| k.frame).unwrap_or(0);
            let last = kfs.last().map(|k| k.frame).unwrap_or(0);
            for kf in kfs.iter_mut() {
                kf.frame = first + last - kf.frame;
            }
            kfs.sort_by_key(|k| k.frame);
        }
    }
    fn delete_track_kf<T: Clone>(anim: &mut crate::core::property::Animatable<T>, frame: u32) {
        if let Some(kfs) = anim.keyframes_mut() {
            kfs.retain(|k| k.frame != frame);
        }
    }
    for (pk, f, cmd) in kf_menu_cmds {
        let t = &mut layer.transform;
        let sel_frames: Vec<u32> = selected_keyframes
            .iter()
            .filter(|(li, p, _)| *li == i && p == pk)
            .map(|(_, _, fr)| *fr)
            .collect();
        match pk {
            "position" => match cmd {
                3 => reverse_track(&mut t.position),
                4 => delete_track_kf(&mut t.position, f),
                m => set_kf_interp(
                    &mut t.position,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    m,
                ),
            },
            "scale" => match cmd {
                3 => reverse_track(&mut t.scale),
                4 => delete_track_kf(&mut t.scale, f),
                m => set_kf_interp(
                    &mut t.scale,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    m,
                ),
            },
            "rotation" => match cmd {
                3 => reverse_track(&mut t.rotation),
                4 => delete_track_kf(&mut t.rotation, f),
                m => set_kf_interp(
                    &mut t.rotation,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    m,
                ),
            },
            "opacity" => match cmd {
                3 => reverse_track(&mut t.opacity),
                4 => delete_track_kf(&mut t.opacity, f),
                m => set_kf_interp(
                    &mut t.opacity,
                    &if sel_frames.is_empty() {
                        vec![f]
                    } else {
                        sel_frames
                    },
                    m,
                ),
            },
            _ if pk.starts_with("pin_") => {
                let pid = pk.strip_prefix("pin_").unwrap_or("");
                if let Some(pin) = layer.puppet_pins.iter_mut().find(|p| p.id == pid) {
                    match cmd {
                        3 => reverse_track(&mut pin.position),
                        4 => delete_track_kf(&mut pin.position, f),
                        m => set_kf_interp(
                            &mut pin.position,
                            &if sel_frames.is_empty() {
                                vec![f]
                            } else {
                                sel_frames
                            },
                            m,
                        ),
                    }
                }
            }
            "repeater_copies" | "repeater_position" | "repeater_scale" | "repeater_rotation"
            | "repeater_opacity" => {
                if let Some(repeater) = layer.shape_repeater.as_mut() {
                    macro_rules! apply_repeater {
                        ($track:expr) => {
                            match cmd {
                                3 => reverse_track($track),
                                4 => delete_track_kf($track, f),
                                m => set_kf_interp(
                                    $track,
                                    &if sel_frames.is_empty() {
                                        vec![f]
                                    } else {
                                        sel_frames.clone()
                                    },
                                    m,
                                ),
                            }
                        };
                    }
                    match pk {
                        "repeater_copies" => {
                            if let Some(a) = repeater.copies_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_position" => {
                            if let Some(a) = repeater.position_offset_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_scale" => {
                            if let Some(a) = repeater.scale_offset_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_rotation" => {
                            if let Some(a) = repeater.rotation_offset_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        "repeater_opacity" => {
                            if let Some(a) = repeater.opacity_animation.as_mut() {
                                apply_repeater!(a);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        *project_changed = true;
    }

    // ── Apply marquee box-selects ──
    for (pk, frames) in box_selects {
        for f in frames {
            selected_keyframes.insert((i, pk.to_string(), f));
        }
        *project_changed = false;
    }

    // ── Group move: selected keyframes follow the dragged one ──
    {
        let t = &mut layer.transform;
        for (pk, dragged_f, delta) in group_moves {
            let followers: Vec<u32> = selected_keyframes
                .iter()
                .filter(|(li, p, fr)| *li == i && p == pk && *fr != dragged_f)
                .map(|(_, _, fr)| *fr)
                .collect();
            if followers.is_empty() {
                continue;
            }
            let new_frames: Vec<(u32, u32)> = followers
                .iter()
                .map(|&f| (f, (f as i64 + delta as i64).max(0) as u32))
                .collect();
            macro_rules! shift_track {
                ($anim:expr) => {{
                    for (old_f, new_f) in &new_frames {
                        move_kf($anim, *old_f, *new_f);
                    }
                    if let Some(kfs) = $anim.keyframes_mut() {
                        kfs.sort_by_key(|k| k.frame);
                    }
                }};
            }
            match pk {
                "position" => shift_track!(&mut t.position),
                "scale" => shift_track!(&mut t.scale),
                "rotation" => shift_track!(&mut t.rotation),
                "opacity" => shift_track!(&mut t.opacity),
                "repeater_copies" => {
                    if let Some(a) = layer
                        .shape_repeater
                        .as_mut()
                        .and_then(|r| r.copies_animation.as_mut())
                    {
                        shift_track!(a);
                    }
                }
                "repeater_position" => {
                    if let Some(a) = layer
                        .shape_repeater
                        .as_mut()
                        .and_then(|r| r.position_offset_animation.as_mut())
                    {
                        shift_track!(a);
                    }
                }
                "repeater_scale" => {
                    if let Some(a) = layer
                        .shape_repeater
                        .as_mut()
                        .and_then(|r| r.scale_offset_animation.as_mut())
                    {
                        shift_track!(a);
                    }
                }
                "repeater_rotation" => {
                    if let Some(a) = layer
                        .shape_repeater
                        .as_mut()
                        .and_then(|r| r.rotation_offset_animation.as_mut())
                    {
                        shift_track!(a);
                    }
                }
                "repeater_opacity" => {
                    if let Some(a) = layer
                        .shape_repeater
                        .as_mut()
                        .and_then(|r| r.opacity_animation.as_mut())
                    {
                        shift_track!(a);
                    }
                }
                _ if pk.starts_with("pin_") => {
                    let pid = pk.strip_prefix("pin_").unwrap_or("");
                    if let Some(pin) = layer.puppet_pins.iter_mut().find(|p| p.id == pid) {
                        shift_track!(&mut pin.position);
                    }
                }
                _ => {}
            }
        }
    }

    // Resolve every keyframe frame of a property (for label dbl-click select-all).
    fn prop_all_frames(layer: &crate::core::timeline::Layer, pk: &str) -> Vec<u32> {
        let mut out = Vec::new();
        let mut push_anim_f32 = |a: &crate::core::property::Animatable<f32>| {
            if let Some(kfs) = a.keyframes() {
                for k in kfs {
                    out.push(k.frame);
                }
            }
        };
        match pk {
            "position" | "scale" => {
                let a = if pk == "position" {
                    &layer.transform.position
                } else {
                    &layer.transform.scale
                };
                if let Some(kfs) = a.keyframes() {
                    for k in kfs {
                        out.push(k.frame);
                    }
                }
            }
            "rotation" => push_anim_f32(&layer.transform.rotation),
            "opacity" => push_anim_f32(&layer.transform.opacity),
            "repeater_copies" => {
                if let Some(anim) = layer
                    .shape_repeater
                    .as_ref()
                    .and_then(|repeater| repeater.copies_animation.as_ref())
                {
                    push_anim_f32(anim);
                }
            }
            "repeater_rotation" => {
                if let Some(anim) = layer
                    .shape_repeater
                    .as_ref()
                    .and_then(|repeater| repeater.rotation_offset_animation.as_ref())
                {
                    push_anim_f32(anim);
                }
            }
            "repeater_position" | "repeater_scale" | "repeater_opacity" => {
                let keyframes = layer.shape_repeater.as_ref().and_then(|repeater| match pk {
                    "repeater_position" => repeater
                        .position_offset_animation
                        .as_ref()
                        .and_then(|a| a.keyframes()),
                    "repeater_scale" => repeater
                        .scale_offset_animation
                        .as_ref()
                        .and_then(|a| a.keyframes()),
                    _ => repeater
                        .opacity_animation
                        .as_ref()
                        .and_then(|a| a.keyframes()),
                });
                if let Some(keyframes) = keyframes {
                    out.extend(keyframes.iter().map(|keyframe| keyframe.frame));
                }
            }
            _ if pk.starts_with("fx_") => {
                use crate::core::effect_params::ParamRefRef;
                let rest = pk.strip_prefix("fx_").unwrap_or("");
                for eff in &layer.effects {
                    if !rest.starts_with(&eff.name) {
                        continue;
                    }
                    let label = rest[eff.name.len()..].trim_start_matches('_');
                    for (plabel, pref) in eff.effect_type.animatable_params_ref() {
                        if plabel != label {
                            continue;
                        }
                        let frames: Vec<u32> = match pref {
                            ParamRefRef::Scalar(a) => a
                                .keyframes()
                                .map(|ks| ks.iter().map(|k| k.frame).collect())
                                .unwrap_or_default(),
                            ParamRefRef::Vec2(a) => a
                                .keyframes()
                                .map(|ks| ks.iter().map(|k| k.frame).collect())
                                .unwrap_or_default(),
                            ParamRefRef::Vec3(a) => a
                                .keyframes()
                                .map(|ks| ks.iter().map(|k| k.frame).collect())
                                .unwrap_or_default(),
                            ParamRefRef::Vec4Color(a) => a
                                .keyframes()
                                .map(|ks| ks.iter().map(|k| k.frame).collect())
                                .unwrap_or_default(),
                        };
                        out.extend(frames);
                        break;
                    }
                }
            }
            _ if pk.starts_with("pin_") => {
                let pid = pk.strip_prefix("pin_").unwrap_or("");
                if let Some(pin) = layer.puppet_pins.iter().find(|p| p.id == pid) {
                    if let Some(kfs) = pin.position.keyframes() {
                        for k in kfs {
                            out.push(k.frame);
                        }
                    }
                }
            }
            _ => {}
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    for pk in select_all_reqs.drain(..) {
        select_requests.push((pk, u32::MAX, false, false));
    }

    for (pk, f, shift, cmd) in select_requests {
        if f == u32::MAX {
            let all = prop_all_frames(layer, pk);
            if shift || cmd {
                for fr in all {
                    selected_keyframes.insert((i, pk.to_string(), fr));
                }
            } else {
                selected_keyframes.clear();
                for fr in all {
                    selected_keyframes.insert((i, pk.to_string(), fr));
                }
            }
            *project_changed = false;
            continue;
        }
        let entry = (i, pk.to_string(), f);
        if shift || cmd {
            if !selected_keyframes.remove(&entry) {
                selected_keyframes.insert(entry);
            }
        } else {
            selected_keyframes.clear();
            selected_keyframes.insert(entry);
        }
        *project_changed = false; // selection alone is not a project edit
    }

    // ── Time Remap row (when enabled) ──
    if let Some(remap) = layer.time_remap.as_mut() {
        let moved_r: &mut bool = project_changed;
        let tr_kfs = get_kfs(remap);
        draw_prop_row(
            ui,
            "  ⏱ Time Remap",
            &tr_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                move_kf(remap, o, n);
                *moved_r = true;
            }),
        );
    }

    // ── Mask property rows (feather / opacity / expansion / path) ──
    for (m_idx, mask) in layer.masks.iter_mut().enumerate() {
        let moved_m: &mut bool = project_changed;
        ui.label(
            egui::RichText::new(format!("  🎭 {}", mask.name))
                .small()
                .strong()
                .color(colors::TEXT_SECONDARY),
        );
        let f_kfs = get_kfs(&mask.feather);
        draw_prop_row(
            ui,
            "    Feather",
            &f_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                move_kf(&mut mask.feather, o, n);
                *moved_m = true;
            }),
        );
        let op_kfs_m = get_kfs(&mask.opacity);
        draw_prop_row(
            ui,
            "    Opacity",
            &op_kfs_m,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                move_kf(&mut mask.opacity, o, n);
                *moved_m = true;
            }),
        );
        let ex_kfs = get_kfs(&mask.expansion);
        draw_prop_row(
            ui,
            "    Expansion",
            &ex_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                move_kf(&mut mask.expansion, o, n);
                *moved_m = true;
            }),
        );
        let p_kfs = get_kfs(&mask.path.vertices);
        draw_prop_row(
            ui,
            &format!("    Path ({} pts)", p_kfs.len()),
            &p_kfs,
            current_frame,
            start_frame,
            zoom_span,
            left_pane_w,
            Some(&mut |o, n| {
                // Vec<[f32;2]> lacks Interpolate — retime inline
                if let Some(kfs) = mask.path.vertices.keyframes_mut() {
                    if let Some(kf) = kfs.iter_mut().find(|k| k.frame == o) {
                        kf.frame = n;
                        kfs.sort_by_key(|k| k.frame);
                    }
                }
                *moved_m = true;
            }),
        );
        let _ = m_idx;
    }

    // ── Pre-comp nested child layers ──
    if let Some((_parent_idx, children, sub_id)) =
        precomp_children.iter().find(|(pi, _, _)| *pi == i)
    {
        if crate::ui::timeline::precomp_children::draw_children_rows(
            ui,
            sub_id,
            children,
            total_frames,
        ) {
            *open_comp_request = Some(sub_id.clone());
        }
    }
}
