//! RAM Preview cache bar + comp markers + beat-detection transients.
use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::LayerType;
use crate::ui::theme::colors;

pub fn draw_ram_ruler(
    app: &mut AfterEffectsApp,
    ui: &mut egui::Ui,
    current_frame: &mut u32,
    total_frames: u32,
) {
    let bar_height = 14.0;
    let avail_w = ui.available_width();
    let (bar_rect, bar_response) =
        ui.allocate_exact_size(egui::vec2(avail_w, bar_height), egui::Sense::click());

    ui.painter().rect_filled(bar_rect, 2.0, colors::BG_DARKEST);

    if total_frames > 0 {
        let frame_w = bar_rect.width() / total_frames as f32;
        // Vertex-count safety: coalesce consecutive cached frames
        // into ONE rect per run instead of one painter call per frame
        // (a 10k-frame comp would otherwise emit 10k quads every frame).
        let mut run_start: Option<u32> = None;
        for f in 0..=total_frames {
            let cached = f < total_frames && app.frame_cache.is_cached(f);
            if cached && run_start.is_none() {
                run_start = Some(f);
            } else if !cached {
                if let Some(s) = run_start.take() {
                    let x = bar_rect.left() + s as f32 * frame_w;
                    let w = ((f - s) as f32 * frame_w).max(1.0);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(x, bar_rect.top()),
                            egui::vec2(w, bar_height),
                        ),
                        0.0,
                        colors::ACCENT_GREEN,
                    );
                }
            }
        }
    }

    // Render Timeline Markers & Beat Detection Transients
    let comp_mut = app.history.current_mut().active_composition_mut();

    // Real beat-detection transient lines from the comp's audio
    // sources (energy-flux onsets, cached per audio file).
    let audio_paths: Vec<String> = comp_mut.layers.iter().filter_map(|l| match &l.layer_type {
        LayerType::Audio { path, .. } => Some(path.clone()),
        LayerType::Video { audio_wav: Some(w), .. } => Some(w.clone()),
        _ => None,
    }).collect();
    let fps_now = comp_mut.fps.max(1) as f32;
    let mut beat_frames: Vec<u32> = Vec::new();
    for ap in &audio_paths {
        let key = egui::Id::new(("beat_frames", ap.as_str()));
        let frames: std::sync::Arc<Vec<u32>> = ui.ctx().data_mut(|d| {
            d.get_temp::<std::sync::Arc<Vec<u32>>>(key)
                .unwrap_or_else(|| {
                    let v = crate::core::audio_engine::detect_beat_frames(
                        std::path::Path::new(ap), total_frames, fps_now,
                    );
                    let arc = std::sync::Arc::new(v);
                    d.insert_temp(key, arc.clone());
                    arc
                })
        });
        beat_frames.extend(frames.iter().copied());
    }
    beat_frames.sort_unstable();
    beat_frames.dedup();
    // Cap drawn lines to keep painter calls bounded (~200)
    if beat_frames.len() > 200 {
        let keep = beat_frames.len() / 200;
        beat_frames = beat_frames.into_iter().step_by(keep).collect();
    }
    for bf in &beat_frames {
        let b_norm = *bf as f32 / total_frames.max(1) as f32;
        let bx = bar_rect.left() + b_norm * bar_rect.width();
        ui.painter().line_segment(
            [egui::pos2(bx, bar_rect.top()), egui::pos2(bx, bar_rect.bottom())],
            egui::Stroke::new(1.0, colors::ACCENT_YELLOW),
        );
    }

    for marker in &comp_mut.markers {
        if total_frames > 0 {
            let norm = marker.frame as f32 / total_frames as f32;
            let mx = bar_rect.left() + norm * bar_rect.width();
            let m_pts = vec![
                egui::pos2(mx - 4.0, bar_rect.top()),
                egui::pos2(mx + 4.0, bar_rect.top()),
                egui::pos2(mx, bar_rect.top() + 7.0),
            ];
            let mc = egui::Color32::from_rgb(
                (marker.color[0] * 255.0) as u8,
                (marker.color[1] * 255.0) as u8,
                (marker.color[2] * 255.0) as u8,
            );
            ui.painter().add(egui::Shape::convex_polygon(m_pts, mc, egui::Stroke::NONE));
        }
    }

    if bar_response.clicked() {
        if let Some(pos) = bar_response.interact_pointer_pos() {
            let norm = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
            *current_frame = (norm * total_frames as f32).round() as u32;
        }
    }
}
