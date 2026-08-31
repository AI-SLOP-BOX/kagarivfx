//! Motion Sketch Real-Time Freehand Tracking & Keyframe Baking Engine (AE Parity).
//!
//! Captures mouse/stylus drag gestures with high-resolution timestamps during playback
//! and resamples the trajectory into smooth position keyframes on the target layer.

#![allow(dead_code)]

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::timeline::Layer;

/// Captured raw motion sketch coordinate point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSketchSample {
    pub time_sec: f64,
    pub position: [f32; 2],
}

/// Active motion sketch recording session.
#[derive(Debug, Clone, Default)]
pub struct MotionSketchSession {
    pub is_recording: bool,
    pub samples: Vec<MotionSketchSample>,
    pub smoothing_radius: usize,
}

impl MotionSketchSession {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            samples: Vec::new(),
            smoothing_radius: 1,
        }
    }

    /// Starts a new recording session.
    pub fn start_recording(&mut self) {
        self.is_recording = true;
        self.samples.clear();
    }

    /// Records a single mouse/pen position sample.
    pub fn record_sample(&mut self, time_sec: f64, position: [f32; 2]) {
        if self.is_recording {
            self.samples.push(MotionSketchSample { time_sec, position });
        }
    }

    /// Stops recording and returns captured samples count.
    pub fn stop_recording(&mut self) -> usize {
        self.is_recording = false;
        self.samples.len()
    }

    /// Resamples captured timestamped gesture into discrete frame keyframes.
    pub fn bake_to_layer(
        &self,
        layer: &mut Layer,
        fps: f32,
        start_frame: u32,
        duration_frames: u32,
    ) {
        if self.samples.len() < 2 || fps <= 0.0 {
            return;
        }

        let start_time = self.samples.first().map(|s| s.time_sec).unwrap_or(0.0);
        let mut keyframes = Vec::new();

        for f in 0..duration_frames {
            let target_time = start_time + (f as f64 / fps as f64);

            // Linear search for enclosing sample interval
            let pos = if target_time <= self.samples[0].time_sec {
                self.samples[0].position
            } else if target_time >= self.samples.last().unwrap().time_sec {
                self.samples.last().unwrap().position
            } else {
                let mut p = self.samples[0].position;
                for i in 0..(self.samples.len() - 1) {
                    let s0 = &self.samples[i];
                    let s1 = &self.samples[i + 1];
                    if target_time >= s0.time_sec && target_time <= s1.time_sec {
                        let dt = (s1.time_sec - s0.time_sec).max(1e-5);
                        let t = ((target_time - s0.time_sec) / dt) as f32;
                        p = [
                            s0.position[0] + (s1.position[0] - s0.position[0]) * t,
                            s0.position[1] + (s1.position[1] - s0.position[1]) * t,
                        ];
                        break;
                    }
                }
                p
            };

            keyframes.push(Keyframe::new(
                start_frame + f,
                pos,
                InterpolationType::Linear,
            ));
        }

        layer.transform.position = crate::core::property::Animatable::new_animated(keyframes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::LayerType;

    #[test]
    fn test_motion_sketch_recording_and_baking() {
        let mut session = MotionSketchSession::new();
        session.start_recording();
        session.record_sample(0.0, [100.0, 100.0]);
        session.record_sample(0.5, [200.0, 150.0]);
        session.record_sample(1.0, [300.0, 200.0]);
        session.stop_recording();

        let mut layer = Layer::new("1".into(), "Sketch Layer".into(), LayerType::Null, 10);
        session.bake_to_layer(&mut layer, 2.0, 0, 3); // 2 fps over 1 second = 3 frames (0, 1, 2)

        assert_eq!(layer.transform.position.evaluate(0), [100.0, 100.0]);
        assert_eq!(layer.transform.position.evaluate(1), [200.0, 150.0]);
        assert_eq!(layer.transform.position.evaluate(2), [300.0, 200.0]);
    }
}
