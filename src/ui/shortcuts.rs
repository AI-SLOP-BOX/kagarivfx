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
        }

        // B → Set Work Area Start, N → Set Work Area End (single-key)
        if allow_single_key && i.key_pressed(Key::B) && !cmd {
            app.work_area_in = Some(*current_frame);
        }
        if allow_single_key && i.key_pressed(Key::N) && !cmd {
            app.work_area_out = Some(*current_frame);
        }

        // ── J / K / L: AE Shuttle Playback Controls ──────────────────────────
        // J = Reverse (press again to increase reverse speed up to 3x)
        // K = Stop playback
        // L = Forward (press again to increase forward speed up to 3x)
        if allow_single_key && !cmd && !shift {
            if i.key_pressed(Key::J) {
                if app.is_playing && app.playback_speed < 0 {
                    // Already playing reverse: increase reverse speed (max -3x)
                    app.playback_speed = (app.playback_speed - 1).max(-3);
                } else {
                    app.is_playing = true;
                    app.playback_speed = -1;
                }
                app.toasts.info(format!("⏪ Reverse {}x", app.playback_speed.abs()));
            }
            if i.key_pressed(Key::K) {
                // K = Stop / Pause
                app.is_playing = false;
                app.playback_speed = 1; // reset to default forward 1x
                app.toasts.info("⏸ Stopped");
            }
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
        // J → previous keyframe of selected layer, L → next keyframe, K → stop playback.
        // Collects keyframe times across all transform properties of the selected layer.
        if allow_single_key && i.key_pressed(Key::K) {
            app.is_playing = false;
        }
        if allow_single_key && (i.key_pressed(Key::J) || i.key_pressed(Key::L)) {
            let going_next = i.key_pressed(Key::L);
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

        // Cmd+Z → Undo, Cmd+Shift+Z → Redo
        if cmd && !shift && i.key_pressed(Key::Z) {
            app.history.undo();
        }
        if cmd && shift && i.key_pressed(Key::Z) {
            app.history.redo();
        }

        // Cmd+K → Composition Settings Dialog
        if cmd && !shift && i.key_pressed(Key::K) {
            app.show_comp_settings = true;
        }

        // Cmd+M → Render Queue / Export Dialog
        if cmd && !shift && i.key_pressed(Key::M) {
            app.show_export_dialog = true;
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
    });
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
