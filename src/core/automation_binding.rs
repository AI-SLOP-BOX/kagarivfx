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
    pub fn sample(&self, time: Time) -> Option<f64> {
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
    pub fn evaluate(&self, time: Time) -> Option<f64> {
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
    fn clock_exposes_samples_and_beats() {
        let clock = ProductionClock {
            tempo: TempoMap::new(120.0),
            sample_rate: 48_000,
        };
        assert_eq!(clock.sample_position(Time::new(1, 1)), 48_000);
        assert!((clock.beat(Time::new(1, 1)) - 2.0).abs() < 1e-9);
    }
}
