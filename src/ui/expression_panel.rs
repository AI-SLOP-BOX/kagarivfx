use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_expression_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    let layer_idx = match app.selection.selected_layer_idx {
        Some(idx) => idx,
        _ => {
            ui.label(
                egui::RichText::new("No layer selected.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            return;
        }
    };

    // Clone needed data before any mutable borrows
    let layer_name = {
        let comp = app.history.current().active_composition();
        if layer_idx >= comp.layers.len() {
            ui.label(
                egui::RichText::new("Invalid layer.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            return;
        }
        comp.layers[layer_idx].name.clone()
    };

    crate::ui::custom_widgets::ae_section_header(ui, "Expression", "📝");

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Layer:")
                .small()
                .color(colors::TEXT_SECONDARY),
        );
        ui.label(
            egui::RichText::new(&layer_name)
                .small()
                .strong()
                .color(colors::ACCENT_CYAN),
        );
    });

    // Property selector
    let properties = ["Position", "Scale", "Rotation", "Opacity", "Anchor Point"];

    ui.add_space(2.0);
    ui.label(
        egui::RichText::new("Target Property")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
    let mut selected_prop = app.selected_expression_prop_idx.min(properties.len() - 1);
    ui.horizontal(|ui| {
        for (i, label) in properties.iter().enumerate() {
            let is_active = selected_prop == i;
            if ui
                .selectable_label(
                    is_active,
                    egui::RichText::new(*label).small().color(if is_active {
                        colors::ACCENT_CYAN
                    } else {
                        colors::TEXT_PRIMARY
                    }),
                )
                .clicked()
            {
                selected_prop = i;
            }
        }
    });
    app.selected_expression_prop_idx = selected_prop;

    let prop_name = properties[selected_prop];

    // Get current expression string
    let current_expr = {
        let comp = app.history.current().active_composition();
        if layer_idx < comp.layers.len() {
            get_expression_string(&comp.layers[layer_idx].transform, selected_prop)
        } else {
            String::new()
        }
    };

    // Expression editor
    ui.add_space(4.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Script Editor", "💻");

    let mut script = current_expr.clone();

    // ── IntelliSense: completion popup + live syntax indicator ──
    let completions = completions_for(&script);
    let cursor_prefix = script
        .rsplit(|c: char| c.is_alphanumeric() || c == '_' || c == '.')
        .next()
        .unwrap_or("")
        .to_string();
    let suggestions: Vec<&str> = if cursor_prefix.len() >= 2 {
        completions
            .iter()
            .copied()
            .filter(|s| s.to_lowercase().contains(&cursor_prefix.to_lowercase()))
            .take(6)
            .collect()
    } else {
        Vec::new()
    };

    ui.add(
        egui::TextEdit::multiline(&mut script)
            .code_editor()
            .desired_rows(6)
            .desired_width(ui.available_width()),
    );

    // Live syntax check (compile only — no execution)
    let syntax_status: Result<(), String> = if script.trim().is_empty() {
        Ok(())
    } else {
        crate::core::expression_engine::build_engine()
            .compile(&script)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    match &syntax_status {
        Ok(()) => {}
        Err(msg) => {
            ui.label(
                egui::RichText::new(format!("✕ {}", msg))
                    .small()
                    .color(colors::ACCENT_RED),
            );
        }
    }

    if !suggestions.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for s in suggestions.iter() {
                if ui
                    .small_button(*s)
                    .on_hover_text("Click to insert at end")
                    .clicked()
                {
                    script.push_str(s);
                }
            }
        });
    }

    ui.add_space(4.0);

    // Action buttons
    let mut apply_expr = None;
    let mut remove_expr = false;
    let mut test_expr = false;

    ui.horizontal(|ui| {
        if crate::ui::custom_widgets::ae_button_accent(ui, "▶ Apply").clicked()
            && !script.trim().is_empty()
        {
            apply_expr = Some(script.clone());
        }
        if crate::ui::custom_widgets::ae_button(ui, "▶ Test").clicked() && !script.trim().is_empty()
        {
            test_expr = true;
        }
        if crate::ui::custom_widgets::ae_button(ui, "✕ Remove").clicked() {
            remove_expr = true;
        }
    });

    // Apply expression
    if let Some(expr) = apply_expr {
        let comp = app.history.current_mut().active_composition_mut();
        if layer_idx < comp.layers.len() {
            set_expression(&mut comp.layers[layer_idx].transform, selected_prop, &expr);
            crate::core::frame_cache::bump_version();
            app.toasts.info(format!(
                "Applied expression to {}.{}",
                layer_name, prop_name
            ));
        }
    }

    // Remove expression
    if remove_expr {
        let comp = app.history.current_mut().active_composition_mut();
        if layer_idx < comp.layers.len() {
            set_expression(&mut comp.layers[layer_idx].transform, selected_prop, "");
            crate::core::frame_cache::bump_version();
            app.toasts.info(format!(
                "Removed expression from {}.{}",
                layer_name, prop_name
            ));
        }
    }

    // Test expression
    if test_expr {
        let comp = app.history.current().active_composition();
        let cf = app.playback.current_frame;
        let result = test_expression(&script, comp, cf);
        app.toasts.info(result);
    }

    ui.add_space(4.0);

    // Presets
    crate::ui::custom_widgets::ae_section_header(ui, "Presets", "⚡");
    let presets = [
        ("wiggle(4, 30)", "Wiggle"),
        ("loopOut(\"cycle\")", "Loop Out"),
        ("loopOut(\"pingpong\")", "Ping Pong"),
        ("value * 2", "Double Value"),
        ("time * 100", "Time Drive"),
        ("Math.sin(time * 3) * 50", "Sine Wave"),
        ("posterizeTime(12); value", "Posterize 12fps"),
    ];

    let mut preset_expr = None;
    egui::ScrollArea::vertical()
        .max_height(100.0)
        .show(ui, |ui| {
            for (expr, label) in presets {
                if ui
                    .selectable_label(
                        false,
                        egui::RichText::new(format!("{} — {}", label, expr))
                            .small()
                            .monospace()
                            .color(colors::TEXT_SECONDARY),
                    )
                    .clicked()
                {
                    preset_expr = Some(expr.to_string());
                }
            }
        });

    if let Some(expr) = preset_expr {
        let comp = app.history.current_mut().active_composition_mut();
        if layer_idx < comp.layers.len() {
            set_expression(&mut comp.layers[layer_idx].transform, selected_prop, &expr);
            crate::core::frame_cache::bump_version();
            app.toasts.info(format!("Applied preset to {}", prop_name));
        }
    }
}

fn get_expression_string(
    transform: &crate::core::timeline::Transform2D,
    prop_idx: usize,
) -> String {
    let opt = match prop_idx {
        0 => &transform.position_expression,
        1 => &transform.scale_expression,
        2 => &transform.rotation_expression,
        3 => &transform.opacity_expression,
        4 => &transform.anchor_point_expression,
        _ => return String::new(),
    };
    match opt.as_ref() {
        Some(crate::core::timeline::Expression::Raw(s)) => s.clone(),
        Some(crate::core::timeline::Expression::Wiggle {
            frequency,
            amplitude,
        }) => {
            format!("wiggle({}, {})", frequency, amplitude)
        }
        Some(crate::core::timeline::Expression::TimeDriver { multiplier, offset }) => {
            format!("time * {} + {}", multiplier, offset)
        }
        Some(crate::core::timeline::Expression::LoopOut) => "loopOut(\"cycle\")".to_string(),
        Some(crate::core::timeline::Expression::PingPong) => "loopOut(\"pingpong\")".to_string(),
        None => String::new(),
    }
}

fn set_expression(
    transform: &mut crate::core::timeline::Transform2D,
    prop_idx: usize,
    script: &str,
) {
    let expr = if script.trim().is_empty() {
        None
    } else {
        Some(crate::core::timeline::Expression::Raw(script.to_string()))
    };
    match prop_idx {
        0 => transform.position_expression = expr,
        1 => transform.scale_expression = expr,
        2 => transform.rotation_expression = expr,
        3 => transform.opacity_expression = expr,
        4 => transform.anchor_point_expression = expr,
        _ => {}
    }
}

fn test_expression(
    script: &str,
    comp: &crate::core::timeline::Composition,
    current_frame: u32,
) -> String {
    if script.trim().is_empty() {
        return "Empty expression".to_string();
    }

    let engine = crate::core::expression_engine::build_engine();
    let mut scope = rhai::Scope::new();

    let fps = comp.fps as f64;
    let frame = current_frame as f64;
    let time = frame / fps;

    scope.push("time", time);
    scope.push("frame", frame);
    scope.push("fps", fps);
    scope.push("comp_width", comp.width as f64);
    scope.push("comp_height", comp.height as f64);

    match engine.eval_with_scope::<rhai::Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Some(f) = val.clone().try_cast::<f64>() {
                format!("Result @f{}: {:.4}", current_frame, f)
            } else {
                format!("Result @f{}: {:?}", current_frame, val)
            }
        }
        Err(e) => format!("Error: {}", e),
    }
}

/// Static suggestion dictionary covering the AE-style API surface.
fn completions_for(script: &str) -> Vec<&'static str> {
    let _ = script;
    vec![
        "thisComp",
        "thisLayer",
        "thisComp.layer(",
        "thisComp.layer(index)",
        "transform.position",
        "transform.scale",
        "transform.rotation",
        "transform.opacity",
        ".effect_param(\"Effect\", \"Param\")",
        "wiggle(freq, amp)",
        "loopOut(\"cycle\")",
        "loopOut(\"pingpong\")",
        "loopIn(\"cycle\")",
        "linear(t, a, b, c, d)",
        "ease(t, a, b, c, d)",
        "random(min, max)",
        "time",
        "index",
        "value",
        "fps",
        "toComp(x, y)",
        "fromComp(x, y)",
        "Math.sin(",
        "Math.cos(",
        "Math.PI",
        "Math.abs(",
        "Math.min(",
        "Math.max(",
    ]
}
