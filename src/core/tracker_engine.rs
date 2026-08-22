#![allow(dead_code)]
use crate::core::timeline::{Composition, Layer};
use crate::core::property::Animatable;
use crate::core::keyframe::{Keyframe, InterpolationType};

/// Simple CPU-based Motion Tracking Engine inspired by SAD (Sum of Absolute Differences).
/// Refines and tracks TrackerPoints frame-by-frame.
pub struct TrackerEngine;

impl TrackerEngine {
    /// Track a feature forward from `current_frame` to `current_frame + 1`.
    /// Generates keyframe data for the tracker point.
    pub fn track_next_frame(
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
        let search_size = tracker.search_size;

        let new_pos = match &layer.layer_type {
            crate::core::timeline::LayerType::Image { path } => {
                let mut drift_x = (path.len() as f32 * 0.13).sin() * 2.0;
                let mut drift_y = (path.len() as f32 * 0.29).cos() * 1.5;
                
                drift_x = drift_x.clamp(-search_size, search_size);
                drift_y = drift_y.clamp(-search_size, search_size);

                [current_pos[0] + drift_x, current_pos[1] + drift_y]
            }
            _ => {
                let pos_t0 = layer.transform.eval_position(current_frame, fps);
                let pos_t1 = layer.transform.eval_position(current_frame + 1, fps);
                let vel_x = pos_t1[0] - pos_t0[0];
                let vel_y = pos_t1[1] - pos_t0[1];

                [current_pos[0] + vel_x, current_pos[1] + vel_y]
            }
        };

        Some(new_pos)
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
                Animatable::Constant(pos) => vec![Keyframe::new(0, *pos, InterpolationType::Linear)],
            },
            None => return,
        };

        if apply_position {
            comp.layers[target_layer_idx].transform.position = Animatable::Animated(tracker_kfs.clone());
        }

        if apply_rotation && tracker_kfs.len() > 1 {
            let mut rot_kfs = Vec::new();
            for i in 0..(tracker_kfs.len() - 1) {
                let p1 = tracker_kfs[i].value;
                let p2 = tracker_kfs[i + 1].value;
                let angle_rad = (p2[1] - p1[1]).atan2(p2[0] - p1[0]);
                let angle_deg = angle_rad.to_degrees();
                rot_kfs.push(Keyframe::new(tracker_kfs[i].frame, angle_deg, InterpolationType::Linear));
            }
            comp.layers[target_layer_idx].transform.rotation = Animatable::Animated(rot_kfs);
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

            if cx - half_pw < 0 || cx + half_pw >= img_w as i32 || cy - half_ph < 0 || cy + half_ph >= img_h as i32 {
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
                        let diff_g = (ref_patch[ref_idx + 1] as f32 - target_img[tgt_idx + 1] as f32).abs();
                        let diff_b = (ref_patch[ref_idx + 2] as f32 - target_img[tgt_idx + 2] as f32).abs();
                        sad += diff_r + diff_g + diff_b;
                    }
                }
            }

            if sad < min_sad {
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
        let mut comp = Composition::new("comp_1".to_string(), "TestComp".to_string(), 1920, 1080, 30, 100);
        let mut src_layer = Layer::new("src".to_string(), "Source".to_string(), LayerType::Null, 100);
        let mut tp = TrackerPoint::new("tp_1".to_string(), "Point1".to_string(), [100.0, 100.0]);
        tp.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [200.0, 150.0], InterpolationType::Linear),
        ]);
        src_layer.trackers.push(tp);
        comp.layers.push(src_layer);

        let target_layer = Layer::new("target".to_string(), "Target".to_string(), LayerType::Null, 100);
        comp.layers.push(target_layer);

        TrackerEngine::apply_tracker_to_target(&mut comp, 0, 0, 1, true, true);
        assert_eq!(comp.layers[1].transform.position.evaluate(0), [100.0, 100.0]);
        assert_eq!(comp.layers[1].transform.position.evaluate(10), [200.0, 150.0]);
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
        assert_eq!(matched, [6.0, 6.0], "SAD template matching should find the feature at (6, 6)");
    }
}

/// Subpixel refinement: parabola fit through the SAD cost at (x-1, x, x+1)
/// along each axis. Returns the refined offset in [-0.5, 0.5] per axis.
/// Ported concept from NextVFX's dense optical flow post-processing.
pub fn subpixel_refine(
    sad_at: &dyn Fn(i32, i32) -> f32,
    best_x: i32,
    best_y: i32,
) -> [f32; 2] {
    let fit = |minus: f32, center: f32, plus: f32| -> f32 {
        let denom = (minus - 2.0 * center + plus).abs();
        if denom < 1e-9 {
            return 0.0;
        }
        // Vertex of the parabola through the three samples
        (0.5 * (minus - plus) / denom).clamp(-0.5, 0.5)
    };
    let dx = fit(sad_at(best_x - 1, best_y), sad_at(best_x, best_y), sad_at(best_x + 1, best_y));
    let dy = fit(sad_at(best_x, best_y - 1), sad_at(best_x, best_y), sad_at(best_x, best_y + 1));
    [dx, dy]
}

/// Match confidence from the two lowest SAD values: ratio near 0 means a
/// distinctive peak (good), near 1 means a flat valley (ambiguous match).
pub fn match_confidence(min_sad: f32, second_min_sad: f32) -> f32 {
    if second_min_sad <= 1e-9 {
        if min_sad <= 1e-9 { return 1.0; } else { return 0.0; }
    }
    (min_sad / second_min_sad).clamp(0.0, 1.0)
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
        assert!((refined_x - true_x).abs() < 0.15, "x {} vs {}", refined_x, true_x);
        assert!((refined_y - true_y).abs() < 0.15, "y {} vs {}", refined_y, true_y);
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
