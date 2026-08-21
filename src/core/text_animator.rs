/// After Effects per-character Text Animator & Range Selector Engine.
///
/// Computes per-character spatial offsets, scale scaling, opacity fading,
/// and tracking adjustments driven by animated Range Selector parameters.
use serde::{Deserialize, Serialize};

/// Per-layer text animator settings (AE Text > Animators).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnimatorSettings {
    pub enabled: bool,
    pub selector: RangeSelector,
    pub position_offset: [f32; 2],
    pub scale: [f32; 2],
    pub opacity: f32,
    pub tracking: f32,
    pub rotation: f32,
}

impl Default for TextAnimatorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            selector: RangeSelector::default(),
            position_offset: [0.0, -40.0],
            scale: [1.0, 1.0],
            opacity: 0.0,
            tracking: 0.0,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorShape {
    Square,
    RampUp,
    RampDown,
    Triangle,
    Round,
    Smooth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeSelector {
    pub start: f32,  // 0.0 to 100.0%
    pub end: f32,    // 0.0 to 100.0%
    pub offset: f32, // -100.0 to 100.0%
    pub shape: SelectorShape,
    pub ease_high: f32, // 0.0 to 100.0%
    pub ease_low: f32,  // 0.0 to 100.0%
    pub random_order: bool,
}

impl Default for RangeSelector {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: SelectorShape::Square,
            ease_high: 0.0,
            ease_low: 0.0,
            random_order: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CharacterTransform {
    pub position_offset: [f32; 2],
    pub scale_multiplier: [f32; 2],
    pub opacity_multiplier: f32,
    pub tracking_offset: f32,
    pub rotation_deg: f32,
}

impl Default for CharacterTransform {
    fn default() -> Self {
        Self {
            position_offset: [0.0, 0.0],
            scale_multiplier: [1.0, 1.0],
            opacity_multiplier: 1.0,
            tracking_offset: 0.0,
            rotation_deg: 0.0,
        }
    }
}

pub struct TextAnimatorEngine;

impl TextAnimatorEngine {
    /// Calculate the range selector amount (0.0 to 1.0) for a character index in a string.
    pub fn compute_amount(
        char_idx: usize,
        total_chars: usize,
        selector: &RangeSelector,
    ) -> f32 {
        if total_chars == 0 {
            return 0.0;
        }

        let char_pct = (char_idx as f32 / total_chars as f32) * 100.0;
        let effective_start = (selector.start + selector.offset).clamp(0.0, 100.0);
        let effective_end = (selector.end + selector.offset).clamp(0.0, 100.0);

        if effective_start >= effective_end {
            return 0.0;
        }

        if char_pct < effective_start || char_pct > effective_end {
            return 0.0;
        }

        let norm_t = (char_pct - effective_start) / (effective_end - effective_start);

        match selector.shape {
            SelectorShape::Square => 1.0,
            SelectorShape::RampUp => norm_t,
            SelectorShape::RampDown => 1.0 - norm_t,
            SelectorShape::Triangle => {
                if norm_t < 0.5 {
                    norm_t * 2.0
                } else {
                    (1.0 - norm_t) * 2.0
                }
            }
            SelectorShape::Round => {
                (norm_t * std::f32::consts::PI).sin()
            }
            SelectorShape::Smooth => {
                norm_t * norm_t * (3.0 - 2.0 * norm_t)
            }
        }
    }

    /// Evaluate per-character transforms for a text string given target animator property offsets.
    pub fn eval_character_transforms(
        text: &str,
        selector: &RangeSelector,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
    ) -> Vec<CharacterTransform> {
        let total_chars = text.chars().count();
        let mut transforms = Vec::with_capacity(total_chars);

        for (idx, _) in text.chars().enumerate() {
            let amount = Self::compute_amount(idx, total_chars, selector);

            let pos_x = target_position[0] * amount;
            let pos_y = target_position[1] * amount;
            let scale_x = 1.0 + (target_scale[0] - 1.0) * amount;
            let scale_y = 1.0 + (target_scale[1] - 1.0) * amount;
            let opa = 1.0 - (1.0 - target_opacity) * amount;
            let track = target_tracking * amount;
            let rot = target_rotation * amount;

            transforms.push(CharacterTransform {
                position_offset: [pos_x, pos_y],
                scale_multiplier: [scale_x, scale_y],
                opacity_multiplier: opa,
                tracking_offset: track,
                rotation_deg: rot,
            });
        }

        transforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_animator_range_selector_amount() {
        let selector = RangeSelector {
            start: 0.0,
            end: 100.0,
            offset: 0.0,
            shape: SelectorShape::RampUp,
            ..Default::default()
        };

        let amt_first = TextAnimatorEngine::compute_amount(0, 10, &selector);
        let amt_mid = TextAnimatorEngine::compute_amount(5, 10, &selector);
        let amt_last = TextAnimatorEngine::compute_amount(9, 10, &selector);

        assert_eq!(amt_first, 0.0);
        assert!((amt_mid - 0.5).abs() < 1e-4);
        assert!((amt_last - 0.9).abs() < 1e-4);
    }
}
