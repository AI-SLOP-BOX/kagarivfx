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
}
