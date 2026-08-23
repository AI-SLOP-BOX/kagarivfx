use eframe::egui;
use rhai::Dynamic;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_scripting_console(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    // Ensure console state exists
    if app.script_console_output.is_none() {
        app.script_console_output = Some(vec![
            "[INFO] Rhai Expression Engine v1.0 initialized".to_string(),
            "[INFO] Available functions: wiggle, loopOut, linear, ease, random, sin, cos, abs, sqrt, floor, ceil".to_string(),
        ]);
    }
    if app.script_console_history.is_none() {
        app.script_console_history = Some(Vec::new());
    }

    let output = app.script_console_output.as_mut().unwrap();
    let history = app.script_console_history.as_mut().unwrap();

    crate::ui::custom_widgets::ae_section_header(ui, "Console", "💻");

    let comp_ref = app.history.current().active_composition();
    let cf = app.current_frame;

    ui.horizontal(|ui| {
        if crate::ui::custom_widgets::ae_button(ui, "▶ Run Script").clicked() {
            // Evaluate the current command
            let cmd = app.script_console_command.clone();
            if !cmd.trim().is_empty() {
                let start = std::time::Instant::now();
                let result = evaluate_script(&cmd, comp_ref, cf);
                let elapsed = start.elapsed().as_secs_f64();

                history.push(cmd.clone());
                match result {
                    Ok(val) => {
                        output.push(format!("[OK] {} → {} ({:.4}s)", cmd, val, elapsed));
                    }
                    Err(e) => {
                        output.push(format!("[ERR] {} → {} ({:.4}s)", cmd, e, elapsed));
                    }
                }
                // Keep output bounded
                if output.len() > 200 {
                    output.drain(0..50);
                }
            }
        }
        if crate::ui::custom_widgets::ae_button(ui, "🗑 Clear").clicked() {
            output.clear();
            output.push("[INFO] Console cleared".to_string());
        }
    });

    ui.add_space(4.0);

    // Console output
    ui.label(egui::RichText::new("Output:").small().color(colors::TEXT_SECONDARY));
    egui::ScrollArea::vertical().max_height(140.0).stick_to_bottom(true).show(ui, |ui| {
        for line in output.iter() {
            let color = if line.starts_with("[ERR]") {
                colors::ACCENT_RED
            } else if line.starts_with("[OK]") {
                colors::ACCENT_GREEN
            } else {
                colors::TEXT_SECONDARY
            };
            ui.label(egui::RichText::new(line).small().monospace().color(color));
        }
    });

    ui.add_space(4.0);
    ui.separator();

    // Command input
    ui.label(egui::RichText::new("Command:").small().color(colors::TEXT_SECONDARY));
    ui.horizontal(|ui| {
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.script_console_command)
                .hint_text("e.g. wiggle(5, 50), 2 + 2, thisComp.activeItem.name")
                .desired_width(ui.available_width() - 80.0),
        );

        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            // Execute on Enter
            let cmd = app.script_console_command.clone();
            if !cmd.trim().is_empty() {
                let start = std::time::Instant::now();
                let result = evaluate_script(&cmd, comp_ref, cf);
                let elapsed = start.elapsed().as_secs_f64();

                history.push(cmd.clone());
                match result {
                    Ok(val) => {
                        output.push(format!("[OK] {} → {} ({:.4}s)", cmd, val, elapsed));
                    }
                    Err(e) => {
                        output.push(format!("[ERR] {} → {} ({:.4}s)", cmd, e, elapsed));
                    }
                }
                if output.len() > 200 {
                    output.drain(0..50);
                }
                app.script_console_command.clear();
            }
        }
    });

    // History
    if !history.is_empty() {
        ui.add_space(2.0);
        ui.collapsing(format!("History ({})", history.len()), |ui| {
            for cmd in history.iter().rev().take(20) {
                ui.label(egui::RichText::new(format!("> {}", cmd)).small().monospace().color(colors::TEXT_MUTED));
            }
        });
    }
}

fn evaluate_script(script: &str, comp: &crate::core::timeline::Composition, current_frame: u32) -> Result<String, String> {
    let engine = crate::core::expression_engine::build_engine();
    let mut scope = rhai::Scope::new();

    // Inject current context
    let fps = comp.fps as f64;
    let frame = current_frame as f64;
    let time = frame / fps;

    scope.push("time", time);
    scope.push("frame", frame);
    scope.push("fps", fps);
    scope.push("comp_width", comp.width as f64);
    scope.push("comp_height", comp.height as f64);
    scope.push("comp_name", comp.name.clone());

    // Add composition info
    scope.push("num_layers", comp.layers.len() as i64);

    // Try to evaluate
    let result = engine.eval_with_scope::<Dynamic>(&mut scope, script);

    match result {
        Ok(val) => {
            if let Some(s) = val.clone().try_cast::<String>() {
                Ok(format!("\"{}\"", s))
            } else if let Some(f) = val.clone().try_cast::<f64>() {
                Ok(format!("{:.4}", f))
            } else if let Some(i) = val.clone().try_cast::<i64>() {
                Ok(format!("{}", i))
            } else if let Some(b) = val.clone().try_cast::<bool>() {
                Ok(if b { "true".to_string() } else { "false".to_string() })
            } else {
                Ok(format!("{:?}", val))
            }
        }
        Err(e) => Err(format!("{}", e)),
    }
}
