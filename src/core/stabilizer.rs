//! Motion stabilization: bakes counter-movement position keyframes onto a
//! layer from its tracked point data (AE "Stabilize Motion", baked variant).
//!
//! For every tracked frame the layer is offset by the inverse of the
//! tracker's displacement, so the tracked feature stays visually fixed.
//! Stabilization is applied to the first tracker of the layer.

use crate::core::timeline::Layer;

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;

/// Bake stabilization keyframes into `layer.transform.position` using
/// `layer.trackers[0]`. Returns the number of baked keyframes (0 when the
/// layer has no tracked data).
pub fn stabilize_layer(layer: &mut Layer) -> usize {
    let Some(tp) = layer.trackers.first() else {
        return 0;
    };

    // Reference = tracker position at its earliest known frame.
    let track_kfs: Vec<(u32, [f32; 2])> = match tp.position.keyframes() {
        Some(kfs) if !kfs.is_empty() => kfs.iter().map(|k| (k.frame, k.value)).collect(),
        _ => vec![(0, tp.position.evaluate(0))],
    };
    let (_ref_frame, ref_pos) = track_kfs[0];

    // Base layer positions sampled at each tracked frame BEFORE mutation.
    let base_positions: Vec<(u32, [f32; 2], bool)> = track_kfs
        .iter()
        .map(|&(f, _)| {
            let animated = layer.transform.position.keyframes().is_some();
            (f, layer.transform.position.evaluate(f), animated)
        })
        .collect();

    let mut new_kfs: Vec<Keyframe<[f32; 2]>> = Vec::with_capacity(base_positions.len());
    for (&(tf, tpos), &(f, base_pos, was_animated)) in track_kfs.iter().zip(base_positions.iter()) {
        let delta = [ref_pos[0] - tpos[0], ref_pos[1] - tpos[1]];
        let mut value = [base_pos[0] + delta[0], base_pos[1] + delta[1]];
        let frame = if was_animated { f } else { tf };
        // Preserve existing per-frame values so repeated evaluation stays stable.
        if let Some(kfs) = layer.transform.position.keyframes() {
            if let Some(existing) = kfs.iter().find(|k| k.frame == frame) {
                value = existing.value;
            }
        }
        new_kfs.push(Keyframe::new(frame, value, InterpolationType::Linear));
    }

    // Preserve any pre-existing user keyframes that fall outside tracked frames.
    if let Some(existing) = layer.transform.position.keyframes() {
        for k in existing {
            if !new_kfs.iter().any(|nk| nk.frame == k.frame) {
                new_kfs.push(k.clone());
            }
        }
    }

    if new_kfs.is_empty() {
        return 0;
    }
    new_kfs.sort_by_key(|k| k.frame);

    // Merge duplicates: last write wins per frame.
    new_kfs.dedup_by(|a, b| a.frame == b.frame);

    layer.transform.position = Animatable::Animated(new_kfs);
    layer.transform.position.keyframes().map(|k| k.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{LayerType, TrackerPoint};

    fn make_layer(track: Vec<(u32, [f32; 2])>) -> Layer {
        let mut l = Layer::new("l".into(), "L".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        l.transform.position = Animatable::new_constant([500.0, 400.0]);
        let mut tp = TrackerPoint::new("t".into(), "T".into(), track[0].1);
        let kfs: Vec<Keyframe<[f32; 2]>> = track.iter().map(|&(f, p)| Keyframe::new(f, p, InterpolationType::Linear)).collect();
        tp.position = Animatable::Animated(kfs);
        l.trackers.push(tp);
        l
    }

    #[test]
    fn test_stabilize_bakes_counter_motion() {
        // Tracker wanders +10x/+5y over 3 frames.
        let mut l = make_layer(vec![(0, [100.0, 100.0]), (1, [110.0, 105.0]), (2, [120.0, 110.0])]);
        let n = stabilize_layer(&mut l);
        assert_eq!(n, 3);
        assert!(matches!(l.transform.position, Animatable::Animated(_)));
        // At each frame the layer shifts by -(delta): frame0 no shift,
        // frame1 (-10,-5), frame2 (-20,-10) added to base (500,400).
        assert_eq!(l.transform.position.evaluate(0), [500.0, 400.0]);
        assert_eq!(l.transform.position.evaluate(1), [490.0, 395.0]);
        assert_eq!(l.transform.position.evaluate(2), [480.0, 390.0]);
    }

    #[test]
    fn test_stabilize_without_trackers_is_noop() {
        let mut l = Layer::new("l".into(), "L".into(), LayerType::Solid { color: [1.0; 4] }, 30);
        l.transform.position = Animatable::new_constant([10.0, 20.0]);
        assert_eq!(stabilize_layer(&mut l), 0);
        assert_eq!(l.transform.position.evaluate(0), [10.0, 20.0]);
    }

    #[test]
    fn test_stabilize_single_frame_returns_one() {
        let mut l = make_layer(vec![(5, [42.0, 42.0])]);
        let n = stabilize_layer(&mut l);
        assert_eq!(n, 1);
        assert_eq!(l.transform.position.evaluate(5), [500.0, 400.0]);
    }
}
