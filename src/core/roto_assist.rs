//! RotoAssist: tracker-driven mask animation — the practical, non-ML stand-in
//! for Rotobrush-style assistance. Bakes a tracked point's motion onto every
//! vertex of a base polygon so the matte follows the footage automatically.

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::Mask;
use crate::core::property::Animatable;
use crate::core::timeline::TrackerPoint;

/// Bake per-vertex keyframes: each polygon vertex follows the tracker's
/// sampled offset curve (position(t) − position(frame0)), preserving shape.
/// Returns a NEW animated mask path; caller assigns to `mask.path`.
pub fn bake_tracked_mask(
    base_mask: &Mask,
    tracker: &TrackerPoint,
    start_frame: u32,
    end_frame: u32,
) -> Result<Animatable<Vec<[f32; 2]>>, String> {
    let base_poly = base_mask.path.to_polygon(start_frame.max(1), 16);
    if base_poly.is_empty() {
        return Err("base mask has no vertices".into());
    }
    let origin = tracker.position.evaluate(start_frame);
    let mut kfs: Vec<Keyframe<Vec<[f32; 2]>>> = Vec::new();
    let step = ((end_frame - start_frame) / 60).max(1); // ≤ ~60 samples
    let mut f = start_frame;
    while f <= end_frame {
        let cur = tracker.position.evaluate(f);
        let dx = cur[0] - origin[0];
        let dy = cur[1] - origin[1];
        let moved: Vec<[f32; 2]> = base_poly.iter().map(|p| [p[0] + dx, p[1] + dy]).collect();
        kfs.push(Keyframe::new(f, moved, InterpolationType::Linear));
        f += step;
    }
    if kfs.is_empty() {
        return Err("no frames to bake".into());
    }
    Ok(Animatable::Animated(kfs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mask::MaskPath;

    fn square_mask() -> Mask {
        // 12x12 square at origin
        crate::core::mask::Mask::new_rect("m".into(), "M".into(), 0.0, 0.0, 12.0, 12.0)
    }

    fn moving_tracker() -> TrackerPoint {
        let mut t = TrackerPoint::new("t".into(), "T".into(), [100.0, 100.0]);
        t.position = Animatable::Animated(vec![
            Keyframe::new(0, [100.0, 100.0], InterpolationType::Linear),
            Keyframe::new(10, [140.0, 90.0], InterpolationType::Linear),
        ]);
        t
    }

    #[test]
    fn test_bake_translates_shape_with_tracker() {
        let m = square_mask();
        let t = moving_tracker();
        let baked = bake_tracked_mask(&m, &t, 0, 10).expect("bakes");
        match &baked {
            Animatable::Animated(kfs) => {
                assert!(kfs.len() >= 2);
                let first = kfs[0].value.clone();
                let last = kfs.last().unwrap().value.clone();
                // Shape translated by (40, -10)
                assert!((last[0][0] - first[0][0] - 40.0).abs() < 0.01);
                assert!((last[0][1] - first[0][1] + 10.0).abs() < 0.01);
            }
            _ => panic!("expected animated"),
        }
    }

    #[test]
    fn test_empty_base_mask_errors() {
        let mut m = Mask::new_rect("e".into(), "E".into(), 0.0, 0.0, 8.0, 8.0);
        m.path.vertices = Animatable::Animated(vec![]); // no vertices at all
        let t = moving_tracker();
        assert!(bake_tracked_mask(&m, &t, 0, 10).is_err());
    }

    #[test]
    fn test_static_tracker_yields_constant_motion() {
        let m = square_mask();
        let t = TrackerPoint::new("t".into(), "T".into(), [5.0, 5.0]);
        let baked = bake_tracked_mask(&m, &t, 0, 20).expect("bakes");
        if let Animatable::Animated(kfs) = &baked {
            let first = kfs[0].value.clone();
            let last = kfs.last().unwrap().value.clone();
            assert_eq!(first[0], last[0], "no motion → identical vertices");
        } else {
            panic!("expected animated");
        }
    }
}
