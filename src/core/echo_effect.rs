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

    let mut intensity = options.starting_intensity;
    for (_idx, history_buf) in echo_history.iter().enumerate().take(options.num_echoes as usize) {
        if history_buf.len() != num_bytes {
            continue;
        }

        intensity *= options.decay;

        for p in 0..num_bytes {
            let base = out_pixels[p] as f32;
            let echo_val = history_buf[p] as f32 * intensity;

            let blended = match options.operator {
                EchoOperator::Add => base + echo_val,
                EchoOperator::Screen => 255.0 - (255.0 - base) * (255.0 - echo_val) / 255.0,
                EchoOperator::Maximum => base.max(echo_val),
                EchoOperator::Minimum => base.min(echo_val),
                EchoOperator::CompositeBehind => {
                    if p % 4 == 3 { base.max(echo_val) } else { base * 0.5 + echo_val * 0.5 }
                }
            };

            out_pixels[p] = blended.round().clamp(0.0, 255.0) as u8;
        }
    }

    out_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_effect_add() {
        let current = vec![50u8; 16];
        let history = vec![vec![100u8; 16]];
        let options = EchoOptions::default();

        let result = apply_echo_effect(&current, &history, 2, 2, &options);
        assert_eq!(result.len(), 16);
        assert!(result[0] > 50);
    }
}
