//! Cross-domain automation bindings for audio, MIDI, and VFX parameters.

use crate::core::unified_time::{FrameRate, TempoMap, Time};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutomationPoint {
    pub time: Time,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationCurve {
    pub points: Vec<AutomationPoint>,
}

impl AutomationCurve {
    /// Inserts a point while preserving the strict time ordering invariant.
    /// An existing point at the same rational time is updated in place.
    pub fn upsert_point(&mut self, time: Time, value: f64) -> Result<(), &'static str> {
        if !time.is_valid() || !value.is_finite() {
            return Err("automation point must use a valid time and finite value");
        }
        if let Some(point) = self.points.iter_mut().find(|point| point.time == time) {
            point.value = value;
        } else {
            self.points.push(AutomationPoint { time, value });
            self.points.sort_by(|left, right| {
                let lhs = i128::from(left.time.numerator) * i128::from(right.time.denominator);
                let rhs = i128::from(right.time.numerator) * i128::from(left.time.denominator);
                lhs.cmp(&rhs)
            });
        }
        self.validate()
    }

    pub fn remove_point_at(&mut self, time: Time) -> Result<bool, &'static str> {
        if !time.is_valid() {
            return Err("automation point must use a valid time");
        }
        let Some(index) = self.points.iter().position(|point| point.time == time) else {
            return Ok(false);
        };
        if self.points.len() == 1 {
            return Err("automation curve must retain one point");
        }
        self.points.remove(index);
        self.validate()?;
        Ok(true)
    }

    pub fn move_point(&mut self, from: Time, to: Time) -> Result<bool, &'static str> {
        if !from.is_valid() || !to.is_valid() {
            return Err("automation point must use a valid time");
        }
        let Some(index) = self.points.iter().position(|point| point.time == from) else {
            return Ok(false);
        };
        let value = self.points[index].value;
        self.points.remove(index);
        self.upsert_point(to, value)?;
        Ok(true)
    }

    pub fn move_point_by_frames(
        &mut self,
        from: Time,
        frames: i64,
        rate: FrameRate,
    ) -> Result<bool, &'static str> {
        if !rate.is_valid() {
            return Err("frame rate must be valid");
        }
        let denominator = i128::from(from.denominator) * i128::from(rate.numerator);
        let numerator = i128::from(from.numerator) * i128::from(rate.numerator)
            + i128::from(frames) * i128::from(from.denominator) * i128::from(rate.denominator);
        let numerator = i64::try_from(numerator).map_err(|_| "automation time overflow")?;
        let denominator = u32::try_from(denominator).map_err(|_| "automation time overflow")?;
        self.move_point(from, Time::new(numerator, denominator))
    }

    /// Imports Logic Pro-style automation points whose time is an absolute
    /// audio-sample offset. The resulting curve uses the shared rational Time
    /// representation, so it can be sampled by video and audio consumers.
    pub fn from_sample_points<I>(points: I, sample_rate: u32) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = (i64, f64)>,
    {
        if sample_rate == 0 {
            return Err("sample rate must be positive");
        }
        let mut previous = None;
        let converted = points
            .into_iter()
            .map(|(sample, value)| {
                if sample < 0 {
                    return Err("sample positions must not be negative");
                }
                if previous.is_some_and(|last| sample <= last) {
                    return Err("sample positions must be strictly increasing");
                }
                previous = Some(sample);
                Ok(AutomationPoint {
                    time: Time::from_samples(sample, sample_rate),
                    value,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_events(converted)
    }

    pub fn from_events<I>(events: I) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = AutomationPoint>,
    {
        let curve = Self {
            points: events.into_iter().collect(),
        };
        curve.validate()?;
        Ok(curve)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.points.is_empty() || self.points.len() > 8192 {
            return Err("automation curve must contain 1..=8192 points");
        }
        if self.points.iter().any(|point| !point.time.is_valid()) {
            return Err("automation times must have a non-zero denominator");
        }
        if self.points.iter().any(|point| !point.value.is_finite()) {
            return Err("automation values must be finite");
        }
        if self
            .points
            .windows(2)
            .any(|pair| !time_before(pair[0].time, pair[1].time))
        {
            return Err("automation times must be strictly increasing");
        }
        Ok(())
    }

    pub fn sample(&self, time: Time) -> Option<f64> {
        self.validate().ok()?;
        let first = *self.points.first()?;
        if time_before(time, first.time) {
            return Some(first.value);
        }
        for pair in self.points.windows(2) {
            let [left, right] = pair else { unreachable!() };
            if !time_before(right.time, time) {
                let span = seconds(right.time) - seconds(left.time);
                if span <= f64::EPSILON {
                    return Some(right.value);
                }
                let amount = ((seconds(time) - seconds(left.time)) / span).clamp(0.0, 1.0);
                return Some(left.value + (right.value - left.value) * amount);
            }
        }
        self.points.last().map(|point| point.value)
    }

    pub fn sample_at_frame(&self, frame: i64, rate: FrameRate) -> Option<f64> {
        if !rate.is_valid() || frame < 0 {
            return None;
        }
        self.sample(Time::from_frame(frame, rate))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationBinding {
    pub source: String,
    pub target: String,
    pub curve: AutomationCurve,
    pub input_min: f64,
    pub input_max: f64,
    pub output_min: f64,
    pub output_max: f64,
}

impl AutomationBinding {
    pub const MAX_ENDPOINT_LENGTH: usize = 4096;

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.source.trim().is_empty()
            || self.target.trim().is_empty()
            || self.source != self.source.trim()
            || self.target != self.target.trim()
        {
            return Err("automation source and target are required");
        }
        if self.source.len() > Self::MAX_ENDPOINT_LENGTH
            || self.target.len() > Self::MAX_ENDPOINT_LENGTH
        {
            return Err("automation source and target are too long");
        }
        if self
            .source
            .chars()
            .chain(self.target.chars())
            .any(char::is_control)
        {
            return Err("automation source and target must not contain control characters");
        }
        if [
            self.input_min,
            self.input_max,
            self.output_min,
            self.output_max,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err("automation ranges must be finite");
        }
        if self.input_max <= self.input_min {
            return Err("automation input range must be increasing");
        }
        self.curve.validate()
    }

    pub fn evaluate(&self, time: Time) -> Option<f64> {
        self.validate().ok()?;
        let value = self.curve.sample(time)?;
        self.map_value(value)
    }

    pub fn map_value(&self, value: f64) -> Option<f64> {
        self.validate().ok()?;
        if !value.is_finite() {
            return None;
        }
        let span = self.input_max - self.input_min;
        let normalized = if span.abs() <= f64::EPSILON {
            0.0
        } else {
            ((value - self.input_min) / span).clamp(0.0, 1.0)
        };
        Some(self.output_min + (self.output_max - self.output_min) * normalized)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionClock {
    pub tempo: TempoMap,
    pub sample_rate: u32,
}

impl ProductionClock {
    pub fn sample_position(&self, time: Time) -> i64 {
        time.to_sample_floor(self.sample_rate.max(1))
    }

    pub fn beat(&self, time: Time) -> f64 {
        self.tempo.beat_at(time)
    }
}

fn seconds(time: Time) -> f64 {
    time.numerator as f64 / f64::from(time.denominator)
}

fn time_before(left: Time, right: Time) -> bool {
    i128::from(left.numerator) * i128::from(right.denominator)
        < i128::from(right.numerator) * i128::from(left.denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_interpolates_and_clamps() {
        let curve = AutomationCurve {
            points: vec![
                AutomationPoint {
                    time: Time::ZERO,
                    value: 0.0,
                },
                AutomationPoint {
                    time: Time::new(1, 1),
                    value: 1.0,
                },
            ],
        };
        assert_eq!(curve.sample(Time::new(-1, 1)), Some(0.0));
        assert_eq!(curve.sample(Time::new(1, 4)), Some(0.25));
        assert_eq!(curve.sample(Time::new(2, 1)), Some(1.0));
    }

    #[test]
    fn imports_sample_based_automation_into_shared_time() {
        let curve =
            AutomationCurve::from_sample_points([(0, 0.0), (24_000, 1.0), (48_000, 0.5)], 48_000)
                .unwrap();
        assert_eq!(curve.points[1].time, Time::new(1, 2));
        assert_eq!(curve.sample(Time::new(1, 4)), Some(0.5));
        assert_eq!(
            curve.sample_at_frame(12, FrameRate::new(24, 1).unwrap()),
            Some(1.0)
        );
    }

    #[test]
    fn rejects_invalid_sample_automation_input() {
        assert!(AutomationCurve::from_sample_points([(0, 0.0)], 0).is_err());
        assert!(AutomationCurve::from_sample_points([(-1, 0.0)], 48_000).is_err());
        assert!(AutomationCurve::from_sample_points([(1, 0.0), (1, 1.0)], 48_000).is_err());
        assert!(AutomationCurve::from_sample_points([(0, f64::NAN)], 48_000).is_err());
    }

    #[test]
    fn upsert_point_keeps_curve_order_and_updates_collisions() {
        let mut curve = AutomationCurve::from_events([
            AutomationPoint {
                time: Time::ZERO,
                value: 0.0,
            },
            AutomationPoint {
                time: Time::new(1, 1),
                value: 1.0,
            },
        ])
        .unwrap();
        curve.upsert_point(Time::new(1, 2), 0.5).unwrap();
        curve.upsert_point(Time::new(1, 1), 0.25).unwrap();
        assert_eq!(curve.points[1].value, 0.5);
        assert_eq!(curve.points[2].value, 0.25);
        assert!(curve.upsert_point(Time::new(1, 3), f64::NAN).is_err());
        assert!(curve.remove_point_at(Time::new(1, 2)).unwrap());
        assert!(!curve.remove_point_at(Time::new(1, 2)).unwrap());
        assert!(curve.remove_point_at(Time::ZERO).unwrap());
        assert!(curve.remove_point_at(Time::new(1, 1)).is_err());
        curve.upsert_point(Time::ZERO, 0.0).unwrap();
        assert!(curve.move_point(Time::new(1, 1), Time::new(1, 3)).unwrap());
        assert_eq!(
            curve.points.last().map(|point| point.time),
            Some(Time::new(1, 3))
        );
        assert!(!curve.move_point(Time::new(1, 1), Time::new(2, 1)).unwrap());
        assert!(curve
            .move_point_by_frames(Time::new(1, 3), 2, FrameRate::new(30, 1).unwrap())
            .unwrap());
        assert_eq!(
            curve.points.last().map(|point| point.time),
            Some(Time::new(2, 5))
        );
    }

    #[test]
    fn moving_point_onto_existing_time_replaces_destination_without_duplicates() {
        let mut curve = AutomationCurve::from_events([
            AutomationPoint {
                time: Time::ZERO,
                value: 0.25,
            },
            AutomationPoint {
                time: Time::new(1, 2),
                value: 0.5,
            },
            AutomationPoint {
                time: Time::new(1, 1),
                value: 0.75,
            },
        ])
        .unwrap();

        assert!(curve.move_point(Time::new(1, 2), Time::new(1, 1)).unwrap());
        assert_eq!(curve.points.len(), 2);
        assert_eq!(curve.sample(Time::new(1, 1)), Some(0.5));
        assert!(curve.validate().is_ok());
    }

    #[test]
    fn moving_point_backwards_by_frames_preserves_exact_rational_time() {
        let mut curve = AutomationCurve::from_events([
            AutomationPoint {
                time: Time::new(1, 1),
                value: 1.0,
            },
            AutomationPoint {
                time: Time::new(2, 1),
                value: 2.0,
            },
        ])
        .unwrap();

        assert!(curve
            .move_point_by_frames(Time::new(2, 1), -15, FrameRate::new(30, 1).unwrap())
            .unwrap());
        assert_eq!(curve.points[1].time, Time::new(3, 2));
    }

    #[test]
    fn rejects_invalid_or_overflowing_frame_moves() {
        let mut curve = AutomationCurve::from_events([AutomationPoint {
            time: Time::ZERO,
            value: 1.0,
        }])
        .unwrap();
        assert!(curve
            .move_point_by_frames(
                Time::new(i64::MAX, 1),
                i64::MAX,
                FrameRate::new(1, 1).unwrap()
            )
            .is_err());
    }

    #[test]
    fn binding_maps_audio_range_to_vfx_range() {
        let binding = AutomationBinding {
            source: "audio.bass".into(),
            target: "vfx.glow.intensity".into(),
            curve: AutomationCurve {
                points: vec![AutomationPoint {
                    time: Time::ZERO,
                    value: 0.5,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 10.0,
            output_max: 50.0,
        };
        assert_eq!(binding.evaluate(Time::ZERO), Some(30.0));
    }

    #[test]
    fn rejects_invalid_binding_data() {
        let curve = AutomationCurve {
            points: vec![
                AutomationPoint {
                    time: Time::new(1, 1),
                    value: 0.0,
                },
                AutomationPoint {
                    time: Time::new(1, 1),
                    value: 1.0,
                },
            ],
        };
        assert!(curve.validate().is_err());
        let binding = AutomationBinding {
            source: String::new(),
            target: "vfx.opacity".into(),
            curve: AutomationCurve {
                points: vec![AutomationPoint {
                    time: Time::ZERO,
                    value: 0.0,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
        };
        assert!(binding.validate().is_err());
    }

    #[test]
    fn rejects_oversized_binding_endpoints() {
        let binding = AutomationBinding {
            source: "s".repeat(AutomationBinding::MAX_ENDPOINT_LENGTH + 1),
            target: "vfx.opacity".into(),
            curve: AutomationCurve {
                points: vec![AutomationPoint {
                    time: Time::ZERO,
                    value: 0.0,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
        };
        assert!(binding.validate().is_err());
    }

    #[test]
    fn event_stream_builds_a_valid_cross_domain_curve() {
        let curve = AutomationCurve::from_events([
            AutomationPoint {
                time: Time::from_samples(0, 48_000),
                value: 0.0,
            },
            AutomationPoint {
                time: Time::from_samples(24_000, 48_000),
                value: 1.0,
            },
        ])
        .expect("ordered events should produce a curve");
        assert_eq!(curve.sample(Time::new(1, 4)), Some(0.5));
    }

    #[test]
    fn event_stream_rejects_unordered_events() {
        let result = AutomationCurve::from_events([
            AutomationPoint {
                time: Time::new(1, 1),
                value: 1.0,
            },
            AutomationPoint {
                time: Time::ZERO,
                value: 0.0,
            },
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_control_characters_in_binding_endpoints() {
        let binding = AutomationBinding {
            source: "audio\0bass".into(),
            target: "vfx.opacity".into(),
            curve: AutomationCurve {
                points: vec![AutomationPoint {
                    time: Time::ZERO,
                    value: 0.0,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
        };
        assert!(binding.validate().is_err());
    }

    #[test]
    fn rejects_untrimmed_binding_endpoints() {
        let binding = AutomationBinding {
            source: " audio.level".into(),
            target: "vfx.opacity".into(),
            curve: AutomationCurve {
                points: vec![AutomationPoint {
                    time: Time::ZERO,
                    value: 0.0,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
        };
        assert!(binding.validate().is_err());
    }

    #[test]
    fn clock_exposes_samples_and_beats() {
        let clock = ProductionClock {
            tempo: TempoMap::new(120.0),
            sample_rate: 48_000,
        };
        assert_eq!(clock.sample_position(Time::new(1, 1)), 48_000);
        assert!((clock.beat(Time::new(1, 1)) - 2.0).abs() < 1e-9);
    }
}
