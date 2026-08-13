use eframe::egui::{self, Key};
use crate::AfterEffectsApp;

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
    let no_text_focus = !crate::ui::focus::is_text_input_focused(ctx);
    if !no_text_focus {
        return;
    }

    ctx.input(|i| {
        let cmd = i.modifiers.command;
        let shift = i.modifiers.shift;

        // Space → Play / Pause RAM Preview
        if i.key_pressed(Key::Space) {
            app.is_playing = !app.is_playing;
        }

        // B → Set Work Area Start, N → Set Work Area End
        if i.key_pressed(Key::B) && !cmd {
            app.work_area_in = Some(*current_frame);
        }
        if i.key_pressed(Key::N) && !cmd {
            app.work_area_out = Some(*current_frame);
        }

        // J → Jump to previous keyframe, K → Jump to next keyframe (when Cmd is NOT pressed)
        if !cmd && !shift {
            if i.key_pressed(Key::J) {
                if let Some(idx) = app.selected_layer_idx {
                    let comp = app.history.current().active_composition();
                    if idx < comp.layers.len() {
                        let layer = &comp.layers[idx];
                        let mut all_frames: Vec<u32> = Vec::new();
                        for kf in layer.transform.position.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        for kf in layer.transform.scale.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        for kf in layer.transform.rotation.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        for kf in layer.transform.opacity.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        all_frames.sort_unstable();
                        if let Some(&prev_f) = all_frames.iter().rev().find(|&&f| f < *current_frame) {
                            *current_frame = prev_f;
                        }
                    }
                }
            }
            if i.key_pressed(Key::K) {
                if let Some(idx) = app.selected_layer_idx {
                    let comp = app.history.current().active_composition();
                    if idx < comp.layers.len() {
                        let layer = &comp.layers[idx];
                        let mut all_frames: Vec<u32> = Vec::new();
                        for kf in layer.transform.position.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        for kf in layer.transform.scale.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        for kf in layer.transform.rotation.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        for kf in layer.transform.opacity.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                        all_frames.sort_unstable();
                        if let Some(&next_f) = all_frames.iter().find(|&&f| f > *current_frame) {
                            *current_frame = next_f;
                        }
                    }
                }
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
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.position { for kf in kfs { kf.interpolation = ez.clone(); } }
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.scale { for kf in kfs { kf.interpolation = ez.clone(); } }
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.rotation { for kf in kfs { kf.interpolation = ez.clone(); } }
                        if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.opacity { for kf in kfs { kf.interpolation = ez.clone(); } }
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                    }
                }
            }
        }

        // Home → first frame, End → last frame
        if i.key_pressed(Key::Home) { *current_frame = 0; }
        if i.key_pressed(Key::End)  { *current_frame = total_frames.saturating_sub(1); }

        // Page Up / ← → frame step backward/forward
        if i.key_pressed(Key::PageUp) || i.key_pressed(Key::ArrowLeft) {
            *current_frame = current_frame.saturating_sub(1);
        }
        if i.key_pressed(Key::PageDown) || i.key_pressed(Key::ArrowRight) {
            *current_frame = (*current_frame + 1).min(total_frames.saturating_sub(1));
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

        // Delete / Backspace → Delete all selected layers
        if (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && !shift {
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

        // Property Selection Shortcuts (P, S, T, R)
        if i.key_pressed(Key::P) && !cmd { app.selected_property = Some("Position X".to_string()); }
        if i.key_pressed(Key::S) && !cmd { app.selected_property = Some("Scale X".to_string()); }
        if i.key_pressed(Key::T) && !cmd { app.selected_property = Some("Opacity".to_string()); }
        if i.key_pressed(Key::R) && !cmd { app.selected_property = Some("Rotation".to_string()); }
    });
}
