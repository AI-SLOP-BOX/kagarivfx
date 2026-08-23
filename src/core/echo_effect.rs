#![allow(dead_code)]
/// Echo Blend Modes matching After Effects Echo effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoOperator {
    Add,
    Screen,
    Maximum,
    Minimum,
    CompositeBehind,
}

/// Options for After Effects Echo effect.
#[derive(Debug, Clone)]
pub struct EchoOptions {
    pub echo_time_sec: f32,    // Time offset per echo in seconds (e.g. -0.033)
    pub num_echoes: u32,        // Number of echoes (e.g. 5)
    pub starting_intensity: f32, // Initial echo opacity (0.0 .. 1.0)
    pub decay: f32,             // Decay factor per echo (e.g. 0.8)
    pub operator: EchoOperator,
}

impl Default for EchoOptions {
    fn default() -> Self {
        Self {
            echo_time_sec: -0.033,
            num_echoes: 3,
            starting_intensity: 1.0,
            decay: 0.8,
            operator: EchoOperator::Add,
        }
    }
}

/// Combines a history of rendered RGBA pixel buffers over time to generate motion echoes.
///
/// RGB channels are blended with the selected operator; the alpha channel is
/// composited so echoes never reduce the current frame's opacity. For
/// [`EchoOperator::CompositeBehind`] a full "under" composite is performed:
/// each echo shows through wherever the accumulated frame is transparent,
/// weighted by its own decayed alpha.
pub fn apply_echo_effect(
    current_pixels: &[u8],
    echo_history: &[Vec<u8>], // Slice of past/future frame buffers ordered by time
    width: u32,
    height: u32,
    options: &EchoOptions,
) -> Vec<u8> {
    let num_bytes = crate::core::software_renderer::rgba_buffer_size(width, height).unwrap_or(0);
    if current_pixels.len() != num_bytes || options.num_echoes == 0 {
        return current_pixels.to_vec();
    }

    let mut out_pixels = current_pixels.to_vec();

    // starting_intensity applies to the FIRST echo; subsequent echoes decay.
    let mut intensity = options.starting_intensity.clamp(0.0, 1.0);
    for history_buf in echo_history.iter().take(options.num_echoes as usize) {
        if history_buf.len() != num_bytes {
            continue;
        }

        for p in (0..num_bytes).step_by(4) {
            let cur_a = out_pixels[p + 3] as f32 / 255.0;
            let echo_a = (history_buf[p + 3] as f32 / 255.0) * intensity;

            match options.operator {
                EchoOperator::CompositeBehind => {
                    // Under-composite: echo visible only where frame is transparent.
                    let out_a = cur_a + echo_a * (1.0 - cur_a);
                    if out_a > 0.001 {
                        for c in 0..3 {
                            let cur_c = out_pixels[p + c] as f32 / 255.0;
                            let echo_c = history_buf[p + c] as f32 / 255.0;
                            let mixed =
                                (cur_c * cur_a + echo_c * echo_a * (1.0 - cur_a)) / out_a;
                            out_pixels[p + c] = (mixed.clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    }
                    out_pixels[p + 3] = (out_a.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                _ => {
                    // Additive-family operators act on premultiplied-style RGB;
                    // alpha only grows (an echo can never erase the present).
                    for c in 0..3 {
                        let base = out_pixels[p + c] as f32;
                        let echo_val = history_buf[p + c] as f32 * intensity;
                        let blended = match options.operator {
                            EchoOperator::Add => base + echo_val,
                            EchoOperator::Screen => {
                                255.0 - (255.0 - base) * (255.0 - echo_val) / 255.0
                            }
                            EchoOperator::Maximum => base.max(echo_val),
                            EchoOperator::Minimum => base.min(echo_val),
                            EchoOperator::CompositeBehind => unreachable!(),
                        };
                        out_pixels[p + c] = blended.round().clamp(0.0, 255.0) as u8;
                    }
                    let new_a = (cur_a.max(echo_a) * 255.0).round().clamp(0.0, 255.0) as u8;
                    out_pixels[p + 3] = new_a;
                }
            }
        }

        intensity *= options.decay;
    }

    out_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_effect_add() {
        let current = vec![50u8, 50, 50, 255, 50, 50, 50, 255, 50, 50, 50, 255, 50, 50, 50, 255];
        let history = vec![vec![100u8; 16]];
        let options = EchoOptions::default();

        let result = apply_echo_effect(&current, &history, 2, 2, &options);
        assert_eq!(result.len(), 16);
        assert!(result[0] > 50);
    }

    #[test]
    fn test_empty_history_is_identity() {
        let current = vec![7u8; 64];
        let result = apply_echo_effect(&current, &[], 4, 4, &EchoOptions::default());
        assert_eq!(result, current);
    }

    #[test]
    fn test_zero_echoes_is_identity() {
        let current = vec![9u8; 64];
        let history = vec![vec![200u8; 64]];
        let options = EchoOptions { num_echoes: 0, ..Default::default() };
        let result = apply_echo_effect(&current, &history, 4, 4, &options);
        assert_eq!(result, current);
    }

    #[test]
    fn test_decay_makes_later_echoes_weaker() {
        // Two identical history frames; the second must contribute less.
        let current = vec![0u8; 64];
        let frame = vec![200u8; 64];
        let history = vec![frame.clone(), frame.clone()];
        let options = EchoOptions {
            num_echoes: 2,
            starting_intensity: 1.0,
            decay: 0.5,
            operator: EchoOperator::Add,
            ..Default::default()
        };
        let one = apply_echo_effect(&current, std::slice::from_ref(&frame), 4, 4, &options);
        let two = apply_echo_effect(&current, &history, 4, 4, &options);
        // Second echo adds 200*0.5=100 → strictly brighter than one echo alone.
        assert!(two[0] > one[0], "second echo must still add light");
        assert!((two[0] as i32) < (one[0] as i32) + 110, "decayed echo adds less than the first");
    }

    #[test]
    fn test_add_operator_grows_alpha_monotonically() {
        let current = vec![10u8, 10, 10, 40]; // semi-transparent foreground
        let history = vec![vec![30u8, 30, 30, 120]];
        let options = EchoOptions { num_echoes: 1, ..Default::default() };
        let before_a = current[3];
        let result = apply_echo_effect(&current, &history, 1, 1, &options);
        assert!(result[3] >= before_a, "alpha must never shrink");
        assert!(result[3] >= 120, "echo alpha contributes");
    }

    #[test]
    fn test_composite_behind_shows_through_transparency() {
        // Current pixel fully transparent → echo shows at its decayed alpha.
        let current = vec![0u8, 0, 0, 0];
        let echo = vec![255u8, 0, 0, 128];
        let options = EchoOptions {
            num_echoes: 1,
            starting_intensity: 1.0,
            decay: 1.0,
            operator: EchoOperator::CompositeBehind,
            ..Default::default()
        };
        let result = apply_echo_effect(&current, &[echo], 1, 1, &options);
        assert_eq!(result[3], 128, "echo alpha fills transparent area");
        assert_eq!(result[0], 255, "echo color shows through");

        // Fully opaque current pixel → echo invisible.
        let opaque = vec![10u8, 20, 30, 255];
        let result2 = apply_echo_effect(&opaque, &[vec![255u8, 0, 0, 255]], 1, 1, &options);
        assert_eq!(&result2[..], &opaque[..], "opaque frame blocks echo entirely");
    }

    #[test]
    fn test_screen_operator_brightens_without_clipping_artifacts() {
        let current = vec![100u8, 100, 100, 255];
        let history = vec![vec![100u8, 100, 100, 255]];
        let options = EchoOptions {
            num_echoes: 1,
            starting_intensity: 1.0,
            decay: 1.0,
            operator: EchoOperator::Screen,
            ..Default::default()
        };
        let result = apply_echo_effect(&current, &history, 1, 1, &options);
        // Screen(100,100) ≈ 100 + 100 - 100*100/255 ≈ 160.8
        assert!(result[0] > 140 && result[0] < 180, "screen blend value {}", result[0]);
    }

    #[test]
    fn test_mismatched_history_buffers_are_skipped() {
        let current = vec![50u8; 64];
        let history = vec![vec![255u8; 32]]; // wrong size — must be ignored
        let options = EchoOptions { num_echoes: 1, ..Default::default() };
        let result = apply_echo_effect(&current, &history, 4, 4, &options);
        assert_eq!(result, current);
    }

    #[test]
    fn test_deterministic_output() {
        let current = gradientish();
        let history = vec![gradientish(), gradientish()];
        let options = EchoOptions { num_echoes: 2, ..Default::default() };
        let a = apply_echo_effect(&current, &history, 4, 4, &options);
        let b = apply_echo_effect(&current, &history, 4, 4, &options);
        assert_eq!(a, b);
    }

    fn gradientish() -> Vec<u8> {
        let mut v = Vec::with_capacity(64);
        for i in 0..16u32 {
            let c = (i * 17 % 256) as u8;
            v.extend_from_slice(&[c, 255 - c, c / 2, 255]);
        }
        v
    }
}
