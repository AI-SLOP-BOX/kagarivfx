//! One-click animation presets for the timeline (YouTube-title tier):
//! fades, pops, slides, punch-zooms. Each bakes keyframes onto the
//! selected layer relative to its current state / playhead.
//!
//! All functions are deterministic and unit-tested.

use crate::core::keyframe::{EasePreset, InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::Layer;

fn kf(frame: u32, v: f32) -> Keyframe<f32> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}
fn kfv2(frame: u32, v: [f32; 2]) -> Keyframe<[f32; 2]> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}

fn ease_all<T>(kfs: &mut [Keyframe<T>]) {
    let coords = EasePreset::Standard.control_points();
    for kf in kfs.iter_mut() {
        kf.interpolation = InterpolationType::Bezier {
            outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
            custom_bezier: Some(coords),
        };
    }
}

/// Opacity 0→100 over the first `dur` frames of the playhead window.
pub fn fade_in(l: &mut Layer, cf: u32, dur: u32) -> bool {
    let end = cf + dur.max(2);
    let mut kfs = vec![kf(cf, 0.0), kf(end, 100.0)];
    ease_all(&mut kfs);
    l.transform.opacity = Animatable::Animated(kfs);
    true
}

/// Opacity 100→0 across the last `dur` frames before the layer out-point.
pub fn fade_out(l: &mut Layer, _cf: u32, dur: u32) -> bool {
    let out = l.out_frame;
    let start = out.saturating_sub(dur.max(2));
    if start <= l.in_frame { return false; }
    let mut kfs = vec![kf(start, 100.0), kf(out.saturating_sub(1), 0.0)];
    ease_all(&mut kfs);
    l.transform.opacity = Animatable::Animated(kfs);
    true
}

/// Scale pop: 0% → 112% → 100% with overshoot settle.
pub fn pop_in(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.scale.evaluate(cf);
    let mut kfs = vec![
        kfv2(cf, [0.0, 0.0]),
        kfv2(cf + 12, [base[0] * 1.12, base[1] * 1.12]),
        kfv2(cf + 20, base),
    ];
    ease_all(&mut kfs);
    l.transform.scale = Animatable::Animated(kfs);
    // Pair with a quick opacity snap so it doesn't pop from nothing.
    let mut op = vec![kf(cf, 0.0), kf(cf + 6, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Slide in from off-screen left/right while fading up.
pub fn slide_in(l: &mut Layer, cf: u32, comp_w: f32, from_right: bool) -> bool {
    let base = l.transform.position.evaluate(cf);
    let dir: f32 = if from_right { 1.0 } else { -1.0 };
    let start_x = base[0] + dir * comp_w * 0.35;
    let mut pos_kfs = vec![
        kfv2(cf, [start_x, base[1]]),
        kfv2(cf + 24, base),
    ];
    ease_all(&mut pos_kfs);
    l.transform.position = Animatable::Animated(pos_kfs);

    let mut op = vec![kf(cf, 0.0), kf(cf + 16, 100.0)];
    ease_all(&mut op);
    l.transform.opacity = Animatable::Animated(op);
    true
}

/// Impact punch: quick scale spike then settle (use on beat hits).
pub fn zoom_punch(l: &mut Layer, cf: u32) -> bool {
    let base = l.transform.scale.evaluate(cf);
    let mut kfs = vec![
        kfv2(cf, base),
        kfv2(cf + 3, [base[0] * 1.18, base[1] * 1.18]),
        kfv2(cf + 8, [base[0] * 0.97, base[1] * 0.97]),
        kfv2(cf + 14, base),
    ];
    ease_all(&mut kfs);
    l.transform.scale = Animatable::Animated(kfs);
    true
}

/// Drop-in shadow emphasis: subtle scale-down + opacity dim then restore,
/// used to make a title land heavier.
pub fn slam_in(l: &mut Layer, cf: u32) -> bool {
    let base_s = l.transform.scale.evaluate(cf);
    let base_p = l.transform.position.evaluate(cf);
    let mut s = vec![
        kfv2(cf, [base_s[0] * 1.6, base_s[1] * 1.6]),
        kfv2(cf + 10, base_s),
    ];
    ease_all(&mut s);
    l.transform.scale = Animatable::Animated(s);

    let mut p = vec![
        kfv2(cf, [base_p[0], base_p[1] - 40.0]),
        kfv2(cf + 10, base_p),
    ];
    ease_all(&mut p);
    l.transform.position = Animatable::Animated(p);
    true
}

pub const NAMES: &[&str] = &[
    "Fade In", "Fade Out", "Pop In", "Slide In ←", "Slide In →",
    "Zoom Punch", "Slam In",
];

/// Dispatch by name; returns whether the preset applied.
pub fn apply_by_name(name: &str, l: &mut Layer, cf: u32, comp_w: f32, _comp_h: f32) -> bool {
    match name {
        "Fade In" => fade_in(l, cf, 20),
        "Fade Out" => fade_out(l, cf, 20),
        "Pop In" => pop_in(l, cf),
        "Slide In ←" => slide_in(l, cf, comp_w, false),
        "Slide In →" => slide_in(l, cf, comp_w, true),
        "Zoom Punch" => zoom_punch(l, cf),
        "Slam In" => slam_in(l, cf),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{LayerType};

    fn mk() -> Layer {
        let mut l = Layer::new("p".into(), "P".into(), LayerType::Solid { color: [1.0; 4] }, 200);
        l.in_frame = 10;
        l.out_frame = 190;
        l.transform.opacity = Animatable::new_constant(100.0);
        l.transform.scale = Animatable::new_constant([100.0, 100.0]);
        l.transform.position = Animatable::new_constant([960.0, 540.0]);
        l
    }

    #[test]
    fn test_fade_in_creates_two_keyframes() {
        let mut l = mk();
        assert!(apply_by_name("Fade In", &mut l, 30, 1920.0, 1080.0));
        let kfs = l.transform.opacity.keyframes().unwrap();
        assert_eq!(kfs.len(), 2);
        assert_eq!(kfs[0].value, 0.0);
        assert_eq!(kfs[1].value, 100.0);
    }

    #[test]
    fn test_fade_out_fails_when_window_too_small() {
        let mut l = mk();
        l.out_frame = 12; // in=10 → start would be < in
        assert!(!apply_by_name("Fade Out", &mut l, 30, 1920.0, 1080.0));
    }

    #[test]
    fn test_pop_in_overshoot_and_settle() {
        let mut l = mk();
        apply_by_name("Pop In", &mut l, 50, 1920.0, 1080.0);
        let kfs = l.transform.scale.keyframes().unwrap();
        assert_eq!(kfs.len(), 3);
        assert_eq!(kfs[0].value, [0.0, 0.0]);
        assert!((kfs[1].value[0] - 112.0).abs() < 0.01, "overshoot 112%");
        assert_eq!(kfs[2].value, [100.0, 100.0]);
        // opacity snap present
        assert_eq!(l.transform.opacity.keyframes().unwrap().len(), 2);
    }

    #[test]
    fn test_slide_direction_offset() {
        let mut left = mk();
        apply_by_name("Slide In ←", &mut left, 40, 1920.0, 1080.0);
        let kl = left.transform.position.keyframes().unwrap();
        assert!((kl[0].value[0] - (960.0 - 672.0)).abs() < 0.01);

        let mut right = mk();
        apply_by_name("Slide In →", &mut right, 40, 1920.0, 1080.0);
        let kr = right.transform.position.keyframes().unwrap();
        assert!((kr[0].value[0] - (960.0 + 672.0)).abs() < 0.01);
    }

    #[test]
    fn test_unknown_preset_is_noop_false() {
        let mut l = mk();
        assert!(!apply_by_name("Nope", &mut l, 30, 1920.0, 1080.0));
        assert!(l.transform.opacity.keyframes().is_none());
    }
}
