//! Cross-domain automation bindings for audio, MIDI, and VFX parameters.

use crate::core::unified_time::{TempoMap, Time};
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
        assert_eq!(curve.sample(Time::new(1, 2)), Some(0.5));
        assert_eq!(curve.sample(Time::new(2, 1)), Some(1.0));
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
