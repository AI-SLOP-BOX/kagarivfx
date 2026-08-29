use eframe::egui;
use crate::core::property::Animatable;
use crate::core::keyframe::{Keyframe, InterpolationType, BezierControlPoint};
use crate::core::timeline::Expression;
use crate::ui::theme::colors;
use crate::ui::custom_widgets;

pub fn draw_easy_ease_button<T: Clone>(ui: &mut egui::Ui, property: &mut Animatable<T>, project_changed: &mut bool) {
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        if custom_widgets::ae_button(ui, "Easy Ease (F9)").on_hover_text("Symmetrical Bezier Ease (F9)").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                let coords = crate::core::keyframe::EasePreset::Standard.control_points();
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                        custom_bezier: Some(coords),
                    };
                }
                *project_changed = true;
            }
        }

        // Smart Ease Curve Preset Selector Dropdown
        let combo_id = ui.make_persistent_id(format!("smart_ease_combo_{:?}", ui.id()));
        egui::ComboBox::from_id_salt(combo_id)
            .selected_text("✨ Smart Presets...")
            .show_ui(ui, |ui| {
                for (preset, label, desc) in [
                    (crate::core::keyframe::EasePreset::Standard, "🟢 Standard Ease", "Symmetrical Smooth Ease"),
                    (crate::core::keyframe::EasePreset::FastIn, "⚡ Fast Acceleration", "Sudden Speed Up"),
                    (crate::core::keyframe::EasePreset::SmoothOut, "🎯 Smooth Deceleration", "Gentle Slow Down"),
                    (crate::core::keyframe::EasePreset::Overshoot, "🏀 Spring Overshoot", "Bounce Back Effect"),
                    (crate::core::keyframe::EasePreset::Sine, "🌊 Sine Wave", "Ultra Smooth Harmonic Ease"),
                    (crate::core::keyframe::EasePreset::FastOut, "🚀 Fast Out", "Explosive Start, Quick Settle"),
                    (crate::core::keyframe::EasePreset::SlowIn, "🐢 Slow In", "Gradual Gentle Acceleration"),
                    (crate::core::keyframe::EasePreset::Elastic, "🎈 Elastic", "Rubber Band Elastic Motion"),
                    (crate::core::keyframe::EasePreset::Bounce, "⚽ Bounce", "Ball Bounce Impact"),
                    (crate::core::keyframe::EasePreset::Cycle, "🔄 Cycle", "Looping Rhythmic Ease"),
                    (crate::core::keyframe::EasePreset::MirrorEase2, "🪞 Mirror", "Symmetrical Back and Forth"),
                    (crate::core::keyframe::EasePreset::EaseIn, "📈 Quadratic In", "Classic Quadratic Acceleration"),
                    (crate::core::keyframe::EasePreset::EaseOut, "📉 Quadratic Out", "Classic Quadratic Deceleration"),
                ] {
                    ui.horizontal(|ui| {
                        // Draw mini bezier thumbnail rect
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, colors::BG_DEEPEST);

                        let pts = preset.control_points();
                        let p0 = egui::pos2(rect.left() + 2.0, rect.bottom() - 2.0);
                        let p3 = egui::pos2(rect.right() - 2.0, rect.top() + 2.0);
                        let p1 = egui::pos2(rect.left() + 2.0 + pts[0] * (rect.width() - 4.0), rect.bottom() - 2.0 - pts[1] * (rect.height() - 4.0));
                        let p2 = egui::pos2(rect.left() + 2.0 + pts[2] * (rect.width() - 4.0), rect.bottom() - 2.0 - pts[3] * (rect.height() - 4.0));

                        // Render preview curve
                        let mut curve_pts = Vec::with_capacity(11);
                        for step in 0..=10 {
                            let t = step as f32 / 10.0;
                            let inv_t = 1.0 - t;
                            let x = inv_t.powi(3) * p0.x + 3.0 * inv_t.powi(2) * t * p1.x + 3.0 * inv_t * t.powi(2) * p2.x + t.powi(3) * p3.x;
                            let y = inv_t.powi(3) * p0.y + 3.0 * inv_t.powi(2) * t * p1.y + 3.0 * inv_t * t.powi(2) * p2.y + t.powi(3) * p3.y;
                            curve_pts.push(egui::pos2(x, y));
                        }
                        for window in curve_pts.windows(2) {
                            ui.painter().line_segment([window[0], window[1]], egui::Stroke::new(1.5, colors::ACCENT_CYAN));
                        }

                        if ui.button(label).on_hover_text(desc).clicked() {
                            if let Animatable::Animated(ref mut keyframes) = property {
                                for kf in keyframes {
                                    kf.interpolation = InterpolationType::Bezier {
                                        outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                        incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                        custom_bezier: Some(pts),
                                    };
                                }
                                *project_changed = true;
                                ui.close_menu();
                            }
                        }
                    });
                }
            });

        // Physics Spring Bounce Auto Generator Button
        if custom_widgets::ae_button(ui, "⚽ Physics Spring").on_hover_text("Apply Physics-based Overshoot & Spring Dynamics").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                let spring_bezier = [0.175, 0.885, 0.32, 1.275]; // Elastic Overshoot Control Points
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint { influence: 0.25, speed: 0.0 },
                        incoming: BezierControlPoint { influence: 0.75, speed: 0.0 },
                        custom_bezier: Some(spring_bezier),
                    };
                }
                *project_changed = true;
            }
        }

        // ⏸ Hold Keyframe Mode (Cmd+Opt+H) Button
        if custom_widgets::ae_button(ui, "⏸ Hold").on_hover_text("Toggle Toggle Hold Keyframe (Cmd+Opt+H): Values step discretely at keyframes").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Hold;
                }
                *project_changed = true;
            }
        }

        // 📈 Linear Keyframe Mode Button
        if custom_widgets::ae_button(ui, "📈 Linear").on_hover_text("Linear Keyframe: Values interpolate smoothly at constant speed").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                for kf in keyframes {
                    kf.interpolation = InterpolationType::Linear;
                }
                *project_changed = true;
            }
        }

        // ⏩ Keyframe Time Compress (2x Speed / 50% Duration)
        if custom_widgets::ae_button(ui, "⏩ 2x Speed").on_hover_text("Compress keyframe duration by 50%").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                if let Some(first_kf) = keyframes.first() {
                    let start_f = first_kf.frame;
                    for kf in keyframes {
                        let offset = kf.frame - start_f;
                        kf.frame = start_f + (offset as f32 * 0.5).round() as u32;
                    }
                    *project_changed = true;
                }
            }
        }

        // ⏪ Keyframe Time Stretch (0.5x Speed / 200% Duration)
        if custom_widgets::ae_button(ui, "⏪ 0.5x Speed").on_hover_text("Stretch keyframe duration by 200%").clicked() {
            if let Animatable::Animated(ref mut keyframes) = property {
                if let Some(first_kf) = keyframes.first() {
                    let start_f = first_kf.frame;
                    for kf in keyframes {
                        let offset = kf.frame - start_f;
                        kf.frame = start_f + (offset as f32 * 2.0).round() as u32;
                    }
                    *project_changed = true;
                }
            }
        }
    });
}

pub fn draw_expression_selector(
    ui: &mut egui::Ui,
    label: &str,
    expr_opt: &mut Option<Expression>,
    project_changed: &mut bool,
    current_frame: Option<u32>,
    fps: Option<u32>,
) {
    ui.horizontal(|ui| {
        ui.small("Expression: ");
        let expr_text = match expr_opt {
            Some(Expression::Wiggle { frequency, amplitude }) => format!("wiggle({}, {})", frequency, amplitude),
            Some(Expression::TimeDriver { multiplier, offset }) => format!("time * {} + {}", multiplier, offset),
            Some(Expression::LoopOut) => "loopOut()".to_string(),
            Some(Expression::PingPong) => "loopOut(\"pingpong\")".to_string(),
            Some(Expression::Raw(s)) => format!("Custom: {}...", &s[..s.len().min(20)]),
            None => "None".to_string(),
        };

        let before = expr_opt.clone();
        let combo_id = ui.make_persistent_id(format!("ae_expr_combo_{}", label));
        egui::ComboBox::from_id_salt(combo_id)
            .selected_text(expr_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(expr_opt, None, "None");
                ui.selectable_value(expr_opt, Some(Expression::Wiggle { frequency: 2.0, amplitude: 50.0 }), "Wiggle (2Hz, 50px)");
                ui.selectable_value(expr_opt, Some(Expression::TimeDriver { multiplier: 30.0, offset: 0.0 }), "Time Spin (30°/s)");
                ui.selectable_value(expr_opt, Some(Expression::LoopOut), "loopOut(\"cycle\")");
                ui.selectable_value(expr_opt, Some(Expression::PingPong), "loopOut(\"pingpong\")");
                ui.selectable_value(expr_opt, Some(Expression::Raw("value".into())), "Custom Script...");
            });

        // Expression Pickwhip button (@)
        if custom_widgets::ae_icon_button(ui, "🌀", "Expression Pickwhip (@): Pick property to auto-generate script expression").clicked() {
            *expr_opt = Some(Expression::Wiggle { frequency: 3.0, amplitude: 25.0 });
            *project_changed = true;
        }

        if before != *expr_opt {
            *project_changed = true;
        }
    });

    // Inline script editor for Raw expressions
    let mut remove_requested = false;
    if let Some(Expression::Raw(script)) = expr_opt {
        ui.indent(format!("expr_editor_{}", label), |ui| {
            // Script editor (multiline)
            let editor_id = ui.make_persistent_id(format!("expr_textedit_{}", label));
            let response = ui.add(
                egui::TextEdit::multiline(script)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("// Rhai expression script...")
                    .id(editor_id),
            );
            if response.changed() {
                *project_changed = true;
            }

            ui.add_space(2.0);

            // ── Toolbar: Presets / Validate / Test ──
            ui.horizontal(|ui| {
                // Preset dropdown
                let presets = [
                    ("wiggle(4, 30)", "Wiggle"),
                    ("loopOut(\"cycle\")", "Loop Cycle"),
                    ("loopOut(\"pingpong\")", "PingPong"),
                    ("value * 2", "Double"),
                    ("time * 100", "Time×100"),
                    ("Math.sin(time * 3) * 50", "Sine"),
                    ("posterizeTime(12); value", "12fps"),
                ];
                let preset_id = ui.make_persistent_id(format!("expr_presets_{}", label));
                egui::ComboBox::from_id_salt(preset_id)
                    .selected_text("Presets ▾")
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for (expr, lbl) in presets {
                            if ui.selectable_label(false, lbl).clicked() {
                                *script = expr.to_string();
                                *project_changed = true;
                            }
                        }
                    });

                // Expression Language Menu (AE Flyout Triangle)
                let lang_menu_id = ui.make_persistent_id(format!("expr_lang_menu_{}", label));
                ui.menu_button("▶ Language ▾", |ui| {
                    ui.label(egui::RichText::new("📖 Expression Language Library").strong().color(colors::ACCENT_CYAN));
                    ui.separator();
                    
                    ui.menu_button("🌐 Global & Comp", |ui| {
                        if ui.button("time (seconds)").clicked() { *script = format!("{}\ntime", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("thisComp.duration").clicked() { *script = format!("{}\nthisComp.duration", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("thisLayer.index").clicked() { *script = format!("{}\nthisLayer.index", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("valueAtTime(time - 0.1)").clicked() { *script = format!("{}\nvalueAtTime(time - 0.1)", script); *project_changed = true; ui.close_menu(); }
                    });

                    ui.menu_button("🎲 Random Numbers", |ui| {
                        if ui.button("wiggle(freq, amp)").clicked() { *script = format!("{}\nwiggle(4, 25)", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("random(min, max)").clicked() { *script = format!("{}\nrandom(0.0, 100.0)", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("noise(time)").clicked() { *script = format!("{}\nnoise(time * 2.0)", script); *project_changed = true; ui.close_menu(); }
                    });

                    ui.menu_button("📈 Interpolation", |ui| {
                        if ui.button("linear(t, tMin, tMax, val1, val2)").clicked() { *script = format!("{}\nlinear(time, 0.0, 2.0, 0.0, 100.0)", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("ease(t, tMin, tMax, val1, val2)").clicked() { *script = format!("{}\nease(time, 0.0, 1.5, 0.0, 200.0)", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("easeIn(t, 0, 1, 0, 100)").clicked() { *script = format!("{}\neaseIn(time, 0.0, 1.0, 0.0, 100.0)", script); *project_changed = true; ui.close_menu(); }
                    });

                    ui.menu_button("🔁 Looping & PingPong", |ui| {
                        if ui.button("loopOut(\"cycle\")").clicked() { *script = format!("{}\nloopOut(\"cycle\")", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("loopOut(\"pingpong\")").clicked() { *script = format!("{}\nloopOut(\"pingpong\")", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("loopIn(\"cycle\")").clicked() { *script = format!("{}\nloopIn(\"cycle\")", script); *project_changed = true; ui.close_menu(); }
                    });

                    ui.menu_button("📐 Vector & Trigonometry Math", |ui| {
                        if ui.button("Math.sin(time * 3.0) * 50.0").clicked() { *script = format!("{}\nMath.sin(time * 3.0) * 50.0", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("Math.atan2(y, x)").clicked() { *script = format!("{}\nMath.atan2(y, x)", script); *project_changed = true; ui.close_menu(); }
                        if ui.button("clamp(val, min, max)").clicked() { *script = format!("{}\nclamp(value, 0.0, 100.0)", script); *project_changed = true; ui.close_menu(); }
                    });
                });

                // Live syntax validation badge
                let engine = crate::core::expression_engine::build_engine();
                let validation = crate::core::expression_engine::validate_script(&engine, script);
                let (icon, color, tip) = match &validation {
                    Ok(()) => ("✓", colors::ACCENT_GREEN, "Script is valid".to_string()),
                    Err(e) => ("✗", colors::ACCENT_RED, e.clone()),
                };
                ui.label(egui::RichText::new(icon).small().color(color)).on_hover_text(tip);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if custom_widgets::ae_icon_button(ui, "✕", "Remove expression").clicked() {
                        remove_requested = true;
                    }
                    if custom_widgets::ae_icon_button(ui, "▶", "Test expression at current frame").clicked() {
                        if let (Some(cf), Some(fps)) = (current_frame, fps) {
                            let snap = ui.ctx().data(|d| {
                                d.get_temp::<std::sync::Arc<crate::core::expression_engine::CompSnapshot>>(
                                    egui::Id::new("ae_expr_comp_snap"),
                                )
                            });
                            let result = match snap {
                                Some(snap) => test_expression_with_comp(script, cf, fps, &snap),
                                None => test_expression_inline(script, cf, fps),
                            };
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new(("expr_test_result", label)), result);
                            });
                        }
                    }
                });
            });

            // Show test result
            let test_result = ui.ctx().data(|d| {
                d.get_temp::<String>(egui::Id::new(("expr_test_result", label)))
            });
            if let Some(result) = test_result {
                ui.label(egui::RichText::new(format!("→ {}", result)).small().monospace().color(colors::TEXT_ACCENT));
            }
        });
    }
    if remove_requested {
        *expr_opt = None;
        *project_changed = true;
    }
}

fn test_expression_inline(script: &str, frame: u32, fps: u32) -> String {
    let engine = crate::core::expression_engine::build_engine();
    let (result, diag) = crate::core::expression_engine::eval_v2_with_diagnostics(
        &engine, script, [0.0, 0.0], frame, fps,
    );
    if let Some(err) = diag {
        format!("Error: {}", err)
    } else {
        format!("({:.2}, {:.2})", result[0], result[1])
    }
}

pub fn draw_property_ui<T: Clone + crate::core::property::Interpolate + PartialEq + std::fmt::Debug + 'static>(
    current_frame: u32,
    ui: &mut egui::Ui,
    label: &str,
    property: &mut Animatable<T>,
    draw_value_widget: impl FnOnce(&mut egui::Ui, &mut T),
) -> Option<u32> {
    let mut next_frame = None;
    ui.horizontal(|ui| {
        ui.label(label);
        
        let has_keyframes = property.keyframes().is_some();
        if has_keyframes
            && custom_widgets::ae_icon_button(ui, "◀", "Jump to Previous Keyframe (J)").clicked() {
                if let Some(kfs) = property.keyframes() {
                    if let Some(target) = kfs.iter().rev().find(|k| k.frame < current_frame) {
                        next_frame = Some(target.frame);
                    }
                }
            }

        let stopwatch_btn = if has_keyframes { "[K]" } else { "[+]" };
        if custom_widgets::ae_button(ui, stopwatch_btn).on_hover_text(if has_keyframes { "Disable Keyframes" } else { "Enable Keyframes / Add Keyframe" }).clicked() {
            if has_keyframes {
                let current_val = property.evaluate(current_frame);
                *property = Animatable::Constant(current_val);
            } else {
                let current_val = property.evaluate(current_frame);
                *property = Animatable::Animated(vec![
                    Keyframe::new(current_frame, current_val, InterpolationType::Linear)
                ]);
            }
        }

        if has_keyframes {
            if custom_widgets::ae_icon_button(ui, "▶", "Jump to Next Keyframe (K)").clicked() {
                if let Some(kfs) = property.keyframes() {
                    if let Some(target) = kfs.iter().find(|k| k.frame > current_frame) {
                        next_frame = Some(target.frame);
                    }
                }
            }
            ui.menu_button("Ease", |ui| {
                if ui.button("Easy Ease (F9)").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
                            };
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Ease In").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.1, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.75, speed: 0.0 },
                                custom_bezier: Some([0.75, 0.0, 1.0, 1.0]),
                            };
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Ease Out").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Bezier {
                                outgoing: BezierControlPoint { influence: 0.75, speed: 0.0 },
                                incoming: BezierControlPoint { influence: 0.1, speed: 0.0 },
                                custom_bezier: Some([0.0, 0.0, 0.25, 1.0]),
                            };
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Linear").clicked() {
                    if let Animatable::Animated(ref mut kfs) = property {
                        for kf in kfs {
                            kf.interpolation = InterpolationType::Linear;
                        }
                    }
                    ui.close_menu();
                }
            });
        }

        // 🔗 Property Link / Quick Presets (@)
        ui.menu_button("@", |ui| {
            ui.label(egui::RichText::new("🔗 Motion Presets (@)").strong());
            if ui.button("⚡ Easy Ease (F9)").clicked() {
                property.easy_ease();
                ui.close_menu();
            }
            if ui.button("🌊 Sine Wave Ease").clicked() {
                if let Animatable::Animated(ref mut kfs) = property {
                    for kf in kfs {
                        kf.interpolation = InterpolationType::Bezier {
                            outgoing: BezierControlPoint { influence: 0.5, speed: 0.0 },
                            incoming: BezierControlPoint { influence: 0.5, speed: 0.0 },
                            custom_bezier: Some([0.37, 0.0, 0.63, 1.0]),
                        };
                    }
                }
                ui.close_menu();
            }
        });

        let mut temp_val = property.evaluate(current_frame);
        draw_value_widget(ui, &mut temp_val);

        match property {
            Animatable::Constant(val) => {
                if *val != temp_val {
                    *val = temp_val;
                }
            }
            Animatable::Animated(keyframes) => {
                let existing_idx = keyframes.iter().position(|kf| kf.frame == current_frame);
                if let Some(idx) = existing_idx {
                    keyframes[idx].value = temp_val;
                } else {
                    let evaluated = property.evaluate(current_frame);
                    if temp_val != evaluated {
                        property.add_keyframe(Keyframe::new(current_frame, temp_val, InterpolationType::Linear));
                    }
                }
            }
        }
    });

    next_frame
}

fn test_expression_with_comp(
    script: &str,
    frame: u32,
    fps: u32,
    snap: &crate::core::expression_engine::CompSnapshot,
) -> String {
    let base_v2 = [0.0f32, 0.0];
    let result = crate::core::expression_engine::eval_v2_with_comp(
        script, base_v2, frame, fps, snap, None,
    );
    format!("({:.2}, {:.2})", result[0], result[1])
}
