use crate::core::keyframe::{solve_bezier_eased_time, InterpolationType, Keyframe};
use serde::{Deserialize, Serialize};

pub trait Interpolate: Clone {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        start + (end - start) * t
    }
}

impl Interpolate for [f32; 2] {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
        ]
    }
}

impl Interpolate for [f32; 3] {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
            start[2] + (end[2] - start[2]) * t,
        ]
    }
}

impl Interpolate for [f32; 4] {
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
            start[2] + (end[2] - start[2]) * t,
            start[3] + (end[3] - start[3]) * t,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
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

    fn sort_keyframes(&mut self) {
        if let Animatable::Animated(keyframes) = self {
            keyframes.sort_by_key(|kf| kf.frame);
        }
    }
}

impl<T: Interpolate> Animatable<T> {
    pub fn evaluate(&self, frame: u32) -> T {
        self.value_at(frame)
    }

    pub fn value_at(&self, frame: u32) -> T {
        match self {
            Animatable::Constant(value) => value.clone(),
            Animatable::Animated(keyframes) => {
                if keyframes.is_empty() {
                    panic!("Animatable has no keyframes");
                }

                if frame <= keyframes[0].frame {
                    return keyframes[0].value.clone();
                }

                let last_idx = keyframes.len() - 1;
                if frame >= keyframes[last_idx].frame {
                    return keyframes[last_idx].value.clone();
                }

                // Fast O(log N) binary search for the active keyframe interval
                let next_idx = keyframes.partition_point(|kf| kf.frame <= frame);
                let start_idx = next_idx.saturating_sub(1);
                let start_kf = &keyframes[start_idx];
                let end_kf = &keyframes[(start_idx + 1).min(last_idx)];

                let total_frames = (end_kf.frame - start_kf.frame) as f32;
                if total_frames <= 0.001 {
                    return start_kf.value.clone();
                }
                let current_offset = (frame - start_kf.frame) as f32;
                let t = current_offset / total_frames;

                match &start_kf.interpolation {
                    InterpolationType::Hold => start_kf.value.clone(),
                    InterpolationType::Linear => {
                        T::interpolate(&start_kf.value, &end_kf.value, t)
                    }
                    InterpolationType::Bezier { custom_bezier, .. } => {
                        let eased_t = if let Some(coords) = custom_bezier {
                            solve_bezier_eased_time(
                                t, coords[0], coords[1], coords[2], coords[3],
                            )
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
