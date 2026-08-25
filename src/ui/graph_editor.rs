use eframe::egui;
use crate::core::timeline::Layer;
use crate::ui::theme::colors;

/// A reusable module for rendering the After Effects keyframe Graph Editor.
///
/// Visualizes animatable property value curves over time, drawing interactive control
/// points and Bezier tangent handles.
/// Resolve "PinX:<id>" / "PinY:<id>" graph properties to the pin's track.
fn pin_anim_mut<'a>(layer: &'a mut Layer, prop: &str) -> Option<&'a mut crate::core::property::Animatable<[f32; 2]>> {
    let id = prop.strip_prefix("PinX:").or_else(|| prop.strip_prefix("PinY:"))?;
    layer.puppet_pins.iter_mut().find(|p| p.id == id).map(|p| &mut p.position)
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
                    ].iter().map(|(a,b)|(a.to_string(),b.to_string())).collect();
                    for pin in &layer.puppet_pins {
                        props.push((format!("PinX:{}", pin.id), format!("\u{1f9f7} {} X", pin.name)));
                        props.push((format!("PinY:{}", pin.id), format!("\u{1f9f7} {} Y", pin.name)));
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
            if ui.button("⚡ Easy Ease (F9)").on_hover_text("Apply smooth cubic Bezier ease curve to all keyframes").clicked() {
                use crate::core::property::Animatable;
                use crate::core::keyframe::InterpolationType;

                macro_rules! apply_easy_ease {
                    ($kfs:expr) => {
                        for kf in $kfs.iter_mut() {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some([0.33, 0.0, 0.67, 1.0]),
                            };


                        }
                    };
                }

                let active_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
                match active_prop.as_str() {
                    "Position X" | "Position Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.position { apply_easy_ease!(kfs); }
                    }
                    "Scale X" | "Scale Y" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.scale { apply_easy_ease!(kfs); }
                    }
                    "Rotation" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.rotation { apply_easy_ease!(kfs); }
                    }
                    "Opacity" => {
                        if let Animatable::Animated(ref mut kfs) = layer.transform.opacity { apply_easy_ease!(kfs); }
                    }
                                        p if p.starts_with("Pin") => {
                        if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                            let ez = crate::core::keyframe::EasePreset::Standard.control_points();
                            for kf in kfs.iter_mut() {
                                kf.interpolation = crate::core::keyframe::InterpolationType::Bezier {
                                    outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                    incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                    custom_bezier: Some(ez),
                                };
                            }
                        }
                    }
                    _ => {}
                }
                *project_changed = true;
            }

            ui.add_space(4.0);
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
                match selected_property.clone().unwrap_or_else(|| "Position X".to_string()).as_str() {
                    "Position X" | "Position Y" => reverse_v2(&mut layer.transform.position),
                    "Scale X" | "Scale Y" => reverse_v2(&mut layer.transform.scale),
                    "Rotation" => reverse_f32(&mut layer.transform.rotation),
                    "Opacity" => reverse_f32(&mut layer.transform.opacity),
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
                                        p if p.starts_with("Pin") => {
                                            if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                                                                                                    for kf in kfs.iter_mut() { ease_out(&mut kf.interpolation); }
                                            }
                                        }
                    _ => {}
                }
                *project_changed = true;
            }

        });

        let graph_prop = selected_property.clone().unwrap_or_else(|| "Position X".to_string());
        let total_f = duration_frames.max(1);

        // Sample values along timeline duration for drawing curve
        let mut samples = Vec::with_capacity(total_f as usize + 1);
        for f in 0..=total_f {
            let raw_val = match graph_prop.as_str() {
                "Position X" => layer.transform.position.evaluate(f)[0],
                "Position Y" => layer.transform.position.evaluate(f)[1],
                "Scale X" => layer.transform.scale.evaluate(f)[0],
                "Scale Y" => layer.transform.scale.evaluate(f)[1],
                "Rotation" => layer.transform.rotation.evaluate(f),
                "Opacity" => layer.transform.opacity.evaluate(f),
                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                    let ci = usize::from(p.starts_with("PinY:"));
                    let pid = p.split(':').nth(1).unwrap_or("");
                    layer.puppet_pins.iter().find(|pp| pp.id == pid)
                        .map(|pp| pp.position.evaluate(f)[ci])
                        .unwrap_or(0.0)
                }
                _ => layer.transform.position.evaluate(f)[0],
            };
            let val = if raw_val.is_nan() { 0.0 } else { raw_val };
            samples.push((f, val));
        }

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
        {
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
        let points: Vec<egui::Pos2> = samples.iter().map(|&(f, v)| {
            let x = rect.left() + (f as f32 / total_f as f32) * rect.width();
            let y = rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);
            egui::pos2(x, y)
        }).collect();

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
                            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                let ci = usize::from(p.starts_with("PinY:"));
                                let pid = p.split(':').nth(1).unwrap_or("");
                                layer.puppet_pins.iter().find(|pp| pp.id == pid)
                                    .map(|pp| chan_kfs(&pp.position, ci))
                                    .unwrap_or_default()
                            }
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
                                p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                                    if let Some(pin) = pin_anim_mut(layer, p) {
                                        let ci = usize::from(p.starts_with("PinY:"));
                                        let mut v = pin.evaluate(new_frame);
                                        v[ci] = new_val;
                                        pin.add_keyframe(GKeyframe::new(new_frame, v, GInterp::Linear));
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

            macro_rules! with_keyframes {
                ($layer:expr, $prop:expr, $kfs:ident => $body:expr) => {{
                    let prop: String = $prop.clone();
                    if matches!(prop.as_str(), "Position X" | "Position Y" | "Scale X" | "Scale Y") {
                        if let Some($kfs) = keyframes_of_vec2($layer, &prop) { Some({ $body }) } else { None }
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
            fn set_keyframe_value<T: AddVal>(kf: &mut crate::core::keyframe::Keyframe<T>, delta: f32, is_y: bool) {
                kf.value.add_val(delta, is_y);
            }

            // Snapshot keyframe positions first (immutable), then edit mutably on drag
            let kf_positions: Vec<(usize, u32, f32)> = if matches!(graph_prop.as_str(), "Position X" | "Position Y" | "Scale X" | "Scale Y") {
                let comp_idx = if graph_prop.ends_with('Y') { 1usize } else { 0usize };
                keyframes_of_vec2(layer, &graph_prop).map(|kfs| {
                    kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value[comp_idx])).collect::<Vec<_>>()
                }).unwrap_or_default()
            } else {
                keyframes_of_f32(layer, &graph_prop).map(|kfs| {
                    kfs.iter().enumerate().map(|(i, kf)| (i, kf.frame, kf.value)).collect::<Vec<_>>()
                }).unwrap_or_default()
            };

            for (kf_idx, kf_frame, kf_val) in &kf_positions {
                let pt = egui::pos2(frame_to_x(*kf_frame), val_to_y(*kf_val));

                // --- Anchor point: drag horizontally to retime, vertically to change value ---
                let anchor_rect = egui::Rect::from_center_size(pt, egui::vec2(14.0, 14.0));
                let anchor_resp = ui.interact(anchor_rect, egui::Id::new(("graph_anchor", kf_idx)), egui::Sense::drag());
                if anchor_resp.dragged() {
                    let delta_frames = (anchor_resp.drag_delta().x / rect.width() * total_f as f32).round() as i32;
                    let new_frame = (*kf_frame as i32 + delta_frames).clamp(0, total_f as i32) as u32;
                    // Vertical drag → value change (screen up = value up)
                    let delta_val = -anchor_resp.drag_delta().y / (rect.height() - 8.0) * val_range;
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
                                            with_keyframes!(layer, graph_prop, kfs => {
                                                if kfs.get(*kf_idx).map(|k| k.frame != new_f).unwrap_or(false) {
                                                    kfs[*kf_idx].frame = new_f;
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *project_changed = true;
                                                }
                                            });
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
