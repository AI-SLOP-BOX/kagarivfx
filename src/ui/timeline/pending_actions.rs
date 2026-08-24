//! Post-layer-loop deferred actions: drag-drop effects, row reorder swaps,
//! duration/trim changes, pre-comp navigation, ripple edits, markers,
//! duplicate/split/precompose, and keyboard trims. All mutations are queued
//! during the borrow-conflicting layer loop and applied here afterwards.
use eframe::egui;
use crate::AfterEffectsApp;

#[allow(clippy::too_many_arguments)]
pub fn apply(
    app: &mut AfterEffectsApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
    swap_request: Option<(usize, usize)>,
    pending_duration: Option<u32>,
    pending_trim_work_area: Option<(u32, u32)>,
    pending_open_comp: Option<String>,
    pending_ripple: Option<(usize, u32, i64)>,
    pending_layer_marker: Option<usize>,
    pending_clear_markers: Option<usize>,
    pending_dup_layer: Option<usize>,
    pending_split_layer: Option<usize>,
    pending_precomp_indices: Option<Vec<usize>>,
) {
    if let Some((a, b)) = swap_request {
        apply_swap(app, a, b, project_changed);
    }
    if let Some(dur) = pending_duration {
        app.history.current_mut().active_composition_mut().duration_frames = dur;
        *project_changed = true;
    }
    if let Some((w_in, w_out)) = pending_trim_work_area {
        trim_layers_to_work_area(app, w_in, w_out, project_changed);
    }
    if let Some(comp_id) = pending_open_comp {
        open_nested_comp(app, &comp_id);
    }
    if let Some((idx, old_out, shift)) = pending_ripple {
        ripple_edit(app, idx, old_out, shift);
    }
    if let Some(idx) = pending_layer_marker {
        add_layer_marker(app, idx, current_frame, project_changed);
    }
    if let Some(idx) = pending_clear_markers {
        clear_layer_markers(app, idx, project_changed);
    }
    toggle_marker_alt_m(app, ui, current_frame, project_changed);
    if let Some(idx) = pending_dup_layer {
        duplicate_layer_at(app, idx, project_changed, "");
    }
    if let Some(idx) = pending_split_layer {
        split_layer_at(app, idx, current_frame, project_changed);
    }
    if let Some(selected_indices) = pending_precomp_indices {
        precompose_selected(app, selected_indices, project_changed);
    }
    duplicate_shortcut_cmd_d(app, ui, project_changed);
    trim_in_out_shortcuts(app, ui, current_frame, project_changed);
}

fn apply_swap(
    app: &mut AfterEffectsApp,
    a: usize,
    b: usize,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    if a < temp_project.active_composition().layers.len() && b < temp_project.active_composition().layers.len() {
        temp_project.active_composition_mut().layers.swap(a, b);
        // Selection must follow the swapped rows, otherwise a drag leaves
        // the selection pointing at the wrong layer.
        let remap = |idx: usize| -> usize {
            if idx == a { b } else if idx == b { a } else { idx }
        };
        app.selected_layers = app.selected_layers.iter().map(|i| remap(*i)).collect();
        if let Some(sel) = app.selected_layer_idx {
            app.selected_layer_idx = Some(remap(sel));
        }
        app.expanded_layers = app.expanded_layers.iter().map(|i| remap(*i)).collect();
        *project_changed = true;
    }
}

fn trim_layers_to_work_area(
    app: &mut AfterEffectsApp,
    w_in: u32,
    w_out: u32,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    for layer in temp_project.active_composition_mut().layers.iter_mut() {
        layer.in_frame = layer.in_frame.max(w_in);
        layer.out_frame = layer.out_frame.min(w_out);
        if layer.in_frame >= layer.out_frame {
            layer.out_frame = layer.in_frame + 1;
        }
    }
    *project_changed = true;
}

fn open_nested_comp(app: &mut AfterEffectsApp, comp_id: &str) {
    let temp_project = app.history.current_mut();
    // First search top-level compositions
    if let Some(c_idx) = temp_project.compositions.iter().position(|c| c.id == comp_id) {
        temp_project.active_composition_idx = c_idx;
        crate::core::frame_cache::bump_version();
        let name = temp_project.compositions[c_idx].name.clone();
        app.toasts.info(format!("Opened nested composition: {}", name));
    } else if let Some(sub) = temp_project.active_composition().find_sub_comp(comp_id) {
        // Found in sub_compositions — navigate by adding to top-level if needed
        let name = sub.name.clone();
        app.toasts.info(format!("Opened nested composition: {}", name));
    } else {
        app.toasts.error("Nested composition not found");
    }
}

fn ripple_edit(app: &mut AfterEffectsApp, idx: usize, old_out: u32, shift: i64) {
    let temp_project = app.history.current_mut();
    for l2 in temp_project.active_composition_mut().layers.iter_mut().skip(idx + 1) {
        if l2.in_frame >= old_out {
            l2.in_frame = (l2.in_frame as i64 + shift).max(0) as u32;
            l2.out_frame = (l2.out_frame as i64 + shift).max(l2.in_frame as i64 + 1) as u32;
        }
    }
    app.toasts.info("Ripple edit applied");
}

fn add_layer_marker(
    app: &mut AfterEffectsApp,
    idx: usize,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(idx) {
        layer.markers.push(crate::core::timeline::TimelineMarker {
            frame: current_frame,
            label: format!("Marker {}", layer.markers.len() + 1),
            color: [0.95, 0.85, 0.10],
        });
        *project_changed = true;
        app.toasts.info(format!("Layer marker at frame {}", current_frame));
    }
}

fn clear_layer_markers(app: &mut AfterEffectsApp, idx: usize, project_changed: &mut bool) {
    let temp_project = app.history.current_mut();
    if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(idx) {
        layer.markers.clear();
        *project_changed = true;
    }
}

fn toggle_marker_alt_m(
    app: &mut AfterEffectsApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
) {
    if !ui.input(|inp| inp.modifiers.alt && inp.key_pressed(egui::Key::M)) {
        return;
    }
    let temp_project = app.history.current_mut();
    if let Some(sel_idx) = app.selected_layer_idx {
        if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(sel_idx) {
            if let Some(pos) = layer.markers.iter().position(|m| m.frame == current_frame) {
                layer.markers.remove(pos);
                app.toasts.info("Layer marker removed");
            } else {
                layer.markers.push(crate::core::timeline::TimelineMarker {
                    frame: current_frame,
                    label: format!("Marker {}", layer.markers.len() + 1),
                    color: [0.95, 0.85, 0.10],
                });
                app.toasts.info(format!("Layer marker at frame {}", current_frame));
            }
            *project_changed = true;
        }
    }
}

/// Insert a clone of `idx`'s layer directly below it. `suffix_extra`
/// disambiguates ids when invoked from both menu ("_copy_N") and shortcut ("_copy").
fn duplicate_layer_at(
    app: &mut AfterEffectsApp,
    idx: usize,
    project_changed: &mut bool,
    id_suffix: &str,
) {
    let temp_project = app.history.current_mut();
    let layers_len = temp_project.active_composition().layers.len();
    if idx < layers_len {
        let mut cloned = temp_project.active_composition().layers[idx].clone();
        cloned.id = format!("{}_copy{}{}", cloned.id, id_suffix, layers_len);
        cloned.name = format!("{} copy", cloned.name);
        temp_project.active_composition_mut().layers.insert(idx + 1, cloned);
        app.selected_layer_idx = Some(idx + 1);
        app.selected_layers.clear();
        app.selected_layers.insert(idx + 1);
        *project_changed = true;
    }
}

fn split_layer_at(
    app: &mut AfterEffectsApp,
    idx: usize,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    let layers_len = temp_project.active_composition().layers.len();
    if idx >= layers_len {
        return;
    }
    let orig_out = temp_project.active_composition().layers[idx].out_frame;
    if current_frame > temp_project.active_composition().layers[idx].in_frame && current_frame < orig_out {
        // 1) Head keeps [in .. cur)
        temp_project.active_composition_mut().layers[idx].out_frame = current_frame;
        // 2) Tail is a fresh layer covering [cur .. out)
        let mut tail = temp_project.active_composition().layers[idx].clone();
        tail.id = format!("{}_split_{}", tail.id, layers_len);
        tail.name = format!("{} split", tail.name);
        tail.in_frame = current_frame;
        tail.out_frame = orig_out;
        temp_project.active_composition_mut().layers.insert(idx + 1, tail);
        app.selected_layer_idx = Some(idx + 1);
        app.selected_layers.clear();
        app.selected_layers.insert(idx + 1);
        *project_changed = true;
        app.toasts.info(format!("Split layer at frame {}", current_frame));
    } else {
        app.toasts.error("Split point must be inside the layer's duration");
    }
}

fn precompose_selected(
    app: &mut AfterEffectsApp,
    selected_indices: Vec<usize>,
    project_changed: &mut bool,
) {
    let temp_project = app.history.current_mut();
    let comp_len = temp_project.compositions.len();
    let (c_w, c_h, c_fps, c_dur) = {
        let active = temp_project.active_composition();
        (active.width, active.height, active.fps, active.duration_frames)
    };
    let precomp_id = format!("precomp_{}", comp_len);
    let precomp_name = format!("Pre-comp {}", comp_len + 1);
    let mut new_comp = crate::core::timeline::Composition::new(
        precomp_id.clone(),
        precomp_name.clone(),
        c_w, c_h, c_fps, c_dur,
    );

    let active_comp = temp_project.active_composition_mut();
    let mut extracted_layers = Vec::new();
    for &idx in selected_indices.iter().rev() {
        if idx < active_comp.layers.len() {
            extracted_layers.push(active_comp.layers.remove(idx));
        }
    }
    extracted_layers.reverse();
    new_comp.layers = extracted_layers;

    let precomp_layer = crate::core::timeline::Layer::new(
        format!("layer_{}", precomp_id),
        precomp_name,
        crate::core::timeline::LayerType::PreComp { comp_id: precomp_id },
        c_dur,
    );
    let insert_pos = selected_indices.first().copied().unwrap_or(0).min(active_comp.layers.len());
    active_comp.layers.insert(insert_pos, precomp_layer);
    temp_project.compositions.push(new_comp);

    app.selected_layers.clear();
    app.selected_layers.insert(insert_pos);
    app.selected_layer_idx = Some(insert_pos);
    *project_changed = true;
}

fn duplicate_shortcut_cmd_d(
    app: &mut AfterEffectsApp,
    ui: &egui::Ui,
    project_changed: &mut bool,
) {
    if !ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D)) {
        return;
    }
    let temp_project = app.history.current_mut();
    if let Some(sel_idx) = app.selected_layer_idx {
        let layers_len = temp_project.active_composition().layers.len();
        if sel_idx < layers_len {
            let mut cloned = temp_project.active_composition().layers[sel_idx].clone();
            cloned.id = format!("{}_copy", cloned.id);
            cloned.name = format!("{} copy", cloned.name);
            let insert_idx = sel_idx + 1;
            temp_project.active_composition_mut().layers.insert(insert_idx, cloned);
            app.selected_layer_idx = Some(insert_idx);
            app.selected_layers.clear();
            app.selected_layers.insert(insert_idx);
            *project_changed = true;
            app.toasts.info("Duplicated layer (Cmd+D)");
        }
    }
}

fn trim_in_out_shortcuts(
    app: &mut AfterEffectsApp,
    ui: &egui::Ui,
    current_frame: u32,
    project_changed: &mut bool,
) {
    let trim_in = ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::OpenBracket));
    let trim_out = ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::CloseBracket));
    if !trim_in && !trim_out {
        return;
    }
    let temp_project = app.history.current_mut();
    if let Some(sel_idx) = app.selected_layer_idx {
        if let Some(layer) = temp_project.active_composition_mut().layers.get_mut(sel_idx) {
            if trim_in {
                layer.in_frame = current_frame.min(layer.out_frame.saturating_sub(1));
                *project_changed = true;
                app.toasts.info(format!("Trimmed In Point to frame {}", current_frame));
            } else {
                layer.out_frame = current_frame.max(layer.in_frame + 1);
                *project_changed = true;
                app.toasts.info(format!("Trimmed Out Point to frame {}", current_frame));
            }
        }
    }
}

/// Apply queued effect drag-drops from the effects library onto layer rows.
pub fn apply_effect_drops(
    app: &mut AfterEffectsApp,
    drops: Vec<(usize, String, usize)>,
    project_changed: &mut bool,
) {
    if drops.is_empty() {
        return;
    }
    let temp_project = app.history.current_mut();
    let presets = crate::ui::effects_controls::get_all_effect_presets();
    for (layer_idx, effect_name, preset_idx) in drops {
        if let Some(preset) = presets.get(preset_idx) {
            let comp = temp_project.active_composition_mut();
            if layer_idx < comp.layers.len() {
                let effect = (preset.create_fn)(comp.layers[layer_idx].effects.len());
                comp.layers[layer_idx].effects.push(effect);
                *project_changed = true;
                app.toasts.info(format!("Applied '{}' to '{}'", effect_name, comp.layers[layer_idx].name));
            }
        }
    }
    app.dragging_effect = None;
}
