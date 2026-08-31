use crate::core::keyframe::{
    solve_bezier_eased_time, BezierControlPoint, InterpolationType, Keyframe,
};
use serde::{Deserialize, Serialize};

pub trait Interpolate: Clone {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self;
    fn default_interpolate() -> Self;
}

impl Interpolate for f32 {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        start + (end - start) * t
    }
    fn default_interpolate() -> Self {
        0.0
    }
}

impl<const N: usize> Interpolate for [f32; N] {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        let mut out = [0.0f32; N];
        for i in 0..N {
            out[i] = start[i] + (end[i] - start[i]) * t;
        }
        out
    }
    fn default_interpolate() -> Self {
        [0.0; N]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Animatable<T> {
    Constant(T),
    Animated(Vec<Keyframe<T>>),
}

impl<T: Clone> Animatable<T> {
    pub fn new_constant(value: T) -> Self {
        Animatable::Constant(value)
    }

    pub fn new_animated(keyframes: Vec<Keyframe<T>>) -> Self {
        Animatable::Animated(keyframes)
    }

    pub fn keyframes(&self) -> Option<&[Keyframe<T>]> {
        match self {
            Animatable::Constant(_) => None,
            Animatable::Animated(keyframes) => Some(keyframes),
        }
    }

    pub fn keyframes_mut(&mut self) -> Option<&mut Vec<Keyframe<T>>> {
        match self {
            Animatable::Constant(_) => None,
            Animatable::Animated(keyframes) => Some(keyframes),
        }
    }

    /// AE "Easy Ease" (F9): give every keyframe smooth bezier handles
    /// (influence 1/3, zero speed on both sides). Constants and animations
    /// with fewer than two keyframes are left untouched.
    pub fn easy_ease(&mut self) {
        if let Some(kfs) = self.keyframes_mut() {
            if kfs.len() < 2 {
                return;
            }
            for kf in kfs.iter_mut() {
                kf.interpolation = InterpolationType::Bezier {
                    outgoing: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    incoming: BezierControlPoint {
                        influence: 0.333,
                        speed: 0.0,
                    },
                    custom_bezier: None,
                };
            }
        }
    }

    pub fn add_keyframe(&mut self, new_kf: Keyframe<T>) {
        match self {
            Animatable::Constant(val) => {
                *self = Animatable::Animated(vec![
                    Keyframe::new(0, (*val).clone(), InterpolationType::Linear),
                    new_kf,
                ]);
                self.sort_keyframes();
            }
            Animatable::Animated(keyframes) => {
                if let Some(existing) = keyframes.iter_mut().find(|kf| kf.frame == new_kf.frame) {
                    *existing = new_kf;
                } else {
                    keyframes.push(new_kf);
                    self.sort_keyframes();
                }
            }
        }
    }

    /// Add or replace a keyframe without retaining a synthetic constant at the
    /// same frame. This is used by live automation writeback at frame zero.
    pub fn set_keyframe(&mut self, new_kf: Keyframe<T>) {
        if new_kf.frame == 0 {
            if let Animatable::Animated(keyframes) = self {
                if let Some(existing) = keyframes.iter_mut().find(|kf| kf.frame == 0) {
                    *existing = new_kf;
                    return;
                }
            }
            if let Animatable::Constant(_) = self {
                *self = Animatable::Animated(vec![new_kf]);
                return;
            }
        }
        self.add_keyframe(new_kf);
    }

    /// Update value at the specified frame: if constant, updates the constant;
    /// if animated, updates the existing keyframe or inserts a new one (AE behavior).
    pub fn set_value_at_frame(&mut self, frame: u32, value: T) {
        match self {
            Animatable::Constant(v) => {
                *v = value;
            }
            Animatable::Animated(keyframes) => {
                if let Some(existing) = keyframes.iter_mut().find(|kf| kf.frame == frame) {
                    existing.value = value;
                } else {
                    keyframes.push(Keyframe::new(frame, value, InterpolationType::Linear));
                    self.sort_keyframes();
                }
            }
        }
    }

    pub fn move_keyframe(&mut self, from_frame: u32, to_frame: u32) -> bool {
        if from_frame == to_frame {
            return false;
        }
        let Some(keyframes) = self.keyframes_mut() else {
            return false;
        };
        let Some(index) = keyframes.iter().position(|key| key.frame == from_frame) else {
            return false;
        };
        let mut key = keyframes.remove(index);
        key.frame = to_frame;
        if let Some(existing) = keyframes
            .iter_mut()
            .find(|candidate| candidate.frame == to_frame)
        {
            *existing = key;
        } else {
            keyframes.push(key);
            keyframes.sort_by_key(|candidate| candidate.frame);
        }
        true
    }

    fn sort_keyframes(&mut self) {
        if let Animatable::Animated(keyframes) = self {
            keyframes.sort_by_key(|kf| kf.frame);
        }
    }
}

impl<T: Interpolate> Animatable<T> {
    pub fn evaluate(&self, frame: u32) -> T {
        self.value_at_f32(frame as f32)
    }

    pub fn value_at(&self, frame: u32) -> T {
        self.value_at_f32(frame as f32)
    }

    pub fn evaluate_with_hint(&self, frame: u32, hint: &mut usize) -> T {
        self.value_at_f32_with_hint(frame as f32, hint)
    }

    pub fn value_at_f32(&self, frame: f32) -> T {
        let mut dummy = 0;
        self.value_at_f32_with_hint(frame, &mut dummy)
    }

    /// Evaluate property at frame with O(1) temporal locality hint caching.
    /// In sequential playback and timeline scrubbing, checking the cached interval index avoids O(log N) binary searches.
    pub fn value_at_f32_with_hint(&self, frame: f32, hint: &mut usize) -> T {
        match self {
            Animatable::Constant(value) => value.clone(),
            Animatable::Animated(keyframes) => {
                if keyframes.is_empty() {
                    log::error!("Animated keyframes is empty, returning default");
                    return T::default_interpolate();
                }

                let last_kf_idx = keyframes.len() - 1;
                if frame <= keyframes[0].frame as f32 {
                    *hint = 0;
                    return keyframes[0].value.clone();
                }
                if frame >= keyframes[last_kf_idx].frame as f32 {
                    *hint = last_kf_idx;
                    return keyframes[last_kf_idx].value.clone();
                }

                // Check O(1) temporal locality hint first (for sequential playback/scrubbing)
                let start_idx = if *hint < last_kf_idx
                    && frame >= keyframes[*hint].frame as f32
                    && frame < keyframes[(*hint + 1).min(last_kf_idx)].frame as f32
                {
                    *hint
                } else if *hint + 1 < last_kf_idx
                    && frame >= keyframes[*hint + 1].frame as f32
                    && frame < keyframes[(*hint + 2).min(last_kf_idx)].frame as f32
                {
                    *hint += 1;
                    *hint
                } else {
                    // Fallback to O(log N) binary search on cache miss/seek
                    let next_idx = keyframes.partition_point(|kf| kf.frame as f32 <= frame);
                    let idx = next_idx.saturating_sub(1);
                    *hint = idx;
                    idx
                };

                let start_kf = &keyframes[start_idx];
                let end_kf = &keyframes[(start_idx + 1).min(last_kf_idx)];

                let total_frames = (end_kf.frame - start_kf.frame) as f32;
                if total_frames <= 0.001 {
                    return start_kf.value.clone();
                }
                let current_offset = frame - start_kf.frame as f32;
                let t = (current_offset / total_frames).clamp(0.0, 1.0);

                match &start_kf.interpolation {
                    InterpolationType::Hold => start_kf.value.clone(),
                    InterpolationType::Linear => T::interpolate(&start_kf.value, &end_kf.value, t),
                    InterpolationType::Bezier { custom_bezier, .. } => {
                        let eased_t = if let Some(coords) = custom_bezier {
                            solve_bezier_eased_time(t, coords[0], coords[1], coords[2], coords[3])
                        } else {
                            solve_bezier_eased_time(t, 0.25, 0.1, 0.25, 1.0)
                        };
                        T::interpolate(&start_kf.value, &end_kf.value, eased_t)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_with_hint_sequential() {
        let kfs = vec![
            Keyframe::new(0, 0.0f32, InterpolationType::Linear),
            Keyframe::new(10, 100.0f32, InterpolationType::Linear),
            Keyframe::new(20, 200.0f32, InterpolationType::Linear),
        ];
        let anim = Animatable::new_animated(kfs);
        let mut hint = 0;

        assert_eq!(anim.evaluate_with_hint(0, &mut hint), 0.0);
        assert_eq!(hint, 0);

        assert_eq!(anim.evaluate_with_hint(5, &mut hint), 50.0);
        assert_eq!(hint, 0);

        assert_eq!(anim.evaluate_with_hint(15, &mut hint), 150.0);
        assert_eq!(hint, 1);

        assert_eq!(anim.evaluate_with_hint(20, &mut hint), 200.0);
        assert_eq!(hint, 2);
    }

    #[test]
    fn move_keyframe_preserves_value_interpolation_and_order() {
        let mut anim = Animatable::new_animated(vec![
            Keyframe::new(5, 10.0, InterpolationType::Hold),
            Keyframe::new(20, 30.0, InterpolationType::Linear),
        ]);
        assert!(anim.move_keyframe(5, 15));
        let keys = anim.keyframes().unwrap();
        assert_eq!(
            keys.iter().map(|key| key.frame).collect::<Vec<_>>(),
            vec![15, 20]
        );
        assert_eq!(keys[0].value, 10.0);
        assert!(matches!(keys[0].interpolation, InterpolationType::Hold));
    }

    #[test]
    fn move_keyframe_replaces_destination_and_rejects_constant() {
        let mut anim = Animatable::new_animated(vec![
            Keyframe::new(5, 10.0, InterpolationType::Linear),
            Keyframe::new(15, 99.0, InterpolationType::Hold),
        ]);
        assert!(anim.move_keyframe(5, 15));
        assert_eq!(anim.keyframes().unwrap().len(), 1);
        assert_eq!(anim.evaluate(15), 10.0);
        let mut constant = Animatable::new_constant(3.0f32);
        assert!(!constant.move_keyframe(0, 4));
    }

    #[test]
    fn move_keyframe_preserves_custom_bezier_handles() {
        let interpolation = InterpolationType::Bezier {
            outgoing: BezierControlPoint {
                influence: 0.2,
                speed: 4.0,
            },
            incoming: BezierControlPoint {
                influence: 0.8,
                speed: -2.0,
            },
            custom_bezier: Some([0.1, 0.4, 0.9, 0.7]),
        };
        let mut anim = Animatable::new_animated(vec![Keyframe::new(7, 3.0f32, interpolation)]);
        assert!(anim.move_keyframe(7, 11));
        assert_eq!(anim.keyframes().unwrap()[0].interpolation, interpolation);
    }
}
