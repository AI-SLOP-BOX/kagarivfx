use eframe::egui;
use rhai::Dynamic;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

/// Immutable snapshot of the context expressions need, taken before any
/// mutable borrows so automation runs can coexist with the console UI.
struct ConsoleCtx {
    frame: f64,
    fps: f64,
    time: f64,
    comp_width: f64,
    comp_height: f64,
    comp_name: String,
    num_layers: i64,
}

pub fn draw_scripting_console(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    // Console state
    if app.script_console_output.is_none() {
        app.script_console_output = Some(vec![
            "[INFO] Rhai Expression Engine v1.0 initialized".to_string(),
            "[INFO] Available functions: wiggle, loopOut, linear, ease, random, sin, cos, abs, sqrt, floor, ceil".to_string(),
        ]);
    }
    if app.script_console_history.is_none() {
        app.script_console_history = Some(Vec::new());
    }

    // Take owned copies to avoid borrow conflicts with automation runs
    let mut output = app.script_console_output.take().unwrap_or_default();
    let mut history = app.script_console_history.take().unwrap_or_default();

    let ctx_data = {
        let comp = app.history.current().active_composition();
        let fps = comp.fps as f64;
        let frame = app.current_frame as f64;
        ConsoleCtx {
            frame,
            fps,
            time: frame / fps.max(1.0),
            comp_width: comp.width as f64,
            comp_height: comp.height as f64,
            comp_name: comp.name.clone(),
            num_layers: comp.layers.len() as i64,
        }
    };

    crate::ui::custom_widgets::ae_section_header(ui, "Console", "💻");

    // ── Mode toggle ──
    let mode_id = egui::Id::new("script_console_automation");
    let automation_mode = ui.ctx().data_mut(|d| d.get_temp::<bool>(mode_id).unwrap_or(false));
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Mode:").small().color(colors::TEXT_SECONDARY));
        for (label, want) in [("Expression", false), ("Automation", true)] {
            if ui
                .selectable_label(automation_mode == want, label)
                .on_hover_text(if want {
                    "Mutate the project: new_comp / add_solid / add_text / set_position / key_position / save_project"
                } else {
                    "Read-only expressions with time/frame context"
                })
                .clicked()
            {
                ui.ctx().data_mut(|d| d.insert_temp(mode_id, want));
            }
        }
    });

    ui.add_space(4.0);

    // ── Run / Clear / Snippets ──
    let mut selected_snippet = None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Snippets:").small().color(colors::TEXT_SECONDARY));
        if ui.small_button("🌊 Wiggle").clicked() { selected_snippet = Some("wiggle(3.0, 25.0)".to_string()); }
        if ui.small_button("🔁 LoopOut").clicked() { selected_snippet = Some("loopOut(\"cycle\")".to_string()); }
        if ui.small_button("⏱ PingPong").clicked() { selected_snippet = Some("loopOut(\"pingpong\")".to_string()); }
        if ui.small_button("✨ Noise").clicked() { selected_snippet = Some("sin(time * 5.0) * 20.0".to_string()); }
    });
    if let Some(snip) = selected_snippet {
        app.script_console_command = snip;
    }

    ui.add_space(2.0);
    let run_requested = ui.horizontal(|ui| {
        let mut run = false;
        if crate::ui::custom_widgets::ae_button(ui, "▶ Run Script").clicked() {
            run = true;
        }
        if crate::ui::custom_widgets::ae_button(ui, "🗑 Clear").clicked() {
            output.clear();
            output.push("[INFO] Console cleared".to_string());
        }
        run
    }).inner;

    // ── Execute (single mutable pass over app) ──
    if run_requested && !app.script_console_command.trim().is_empty() {
        let cmd = std::mem::take(&mut app.script_console_command);
        let start = std::time::Instant::now();
        let result: Result<String, String> = if automation_mode {
            run_automation(app, &cmd).map(|lines| lines.join("\n"))
        } else {
            evaluate_script(&cmd, &ctx_data)
        };
        let elapsed = start.elapsed().as_secs_f64();

        history.push(cmd.clone());
        match result {
            Ok(val) => output.push(format!("[OK] {} → {} ({:.4}s)", cmd, val, elapsed)),
            Err(e) => output.push(format!("[ERR] {} → {} ({:.4}s)", cmd, e, elapsed)),
        }
        if automation_mode {
            crate::core::frame_cache::bump_version();
        }
        if output.len() > 200 {
            output.drain(0..50);
        }
    }

    // ── Output ──
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

    // ── Command input ──
    ui.label(egui::RichText::new("Command:").small().color(colors::TEXT_SECONDARY));
    let response = ui.add(
        egui::TextEdit::singleline(&mut app.script_console_command)
            .hint_text(if automation_mode {
                "e.g. add_text(\"T\", \"Hi\", 48); key_position(\"T\", 0, 100.0, 100.0)"
            } else {
                "e.g. thisComp.layer(\"Layer 1\").transform.position, wiggle(5, 50)"
            })
            .desired_width(ui.available_width() - 80.0),
    );

    // ── IntelliSense Dynamic Layer / Property Autocomplete ──
    let mut autocomplete_token = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("💡 IntelliSense:").small().color(colors::TEXT_MUTED));
        let comp = app.history.current().active_composition();
        for layer in comp.layers.iter().take(4) {
            let token = format!("thisComp.layer(\"{}\")", layer.name);
            if ui.small_button(&layer.name).on_hover_text(&token).clicked() {
                autocomplete_token = Some(token);
            }
        }
        for prop_token in [".transform.position", ".transform.opacity", ".transform.rotation", "time", "frame", "fps"] {
            if ui.small_button(prop_token).clicked() {
                autocomplete_token = Some(prop_token.to_string());
            }
        }
    });
    if let Some(token) = autocomplete_token {
        app.script_console_command.push_str(&token);
    }
    let enter_run = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

    if enter_run && !app.script_console_command.trim().is_empty() {
        let cmd = std::mem::take(&mut app.script_console_command);
        let start = std::time::Instant::now();
        let result: Result<String, String> = if automation_mode {
            run_automation(app, &cmd).map(|lines| lines.join("\n"))
        } else {
            evaluate_script(&cmd, &ctx_data)
        };
        let elapsed = start.elapsed().as_secs_f64();
        history.push(cmd.clone());
        match result {
            Ok(val) => output.push(format!("[OK] {} → {} ({:.4}s)", cmd, val, elapsed)),
            Err(e) => output.push(format!("[ERR] {} → {} ({:.4}s)", cmd, e, elapsed)),
        }
        if automation_mode {
            crate::core::frame_cache::bump_version();
        }
        if output.len() > 200 {
            output.drain(0..50);
        }
    }

    // ── History ──
    if !history.is_empty() {
        ui.add_space(2.0);
        ui.collapsing(format!("History ({})", history.len()), |ui| {
            for cmd in history.iter().rev().take(20) {
                ui.label(egui::RichText::new(format!("> {}", cmd)).small().monospace().color(colors::TEXT_MUTED));
            }
        });
    }

    // Return owned state
    app.script_console_output = Some(output);
    app.script_console_history = Some(history);
}

fn evaluate_script(script: &str, c: &ConsoleCtx) -> Result<String, String> {
    let engine = crate::core::expression_engine::build_engine();
    let mut scope = rhai::Scope::new();
    scope.push("time", c.time);
    scope.push("frame", c.frame);
    scope.push("fps", c.fps);
    scope.push("comp_width", c.comp_width);
    scope.push("comp_height", c.comp_height);
    scope.push("comp_name", c.comp_name.clone());
    scope.push("num_layers", c.num_layers);

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

/// Execute an automation snippet against the live project.
fn run_automation(app: &mut AfterEffectsApp, source: &str) -> Result<Vec<String>, String> {
    let project = app.history.current_mut();
    crate::core::automation::run_script(project, source)
}
