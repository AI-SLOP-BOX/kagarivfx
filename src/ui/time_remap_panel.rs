use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;

fn rescale_track<T: Clone>(track: &mut Animatable<T>, in_frame: u32, factor: f32) {
    if let Animatable::Animated(kfs) = track {
        for kf in kfs.iter_mut() {
            let rel = kf.frame.saturating_sub(in_frame) as f32 * factor;
            kf.frame = in_frame.saturating_add(rel.round() as u32);
        }
        kfs.sort_by_key(|k| k.frame);
    }
}

pub fn draw_time_remap_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Time Stretch & Time Remapping");
    ui.separator();

    let layer_info = if let Some(idx) = app.selected_layer_idx {
        let comp = app.history.current().active_composition();
        if idx < comp.layers.len() {
            Some((idx, comp.layers[idx].name.clone(), comp.layers[idx].in_frame, comp.layers[idx].out_frame))
        } else { None }
    } else { None };

    if let Some((idx, layer_name, in_frame, out_frame)) = layer_info {
        ui.label(format!("Selected Layer: {}", layer_name));

        ui.add_space(4.0);
        if ui.button("⏱ Enable Time Remapping (Cmd+Alt+T)").on_hover_text("Adds Time Remap keyframe track for speed control").clicked() {
            let mut temp_proj = app.history.current().clone();
            let comp_mut = temp_proj.active_composition_mut();
            if idx < comp_mut.layers.len() {
                let dur = out_frame - in_frame;
                comp_mut.layers[idx].time_remap = Some(Animatable::Animated(vec![
                    Keyframe::new(in_frame, 0.0, InterpolationType::Linear),
                    Keyframe::new(out_frame, dur as f32, InterpolationType::Linear),
                ]));
                app.history.commit(temp_proj);
                crate::core::frame_cache::bump_version();
                app.toasts.info(format!("Enabled Time Remapping on {}", layer_name));
            }
        }

        // ── Loop: bake repeating time-remap keyframes over the layer duration ──
        ui.add_space(6.0);
        ui.label(egui::RichText::new("🔄 Auto Loop (bakes time-remap keys)").small().strong().color(colors::ACCENT_CYAN));
        let comp_dur = app.history.current().active_composition().duration_frames;
        ui.horizontal(|ui| {
            for (label, mode, tip) in [
                ("🔁 Loop Cycle", 0usize, "Repeat source forward continuously"),
                ("🏓 Loop PingPong", 1usize, "Alternate forward / reverse"),
            ] {
                if ui.button(label).on_hover_text(tip).clicked() {
                    let mut temp_proj = app.history.current().clone();
                    let comp_mut = temp_proj.active_composition_mut();
                    if idx < comp_mut.layers.len() {
                        let src_dur = (out_frame - in_frame).max(1) as f32;
                        let mut kfs: Vec<Keyframe<f32>> = Vec::new();
                        let mut t = in_frame as f32;
                        let mut cycle: u32 = 0;
                        while t < comp_dur as f32 {
                            match mode {
                                0 => {
                                    kfs.push(Keyframe::new(t as u32, 0.0, InterpolationType::Linear));
                                    kfs.push(Keyframe::new((t + src_dur - 1.0) as u32, src_dur - 1.0, InterpolationType::Linear));
                                }
                                _ => {
                                    kfs.push(Keyframe::new(t as u32, 0.0, InterpolationType::Linear));
                                    kfs.push(Keyframe::new((t + src_dur - 1.0) as u32, src_dur - 1.0, InterpolationType::Linear));
                                    cycle += 1;
                                    let back_start = t + src_dur;
                                    kfs.push(Keyframe::new(back_start as u32, src_dur - 1.0, InterpolationType::Linear));
                                    kfs.push(Keyframe::new((back_start + src_dur - 1.0) as u32, 0.0, InterpolationType::Linear));
                                    let _ = cycle;
                                }
                            }
                            t += src_dur * if mode == 0 { 1.0 } else { 2.0 };
                        }
                        comp_mut.layers[idx].time_remap = Some(Animatable::Animated(kfs));
                        app.history.commit(temp_proj);
                        crate::core::frame_cache::bump_version();
                        app.toasts.info(format!("Baked {} loop onto {}", if mode == 0 { "cycle" } else { "pingpong" }, layer_name));
                    }
                }
            }
        });

        ui.add_space(8.0);
        ui.separator();

        // ── Time Stretch: scales duration AND all keyframe times around the In point ──
        let stretch_id = egui::Id::new(format!("ae_time_stretch_{}", idx));
        let mut stretch_factor: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(stretch_id, || 100.0));
        ui.horizontal(|ui| {
            ui.label("Stretch Factor:");
            ui.add(egui::DragValue::new(&mut stretch_factor).range(1.0..=1000.0).suffix(" %").speed(1.0));
            if ui.button("Apply Stretch").on_hover_text("200% = half speed (duration doubles), 50% = double speed").clicked() {
                let factor = (stretch_factor / 100.0).max(0.01);
                let mut temp_proj = app.history.current().clone();
                let comp_mut = temp_proj.active_composition_mut();
                if idx < comp_mut.layers.len() {
                    let layer = &mut comp_mut.layers[idx];
                    let span = (layer.out_frame - layer.in_frame).max(1);
                    let new_span = ((span as f32 * factor).round() as u32).max(1);
                    layer.out_frame = layer.in_frame + new_span;

                    let in_f = layer.in_frame;
                    rescale_track(&mut layer.transform.position, in_f, factor);
                    rescale_track(&mut layer.transform.scale, in_f, factor);
                    rescale_track(&mut layer.transform.rotation, in_f, factor);
                    rescale_track(&mut layer.transform.opacity, in_f, factor);
                    rescale_track(&mut layer.transform.anchor_point, in_f, factor);
                    if let Some(remap) = layer.time_remap.as_mut() {
                        rescale_track(remap, in_f, factor);
                    }
                    app.history.commit(temp_proj);
                    crate::core::frame_cache::bump_version();
                    app.toasts.info(format!("Stretched {} to {:.0}% ({} → {} frames)", layer_name, stretch_factor, span, new_span));
                }
            }
        });
        ui.small(
            egui::RichText::new("Keyframe times on Position / Scale / Rotation / Opacity / Anchor / Time Remap are rescaled relative to the layer In point.")
                .color(colors::TEXT_MUTED),
        );

        ui.add_space(6.0);
        ui.label("Frame Blending Mode:");
        let blend_id = egui::Id::new("ae_frame_blending_mode");
        let mut blend_idx = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(blend_id, || 0));

        ui.horizontal(|ui| {
            if ui.selectable_value(&mut blend_idx, 0, "Off").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
            }
            if ui.selectable_value(&mut blend_idx, 1, "Frame Mix").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
            }
            if ui.selectable_value(&mut blend_idx, 2, "Pixel Motion").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(blend_id, blend_idx));
            }
        });
    } else {
        ui.weak("Select a layer to adjust time stretch & remapping.");
    }
}
