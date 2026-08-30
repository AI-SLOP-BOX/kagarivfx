#![allow(dead_code)]
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer};

/// Simple CPU-based Motion Tracking Engine inspired by SAD (Sum of Absolute Differences).
/// Refines and tracks TrackerPoints frame-by-frame.
pub struct TrackerEngine;

impl TrackerEngine {
    pub fn apply_pose_as_tracker_points(
        layer: &mut Layer,
        pose: &crate::core::optical_flow_timewarp::MarkerlessPoseTrack,
        minimum_confidence: f32,
    ) -> usize {
        if pose.frames.is_empty() { return 0; }
        let threshold = if minimum_confidence.is_finite() { minimum_confidence.clamp(0.0, 1.0) } else { 0.0 };
        let names = crate::core::optical_flow_timewarp::standard_humanoid_joint_names(
            pose.frames.first().map(|frame| frame.joints.len()).unwrap_or(0),
        );
        let mut active_ids = std::collections::HashSet::new();
        let mut created = 0;
        for (joint, name) in names.into_iter().enumerate() {
            let keyframes = pose.frames.iter().filter_map(|frame| {
                let point = *frame.joints.get(joint)?;
                (frame.confidence >= threshold && point.iter().all(|value| value.is_finite())).then(|| {
                    Keyframe::new(frame.frame, point, InterpolationType::Linear)
                })
            }).collect::<Vec<_>>();
            if keyframes.is_empty() { continue; }
            let initial = keyframes[0].value;
            let id = format!("pose_{}", name);
            active_ids.insert(id.clone());
            let position = if keyframes.len() == 1 { Animatable::Constant(initial) } else { Animatable::Animated(keyframes) };
            if let Some(tracker) = layer.trackers.iter_mut().find(|tracker| tracker.id == id) {
                tracker.position = position;
            } else {
                let mut tracker = crate::core::timeline::TrackerPoint::new(
                    id, format!("Pose {}", name.replace('_', " ")), initial,
                );
                tracker.position = position;
                layer.trackers.push(tracker);
            }
            created += 1;
        }
        if created > 0 {
            layer.trackers.retain(|tracker| !tracker.id.starts_with("pose_") || active_ids.contains(&tracker.id));
        }
        created
    }

    pub fn apply_pose3d_as_tracker_points(
        layer: &mut Layer,
        pose: &crate::core::optical_flow_timewarp::MarkerlessPose3DTrack,
        camera: crate::core::optical_flow_timewarp::PoseCameraModel,
        rotation_degrees: [f32; 3],
        minimum_confidence: f32,
    ) -> usize {
        if pose.frames.is_empty() { return 0; }
        let threshold = if minimum_confidence.is_finite() { minimum_confidence.clamp(0.0, 1.0) } else { 0.0 };
        let names = crate::core::optical_flow_timewarp::standard_humanoid_joint_names(
            pose.frames.iter().map(|frame| frame.joints.len()).max().unwrap_or(0),
        );
        let mut active_ids = std::collections::HashSet::new();
        let mut created = 0;
        for (joint, name) in names.into_iter().enumerate() {
            let keyframes = pose.frames.iter().filter_map(|frame| {
                let point = *frame.joints.get(joint)?;
                let position = crate::core::optical_flow_timewarp::project_pose3d_point_with_rotation(point, camera, rotation_degrees)?;
                (frame.confidence >= threshold).then(|| Keyframe::new(frame.frame, position, InterpolationType::Linear))
            }).collect::<Vec<_>>();
            if keyframes.is_empty() { continue; }
            let id = format!("pose3d_{}", name);
            active_ids.insert(id.clone());
            let initial = keyframes[0].value;
            let position = if keyframes.len() == 1 { Animatable::Constant(initial) } else { Animatable::Animated(keyframes) };
            if let Some(tracker) = layer.trackers.iter_mut().find(|tracker| tracker.id == id) {
                tracker.position = position;
            } else {
                let mut tracker = crate::core::timeline::TrackerPoint::new(id, format!("Pose 3D {}", name.replace('_', " ")), initial);
                tracker.position = position;
                layer.trackers.push(tracker);
            }
            created += 1;
        }
        if created > 0 {
            layer.trackers.retain(|tracker| !tracker.id.starts_with("pose3d_") || active_ids.contains(&tracker.id));
        }
        created
    }

    pub fn estimate_markerless_pose(
        layer: &Layer,
        start_frame: u32,
        end_frame: u32,
        max_features: usize,
        feature_spacing: u32,
        block_radius: i32,
        search_radius: i32,
    ) -> Option<crate::core::optical_flow_timewarp::MarkerlessPoseTrack> {
        if end_frame < start_frame { return None; }
        let mut frames = Vec::new();
        let mut width = 0u32;
        let mut height = 0u32;
        for frame in start_frame..=end_frame {
            let (current, _next, w, h) = Self::load_tracker_frames(layer, frame)?;
            width = w as u32;
            height = h as u32;
            frames.push(current);
        }
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        Some(crate::core::optical_flow_timewarp::estimate_markerless_pose(
            &refs, width, height, max_features, feature_spacing, block_radius, search_radius,
        ))
    }

    pub fn analyze_markerless_tracks(
        layer: &mut Layer,
        start_frame: u32,
        end_frame: u32,
        max_features: usize,
        feature_spacing: u32,
        block_radius: i32,
        search_radius: i32,
        minimum_confidence: f32,
    ) -> usize {
        if end_frame < start_frame {
            return 0;
        }
        if layer.trackers.is_empty() {
            let Some((first, _, width, height)) = Self::load_tracker_frames(layer, start_frame) else {
                return 0;
            };
            let seeds = crate::core::optical_flow_timewarp::detect_markerless_features(
                &first, width as u32, height as u32, max_features, feature_spacing,
            );
            for (index, position) in seeds.into_iter().enumerate() {
                layer.trackers.push(crate::core::timeline::TrackerPoint::new(
                    format!("mocap_{index}"), format!("Mocap Feature {}", index + 1), position,
                ));
            }
        }
        if layer.trackers.is_empty() { return 0; }
        let mut frames = Vec::new();
        let mut width = 0u32;
        let mut height = 0u32;
        for frame in start_frame..=end_frame {
            let Some((current, next, w, h)) = Self::load_tracker_frames(layer, frame) else {
                return 0;
            };
            width = w as u32;
            height = h as u32;
            frames.push(current);
            if frame == end_frame {
                frames.push(next);
            }
        }
        let seeds = layer
            .trackers
            .iter()
            .map(|tracker| tracker.position.evaluate(start_frame))
            .collect::<Vec<_>>();
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let tracks = crate::core::optical_flow_timewarp::track_markerless_motion(
            &refs, width, height, &seeds, block_radius, search_radius,
        );
        let mut total = 0;
        for (tracker, track) in layer.trackers.iter_mut().zip(tracks.iter()) {
            total += crate::core::optical_flow_timewarp::apply_markerless_track_to_tracker_point(
                tracker, track, minimum_confidence,
            );
        }
        total
    }

    pub fn analyze_markerless_track(
        layer: &mut Layer,
        tracker_idx: usize,
        start_frame: u32,
        end_frame: u32,
        block_radius: i32,
        search_radius: i32,
        minimum_confidence: f32,
    ) -> usize {
        if tracker_idx >= layer.trackers.len() || end_frame < start_frame {
            return 0;
        }
        let mut frames = Vec::new();
        let mut width = 0u32;
        let mut height = 0u32;
        for frame in start_frame..=end_frame {
            let Some((current, next, w, h)) = Self::load_tracker_frames(layer, frame) else {
                return 0;
            };
            width = w as u32;
            height = h as u32;
            frames.push(current);
            if frame == end_frame {
                frames.push(next);
            }
        }
        let seed = layer.trackers[tracker_idx].position.evaluate(start_frame);
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let tracks = crate::core::optical_flow_timewarp::track_markerless_motion(
            &refs,
            width,
            height,
            &[seed],
            block_radius,
            search_radius,
        );
        let Some(track) = tracks.into_iter().next() else {
            return 0;
        };
        crate::core::optical_flow_timewarp::apply_markerless_track_to_tracker_point(
            &mut layer.trackers[tracker_idx],
            &track,
            minimum_confidence,
        )
    }

    /// Track a feature forward from `current_frame` to `current_frame + 1`.
    /// Generates keyframe data for the tracker point.
    pub fn track_next_frame(
        layer: &Layer,
        fps: u32,
        tracker_idx: usize,
        current_frame: u32,
    ) -> Option<[f32; 2]> {
        // Real image-based tracking when pixel sources are available
        if let Some((curr_img, next_img, img_w, img_h)) =
            Self::load_tracker_frames(layer, current_frame)
        {
            return Self::track_next_frame_pixels(
                layer,
                tracker_idx,
                current_frame,
                &curr_img,
                &next_img,
                img_w,
                img_h,
            );
        }
        // Fallback: transform-velocity extrapolation (Null/Shape/etc.)
        Self::track_next_frame_synthetic(layer, fps, tracker_idx, current_frame)
    }

    /// Loads the RGBA buffers for `frame` and `frame + 1` from the layer's media.
    /// Video layers use their extracted sequence; image layers reuse the same file.
    fn load_tracker_frames(layer: &Layer, frame: u32) -> Option<(Vec<u8>, Vec<u8>, usize, usize)> {
        use crate::core::image_cache::with_image_cache;

        let (dir_a, dir_b): (Option<String>, Option<String>) = match &layer.layer_type {
            crate::core::timeline::LayerType::Video {
                frames_dir,
                frame_count,
                ..
            } => {
                let seq_a = frame.min(frame_count.saturating_sub(1));
                let seq_b = (frame + 1).min(frame_count.saturating_sub(1));
                (
                    Some(format!(
                        "{}/frame_{:05}.png",
                        frames_dir.trim_end_matches('/'),
                        seq_a
                    )),
                    Some(format!(
                        "{}/frame_{:05}.png",
                        frames_dir.trim_end_matches('/'),
                        seq_b
                    )),
                )
            }
            crate::core::timeline::LayerType::Image { path } => {
                (Some(path.clone()), Some(path.clone()))
            }
            _ => (None, None),
        };

        with_image_cache(|cache| {
            let a = cache.load_image(&dir_a?)?;
            let w = a.width as usize;
            let h = a.height as usize;
            let buf_a = a.pixels.clone();
            let b = cache.load_image(&dir_b?)?;
            if b.width as usize != w || b.height as usize != h {
                return None;
            }
            Some((buf_a, b.pixels.clone(), w, h))
        })
    }

    /// Real SAD block matching over decoded frames, with subpixel refinement.
    pub fn track_next_frame_pixels(
        layer: &Layer,
        tracker_idx: usize,
        current_frame: u32,
        curr_img: &[u8],
        next_img: &[u8],
        img_w: usize,
        img_h: usize,
    ) -> Option<[f32; 2]> {
        if tracker_idx >= layer.trackers.len() {
            return None;
        }
        let tracker = &layer.trackers[tracker_idx];
        let pos = tracker.position.evaluate(current_frame);

        let feature = (tracker.feature_size.max(8.0) as usize) & !1; // even
        let half = feature / 2;
        let cx = pos[0] as i32;
        let cy = pos[1] as i32;
        if (cx - half as i32) < 0 || (cy - half as i32) < 0 {
            return Some(pos);
        }

        // Reference patch: prefer the tracker's persistent template when one
        // of matching size is stored (appearance-locked tracking), otherwise
        // slice the current frame around the position as before.
        let template_len = feature * feature * 4;
        let ref_patch: Vec<u8> = match tracker
            .reference_pattern
            .as_deref()
            .filter(|t| t.len() == template_len)
        {
            // Stored patterns are normalised 0..1 floats — quantise to u8.
            Some(stored) => stored
                .iter()
                .map(|&f| (f.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect(),
            None => {
                let mut patch = vec![0u8; template_len];
                for py in 0..feature {
                    for px in 0..feature {
                        let sx = cx - half as i32 + px as i32;
                        let sy = cy - half as i32 + py as i32;
                        if sx < 0 || sy < 0 || sx >= img_w as i32 || sy >= img_h as i32 {
                            continue;
                        }
                        let src = ((sy * img_w as i32 + sx) * 4) as usize;
                        let dst = (py * feature + px) * 4;
                        patch[dst..dst + 4].copy_from_slice(&curr_img[src..src + 4]);
                    }
                }
                patch
            }
        };

        let search_radius = (tracker.search_size as i32).clamp(2, 64);
        let [bx, by] = compute_sad_match(
            &ref_patch,
            feature,
            feature,
            next_img,
            img_w,
            img_h,
            cx,
            cy,
            search_radius,
        );

        // Subpixel refinement along each axis
        let sad_at = |x: i32, y: i32| -> f32 {
            let mut shifted = [bx, by];
            shifted[0] = x as f32;
            shifted[1] = y as f32;
            let _ = shifted;
            // Recompute SAD at integer offsets around best
            let mut total = 0.0f32;
            for py in 0..feature {
                for px in 0..feature {
                    let tx = x - half as i32 + px as i32;
                    let ty = y - half as i32 + py as i32;
                    if tx < 0 || ty < 0 || tx >= img_w as i32 || ty >= img_h as i32 {
                        continue;
                    }
                    let ref_idx = (py * feature + px) * 4;
                    let tgt_idx = ((ty * img_w as i32 + tx) * 4) as usize;
                    if ref_idx + 3 < ref_patch.len() && tgt_idx + 3 < next_img.len() {
                        total += (ref_patch[ref_idx] as f32 - next_img[tgt_idx] as f32).abs()
                            + (ref_patch[ref_idx + 1] as f32 - next_img[tgt_idx + 1] as f32).abs();
                    }
                }
            }
            total
        };
        let [dx_sub, dy_sub] = subpixel_refine(&sad_at, bx as i32, by as i32);

        // New position = original anchor + (matched center - original center) + subpixel
        let moved_x = bx - cx as f32 + dx_sub;
        let moved_y = by - cy as f32 + dy_sub;
        Some([pos[0] + moved_x, pos[1] + moved_y])
    }

    /// Fallback tracking for layers without pixel data (velocity extrapolation).
    pub fn track_next_frame_synthetic(
        layer: &Layer,
        fps: u32,
        tracker_idx: usize,
        current_frame: u32,
    ) -> Option<[f32; 2]> {
        if tracker_idx >= layer.trackers.len() {
            return None;
        }
        let tracker = &layer.trackers[tracker_idx];
        let current_pos = tracker.position.evaluate(current_frame);
        let pos_t0 = layer.transform.eval_position(current_frame, fps);
        let pos_t1 = layer.transform.eval_position(current_frame + 1, fps);
        Some([
            current_pos[0] + (pos_t1[0] - pos_t0[0]),
            current_pos[1] + (pos_t1[1] - pos_t0[1]),
        ])
    }

    /// Run tracking analysis over a range of frames.
    pub fn analyze_track(
        comp: &mut Composition,
        layer_idx: usize,
        tracker_idx: usize,
        start_frame: u32,
        end_frame: u32,
    ) {
        Self::analyze_track_cancellable(comp, layer_idx, tracker_idx, start_frame, end_frame, None);
    }

    /// Run tracking analysis over a range of frames with optional cancellation flag.
    pub fn analyze_track_cancellable(
        comp: &mut Composition,
        layer_idx: usize,
        tracker_idx: usize,
        start_frame: u32,
        end_frame: u32,
        cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) {
        if layer_idx >= comp.layers.len() {
            return;
        }

        let fps = comp.fps;
        let mut current_positions = Vec::new();

        {
            let layer = &comp.layers[layer_idx];
            if tracker_idx >= layer.trackers.len() {
                return;
            }
            let tracker = &layer.trackers[tracker_idx];
            let start_pos = tracker.position.evaluate(start_frame);
            current_positions.push((start_frame, start_pos));

            for f in (start_frame + 1)..=end_frame {
                if let Some(ref flag) = cancel_flag {
                    if flag.load(std::sync::atomic::Ordering::Relaxed) {
                        log::info!("Motion tracking canceled via thread lifecycle guard");
                        return;
                    }
                }
                if let Some(next_pos) = Self::track_next_frame(layer, fps, tracker_idx, f - 1) {
                    current_positions.push((f, next_pos));
                }
            }
        }

        let tracker = &mut comp.layers[layer_idx].trackers[tracker_idx];
        let mut keyframes = Vec::new();

        for (f, pos) in current_positions {
            keyframes.push(Keyframe::new(f, pos, InterpolationType::Linear));
        }

        tracker.position = Animatable::Animated(keyframes);
    }

    /// Bake and apply motion tracker keyframe data to a target layer's position or rotation.
    #[allow(dead_code)]
    pub fn apply_tracker_to_target(
        comp: &mut Composition,
        source_layer_idx: usize,
        tracker_idx: usize,
        target_layer_idx: usize,
        apply_position: bool,
        apply_rotation: bool,
    ) {
        if source_layer_idx >= comp.layers.len() || target_layer_idx >= comp.layers.len() {
            return;
        }

        let tracker_kfs = match comp.layers[source_layer_idx].trackers.get(tracker_idx) {
            Some(t) => match &t.position {
                Animatable::Animated(kfs) => kfs.clone(),
                Animatable::Constant(pos) => {
                    vec![Keyframe::new(0, *pos, InterpolationType::Linear)]
                }
            },
            None => return,
        };

        if apply_position {
            comp.layers[target_layer_idx].transform.position =
                Animatable::Animated(tracker_kfs.clone());
        }

        if apply_rotation && tracker_kfs.len() > 1 {
            let mut rot_kfs = Vec::new();
            for i in 0..(tracker_kfs.len() - 1) {
                let p1 = tracker_kfs[i].value;
                let p2 = tracker_kfs[i + 1].value;
                let angle_rad = (p2[1] - p1[1]).atan2(p2[0] - p1[0]);
                let angle_deg = angle_rad.to_degrees();
                rot_kfs.push(Keyframe::new(
                    tracker_kfs[i].frame,
                    angle_deg,
                    InterpolationType::Linear,
                ));
            }
            comp.layers[target_layer_idx].transform.rotation = Animatable::Animated(rot_kfs);
        }
    }

    /// Apply motion tracking data as reverse stabilization (camera shake compensation) to anchor point / position.
    #[allow(dead_code)]
    pub fn apply_stabilize_to_layer(
        comp: &mut Composition,
        layer_idx: usize,
        tracker_idx: usize,
        stabilize_position: bool,
        stabilize_rotation: bool,
    ) {
        if layer_idx >= comp.layers.len() {
            return;
        }

        let tracker_kfs = match comp.layers[layer_idx].trackers.get(tracker_idx) {
            Some(t) => match &t.position {
                Animatable::Animated(kfs) => kfs.clone(),
                Animatable::Constant(pos) => {
                    vec![Keyframe::new(0, *pos, InterpolationType::Linear)]
                }
            },
            None => return,
        };

        if tracker_kfs.is_empty() {
            return;
        }

        let base_pos = tracker_kfs[0].value;
        let init_pos = comp.layers[layer_idx].transform.position.evaluate(0);

        if stabilize_position {
            let mut stab_pos_kfs = Vec::new();
            for kf in &tracker_kfs {
                // Invert drift: pos(t) = init_pos - (track(t) - base_pos)
                let dx = kf.value[0] - base_pos[0];
                let dy = kf.value[1] - base_pos[1];
                stab_pos_kfs.push(Keyframe::new(
                    kf.frame,
                    [init_pos[0] - dx, init_pos[1] - dy],
                    InterpolationType::Linear,
                ));
            }
            comp.layers[layer_idx].transform.position = Animatable::Animated(stab_pos_kfs);
        }

        if stabilize_rotation && tracker_kfs.len() > 1 {
            let mut stab_rot_kfs = Vec::new();
            let base_angle = (tracker_kfs[1].value[1] - tracker_kfs[0].value[1])
                .atan2(tracker_kfs[1].value[0] - tracker_kfs[0].value[0])
                .to_degrees();
            let init_rot = comp.layers[layer_idx].transform.rotation.evaluate(0);

            for i in 0..(tracker_kfs.len() - 1) {
                let p1 = tracker_kfs[i].value;
                let p2 = tracker_kfs[i + 1].value;
                let cur_angle = (p2[1] - p1[1]).atan2(p2[0] - p1[0]).to_degrees();
                let delta_angle = cur_angle - base_angle;
                stab_rot_kfs.push(Keyframe::new(
                    tracker_kfs[i].frame,
                    init_rot - delta_angle,
                    InterpolationType::Linear,
                ));
            }
            comp.layers[layer_idx].transform.rotation = Animatable::Animated(stab_rot_kfs);
        }
    }
}

/// Compute real Sum of Absolute Differences (SAD) template matching between reference RGBA buffer and target RGBA buffer.
#[allow(dead_code, clippy::too_many_arguments)]
pub fn compute_sad_match(
    ref_patch: &[u8],
    patch_w: usize,
    patch_h: usize,
    target_img: &[u8],
    img_w: usize,
    img_h: usize,
    search_center_x: i32,
    search_center_y: i32,
    search_radius: i32,
) -> [f32; 2] {
    let mut min_sad = f32::MAX;
    let mut best_x = search_center_x as f32;
    let mut best_y = search_center_y as f32;

    let half_pw = (patch_w / 2) as i32;
    let half_ph = (patch_h / 2) as i32;

    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let cx = search_center_x + dx;
            let cy = search_center_y + dy;

            if cx - half_pw < 0
                || cx + half_pw >= img_w as i32
                || cy - half_ph < 0
                || cy + half_ph >= img_h as i32
            {
                continue;
            }

            let mut sad = 0.0f32;
            for py in 0..patch_h {
                for px in 0..patch_w {
                    let rx = px;
                    let ry = py;
                    let ref_idx = (ry * patch_w + rx) * 4;

                    let tx = (cx - half_pw) as usize + px;
                    let ty = (cy - half_ph) as usize + py;
                    let tgt_idx = (ty * img_w + tx) * 4;

                    if ref_idx + 3 < ref_patch.len() && tgt_idx + 3 < target_img.len() {
                        let diff_r = (ref_patch[ref_idx] as f32 - target_img[tgt_idx] as f32).abs();
                        let diff_g =
                            (ref_patch[ref_idx + 1] as f32 - target_img[tgt_idx + 1] as f32).abs();
                        let diff_b =
                            (ref_patch[ref_idx + 2] as f32 - target_img[tgt_idx + 2] as f32).abs();
                        sad += diff_r + diff_g + diff_b;
                    }
                }
            }

            // Tie-break toward the search center: flat regions (uniform areas)
            // have many equal-SAD offsets, and without this the first-scanned
            // corner always won, causing spurious drift.
            let is_better = sad < min_sad
                || ((sad - min_sad).abs() <= 1e-3
                    && (dx.abs() + dy.abs())
                        < (best_x as i32 - search_center_x).abs()
                            + (best_y as i32 - search_center_y).abs());
            if is_better {
                min_sad = sad;
                best_x = cx as f32;
                best_y = cy as f32;
            }
        }
    }

    [best_x, best_y]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType, TrackerPoint};

    #[test]
    fn test_tracker_apply_to_target() {
        let mut comp = Composition::new(
            "comp_1".to_string(),
            "TestComp".to_string(),
            1920,
            1080,
            30,
            100,
        );
        let mut src_layer = Layer::new(
            "src".to_string(),
            "Source".to_string(),
            LayerType::Null,
            100,
        );
        let mut tp = TrackerPoint::new("tp_1".to_string(), "Point1".to_string(), [100.0, 100.0]);
        tp.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [200.0, 150.0], InterpolationType::Linear),
        ]);
        src_layer.trackers.push(tp);
        comp.layers.push(src_layer);

        let target_layer = Layer::new(
            "target".to_string(),
            "Target".to_string(),
            LayerType::Null,
            100,
        );
        comp.layers.push(target_layer);

        TrackerEngine::apply_tracker_to_target(&mut comp, 0, 0, 1, true, true);
        assert_eq!(
            comp.layers[1].transform.position.evaluate(0),
            [100.0, 100.0]
        );
        assert_eq!(
            comp.layers[1].transform.position.evaluate(10),
            [200.0, 150.0]
        );
    }

    #[test]
    fn test_sad_template_matching() {
        // Create 10x10 target image with a white 2x2 dot at (5, 5)
        let mut target_img = vec![0u8; 10 * 10 * 4];
        for y in 5..=6 {
            for x in 5..=6 {
                let idx = (y * 10 + x) * 4;
                target_img[idx] = 255;
                target_img[idx + 1] = 255;
                target_img[idx + 2] = 255;
                target_img[idx + 3] = 255;
            }
        }

        // Reference patch 2x2 white dot
        let ref_patch = vec![255u8; 2 * 2 * 4];

        let matched = compute_sad_match(&ref_patch, 2, 2, &target_img, 10, 10, 3, 3, 4);
        assert_eq!(
            matched,
            [6.0, 6.0],
            "SAD template matching should find the feature at (6, 6)"
        );
    }
}

/// Subpixel refinement: parabola fit through the SAD cost at (x-1, x, x+1)
/// along each axis. Returns the refined offset in [-0.5, 0.5] per axis.
/// Ported concept from NextVFX's dense optical flow post-processing.
pub fn subpixel_refine(sad_at: &dyn Fn(i32, i32) -> f32, best_x: i32, best_y: i32) -> [f32; 2] {
    let fit = |minus: f32, center: f32, plus: f32| -> f32 {
        let denom = (minus - 2.0 * center + plus).abs();
        if denom < 1e-9 {
            return 0.0;
        }
        // Vertex of the parabola through the three samples
        (0.5 * (minus - plus) / denom).clamp(-0.5, 0.5)
    };
    let dx = fit(
        sad_at(best_x - 1, best_y),
        sad_at(best_x, best_y),
        sad_at(best_x + 1, best_y),
    );
    let dy = fit(
        sad_at(best_x, best_y - 1),
        sad_at(best_x, best_y),
        sad_at(best_x, best_y + 1),
    );
    [dx, dy]
}

/// Match confidence from the two lowest SAD values: ratio near 0 means a
/// distinctive peak (good), near 1 means a flat valley (ambiguous match).
pub fn match_confidence(min_sad: f32, second_min_sad: f32) -> f32 {
    if second_min_sad <= 1e-9 {
        if min_sad <= 1e-9 {
            return 1.0;
        } else {
            return 0.0;
        }
    }
    (min_sad / second_min_sad).clamp(0.0, 1.0)
}

/// Extracts a `feature`×`feature` RGBA patch centred at integer (cx,cy) —
/// used to seed a tracker's persistent reference pattern
/// ([`crate::core::timeline::TrackerPoint::reference_pattern`]).
/// Out-of-bounds centres yield None.
pub fn extract_patch(
    frame: &[u8],
    img_w: u32,
    img_h: u32,
    cx: i32,
    cy: i32,
    feature: usize,
) -> Option<Vec<u8>> {
    let half = (feature / 2) as i32;
    if feature == 0 || half > cx || half > cy {
        return None;
    }
    let max_x = cx + (feature as i32 - 1 - half);
    let max_y = cy + (feature as i32 - 1 - half);
    if max_x >= img_w as i32
        || max_y >= img_h as i32
        || frame.len() < img_w as usize * img_h as usize * 4
    {
        return None;
    }
    let mut patch = vec![0u8; feature * feature * 4];
    for py in 0..feature {
        for px in 0..feature {
            let sx = cx - half + px as i32;
            let sy = cy - half + py as i32;
            let src = ((sy * img_w as i32 + sx) * 4) as usize;
            let dst = (py * feature + px) * 4;
            patch[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
        }
    }
    Some(patch)
}

/// Blends the current appearance into a persistent template by `blend`
/// (0 = frozen, 1 = fully adopt current). Mismatched lengths are ignored.
/// Slow blends (e.g. 0.05/frame) track gradual appearance drift while
/// resisting one-frame occlusions.
pub fn blend_template(template: &mut [u8], current: &[u8], blend: f32) {
    let b = blend.clamp(0.0, 1.0);
    if b <= 0.0 || template.len() != current.len() {
        return;
    }
    for (t, &c) in template.iter_mut().zip(current.iter()) {
        *t = (*t as f32 * (1.0 - b) + c as f32 * b)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
}

// ── Four-point perspective tracking (AE Perspective Corner Pin workflow) ──

/// Per-frame positions of the four tracked corner features.
///
/// Corner order matches AE: [top_left, top_right, bottom_right, bottom_left].
/// Positions after the first analysed frame are produced by single-step
/// tracking (the same convention as [`TrackerEngine::analyze_track`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuadTrackData {
    pub frames: Vec<u32>,
    pub corners: Vec<[[f32; 2]; 4]>,
}

/// Solves the 3×3 planar homography H mapping `from` → `to` via the Direct
/// Linear Transform with h33 normalised to 1. Returns None when the point
/// configuration is degenerate (duplicates or collinear corners).
pub fn compute_homography(from: [[f32; 2]; 4], to: [[f32; 2]; 4]) -> Option<[[f64; 3]; 3]> {
    let mut a = [[0.0f64; 8]; 8];
    let mut b = [0.0f64; 8];
    for i in 0..4 {
        let [x, y] = [from[i][0] as f64, from[i][1] as f64];
        let [u, v] = [to[i][0] as f64, to[i][1] as f64];
        let r1 = i * 2;
        let r2 = r1 + 1;
        a[r1] = [-x, -y, -1.0, 0.0, 0.0, 0.0, u * x, u * y];
        b[r1] = -u;
        a[r2] = [0.0, 0.0, 0.0, -x, -y, -1.0, v * x, v * y];
        b[r2] = -v;
    }

    // Gaussian elimination with partial pivoting.
    for col in 0..8 {
        let pivot_row = (col..8)
            .max_by(|&l, &r| {
                a[l][col]
                    .abs()
                    .partial_cmp(&a[r][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);
        if a[pivot_row][col].abs() < 1e-10 {
            return None;
        }
        a.swap(col, pivot_row);
        b.swap(col, pivot_row);
        for row in (col + 1)..8 {
            let factor = a[row][col] / a[col][col];
            let pivot = a[col]; // owned copy: avoids aliasing borrows below
            for (ac, bc) in a[row][col..].iter_mut().zip(pivot[col..].iter()) {
                *ac -= factor * bc;
            }
            b[row] -= factor * b[col];
        }
    }

    // Back substitution.
    let mut h = [0.0f64; 8];
    for row in (0..8).rev() {
        let sum: f64 = a[row][row + 1..]
            .iter()
            .zip(h[row + 1..].iter())
            .map(|(&av, &hv)| av * hv)
            .sum();
        h[row] = (b[row] - sum) / a[row][row];
    }

    let result = [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]];

    // Residual validation: the linear system can be solvable even when no
    // true homography exists (e.g. two source corners mapped to one target).
    // Reject any solution that does not reproduce the correspondences.
    for (f, t) in from.iter().zip(to.iter()) {
        let m = apply_homography(&result, *f);
        if !m[0].is_finite()
            || !m[1].is_finite()
            || (m[0] - t[0]).abs() > 1e-2
            || (m[1] - t[1]).abs() > 1e-2
        {
            return None;
        }
    }

    Some(result)
}

/// Applies a 3×3 homography to a 2D point. Points mapping through w≈0
/// (infinity) return f32::MAX components.
pub fn apply_homography(h: &[[f64; 3]; 3], p: [f32; 2]) -> [f32; 2] {
    let [x, y] = [p[0] as f64, p[1] as f64];
    if !x.is_finite() || !y.is_finite() || h.iter().flatten().any(|value| !value.is_finite()) {
        return [f32::MAX, f32::MAX];
    }
    let w = h[2][0] * x + h[2][1] * y + h[2][2];
    if !w.is_finite() || w.abs() < 1e-12 {
        return [f32::MAX, f32::MAX];
    }
    let mapped_x = (h[0][0] * x + h[0][1] * y + h[0][2]) / w;
    let mapped_y = (h[1][0] * x + h[1][1] * y + h[1][2]) / w;
    if !mapped_x.is_finite() || !mapped_y.is_finite() {
        return [f32::MAX, f32::MAX];
    }
    [
        mapped_x.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32,
        mapped_y.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32,
    ]
}

impl TrackerEngine {
    /// Tracks four corner features over a frame range for perspective
    /// corner-pin workflows. Every referenced tracker index must exist on the
    /// layer, otherwise the result is empty.
    pub fn analyze_quad_track(
        comp: &Composition,
        layer_idx: usize,
        tracker_indices: [usize; 4],
        start_frame: u32,
        end_frame: u32,
    ) -> QuadTrackData {
        let mut out = QuadTrackData::default();
        let Some(layer) = comp.layers.get(layer_idx) else {
            return out;
        };
        if tracker_indices.iter().any(|&i| i >= layer.trackers.len()) {
            return out;
        }
        let fps = comp.fps;
        for f in start_frame..=end_frame {
            let mut frame_corners = [[0.0f32; 2]; 4];
            for (slot, &ti) in tracker_indices.iter().enumerate() {
                frame_corners[slot] = Self::track_next_frame(layer, fps, ti, f.saturating_sub(1))
                    .unwrap_or_else(|| layer.trackers[ti].position.evaluate(f));
            }
            out.frames.push(f);
            out.corners.push(frame_corners);
        }
        out
    }
}

/// Computes per-frame homographies mapping `source_rect` onto each tracked
/// quad. Degenerate frames yield None entries.
pub fn quad_homographies(
    track: &QuadTrackData,
    source_rect: [[f32; 2]; 4],
) -> Vec<Option<[[f64; 3]; 3]>> {
    track
        .corners
        .iter()
        .map(|quad| compute_homography(source_rect, *quad))
        .collect()
}

/// Removes single-frame position spikes from a tracked quad. A corner sample
/// that jumps more than `max_jump` pixels from BOTH neighbours is replaced by
/// their midpoint. Motion-free tracks pass through unchanged.
pub fn smooth_quad_track(track: &QuadTrackData, max_jump: f32) -> QuadTrackData {
    let n = track.frames.len();
    if n < 3 || max_jump <= 0.0 {
        return track.clone();
    }
    let mut out = track.clone();
    for corner in 0..4 {
        for i in 1..n - 1 {
            let prev = track.corners[i - 1][corner];
            let cur = track.corners[i][corner];
            let next = track.corners[i + 1][corner];
            let d_prev = (cur[0] - prev[0]).hypot(cur[1] - prev[1]);
            let d_next = (next[0] - cur[0]).hypot(next[1] - cur[1]);
            if d_prev > max_jump && d_next > max_jump {
                out.corners[i][corner] = [(prev[0] + next[0]) * 0.5, (prev[1] + next[1]) * 0.5];
            }
        }
    }
    out
}

/// Smooths 2D tracker keyframes using a Gaussian / weighted moving average window.
pub fn smooth_tracker_keyframes(
    kfs: &[crate::core::keyframe::Keyframe<[f32; 2]>],
    window_radius: usize,
) -> Vec<crate::core::keyframe::Keyframe<[f32; 2]>> {
    if kfs.len() <= 2 || window_radius == 0 {
        return kfs.to_vec();
    }

    let n = kfs.len();
    let mut smoothed = Vec::with_capacity(n);

    for i in 0..n {
        let min_idx = i.saturating_sub(window_radius);
        let max_idx = (i + window_radius).min(n - 1);

        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut weight_sum = 0.0f32;

        for j in min_idx..=max_idx {
            let dist = (i as isize - j as isize).abs() as f32;
            let sigma = (window_radius as f32 * 0.5).max(1.0);
            let weight = (-dist * dist / (2.0 * sigma * sigma)).exp();

            sum_x += kfs[j].value[0] * weight;
            sum_y += kfs[j].value[1] * weight;
            weight_sum += weight;
        }

        let avg_x = if weight_sum > 0.0 {
            sum_x / weight_sum
        } else {
            kfs[i].value[0]
        };
        let avg_y = if weight_sum > 0.0 {
            sum_y / weight_sum
        } else {
            kfs[i].value[1]
        };

        smoothed.push(crate::core::keyframe::Keyframe::new(
            kfs[i].frame,
            [avg_x, avg_y],
            kfs[i].interpolation,
        ));
    }

    smoothed
}

/// Per-frame tracking quality in 0..1: how well the tracked quad preserves the
/// source rectangle's area and convexity. Flipped or collapsed quads (failed
/// SAD matches) score near zero.
pub fn quad_track_confidence(track: &QuadTrackData, source_rect: [[f32; 2]; 4]) -> Vec<f32> {
    fn signed_area(quad: &[[f32; 2]; 4]) -> f32 {
        let mut sum = 0.0;
        for i in 0..4 {
            let a = quad[i];
            let b = quad[(i + 1) % 4];
            sum += a[0] * b[1] - b[0] * a[1];
        }
        sum * 0.5
    }

    let src_area = signed_area(&source_rect).abs().max(1e-6);
    track
        .corners
        .iter()
        .map(|quad| {
            let area = signed_area(quad);
            if area <= 0.0 {
                return 0.0; // flipped / self-intersecting → lost lock
            }
            let ratio = area / src_area;
            let area_score = if ratio >= 1.0 { 1.0 / ratio } else { ratio };
            let e01 = (quad[1][0] - quad[0][0]).hypot(quad[1][1] - quad[0][1]);
            let e32 = (quad[2][0] - quad[3][0]).hypot(quad[2][1] - quad[3][1]);
            let e12 = (quad[2][0] - quad[1][0]).hypot(quad[2][1] - quad[1][1]);
            let e03 = (quad[3][0] - quad[0][0]).hypot(quad[3][1] - quad[0][1]);
            let r1 = if e01.max(e32) > 1e-6 {
                e01.min(e32) / e01.max(e32)
            } else {
                0.0
            };
            let r2 = if e12.max(e03) > 1e-6 {
                e12.min(e03) / e12.max(e03)
            } else {
                0.0
            };
            let shape_score = (r1 + r2) * 0.5;
            (area_score * 0.7 + shape_score * 0.3).clamp(0.0, 1.0)
        })
        .collect()
}

/// Post-analysis cleanup: frames whose lock confidence falls below
/// `conf_threshold` are replaced by their spike-smoothed counterparts while
/// healthy frames stay bit-identical. Combines [`quad_track_confidence`] and
/// [`smooth_quad_track`] into the standard refinement pass.
pub fn refine_quad_track(
    track: &QuadTrackData,
    source_rect: [[f32; 2]; 4],
    conf_threshold: f32,
    max_jump: f32,
) -> QuadTrackData {
    let smoothed = smooth_quad_track(track, max_jump);
    let conf = quad_track_confidence(track, source_rect);
    let mut out = track.clone();
    for i in 0..out.frames.len() {
        if conf.get(i).copied().unwrap_or(1.0) < conf_threshold {
            if let Some(s) = smoothed.corners.get(i) {
                out.corners[i] = *s;
            }
        }
    }
    out
}

#[cfg(test)]
mod subpixel_tests {
    use super::*;

    #[test]
    fn test_subpixel_refine_finds_true_minimum() {
        // Synthetic SAD valley centered at x = 10.4, y = 7.6
        let true_x = 10.4f32;
        let true_y = 7.6f32;
        let sad = |x: i32, y: i32| -> f32 {
            let fx = x as f32 - true_x;
            let fy = y as f32 - true_y;
            fx * fx + fy * fy
        };
        // Integer search finds (10, 8)
        let [dx, dy] = subpixel_refine(&sad, 10, 8);
        let refined_x = 10.0 + dx;
        let refined_y = 8.0 + dy;
        assert!(
            (refined_x - true_x).abs() < 0.15,
            "x {} vs {}",
            refined_x,
            true_x
        );
        assert!(
            (refined_y - true_y).abs() < 0.15,
            "y {} vs {}",
            refined_y,
            true_y
        );
    }

    #[test]
    fn test_confidence_distinguishes_sharp_vs_flat() {
        // Sharp peak: best is far below second-best → low ratio (high confidence)
        let sharp = match_confidence(10.0, 100.0);
        // Flat valley: nearly equal → high ratio (ambiguous)
        let flat = match_confidence(100.0, 101.0);
        assert!(sharp < 0.2 && flat > 0.99);
    }
}

#[cfg(test)]
mod pixel_tracking_tests {
    use super::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Layer, LayerType, TrackerPoint};

    /// Builds a 64x64 gray frame with a bright square at `sq_x`.
    fn frame_with_square(sq_x: usize) -> Vec<u8> {
        let w = 64usize;
        let h = 64usize;
        let mut px = vec![30u8; w * h * 4];
        for y in 20..40 {
            for x in sq_x..sq_x + 12 {
                let idx = (y * w + x) * 4;
                px[idx] = 240;
                px[idx + 1] = 240;
                px[idx + 2] = 240;
                px[idx + 3] = 255;
            }
        }
        px
    }

    #[test]
    fn test_real_pixel_tracking_follows_moving_feature() {
        // Square at x=10 in frame A, x=16 in frame B → tracker must move ~+6
        let curr = frame_with_square(10);
        let next = frame_with_square(16);
        let (w, h) = (64usize, 64usize);

        let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
        layer.trackers.push(TrackerPoint {
            id: "t1".into(),
            name: "Track".into(),
            // Straddle the square's LEFT EDGE (square spans x 10..22): gives the
            // patch a unique texture so the SAD minimum is well-defined.
            position: Animatable::new_constant([10.0, 32.0]),
            search_size: 24.0,
            feature_size: 12.0,
            reference_pattern: None,
        });

        let result = TrackerEngine::track_next_frame_pixels(&layer, 0, 0, &curr, &next, w, h);
        assert!(result.is_some());
        let new_pos = result.unwrap();
        // The tracked edge pattern moved +6 px; tracker follows onto the new
        // edge position.
        assert!(
            (new_pos[0] - 16.0).abs() < 1.5,
            "expected x≈16 (edge followed +6), got {}",
            new_pos[0]
        );
        // Y should stay put
        assert!(
            (new_pos[1] - 32.0).abs() < 1.5,
            "expected y≈32, got {}",
            new_pos[1]
        );
    }

    #[test]
    fn test_tracking_stationary_scene_stays_put() {
        let f = frame_with_square(20);
        let mut layer = Layer::new("l".into(), "L".into(), LayerType::Null, 30);
        layer.trackers.push(TrackerPoint {
            id: "t1".into(),
            name: "T".into(),
            position: Animatable::new_constant([20.0, 32.0]), // on the edge
            search_size: 20.0,
            feature_size: 12.0,
            reference_pattern: None,
        });
        let r =
            TrackerEngine::track_next_frame_pixels(&layer, 0, 0, &f, &f.clone(), 64, 64).unwrap();
        assert!(
            (r[0] - 20.0).abs() < 0.6 && (r[1] - 32.0).abs() < 0.6,
            "{:?}",
            r
        );
    }
}

#[cfg(test)]
mod quad_track_tests {
    use super::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition as QComp, Layer as QLayer, LayerType as QLayerType};

    const RECT: [[f32; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

    #[test]
    fn test_identity_homography_is_identity() {
        let h = compute_homography(RECT, RECT).expect("identity solvable");
        for (r, row) in h.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((val - expected).abs() < 1e-9, "H[{r}][{c}]={val}");
            }
        }
    }

    #[test]
    fn test_translation_homography_maps_interior_point() {
        let to: [[f32; 2]; 4] = RECT.map(|p| [p[0] + 25.0, p[1] - 15.0]);
        let h = compute_homography(RECT, to).expect("translation solvable");
        let m = apply_homography(&h, [60.0, 60.0]);
        assert!(
            (m[0] - 85.0).abs() < 1e-3 && (m[1] - 45.0).abs() < 1e-3,
            "{m:?}"
        );
    }

    #[test]
    fn test_homography_rejects_nonfinite_inputs_without_propagation() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(apply_homography(&identity, [f32::NAN, 1.0]), [f32::MAX; 2]);
        assert_eq!(
            apply_homography(&identity, [1.0, f32::INFINITY]),
            [f32::MAX; 2]
        );

        let invalid = [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(apply_homography(&invalid, [1.0, 1.0]), [f32::MAX; 2]);

        let overflow = [[f64::MAX, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mapped = apply_homography(&overflow, [2.0, 1.0]);
        assert!(mapped.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn test_perspective_maps_corners_and_rejects_degenerate() {
        let dst = [[12.0f32, 5.0], [70.0, 0.0], [84.0, 62.0], [4.0, 58.0]];
        let h = compute_homography(RECT, dst).expect("perspective solvable");
        for (s, d) in RECT.iter().zip(dst.iter()) {
            let m = apply_homography(&h, *s);
            assert!(
                (m[0] - d[0]).abs() < 1e-3 && (m[1] - d[1]).abs() < 1e-3,
                "{m:?} vs {d:?}"
            );
        }
        let bad = [[0.0, 0.0], [0.0, 0.0], [84.0, 62.0], [4.0, 58.0]];
        assert!(
            compute_homography(RECT, bad).is_none(),
            "duplicates degenerate"
        );
    }

    #[test]
    fn test_analyze_quad_tracks_all_four_corners() {
        let mut comp = QComp::new("c".into(), "Q".into(), 320, 240, 30, 30);
        let mut layer = QLayer::new("l".into(), "L".into(), QLayerType::Null, 30);
        let base: [[f32; 2]; 4] = [[40.0, 30.0], [280.0, 30.0], [280.0, 210.0], [40.0, 210.0]];
        for (i, b) in base.iter().enumerate() {
            let kfs = (0..=10u32)
                .map(|f| {
                    Keyframe::new(
                        f,
                        [b[0] + f as f32, b[1] - f as f32],
                        InterpolationType::Linear,
                    )
                })
                .collect();
            let mut tp =
                crate::core::timeline::TrackerPoint::new(format!("t{i}"), format!("T{i}"), *b);
            tp.position = Animatable::Animated(kfs);
            layer.trackers.push(tp);
        }
        comp.layers.push(layer);

        let track = TrackerEngine::analyze_quad_track(&comp, 0, [0, 1, 2, 3], 0, 10);
        assert_eq!(track.frames.len(), 11);
        // Synthetic single-step convention: frame 5 carries frame-4 state (+4, −4).
        let q5 = track.corners[5];
        for (b, t) in base.iter().zip(q5.iter()) {
            assert!((t[0] - (b[0] + 4.0)).abs() < 1e-4, "{t:?} vs {b:?}");
            assert!((t[1] - (b[1] - 4.0)).abs() < 1e-4);
        }
        let hs = quad_homographies(&track, base);
        assert_eq!(hs.len(), 11);
        let centre = apply_homography(hs[5].as_ref().expect("frame 5 H"), [160.0, 120.0]);
        assert!(
            centre[0] > 160.0 && centre[1] < 120.0,
            "centre follows motion: {centre:?}"
        );

        // Bad indices → empty.
        assert!(
            TrackerEngine::analyze_quad_track(&comp, 0, [0, 1, 2, 9], 0, 5)
                .frames
                .is_empty()
        );
        assert!(TrackerEngine::analyze_quad_track(&comp, 7, [0; 4], 0, 5)
            .frames
            .is_empty());
    }

    #[test]
    fn test_smooth_quad_track_removes_spikes_only() {
        // Uniform diagonal motion with one 40px spike at frame 3.
        let mut track = QuadTrackData::default();
        for f in 0..8u32 {
            let off = f as f32 * 2.0 + if f == 3 { 40.0 } else { 0.0 };
            track.frames.push(f);
            track.corners.push([[10.0 + off, 20.0 - off]; 4]);
        }
        let smoothed = smooth_quad_track(&track, 10.0);
        // Spike frame replaced by neighbour midpoint.
        let c = smoothed.corners[3][0];
        assert!((c[0] - 16.0).abs() < 1e-4, "spike x {c:?}");
        // Clean frames untouched.
        assert_eq!(smoothed.corners[2][0], track.corners[2][0]);
        // Short/degenerate inputs pass through unchanged.
        assert_eq!(
            smooth_quad_track(&QuadTrackData::default(), 10.0),
            QuadTrackData::default()
        );
    }

    #[test]
    fn test_confidence_scores_lock_quality() {
        // Rigid translated rectangle keeps near-perfect lock.
        let mut good = QuadTrackData::default();
        for f in 0..5u32 {
            good.frames.push(f);
            let o = f as f32;
            good.corners.push([
                [10.0 + o, 10.0],
                [110.0 + o, 10.0],
                [110.0 + o, 110.0],
                [10.0 + o, 110.0],
            ]);
        }
        let conf = quad_track_confidence(&good, RECT);
        assert!(conf.iter().all(|&c| c > 0.95), "rigid motion: {conf:?}");

        // Flipped quad (corner crossing) → zero confidence.
        let mut bad = QuadTrackData::default();
        bad.frames.push(0);
        bad.corners
            .push([[0.0, 0.0], [100.0, 0.0], [0.0, 100.0], [100.0, 100.0]]);
        assert!(
            quad_track_confidence(&bad, RECT).iter().all(|&c| c == 0.0),
            "flipped quad must score zero"
        );
    }

    #[test]
    fn test_extract_patch_exact_pixels_and_oob() {
        let mut frame = vec![7u8; 16 * 16 * 4];
        // Stamp a red pixel at (5,5).
        let idx = ((5 * 16 + 5) * 4) as usize;
        frame[idx] = 250;
        frame[idx + 1] = 10;
        frame[idx + 2] = 10;
        let patch = extract_patch(&frame, 16, 16, 5, 5, 3).expect("in-bounds");
        assert_eq!(patch.len(), 3 * 3 * 4);
        // Centre of the 3×3 patch is the red pixel.
        assert_eq!(&patch[16..20], &[250, 10, 10, 7]);
        // Corners remain background.
        assert_eq!(patch[0], 7);
        // Fully out of bounds rejected.
        assert!(extract_patch(&frame, 16, 16, 15, 15, 3).is_none());
        assert!(extract_patch(&frame, 16, 16, -1, 5, 3).is_none());
    }

    #[test]
    fn test_blend_template_semantics() {
        let mut tmpl = vec![100u8, 200];
        // Frozen.
        blend_template(&mut tmpl, &[0, 0], 0.0);
        assert_eq!(tmpl, vec![100, 200]);
        // Half-way toward current.
        blend_template(&mut tmpl, &[0, 0], 0.5);
        assert_eq!(tmpl, vec![50, 100]);
        // Full adopt.
        blend_template(&mut tmpl, &[90, 90], 1.0);
        assert_eq!(tmpl, vec![90, 90]);
        // Length mismatch ignored.
        let mut keep = vec![1u8];
        blend_template(&mut keep, &[9, 9, 9], 1.0);
        assert_eq!(keep, vec![1]);
    }

    #[test]
    fn test_persistent_template_follows_appearance_not_local_pixels() {
        // Gray field; an 8×8 checker sits centred at (40,32). The tracker is
        // seeded ON FLAT GRAY but carries the checker as its persistent
        // template — it must slide to the checker, not stay put.
        let w = 64usize;
        let h = 64usize;
        let mut frame = vec![30u8; w * h * 4];
        for dy in 0..8usize {
            for dx in 0..8usize {
                let v = if (dx + dy) % 2 == 0 { 240 } else { 15 };
                let idx = ((28 + dy) * w + 36 + dx) * 4;
                frame[idx] = v;
                frame[idx + 1] = v;
                frame[idx + 2] = v;
                frame[idx + 3] = 255;
            }
        }

        let template: Vec<f32> = extract_patch(&frame, 64, 64, 40, 32, 8)
            .expect("template in bounds")
            .iter()
            .map(|&b| b as f32 / 255.0)
            .collect();
        let mut layer = QLayer::new("l".into(), "L".into(), QLayerType::Null, 30);
        layer.trackers.push(crate::core::timeline::TrackerPoint {
            id: "t".into(),
            name: "T".into(),
            position: Animatable::new_constant([20.0, 32.0]),
            search_size: 30.0,
            feature_size: 8.0,
            reference_pattern: Some(template),
        });

        let result =
            TrackerEngine::track_next_frame_pixels(&layer, 0, 0, &frame, &frame.clone(), 64, 64);
        let new_pos = result.expect("match found");
        assert!(
            (new_pos[0] - 40.0).abs() < 2.0 && (new_pos[1] - 32.0).abs() < 2.0,
            "tracker must lock onto the templated checker: {new_pos:?}"
        );
    }

    #[test]
    fn test_refine_replaces_only_low_confidence_frames() {
        // Rigid translation with frame 3 collapsed to a degenerate point.
        let mut track = QuadTrackData::default();
        for f in 0..6u32 {
            let o = f as f32 * 5.0;
            let quad = if f == 3 {
                [[50.0, 50.0]; 4]
            } else {
                [
                    [10.0 + o, 10.0],
                    [60.0 + o, 10.0],
                    [60.0 + o, 60.0],
                    [10.0 + o, 60.0],
                ]
            };
            track.frames.push(f);
            track.corners.push(quad);
        }
        let refined = refine_quad_track(&track, RECT, 0.5, 25.0);

        // Healthy frames bit-identical.
        assert_eq!(refined.corners[0], track.corners[0]);
        assert_eq!(refined.corners[4], track.corners[4]);
        // Degenerate frame pulled toward the smoothed midpoint: neighbour
        // corner-0 positions are (20,10)/(30,10) → midpoint (25,10).
        assert_ne!(refined.corners[3], track.corners[3]);
        let c = refined.corners[3][0];
        assert!(
            (c[0] - 25.0).abs() < 1e-4 && (c[1] - 10.0).abs() < 1e-4,
            "midpoint {c:?}"
        );
    }

    #[test]
    fn test_tracker_apply_stabilize() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut layer = Layer::new(
            "l".into(),
            "Shaky Layer".into(),
            crate::core::timeline::LayerType::Null,
            30,
        );
        layer.transform.position = Animatable::new_constant([50.0, 50.0]);

        // Tracker detects camera drifted by +10px X at frame 15
        let track_kfs = vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(15, [110.0, 100.0], InterpolationType::Linear),
        ];
        layer.trackers.push(crate::core::timeline::TrackerPoint {
            id: "t1".into(),
            name: "Feature".into(),
            position: Animatable::Animated(track_kfs),
            search_size: 20.0,
            feature_size: 10.0,
            reference_pattern: None,
        });
        comp.add_layer(layer);

        TrackerEngine::apply_stabilize_to_layer(&mut comp, 0, 0, true, false);

        // Frame 0: position should be 50.0
        let p0 = comp.layers[0].transform.position.evaluate(0);
        assert_eq!(p0, [50.0, 50.0]);

        // Frame 15: position must counteract the +10px drift by moving to 40.0
        let p15 = comp.layers[0].transform.position.evaluate(15);
        assert_eq!(p15, [40.0, 50.0]);
    }

    #[test]
    fn test_smooth_tracker_keyframes_reduces_noise() {
        let kfs = vec![
            Keyframe::new(0, [10.0, 10.0], InterpolationType::Linear),
            Keyframe::new(1, [100.0, 10.0], InterpolationType::Linear), // Noise spike
            Keyframe::new(2, [10.0, 10.0], InterpolationType::Linear),
        ];
        let smoothed = smooth_tracker_keyframes(&kfs, 1);
        assert_eq!(smoothed.len(), 3);
        // Spike at frame 1 should be damped significantly by its neighbours
        assert!(smoothed[1].value[0] < 100.0);
    }

    #[test]
    fn pose_application_is_idempotent_for_repeated_analysis() {
        let mut layer = Layer::new(
            "l".into(), "Pose Layer".into(), crate::core::timeline::LayerType::Null, 10,
        );
        let pose = crate::core::optical_flow_timewarp::MarkerlessPoseTrack {
            frames: vec![
                crate::core::optical_flow_timewarp::MarkerlessPoseFrame { frame: 0, joints: vec![[1.0, 2.0]], root: [1.0, 2.0], confidence: 1.0 },
                crate::core::optical_flow_timewarp::MarkerlessPoseFrame { frame: 1, joints: vec![[2.0, 2.0]], root: [2.0, 2.0], confidence: 1.0 },
            ], bones: vec![], bone_lengths: vec![],
        };
        assert_eq!(TrackerEngine::apply_pose_as_tracker_points(&mut layer, &pose, 0.5), 1);
        assert_eq!(TrackerEngine::apply_pose_as_tracker_points(&mut layer, &pose, 0.5), 1);
        assert_eq!(layer.trackers.len(), 1);
        assert_eq!(layer.trackers[0].position.evaluate(1), [2.0, 2.0]);
    }

    #[test]
    fn pose_application_removes_stale_pose_points_but_keeps_manual_points() {
        let mut layer = Layer::new(
            "l".into(), "Pose Layer".into(), crate::core::timeline::LayerType::Null, 10,
        );
        layer.trackers.push(crate::core::timeline::TrackerPoint::new("manual".into(), "Manual".into(), [9.0, 9.0]));
        layer.trackers.push(crate::core::timeline::TrackerPoint::new("pose_old_joint".into(), "Old".into(), [8.0, 8.0]));
        let pose = crate::core::optical_flow_timewarp::MarkerlessPoseTrack {
            frames: vec![crate::core::optical_flow_timewarp::MarkerlessPoseFrame {
                frame: 0, joints: vec![[1.0, 2.0]], root: [1.0, 2.0], confidence: 1.0,
            }], bones: vec![], bone_lengths: vec![],
        };
        TrackerEngine::apply_pose_as_tracker_points(&mut layer, &pose, 0.0);
        assert!(layer.trackers.iter().any(|tracker| tracker.id == "manual"));
        assert!(!layer.trackers.iter().any(|tracker| tracker.id == "pose_old_joint"));
    }

    #[test]
    fn failed_pose_estimation_does_not_delete_existing_pose_points() {
        let mut layer = Layer::new(
            "l".into(), "Pose Layer".into(), crate::core::timeline::LayerType::Null, 10,
        );
        layer.trackers.push(crate::core::timeline::TrackerPoint::new("pose_head".into(), "Pose head".into(), [1.0, 2.0]));
        let empty = crate::core::optical_flow_timewarp::MarkerlessPoseTrack { frames: vec![], bones: vec![], bone_lengths: vec![] };
        assert_eq!(TrackerEngine::apply_pose_as_tracker_points(&mut layer, &empty, 0.5), 0);
        assert_eq!(layer.trackers.len(), 1);
        assert_eq!(layer.trackers[0].id, "pose_head");
    }

    #[test]
    fn all_invalid_pose_frames_do_not_delete_existing_pose_points() {
        let mut layer = Layer::new(
            "l".into(), "Pose Layer".into(), crate::core::timeline::LayerType::Null, 10,
        );
        layer.trackers.push(crate::core::timeline::TrackerPoint::new("pose_head".into(), "Pose head".into(), [1.0, 2.0]));
        let invalid = crate::core::optical_flow_timewarp::MarkerlessPoseTrack {
            frames: vec![crate::core::optical_flow_timewarp::MarkerlessPoseFrame {
                frame: 0, joints: vec![[f32::NAN, f32::NAN]], root: [0.0, 0.0], confidence: 0.0,
            }], bones: vec![], bone_lengths: vec![],
        };
        assert_eq!(TrackerEngine::apply_pose_as_tracker_points(&mut layer, &invalid, 0.5), 0);
        assert_eq!(layer.trackers.len(), 1);
    }

    #[test]
    fn projected_3d_pose_becomes_editable_tracker_keyframes() {
        let mut layer = Layer::new(
            "l".into(), "3D Pose Layer".into(), crate::core::timeline::LayerType::Null, 10,
        );
        let pose = crate::core::optical_flow_timewarp::MarkerlessPose3DTrack {
            frames: vec![
                crate::core::optical_flow_timewarp::MarkerlessPose3DFrame { frame: 2, joints: vec![[0.0, 0.0, 10.0]], confidence: 1.0 },
                crate::core::optical_flow_timewarp::MarkerlessPose3DFrame { frame: 3, joints: vec![[1.0, 0.0, 10.0]], confidence: 1.0 },
            ], bones: vec![],
        };
        let camera = crate::core::optical_flow_timewarp::PoseCameraModel { focal_length: 100.0, principal_point: [50.0, 40.0], position: [0.0, 0.0, 0.0] };
        assert_eq!(TrackerEngine::apply_pose3d_as_tracker_points(&mut layer, &pose, camera, [0.0; 3], 0.5), 1);
        assert_eq!(layer.trackers[0].position.evaluate(3), [60.0, 40.0]);
    }
}
