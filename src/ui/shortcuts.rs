use eframe::egui::{self, Key};
use crate::AfterEffectsApp;

/// Return platform-native command modifier label ("Cmd" on macOS, "Ctrl" on Windows/Linux)
pub fn cmd_name() -> &'static str {
    if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" }
}

/// Return platform-native option modifier label ("Option" on macOS, "Alt" on Windows/Linux)
pub fn option_name() -> &'static str {
    if cfg!(target_os = "macos") { "Option" } else { "Alt" }
}

/// Format a keyboard shortcut string dynamically based on host OS target platform.
pub fn format_shortcut(key: &str, cmd: bool, shift: bool, alt: bool) -> String {
    let mut parts = Vec::new();
    if cmd { parts.push(cmd_name()); }
    if alt { parts.push(option_name()); }
    if shift { parts.push("Shift"); }
    parts.push(key);
    parts.join("+")
}

/// Centralized Keyboard Shortcut Manager for After Effects OSS.
/// Encapsulates global keybindings: Spacebar playback, frame stepping, keyframe navigation (J/K),
/// Easy Ease (F9), Undo/Redo, Pre-compose (Cmd+Shift+C), Duplicate/Split (Cmd+D / Cmd+Shift+D),
/// Layer Deletion, property selection (P, S, T, R), and dialog triggers.
pub fn handle_global_shortcuts(
    app: &mut AfterEffectsApp,
    ctx: &egui::Context,
    current_frame: &mut u32,
    total_frames: u32,
) {
    // ── Two-Tier Focus Guard ────────────────────────────────────────────────
    // Tier 1: is a text-edit widget actively focused?
    let text_focused = crate::ui::focus::is_text_input_focused(ctx);
    // Tier 2: does egui report that it wants all keyboard input right now?
    //   (e.g. an active popup/combobox that routes every key through its own handler)
    let wants_all_keys = ctx.wants_keyboard_input();

    // When `wants_all_keys` is true, block ALL shortcuts unconditionally.
    if wants_all_keys {
        return;
    }

    // When a text field is focused, only allow modifier-based shortcuts
    // (Cmd+Z, Cmd+Shift+Z, Cmd+M, etc.) to pass through.
    // Single-character shortcuts (Space, J, K, P, S, T, R, Delete …) are suppressed.
    let allow_single_key = !text_focused;

    ctx.input(|i| {
        let cmd = i.modifiers.command;
        let shift = i.modifiers.shift;

        // Space → Play / Pause RAM Preview (single-key: suppressed while typing)
        if allow_single_key && i.key_pressed(Key::Space) {
            app.is_playing = !app.is_playing;
            if !app.is_playing && app.motion_sketch_active {
                app.motion_sketch_active = false;
                app.toasts.info("Motion Sketch OFF");
            }
        }

        // ── Tool Switching Shortcuts (AE standard) ──
        // Only when no modifier keys are pressed and text is not focused
        if allow_single_key && !cmd && !shift && !i.modifiers.alt {
            if i.key_pressed(Key::V) { app.active_tool = crate::ui::toolbar::ActiveTool::Selection; }
            if i.key_pressed(Key::H) { app.active_tool = crate::ui::toolbar::ActiveTool::Hand; }
            if i.key_pressed(Key::Z) { app.active_tool = crate::ui::toolbar::ActiveTool::Zoom; }
            if i.key_pressed(Key::W) { app.active_tool = crate::ui::toolbar::ActiveTool::Rotation; }
            if i.key_pressed(Key::Y) { app.active_tool = crate::ui::toolbar::ActiveTool::AnchorPoint; }
            if i.key_pressed(Key::Q) { app.active_tool = crate::ui::toolbar::ActiveTool::Rectangle; }
            if i.key_pressed(Key::G) { app.active_tool = crate::ui::toolbar::ActiveTool::Pen; }
            if i.key_pressed(Key::C) { app.active_tool = crate::ui::toolbar::ActiveTool::Camera3D; }
        }
        // Cmd+T → Text tool (bare T reveals Opacity, AE parity)
        if cmd && !shift && i.key_pressed(Key::T) {
            app.active_tool = crate::ui::toolbar::ActiveTool::Text;
        }

        // Shift+Z → Viewport zoom to fit (AE parity)
        if shift && !cmd && i.key_pressed(Key::Z) {
            app.viewport_mag_ratio = 0.0; // 0.0 = fit mode
            app.viewport_pan = egui::Vec2::ZERO;
        }
        // Cmd+0 → Viewport zoom to fit (AE standard)
        if cmd && i.key_pressed(Key::Num0) {
            app.viewport_mag_ratio = 0.0;
            app.viewport_pan = egui::Vec2::ZERO;
        }

        // B → Set Work Area Start, N → Set Work Area End (single-key)
        if allow_single_key && i.key_pressed(Key::B) && !cmd {
            app.work_area_in = Some(*current_frame);
        }
        if allow_single_key && i.key_pressed(Key::N) && !cmd {
            app.work_area_out = Some(*current_frame);
        }

        // ── I / O → Jump to selected layer's in-point / out-point (AE parity) ──
        if allow_single_key && !cmd && i.key_pressed(Key::I) {
            if let Some(idx) = app.selected_layer_idx {
                let comp = app.history.current().active_composition();
                if let Some(layer) = comp.layers.get(idx) {
                    *current_frame = layer.in_frame;
                }
            }
        }
        if allow_single_key && !cmd && i.key_pressed(Key::O) {
            if let Some(idx) = app.selected_layer_idx {
                let comp = app.history.current().active_composition();
                if let Some(layer) = comp.layers.get(idx) {
                    *current_frame = layer.out_frame;
                }
            }
        }

        // ── L: Shuttle Forward (press again to increase speed up to 3x) ──
        // J/K are reserved for keyframe navigation below (AE standard).
        if allow_single_key && !cmd && !shift {
            if i.key_pressed(Key::L) {
                if app.is_playing && app.playback_speed > 0 {
                    // Already playing forward: increase speed (max 3x)
                    app.playback_speed = (app.playback_speed + 1).min(3);
                } else {
                    app.is_playing = true;
                    app.playback_speed = 1;
                }
                app.toasts.info(format!("▶ Forward {}x", app.playback_speed));
            }
            if i.key_pressed(Key::F9) {
                if let Some(idx) = app.selected_layer_idx {
                    let mut temp_proj = app.history.current().clone();
                    let comp = temp_proj.active_composition_mut();
                    if idx < comp.layers.len() {
                        let layer = &mut comp.layers[idx];
                        let ez = if shift {
                            // Easy Ease In (Shift+F9)
                            crate::core::keyframe::InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.0, speed: 0.0 },
                                incoming: crate::core::keyframe::BezierControlPoint { influence: 0.85, speed: 0.0 },
                                custom_bezier: Some([0.85, 0.0, 1.0, 1.0]),
                            }
                        } else if cmd {
                            // Easy Ease Out (Cmd+Shift+F9 / Cmd+F9)
                            crate::core::keyframe::InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.85, speed: 0.0 },
                                incoming: crate::core::keyframe::BezierControlPoint { influence: 0.0, speed: 0.0 },
                                custom_bezier: Some([0.0, 0.0, 0.15, 1.0]),
                            }
                        } else {
                            // Easy Ease (F9)
                            crate::core::keyframe::InterpolationType::Bezier {
                                outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
                            }
                        };
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.position { for kf in kfs { kf.interpolation = ez; } }
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.scale { for kf in kfs { kf.interpolation = ez; } }
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.rotation { for kf in kfs { kf.interpolation = ez; } }
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.opacity { for kf in kfs { kf.interpolation = ez; } }
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                    }
                }
            }
        }

        // ── AE-style keyframe navigation & transport ──
        // J → previous keyframe, K → next keyframe (AE standard).
        // Collects keyframe times across all transform properties of the selected layer.
        if allow_single_key && (i.key_pressed(Key::J) || i.key_pressed(Key::K)) {
            let going_next = i.key_pressed(Key::K);
            if let Some(idx) = app.selected_layer_idx {
                let comp = app.history.current().active_composition();
                if let Some(layer) = comp.layers.get(idx) {
                    let t = &layer.transform;
                    let mut kf_frames: Vec<u32> = Vec::new();
                    for prop in [&t.position, &t.scale] {
                        if let Some(kfs) = prop.keyframes() {
                            kf_frames.extend(kfs.iter().map(|k| k.frame));
                        }
                    }
                    for prop in [&t.rotation, &t.opacity] {
                        if let Some(kfs) = prop.keyframes() {
                            kf_frames.extend(kfs.iter().map(|k| k.frame));
                        }
                    }
                    kf_frames.sort_unstable();
                    kf_frames.dedup();
                    let cur = *current_frame;
                    let target = if going_next {
                        kf_frames.iter().find(|&&f| f > cur).copied()
                    } else {
                        kf_frames.iter().rev().find(|&&f| f < cur).copied()
                    };
                    if let Some(f) = target {
                        *current_frame = f;
                    }
                }
            }
        }

        // Home → first frame, End → last frame (single-key)
        if allow_single_key && i.key_pressed(Key::Home) { *current_frame = 0; }
        if allow_single_key && i.key_pressed(Key::End)  { *current_frame = total_frames.saturating_sub(1); }

        // Page Up / Down → frame step backward/forward (always available)
        if allow_single_key && i.key_pressed(Key::PageUp) {
            *current_frame = current_frame.saturating_sub(1);
        }
        if allow_single_key && i.key_pressed(Key::PageDown) {
            *current_frame = (*current_frame + 1).min(total_frames.saturating_sub(1));
        }

        // Arrow keys → nudge selected layer position by 1 px (Shift = 10 px),
        // matching AE. Falls back to frame stepping when no layer is selected.
        let step = if i.modifiers.shift { 10.0 } else { 1.0 };
        let cur_frame = *current_frame;
        let mut arrow_nudge = |dx: f32, dy: f32| -> bool {
            let Some(idx) = app.selected_layer_idx else { return false };
            let project = app.history.current_mut();
            let Some(comp) = project.active_composition_mut().layers.get_mut(idx) else {
                return false;
            };
            let cur = comp.transform.position.evaluate(cur_frame);
            comp.transform.position = crate::core::property::Animatable::new_constant([
                cur[0] + dx,
                cur[1] + dy,
            ]);
            true
        };
        let mut nudged = false;
        if allow_single_key && i.key_pressed(Key::ArrowLeft) {
            nudged = arrow_nudge(-step, 0.0);
            if !nudged {
                *current_frame = cur_frame.saturating_sub(1);
            }
        }
        if allow_single_key && i.key_pressed(Key::ArrowRight) {
            nudged = arrow_nudge(step, 0.0);
            if !nudged {
                *current_frame = (cur_frame + 1).min(total_frames.saturating_sub(1));
            }
        }
        if allow_single_key && i.key_pressed(Key::ArrowUp) {
            nudged = arrow_nudge(0.0, -step);
        }
        if allow_single_key && i.key_pressed(Key::ArrowDown) {
            nudged = arrow_nudge(0.0, step);
        }
        if nudged {
            crate::core::frame_cache::bump_version();
        }

        // ── Batch-move selected keyframes with , / . (comma/period) ──
        // Comma shifts all selected keyframes 1 frame left, period right (Shift = 10).
        if allow_single_key && (i.key_pressed(Key::Comma) || i.key_pressed(Key::Period)) {
            let delta: i32 = if i.key_pressed(Key::Comma) { -1 } else { 1 };
            let delta = if i.modifiers.shift { delta * 10 } else { delta };
            if !app.selected_keyframes.is_empty() {
                // Group by layer so we can borrow layers one at a time
                use std::collections::HashMap;
                let mut by_layer: HashMap<usize, Vec<(String, u32)>> = HashMap::new();
                for (li, pk, f) in app.selected_keyframes.iter() {
                    by_layer.entry(*li).or_default().push((pk.clone(), *f));
                }
                let project = app.history.current_mut();
                for (li, kfs) in by_layer {
                    let Some(comp) = project.active_composition_mut().layers.get_mut(li) else { continue };
                    let t = &mut comp.transform;
                    for (pk, old_f) in kfs {
                        let new_f = ((old_f as i32) + delta).max(0) as u32;
                        match pk.as_str() {
                            "position" => move_kf_in(&mut t.position, old_f, new_f),
                            "scale" => move_kf_in(&mut t.scale, old_f, new_f),
                            "rotation" => move_kf_in(&mut t.rotation, old_f, new_f),
                            "opacity" => move_kf_in(&mut t.opacity, old_f, new_f),
                            _ => {}
                        }
                    }
                }
                // Remap selection to the moved frames
                app.selected_keyframes = app
                    .selected_keyframes
                    .iter()
                    .map(|(li, pk, f)| (*li, pk.clone(), ((*f as i32) + delta).max(0) as u32))
                    .collect();
                crate::core::frame_cache::bump_version();
            }
        }

        // ── Delete / Backspace removes all selected keyframes ──
        if allow_single_key
            && (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
            && !app.selected_keyframes.is_empty()
        {
                use std::collections::HashMap;
                let mut by_layer: HashMap<usize, Vec<(String, u32)>> = HashMap::new();
                for (li, pk, f) in app.selected_keyframes.iter() {
                    by_layer.entry(*li).or_default().push((pk.clone(), *f));
                }
                let project = app.history.current_mut();
                for (li, kfs) in by_layer {
                    let Some(comp) = project.active_composition_mut().layers.get_mut(li) else { continue };
                    let t = &mut comp.transform;
                    for (pk, frame) in kfs {
                        match pk.as_str() {
                            "position" => delete_kf_at(&mut t.position, frame),
                            "scale" => delete_kf_at(&mut t.scale, frame),
                            "rotation" => delete_kf_at(&mut t.rotation, frame),
                            "opacity" => delete_kf_at(&mut t.opacity, frame),
                            _ => {}
                        }
                    }
                }
                app.selected_keyframes.clear();
                crate::core::frame_cache::bump_version();
        }

        // ── Cmd+A: select all layers (AE parity) ──
        if cmd && !shift && i.key_pressed(Key::A) {
            let count = app.history.current().active_composition().layers.len();
            app.selected_layers.clear();
            app.selected_layer_idx = Some(count.saturating_sub(1));
            for i in 0..count {
                app.selected_layers.insert(i);
            }
        }

        // ── Cmd+C / Cmd+V: copy & paste selected keyframes ──
        // Clipboard entries: (prop_key, frame_offset_from_anchor, serialized keyframe)
        if cmd && !shift && i.key_pressed(Key::C) && !app.selected_keyframes.is_empty() {
            let anchor = *current_frame;
            let mut clip: Vec<(String, i32, serde_json::Value)> = Vec::new();
            let mut selection: Vec<(usize, String, u32)> =
                app.selected_keyframes.iter().cloned().collect();
            selection.sort_by_key(|(_, _, f)| *f);

            let project = app.history.current();
            for (li, pk, frame) in selection {
                let Some(comp) = project.active_composition().layers.get(li) else { continue };
                let t = &comp.transform;
                // Serialize per-arm: value types differ across properties
                let kf_json = match pk.as_str() {
                    "position" => t.position.keyframes()
                        .and_then(|k| k.iter().find(|k| k.frame == frame))
                        .and_then(|k| serde_json::to_value(k).ok()),
                    "scale" => t.scale.keyframes()
                        .and_then(|k| k.iter().find(|k| k.frame == frame))
                        .and_then(|k| serde_json::to_value(k).ok()),
                    "rotation" => t.rotation.keyframes()
                        .and_then(|k| k.iter().find(|k| k.frame == frame))
                        .and_then(|k| serde_json::to_value(k).ok()),
                    "opacity" => t.opacity.keyframes()
                        .and_then(|k| k.iter().find(|k| k.frame == frame))
                        .and_then(|k| serde_json::to_value(k).ok()),
                    _ if pk.starts_with("fx_") => {
                        // Effect keyframe: find the effect + param and serialize
                        let parts: Vec<&str> = pk.strip_prefix("fx_").unwrap_or("").splitn(2, '_').collect();
                        if parts.len() == 2 {
                            let fx_name = parts[0];
                            let param_label = parts[1];
                            if let Some(effect) = comp.effects.iter().find(|e| e.name == fx_name) {
                                use crate::core::effect_params::ParamRefRef;
                                let mut found_json = None;
                                for (label, param) in effect.effect_type.animatable_params_ref() {
                                    if label == param_label {
                                        match param {
                                            ParamRefRef::Scalar(anim) => {
                                                if let Some(kfs) = anim.keyframes() {
                                                    found_json = kfs.iter().find(|k| k.frame == frame)
                                                        .and_then(|k| serde_json::to_value(k).ok());
                                                }
                                            }
                                            ParamRefRef::Vec2(anim) => {
                                                if let Some(kfs) = anim.keyframes() {
                                                    found_json = kfs.iter().find(|k| k.frame == frame)
                                                        .and_then(|k| serde_json::to_value(k).ok());
                                                }
                                            }
                                            ParamRefRef::Vec4Color(anim) => {
                                                if let Some(kfs) = anim.keyframes() {
                                                    found_json = kfs.iter().find(|k| k.frame == frame)
                                                        .and_then(|k| serde_json::to_value(k).ok());
                                                }
                                            }
                                        }
                                        break;
                                    }
                                }
                                found_json
                            } else { None }
                        } else { None }
                    }
                    _ => None,
                };
                if let Some(v) = kf_json {
                    let offset = frame as i32 - anchor as i32;
                    clip.push((pk.clone(), offset, v));
                }
            }
            if !clip.is_empty() {
                app.kf_clipboard = clip;
                app.kf_clipboard_anchor = anchor;
            }
        }
        if cmd && !shift && i.key_pressed(Key::V) && !app.kf_clipboard.is_empty() {
            let paste_origin = *current_frame;
            let target_layer_idx = app.selected_layer_idx.unwrap_or(0);
            let project = app.history.current_mut();
            let Some(layer) = project.active_composition_mut().layers.get_mut(target_layer_idx) else { return };
            let t = &mut layer.transform;

            macro_rules! paste_into {
                ($anim:expr, $ty:ty, $value_json:expr) => {{
                    if let Ok(mut kf) = serde_json::from_value::<crate::core::keyframe::Keyframe<$ty>>($value_json.clone()) {
                        kf.frame = ((kf.frame as i64 + paste_origin as i64
                            - app.kf_clipboard_anchor as i64)
                            .max(0)) as u32;
                        if let Some(kfs) = $anim.keyframes_mut() {
                            kfs.retain(|k| k.frame != kf.frame);
                            kfs.push(kf);
                            kfs.sort_by_key(|k| k.frame);
                        }
                    }
                }};
            }

            for (pk, _offset, value_json) in &app.kf_clipboard {
                match pk.as_str() {
                    "position" => paste_into!(t.position, [f32; 2], value_json),
                    "scale" => paste_into!(t.scale, [f32; 2], value_json),
                    "rotation" => paste_into!(t.rotation, f32, value_json),
                    "opacity" => paste_into!(t.opacity, f32, value_json),
                    _ if pk.starts_with("fx_") => {
                        // Paste into effect param
                        let stripped = pk.strip_prefix("fx_").unwrap_or("");
                        let parts: Vec<&str> = stripped.splitn(2, '_').collect();
                        if parts.len() == 2 {
                            let fx_name = parts[0];
                            let param_label = parts[1];
                            if let Some(effect) = layer.effects.iter_mut().find(|e| e.name == fx_name) {
                                use crate::core::effect_params::ParamRef;
                                for (label, param) in effect.effect_type.animatable_params() {
                                    if label == param_label {
                                        match param {
                                            ParamRef::Scalar(anim) => {
                                                paste_into!(anim, f32, value_json);
                                            }
                                            ParamRef::Vec2(anim) => {
                                                paste_into!(anim, [f32; 2], value_json);
                                            }
                                            ParamRef::Vec4Color(anim) => {
                                                paste_into!(anim, [f32; 4], value_json);
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            crate::core::frame_cache::bump_version();
        }

        // ── Escape: clear keyframe selection, then layer selection ──
        if allow_single_key && i.key_pressed(Key::Escape) {
            if !app.selected_keyframes.is_empty() {
                app.selected_keyframes.clear();
            } else {
                app.selected_layers.clear();
                app.selected_layer_idx = None;
            }
        }

        // F2 → Rename selected layer (standard file manager shortcut)
        if allow_single_key && i.key_pressed(Key::F2) {
            if let Some(idx) = app.selected_layer_idx {
                app.renaming_layer = Some(idx);
            }
        }

        if cmd && !shift && i.key_pressed(Key::Z) && app.history.can_undo() {
            app.history.undo();
            app.toasts.info("Undo");
        }
        if cmd && shift && i.key_pressed(Key::Z) && app.history.can_redo() {
            app.history.redo();
            app.toasts.info("Redo");
        }

        // Cmd+N → New Composition
        if cmd && !shift && i.key_pressed(Key::N) {
            let count = app.history.current().compositions.len();
            let new_comp = crate::core::timeline::Composition::new(
                format!("comp_{}", count), "Composition 1".to_string(), 1920, 1080, 30, 300,
            );
            let proj = app.history.current_mut();
            proj.compositions.push(new_comp);
            proj.active_composition_idx = proj.compositions.len() - 1;
            crate::core::frame_cache::bump_version();
            app.toasts.info("New 1920x1080 @ 30fps");
        }

        // Cmd+S → Save Project (overwrite)
        if cmd && !shift && i.key_pressed(Key::S) {
            let path = app.project_path.clone();
            let proj = app.history.current();
            match crate::core::project_migration::save_project_atomic(proj, &path) {
                Ok(()) => {
                    app.toasts.info(format!("Saved: {}", path));
                }
                Err(e) => {
                    app.toasts.error(format!("Save failed: {}", e));
                }
            }
        }

        // Cmd+K → Composition Settings Dialog
        if cmd && !shift && i.key_pressed(Key::K) {
            app.show_comp_settings = true;
        }

        // Cmd+M → Render Queue / Export Dialog
        if cmd && !shift && i.key_pressed(Key::M) {
            app.show_export_dialog = true;
        }

        // ── Timeline markers: M adds/removes a marker at the playhead ──
        // (single-key M is free; Cmd+M opens export)
        if allow_single_key && !cmd && i.key_pressed(Key::M) {
            let frame = *current_frame;
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                if let Some(existing) = comp.markers.iter().position(|m| m.frame == frame) {
                    comp.markers.remove(existing);
                } else {
                    comp.markers.push(crate::core::timeline::TimelineMarker {
                        frame,
                        label: format!("M{}", comp.markers.len() + 1),
                        color: [0.2, 0.9, 0.5],
                    });
                }
            });
        }

        // ── Shift+; → Go to Next Marker, Cmd+; → Go to Previous Marker ──
        if i.key_pressed(Key::Semicolon) && shift && !cmd {
            let comp = app.history.current().active_composition();
            let cur = *current_frame;
            let next = comp.markers.iter()
                .map(|m| m.frame)
                .filter(|&f| f > cur)
                .min();
            if let Some(f) = next {
                *current_frame = f;
            } else if let Some(f) = comp.markers.iter().map(|m| m.frame).min() {
                *current_frame = f;
            }
        }
        if i.key_pressed(Key::Semicolon) && cmd {
            let comp = app.history.current().active_composition();
            let cur = *current_frame;
            let prev = comp.markers.iter()
                .map(|m| m.frame)
                .filter(|&f| f < cur)
                .max();
            if let Some(f) = prev {
                *current_frame = f;
            } else if let Some(f) = comp.markers.iter().map(|m| m.frame).max() {
                *current_frame = f;
            }
        }

        // ── Jump between markers: Shift+M cycles forward, Alt+M backward? Use [ ] keys ──
        if allow_single_key && (i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::CloseBracket)) {
            let forward = i.key_pressed(Key::CloseBracket);
            let project = app.history.current();
            let mut frames: Vec<u32> = project
                .active_composition()
                .markers
                .iter()
                .map(|m| m.frame)
                .collect();
            frames.sort_unstable();
            frames.dedup();
            if !frames.is_empty() {
                let target = if forward {
                    frames.iter().find(|&&f| f > *current_frame).copied()
                } else {
                    frames.iter().rev().find(|&&f| f < *current_frame).copied()
                };
                if let Some(f) = target {
                    *current_frame = f;
                } else {
                    // wrap around
                    *current_frame = if forward { frames[0] } else { frames[frames.len() - 1] };
                }
            }
        }

        // Cmd+Shift+C → Pre-Compose Selected Layers
        if cmd && shift && i.key_pressed(Key::C) {
            let mut temp_project = app.history.current().clone();
            let selected_indices: Vec<usize> = if !app.selected_layers.is_empty() {
                let mut s: Vec<usize> = app.selected_layers.iter().copied().collect();
                s.sort();
                s
            } else if let Some(idx) = app.selected_layer_idx {
                vec![idx]
            } else {
                vec![]
            };

            if !selected_indices.is_empty() {
                let comp_len = temp_project.compositions.len();
                let (width, height, fps, duration_frames) = {
                    let comp = temp_project.active_composition();
                    (comp.width, comp.height, comp.fps, comp.duration_frames)
                };

                let precomp_id = format!("precomp_{}", comp_len);
                let precomp_name = format!("Pre-comp {}", comp_len + 1);
                let mut new_comp = crate::core::timeline::Composition::new(
                    precomp_id.clone(),
                    precomp_name.clone(),
                    width,
                    height,
                    fps,
                    duration_frames,
                );

                let comp_mut = temp_project.active_composition_mut();
                let mut extracted_layers = Vec::new();
                for &idx in selected_indices.iter().rev() {
                    if idx < comp_mut.layers.len() {
                        extracted_layers.push(comp_mut.layers.remove(idx));
                    }
                }
                extracted_layers.reverse();
                new_comp.layers = extracted_layers;

                let precomp_layer = crate::core::timeline::Layer::new(
                    format!("layer_{}", precomp_id),
                    precomp_name,
                    crate::core::timeline::LayerType::PreComp { comp_id: precomp_id },
                    duration_frames,
                );
                let insert_pos = selected_indices.first().copied().unwrap_or(0).min(comp_mut.layers.len());
                comp_mut.layers.insert(insert_pos, precomp_layer);
                temp_project.compositions.push(new_comp);

                app.selected_layers.clear();
                app.selected_layers.insert(insert_pos);
                app.selected_layer_idx = Some(insert_pos);
                app.history.commit(temp_project);
                crate::core::frame_cache::bump_version();
            }
        }

        // Cmd+D → Duplicate selected layer, Cmd+Shift+D → Split layer at current frame
        if cmd && i.key_pressed(Key::D) {
            if let Some(idx) = app.selected_layer_idx {
                let mut proj = app.history.current().clone();
                let cf = *current_frame;
                let comp = proj.active_composition_mut();
                if idx < comp.layers.len() {
                    if !shift {
                        let mut dup = comp.layers[idx].clone();
                        dup.id = format!("{}_dup_{}", dup.id, comp.layers.len());
                        dup.name = format!("{} Copy", dup.name);
                        comp.layers.insert(idx + 1, dup);
                        app.selected_layer_idx = Some(idx + 1);
                    } else {
                        let mut split_b = comp.layers[idx].clone();
                        comp.layers[idx].out_frame = cf;
                        split_b.in_frame = cf;
                        split_b.id = format!("{}_split_{}", split_b.id, comp.layers.len());
                        split_b.name = format!("{} Split", split_b.name);
                        comp.layers.insert(idx + 1, split_b);
                        app.selected_layer_idx = Some(idx + 1);
                    }
                    app.history.commit(proj);
                    crate::core::frame_cache::bump_version();
                }
            }
        }

        // Cmd+[ → Send Backward, Cmd+] → Bring Forward (layer stacking order)
        if cmd && (i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::CloseBracket)) {
            if let Some(idx) = app.selected_layer_idx {
                let forward = i.key_pressed(Key::CloseBracket);
                let len = app.history.current().active_composition().layers.len();
                let target = if forward {
                    (idx + 1 < len).then_some(idx + 1)
                } else {
                    (idx > 0).then_some(idx - 1)
                };
                if let Some(new_idx) = target {
                    let mut proj = app.history.current().clone();
                    proj.active_composition_mut().layers.swap(idx, new_idx);
                    app.selected_layer_idx = Some(new_idx);
                    app.history.commit(proj);
                    crate::core::frame_cache::bump_version();
                }
            }
        }

        // Cmd+[ → Send Backward, Cmd+] → Bring Forward (layer stacking order)
        if cmd && (i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::CloseBracket)) {
            if let Some(idx) = app.selected_layer_idx {
                let forward = i.key_pressed(Key::CloseBracket);
                let len = app.history.current().active_composition().layers.len();
                let target = if forward {
                    (idx + 1 < len).then_some(idx + 1)
                } else {
                    (idx > 0).then_some(idx - 1)
                };
                if let Some(new_idx) = target {
                    let mut proj = app.history.current().clone();
                    proj.active_composition_mut().layers.swap(idx, new_idx);
                    app.selected_layer_idx = Some(new_idx);
                    app.history.commit(proj);
                    crate::core::frame_cache::bump_version();
                }
            }
        }

        // Delete / Backspace → Delete all selected layers (single-key)
        if allow_single_key && (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && !shift {
            let mut indices: Vec<usize> = app.selected_layers.iter().copied().collect();
            if indices.is_empty() {
                if let Some(s) = app.selected_layer_idx {
                    indices.push(s);
                }
            }
            if !indices.is_empty() {
                indices.sort_unstable_by(|a, b| b.cmp(a));
                app.modify_project(|project| {
                    let comp = project.active_composition_mut();
                    for idx in indices {
                        if idx < comp.layers.len() {
                            comp.layers.remove(idx);
                        }
                    }
                });
                app.selected_layers.clear();
                app.selected_layer_idx = None;
            }
        }

        // Property Selection Shortcuts (P, S, T, R) — single-key, suppressed while typing
        if allow_single_key && i.key_pressed(Key::P) && !cmd { app.selected_property = Some("Position X".to_string()); }
        if allow_single_key && i.key_pressed(Key::S) && !cmd { app.selected_property = Some("Scale X".to_string()); }
        if allow_single_key && i.key_pressed(Key::T) && !cmd { app.selected_property = Some("Opacity".to_string()); }
        if allow_single_key && i.key_pressed(Key::R) && !cmd { app.selected_property = Some("Rotation".to_string()); }

        // ── I / O: Jump to Layer In / Out Point ─────────────────────────────
        // I = jump CTI to selected layer's in_frame
        // O = jump CTI to selected layer's out_frame - 1
        if allow_single_key && !cmd && i.key_pressed(Key::I) {
            if let Some(idx) = app.selected_layer_idx {
                let comp = app.history.current().active_composition();
                if idx < comp.layers.len() {
                    *current_frame = comp.layers[idx].in_frame;
                    app.toasts.info(format!("Jumped to Layer In Point: frame {}", current_frame));
                }
            }
        }
        if allow_single_key && !cmd && i.key_pressed(Key::O) {
            if let Some(idx) = app.selected_layer_idx {
                let comp = app.history.current().active_composition();
                if idx < comp.layers.len() {
                    *current_frame = comp.layers[idx].out_frame.saturating_sub(1);
                    app.toasts.info(format!("Jumped to Layer Out Point: frame {}", current_frame));
                }
            }
        }

        // ── U / UU: Reveal Keyframed / All Modified Properties ──────────────
        // U        = Reveal all animated (keyframed) properties for selected layer
        // UU (x2)  = Reveal ALL modified (non-default) properties
        if allow_single_key && !cmd && i.key_pressed(Key::U) {
            let now = i.time;
            let is_double = app.u_key_last_press
                .map(|last| (now - last) < 0.4)
                .unwrap_or(false);
            if is_double {
                // UU → reveal all modified properties (expand layer + select "All Modified")
                if let Some(idx) = app.selected_layer_idx {
                    app.expanded_layers.insert(idx);
                }
                app.selected_property = Some("All Modified".to_string());
                app.toasts.info("UU: Reveal All Modified Properties");
                app.u_key_last_press = None;
            } else {
                // U → reveal keyframed properties
                if let Some(idx) = app.selected_layer_idx {
                    app.expanded_layers.insert(idx);
                }
                app.selected_property = Some("Keyframed".to_string());
                app.toasts.info("U: Reveal Keyframed Properties");
                app.u_key_last_press = Some(now);
            }
        }
        // ── A / AA: Reveal Anchor Point / Position ──────────────────────────
        // A = reveal Anchor Point property in timeline for selected layer
        if allow_single_key && !cmd && i.key_pressed(Key::A) {
            if let Some(idx) = app.selected_layer_idx {
                app.expanded_layers.insert(idx);
            }
            app.selected_property = Some("Anchor Point".to_string());
            app.toasts.info("A: Reveal Anchor Point");
        }

        // ── Shift+Home / Shift+End: Jump to Work Area In / Out ──────────────
        if allow_single_key && shift && i.key_pressed(Key::Home) {
            *current_frame = app.work_area_in.unwrap_or(0);
        }
        if allow_single_key && shift && i.key_pressed(Key::End) {
            let last = total_frames.saturating_sub(1);
            *current_frame = app.work_area_out.unwrap_or(last).min(last);
        }

        // ── Cmd+1~4: Composition Tab Switcher ───────────────────────────────
        if cmd {
            for (key, idx) in [
                (Key::Num1, 0usize), (Key::Num2, 1), (Key::Num3, 2), (Key::Num4, 3),
            ] {
                if i.key_pressed(key) {
                    let comp_count = app.history.current().compositions.len();
                    if idx < comp_count {
                        app.history.current_mut().active_composition_idx = idx;
                        crate::core::frame_cache::bump_version();
                        let name = app.history.current().compositions[idx].name.clone();
                        app.toasts.info(format!("Switched to Composition: {}", name));
                    }
                }
            }
        }

        // ── Tab / Shift+Tab → Cycle selected layer down / up (AE parity) ──
        if allow_single_key && i.key_pressed(Key::Tab) {
            let count = app.history.current().active_composition().layers.len();
            if count > 0 {
                let next = if shift {
                    app.selected_layer_idx.map_or(count - 1, |i| i.saturating_sub(1))
                } else {
                    app.selected_layer_idx.map_or(0, |i| (i + 1).min(count - 1))
                };
                app.selected_layer_idx = Some(next);
                app.selected_layers.clear();
                app.selected_layers.insert(next);
            }
        }

        // ── Numpad 0 → RAM Preview (force work-area pre-render + play) ──
        if allow_single_key && i.key_pressed(Key::Num0) && !app.is_playing {
            app.is_playing = true;
        }

        // ── Ctrl+Shift+K → Toggle Motion Sketch ──
        if cmd && shift && i.key_pressed(Key::K) {
            app.motion_sketch_active = !app.motion_sketch_active;
            if app.motion_sketch_active {
                app.is_playing = true;
                app.toasts.info("Motion Sketch ON — drag layer to record position");
            } else {
                app.toasts.info("Motion Sketch OFF");
            }
        }

        // Shift+F3 → Toggle Graph Editor / Tracks Mode
        if i.key_pressed(Key::F3) && shift {
            app.show_graph_editor = !app.show_graph_editor;
        }
    });
}

/// Moves a keyframe within an animatable track (used by batch keyframe nudging).
fn move_kf_in<T: Clone + crate::core::property::Interpolate>(
    anim: &mut crate::core::property::Animatable<T>,
    old_frame: u32,
    new_frame: u32,
) {
    if let Some(kfs) = anim.keyframes_mut() {
        if let Some(kf) = kfs.iter_mut().find(|k| k.frame == old_frame) {
            kf.frame = new_frame;
            kfs.sort_by_key(|k| k.frame);
        }
    }
}

/// Deletes the keyframe at `frame` from a track (keeps at least nothing —
/// emptying a track turns it back into a constant, which Animatable handles).
fn delete_kf_at<T: Clone>(anim: &mut crate::core::property::Animatable<T>, frame: u32) {
    if let Some(kfs) = anim.keyframes_mut() {
        kfs.retain(|k| k.frame != frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_shortcut_cross_platform() {
        let sc = format_shortcut("Z", true, true, false);
        assert!(sc.contains("Z"));
        assert!(sc.contains("Shift"));
    }
}
