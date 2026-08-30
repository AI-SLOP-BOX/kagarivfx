//! Exact time conversions shared by video, audio, MIDI, and tempo-aware tools.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Time {
    pub numerator: i64,
    pub denominator: u32,
}

impl Time {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    pub fn new(numerator: i64, denominator: u32) -> Self {
        if denominator == 0 {
            return Self::ZERO;
        }
        let divisor = gcd(numerator.unsigned_abs(), u64::from(denominator));
        Self {
            numerator: numerator / divisor as i64,
            denominator: denominator / divisor as u32,
        }
    }

    pub fn is_valid(self) -> bool {
        self.denominator != 0
    }

    pub fn from_frame(frame: i64, rate: FrameRate) -> Self {
        if !rate.is_valid() {
            return Self::ZERO;
        }
        Self::new(
            frame.saturating_mul(i64::from(rate.denominator)),
            rate.numerator,
        )
    }

    pub fn from_samples(samples: i64, sample_rate: u32) -> Self {
        Self::new(samples, sample_rate)
    }

    pub fn to_frame_floor(self, rate: FrameRate) -> i64 {
        if !self.is_valid() || !rate.is_valid() {
            return 0;
        }
        div_floor(
            i128::from(self.numerator) * i128::from(rate.numerator),
            i128::from(self.denominator) * i128::from(rate.denominator),
        ) as i64
    }

    pub fn to_sample_floor(self, sample_rate: u32) -> i64 {
        div_floor(
            i128::from(self.numerator) * i128::from(sample_rate),
            i128::from(self.denominator),
        ) as i64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        let divisor = gcd(u64::from(numerator), u64::from(denominator)) as u32;
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub fn is_valid(self) -> bool {
        self.numerator != 0 && self.denominator != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    pub at: Time,
    pub bpm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoMap {
    pub changes: Vec<TempoChange>,
}

impl Default for TempoMap {
    fn default() -> Self {
        Self::new(120.0)
    }
}

impl TempoMap {
    pub fn new(bpm: f64) -> Self {
        Self {
            changes: vec![TempoChange {
                at: Time::ZERO,
                bpm: bpm.max(f64::EPSILON),
            }],
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.changes.is_empty() {
            return Err("tempo map must contain an initial tempo");
        }
        if self.changes[0].at != Time::ZERO {
            return Err("tempo map must start at time zero");
        }
        if self.changes.iter().any(|change| !change.at.is_valid()) {
            return Err("tempo change times must have a non-zero denominator");
        }
        if self
            .changes
            .iter()
            .any(|change| !change.bpm.is_finite() || change.bpm <= 0.0)
        {
            return Err("tempo changes must use finite positive BPM values");
        }
        if self
            .changes
            .windows(2)
            .any(|pair| !time_before(pair[0].at, pair[1].at))
        {
            return Err("tempo changes must be strictly increasing");
        }
        Ok(())
    }

    pub fn beat_at(&self, time: Time) -> f64 {
        if self.validate().is_err() {
            return 0.0;
        }
        let mut beat = 0.0;
        let mut previous = Time::ZERO;
        let mut bpm = self.changes.first().map_or(120.0, |change| change.bpm);
        for change in self
            .changes
            .iter()
            .filter(|change| time_le(change.at, time))
        {
            beat += seconds_between(previous, change.at) * bpm / 60.0;
            previous = change.at;
            bpm = change.bpm.max(f64::EPSILON);
        }
        beat + seconds_between(previous, time) * bpm / 60.0
    }

    pub fn time_at_beat(&self, target_beat: f64) -> Time {
        if self.validate().is_err() || !target_beat.is_finite() {
            return Time::ZERO;
        }
        let mut beat = 0.0;
        let mut previous = Time::ZERO;
        let mut bpm = self.changes.first().map_or(120.0, |change| change.bpm);
        for change in self
            .changes
            .iter()
            .filter(|change| time_le(Time::ZERO, change.at))
        {
            let segment_beats = seconds_between(previous, change.at) * bpm / 60.0;
            if beat + segment_beats >= target_beat {
                return from_seconds(
                    previous_seconds(previous) + (target_beat - beat) * 60.0 / bpm,
                );
            }
            beat += segment_beats;
            previous = change.at;
            bpm = change.bpm.max(f64::EPSILON);
        }
        from_seconds(previous_seconds(previous) + (target_beat - beat) * 60.0 / bpm)
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

fn div_floor(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder != 0 && (remainder < 0) != (denominator < 0) {
        quotient - 1
    } else {
        quotient
    }
}

fn previous_seconds(time: Time) -> f64 {
    time.numerator as f64 / f64::from(time.denominator)
}

fn seconds_between(start: Time, end: Time) -> f64 {
    previous_seconds(end) - previous_seconds(start)
}

fn time_le(left: Time, right: Time) -> bool {
    i128::from(left.numerator) * i128::from(right.denominator)
        <= i128::from(right.numerator) * i128::from(left.denominator)
}

fn time_before(left: Time, right: Time) -> bool {
    i128::from(left.numerator) * i128::from(right.denominator)
        < i128::from(right.numerator) * i128::from(left.denominator)
}

fn from_seconds(seconds: f64) -> Time {
    const SCALE: f64 = 1_000_000_000.0;
    Time::new((seconds * SCALE).round() as i64, SCALE as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_frame_and_sample_conversions() {
        let rate = FrameRate::new(24_000, 1_001).unwrap();
        assert_eq!(Time::from_frame(1, rate), Time::new(1_001, 24_000));
        assert_eq!(Time::from_frame(1, rate).to_frame_floor(rate), 1);
        assert_eq!(
            Time::from_samples(48_000, 48_000).to_frame_floor(FrameRate::new(30, 1).unwrap()),
            30
        );
    }

    #[test]
    fn rejects_invalid_frame_rates() {
        assert!(FrameRate::new(0, 1).is_none());
        assert!(FrameRate::new(24, 0).is_none());
    }

    #[test]
    fn deserialized_zero_denominator_time_is_invalid() {
        let time: Time = serde_json::from_str(r#"{"numerator":1,"denominator":0}"#).unwrap();
        assert!(!time.is_valid());
    }

    #[test]
    fn malformed_frame_rate_conversions_fail_closed() {
        let malformed = FrameRate {
            numerator: 24,
            denominator: 0,
        };
        assert!(!malformed.is_valid());
        assert_eq!(Time::from_frame(100, malformed), Time::ZERO);
        assert_eq!(Time::new(1, 1).to_frame_floor(malformed), 0);
    }

    #[test]
    fn tempo_map_handles_tempo_change() {
        let mut map = TempoMap::new(120.0);
        map.changes.push(TempoChange {
            at: Time::new(2, 1),
            bpm: 60.0,
        });
        assert!((map.beat_at(Time::new(3, 1)) - 5.0).abs() < 1e-9);
        assert_eq!(map.time_at_beat(5.0), Time::new(3, 1));
    }

    #[test]
    fn tempo_map_rejects_invalid_changes() {
        let mut map = TempoMap::new(120.0);
        map.changes[0].bpm = f64::NAN;
        assert!(map.validate().is_err());

        let mut map = TempoMap::new(120.0);
        map.changes.push(TempoChange {
            at: Time::ZERO,
            bpm: 90.0,
        });
        assert!(map.validate().is_err());

        let mut map = TempoMap::new(120.0);
        map.changes[0].at = Time::new(1, 1);
        assert!(map.validate().is_err());
    }

    #[test]
    fn invalid_tempo_map_fails_closed_for_queries() {
        let mut map = TempoMap::new(120.0);
        map.changes[0].bpm = f64::INFINITY;
        assert_eq!(map.beat_at(Time::new(1, 1)), 0.0);
        assert_eq!(map.time_at_beat(1.0), Time::ZERO);
        assert_eq!(map.time_at_beat(f64::NAN), Time::ZERO);
    }
}
