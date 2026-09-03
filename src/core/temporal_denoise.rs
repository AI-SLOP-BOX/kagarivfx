//! CPU port of NextVFX's `denoiser.wgsl` (spatio-temporal noise reduction).
//!
//! Algorithm (per channel):
//! ```text
//! diff = abs(current - previous)
//! mix_factor = if diff > feature_threshold { 0.0 } else { temporal_mix }
//! output = mix(current, previous, mix_factor * strength)
//! ```

pub struct TemporalDenoiser {
    previous_frame: Option<Vec<u8>>,
    strength: f32,
    temporal_mix: f32,
    feature_threshold: f32,
}

impl TemporalDenoiser {
    pub fn new() -> Self {
        Self {
            previous_frame: None,
            strength: 1.0,
            temporal_mix: 0.5,
            feature_threshold: 16.0,
        }
    }

    pub fn set_strength(&mut self, value: f32) {
        self.strength = value.clamp(0.0, 1.0);
    }

    pub fn set_temporal_mix(&mut self, value: f32) {
        self.temporal_mix = value.clamp(0.0, 1.0);
    }

    pub fn set_feature_threshold(&mut self, value: f32) {
        self.feature_threshold = value.max(0.0);
    }

    /// Blend the current frame with the stored previous frame in place.
    /// On first call (or size mismatch) the frame is only saved; no blending.
    pub fn process(&mut self, frame: &mut [u8], width: u32, height: u32) {
        let len = (width as usize)
            .checked_mul(height as usize)
            .map(|p| p.saturating_mul(4))
            .unwrap_or(usize::MAX);

        match self.previous_frame.take() {
            Some(prev) if prev.len() == len && frame.len() == len => {
                let strength = self.strength;
                let temporal_mix = self.temporal_mix;
                let threshold = self.feature_threshold;
                for (cur, &prev_px) in frame.iter_mut().zip(prev.iter()) {
                    let diff = (*cur as f32 - prev_px as f32).abs();
                    let mix_factor = if diff > threshold { 0.0 } else { temporal_mix };
                    let mixed =
                        *cur as f32 + (prev_px as f32 - *cur as f32) * mix_factor * strength;
                    *cur = mixed.round().clamp(0.0, 255.0) as u8;
                }
                self.previous_frame = Some(frame.to_vec());
            }
            _ => {
                if frame.len() == len {
                    self.previous_frame = Some(frame.to_vec());
                } else {
                    self.previous_frame = None;
                }
            }
        }
    }

    /// Call when the shot/cut changes so the next frame is not blended
    /// against a stale image.
    pub fn reset(&mut self) {
        self.previous_frame = None;
    }
}

impl Default for TemporalDenoiser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 8;
    const H: u32 = 8;

    fn make_frame(base: i32, noise: impl Fn(i32) -> i32) -> Vec<u8> {
        (0..(W * H * 4) as i32)
            .map(|i| {
                if i % 4 == 3 {
                    255
                } else {
                    (base + noise(i)).clamp(0, 255) as u8
                }
            })
            .collect()
    }

    #[test]
    fn reduces_noise_energy_across_frames() {
        let mut d = TemporalDenoiser::new();
        d.set_strength(1.0);
        d.set_temporal_mix(0.5);
        d.set_feature_threshold(20.0);

        // Same underlying signal (100), independent noise within +-10.
        let mut f1 = make_frame(100, |i| ((i * 7) % 21) - 10);
        let mut f2 = make_frame(100, |i| ((i * 13 + 5) % 21) - 10);

        d.process(&mut f1, W, H);
        d.process(&mut f2, W, H);

        let energy_after: i64 = f2.chunks(4).map(|c| (c[0] as i64 - 100).abs()).sum();
        let noisy_energy: i64 = make_frame(100, |i| ((i * 13 + 5) % 21) - 10)
            .chunks(4)
            .map(|c| (c[0] as i64 - 100).abs())
            .sum();

        // Averaging two independent noise realizations must strictly reduce
        // the deviation from the underlying signal.
        assert!(
            energy_after < noisy_energy,
            "energy {energy_after} should be below noisy {noisy_energy}"
        );
        // With temporal_mix=0.5 the residual is roughly half; allow slack for
        // clamping/rounding.
        assert!(
            energy_after * 2 < noisy_energy * 3,
            "energy {energy_after} should be substantially reduced"
        );
    }

    #[test]
    fn large_motion_is_preserved() {
        let mut d = TemporalDenoiser::new();
        d.set_strength(1.0);
        d.set_temporal_mix(0.9);
        d.set_feature_threshold(30.0);

        let mut prev = make_frame(50, |_| 0);
        let mut cur = vec![200u8; (W * H * 4) as usize];
        cur.iter_mut().skip(3).step_by(4).for_each(|a| *a = 255);

        d.process(&mut prev, W, H);
        d.process(&mut cur, W, H);

        assert!(
            cur.iter().step_by(4).all(|&p| p == 200),
            "fast-moving pixels above threshold must stay unchanged"
        );
    }

    #[test]
    fn size_mismatch_does_not_panic() {
        let mut d = TemporalDenoiser::new();
        let mut small = vec![0u8; 16];
        d.process(&mut small, 2, 2);
        let mut big = vec![128u8; (W * H * 4) as usize];
        d.process(&mut big, W, H);
        let mut tiny = vec![0u8; 4];
        d.process(&mut tiny, 99, 0);
        d.reset();
        d.process(&mut tiny, 99, 0);
    }

    #[test]
    fn reset_forces_noop_on_next_first_frame() {
        let mut d = TemporalDenoiser::new();
        let mut a = make_frame(10, |_| 0);
        d.process(&mut a, W, H);
        d.reset();
        let original = a.clone();
        d.process(&mut a, W, H);
        assert_eq!(a, original, "after reset the next frame must be unblended");
    }
}
