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
    /// Per-character blur amount at full selector amount (in px). Rendered value
    /// is `blur_amount * amount` per character.
    #[serde(default)]
    pub blur_amount: f32,
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
            blur_amount: 0.0,
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
    /// Per-character amount oscillates deterministically by character index.
    Wobble,
    /// Per-character amount is a deterministic pseudo-random value in [0, 1).
    Random,
    /// Per-character amount is driven by a Rhai expression. The expression
    /// receives `index` (0-based char index), `total` (char count), and
    /// `time` (seconds), and should return a value in 0..1.
    Expression,
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
    /// Inclusive character index range (0-based). -1 means unbounded on that side.
    #[serde(default = "default_unbounded")]
    pub char_start: i32,
    #[serde(default = "default_unbounded")]
    pub char_end: i32,
    /// When present, overrides `offset` each frame (for typewriter sweeps etc.).
    #[serde(default)]
    pub offset_anim: Option<crate::core::property::Animatable<f32>>,
    /// Rhai expression for `SelectorShape::Expression`. Receives `index`,
    /// `total`, and `time`; returns amount in 0..1.
    #[serde(default)]
    pub expression: Option<String>,
}

fn default_unbounded() -> i32 {
    -1
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
            char_start: -1,
            char_end: -1,
            offset_anim: None,
            expression: None,
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
    /// Blur amount in px for this character (0.0 = no blur).
    pub blur: f32,
}

impl Default for CharacterTransform {
    fn default() -> Self {
        Self {
            position_offset: [0.0, 0.0],
            scale_multiplier: [1.0, 1.0],
            opacity_multiplier: 1.0,
            tracking_offset: 0.0,
            rotation_deg: 0.0,
            blur: 0.0,
        }
    }
}

pub struct TextAnimatorEngine;

impl TextAnimatorEngine {
    /// Deterministic pseudo-random value in [0, 1) for a character index.
    fn hash_random(idx: usize, total_chars: usize) -> f32 {
        let mut h = (idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= total_chars as u64;
        h ^= h >> 33;
        h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        h ^= h >> 33;
        ((h >> 11) as f32 / (1u64 << 53) as f32).fract()
    }

    /// Remap a character index for `random_order` selection (deterministic shuffle).
    fn random_order_index(idx: usize, total_chars: usize) -> usize {
        // Multiplicative permutation: bijective when modulus is coprime with multiplier.
        ((idx as u64 + 1).wrapping_mul(2654435761) % total_chars as u64) as usize
    }

    fn in_char_range(char_idx: usize, selector: &RangeSelector) -> bool {
        let i = char_idx as i32;
        if selector.char_start >= 0 && i < selector.char_start {
            return false;
        }
        if selector.char_end >= 0 && i > selector.char_end {
            return false;
        }
        true
    }

    /// Evaluate a per-character Rhai expression for the expression selector.
    fn eval_expression_amount(expr_src: &str, char_idx: usize, total_chars: usize, time: f32) -> f32 {
        let v = crate::core::expression_engine::eval_expression_f64(expr_src, &[
            ("index", char_idx as f64),
            ("textIndex", (char_idx + 1) as f64), // AE standard: 1-based character index
            ("textTotal", total_chars as f64),    // AE standard: total character count
            ("total", total_chars as f64),
            ("time", time as f64),
            ("value", 1.0),
        ]);
        (v as f32).clamp(0.0, 1.0)
    }

    pub fn compute_amount(
        char_idx: usize,
        total_chars: usize,
        selector: &RangeSelector,
        time: f32,
    ) -> f32 {
        if total_chars == 0 || !Self::in_char_range(char_idx, selector) {
            return 0.0;
        }

        // Expression selector: evaluate per-character Rhai expression.
        if selector.shape == SelectorShape::Expression {
            if let Some(ref expr_src) = selector.expression {
                return Self::eval_expression_amount(expr_src, char_idx, total_chars, time);
            }
            return 0.0;
        }

        // For Wobble/Random shapes the amount is index-driven, not range-driven.
        match selector.shape {
            SelectorShape::Wobble => {
                return (char_idx as f32 * 1.7 + std::f32::consts::PI * 0.5).sin() * 0.5 + 0.5;
            }
            SelectorShape::Random => {
                return Self::hash_random(
                    if selector.random_order {
                        Self::random_order_index(char_idx, total_chars)
                    } else {
                        char_idx
                    },
                    total_chars,
                );
            }
            _ => {}
        }

        let sel_idx = if selector.random_order {
            Self::random_order_index(char_idx, total_chars)
        } else {
            char_idx
        };

        let char_pct = (sel_idx as f32 / total_chars as f32) * 100.0;
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
            SelectorShape::Expression => unreachable!("handled above"),
            SelectorShape::Wobble | SelectorShape::Random => 1.0,
        }
    }

    /// Evaluate per-character transforms for a text string given target animator property offsets.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_character_transforms(
        text: &str,
        selector: &RangeSelector,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
        time: f32,
    ) -> Vec<CharacterTransform> {
        Self::eval_character_transforms_extended(
            text,
            selector,
            target_position,
            target_scale,
            target_opacity,
            target_tracking,
            target_rotation,
            0.0,
            false,
            time,
        )
    }

    /// Extended evaluation with per-character blur amount and optional cumulative
    /// (AE-style) tracking, where each character's tracking offset includes the
    /// accumulated offsets of all preceding characters in the selection.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_character_transforms_extended(
        text: &str,
        selector: &RangeSelector,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
        target_blur: f32,
        cumulative_tracking: bool,
        time: f32,
    ) -> Vec<CharacterTransform> {
        let total_chars = text.chars().count();
        let amounts: Vec<f32> = (0..total_chars)
            .map(|idx| Self::compute_amount(idx, total_chars, selector, time))
            .collect();
        Self::eval_with_amounts(
            amounts,
            target_position,
            target_scale,
            target_opacity,
            target_tracking,
            target_rotation,
            target_blur,
            cumulative_tracking,
        )
    }

    /// Core interpolation from precomputed per-character selector amounts
    /// (each in 0..1) to transforms. Public so unit-based selectors
    /// (words/lines) can drive the same math.
    #[allow(clippy::too_many_arguments)]
    pub fn eval_with_amounts(
        amounts: Vec<f32>,
        target_position: [f32; 2],
        target_scale: [f32; 2],
        target_opacity: f32,
        target_tracking: f32,
        target_rotation: f32,
        target_blur: f32,
        cumulative_tracking: bool,
    ) -> Vec<CharacterTransform> {
        let total_chars = amounts.len();
        let mut transforms = Vec::with_capacity(total_chars);
        let mut tracking_accum = 0.0;

        for amount in amounts.iter().copied() {
            let pos_x = target_position[0] * amount;
            let pos_y = target_position[1] * amount;
            let scale_x = 1.0 + (target_scale[0] - 1.0) * amount;
            let scale_y = 1.0 + (target_scale[1] - 1.0) * amount;
            let opa = 1.0 - (1.0 - target_opacity) * amount;
            let track = target_tracking * amount + if cumulative_tracking { tracking_accum } else { 0.0 };
            tracking_accum += target_tracking * amount;
            let rot = target_rotation * amount;
            let blur = target_blur.max(0.0) * amount;

            transforms.push(CharacterTransform {
                position_offset: [pos_x, pos_y],
                scale_multiplier: [scale_x, scale_y],
                opacity_multiplier: opa,
                tracking_offset: track,
                rotation_deg: rot,
                blur,
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

        let amt_first = TextAnimatorEngine::compute_amount(0, 10, &selector, 0.0);
        let amt_mid = TextAnimatorEngine::compute_amount(5, 10, &selector, 0.0);
        let amt_last = TextAnimatorEngine::compute_amount(9, 10, &selector, 0.0);

        assert_eq!(amt_first, 0.0);
        assert!((amt_mid - 0.5).abs() < 1e-4);
        assert!((amt_last - 0.9).abs() < 1e-4);
    }

    #[test]
    fn test_char_range_selection() {
        let mut selector = RangeSelector {
            shape: SelectorShape::Square,
            char_start: 2,
            char_end: 4,
            ..Default::default()
        };

        let amounts: Vec<f32> = (0..6)
            .map(|i| TextAnimatorEngine::compute_amount(i, 6, &selector, 0.0))
            .collect();

        assert_eq!(amounts[0], 0.0);
        assert_eq!(amounts[1], 0.0);
        assert_eq!(amounts[2], 1.0);
        assert_eq!(amounts[3], 1.0);
        assert_eq!(amounts[4], 1.0);
        assert_eq!(amounts[5], 0.0);

        // char_start only (open-ended end)
        selector.char_end = -1;
        assert_eq!(TextAnimatorEngine::compute_amount(5, 6, &selector, 0.0), 1.0);
    }

    #[test]
    fn test_wobble_shape_bounded_and_deterministic() {
        let selector = RangeSelector { shape: SelectorShape::Wobble, ..Default::default() };

        for i in 0..20 {
            let a = TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0);
            assert!((0.0..=1.0).contains(&a), "amount {a} out of range at idx {i}");
            assert_eq!(a, TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0));
        }
    }

    #[test]
    fn test_random_shape_deterministic() {
        let selector = RangeSelector { shape: SelectorShape::Random, ..Default::default() };

        for i in 0..20 {
            let a = TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0);
            assert!((0.0..1.0).contains(&a));
            assert_eq!(a, TextAnimatorEngine::compute_amount(i, 20, &selector, 0.0));
        }
    }

    #[test]
    fn test_random_order_is_bijective_permutation() {
        let selector = RangeSelector {
            random_order: true,
            ..Default::default()
        };

        // Square shape with random_order: every char gets amount 1 exactly once.
        let ones = (0..10)
            .filter(|&i| TextAnimatorEngine::compute_amount(i, 10, &selector, 0.0) == 1.0)
            .count();
        assert_eq!(ones, 10);
    }

    #[test]
    fn test_blur_evaluation() {
        let transforms = TextAnimatorEngine::eval_character_transforms_extended(
            "abcd",
            &RangeSelector {
                shape: SelectorShape::RampUp,
                ..Default::default()
            },
            [0.0, 0.0],
            [1.0, 1.0],
            1.0,
            0.0,
            0.0,
            8.0,
            false,
            0.0,
        );

        assert_eq!(transforms.len(), 4);
        assert_eq!(transforms[0].blur, 0.0);
        assert!((transforms[3].blur - 6.0).abs() < 1e-4); // 8.0 * 0.75

        // Legacy API yields zero blur.
        let legacy = TextAnimatorEngine::eval_character_transforms(
            "abcd",
            &RangeSelector::default(),
            [0.0, 0.0],
            [1.0, 1.0],
            1.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(legacy.iter().all(|t| t.blur == 0.0));
    }

    #[test]
    fn test_cumulative_tracking() {
        let tracking = 2.0;
        let transforms = TextAnimatorEngine::eval_character_transforms_extended(
            "abc",
            &RangeSelector::default(), // Square, all chars amount = 1
            [0.0, 0.0],
            [1.0, 1.0],
            1.0,
            tracking,
            0.0,
            0.0,
            true,
            0.0,
        );

        // AE-style accumulation: offsets are 2, 4, 6 instead of flat 2 each.
        assert!((transforms[0].tracking_offset - 2.0).abs() < 1e-4);
        assert!((transforms[1].tracking_offset - 4.0).abs() < 1e-4);
        assert!((transforms[2].tracking_offset - 6.0).abs() < 1e-4);
    }

    #[test]
    fn test_serde_backward_compat_without_new_fields() {
        // Old project JSON lacking blur_amount / char range fields must deserialize.
        let json = r#"{
            "enabled": true,
            "selector": {"start": 0.0, "end": 100.0, "offset": 0.0,
                         "shape": "Square", "ease_high": 0.0, "ease_low": 0.0,
                         "random_order": false},
            "position_offset": [0.0, -40.0],
            "scale": [1.0, 1.0],
            "opacity": 0.0,
            "tracking": 0.0,
            "rotation": 0.0
        }"#;
        let settings: TextAnimatorSettings = serde_json::from_str(json).unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.blur_amount, 0.0);
        assert_eq!(settings.selector.char_start, -1);
        assert_eq!(settings.selector.char_end, -1);
    }

    #[test]
    fn test_expression_selector_basic() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("index / total".into()),
            ..Default::default()
        };
        let a = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        let b = TextAnimatorEngine::compute_amount(5, 10, &sel, 0.0);
        let c = TextAnimatorEngine::compute_amount(9, 10, &sel, 0.0);
        assert!((a - 0.0).abs() < 0.01, "idx=0: {}", a);
        assert!((b - 0.5).abs() < 0.01, "idx=5: {}", b);
        assert!((c - 0.9).abs() < 0.01, "idx=9: {}", c);
    }

    #[test]
    fn test_expression_selector_uses_time() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("if time > 0.5 { 1.0 } else { 0.0 }".into()),
            ..Default::default()
        };
        let at_0 = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        let at_1 = TextAnimatorEngine::compute_amount(0, 10, &sel, 1.0);
        assert!((at_0 - 0.0).abs() < 0.01);
        assert!((at_1 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_expression_selector_clamped_01() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("2.0".into()),
            ..Default::default()
        };
        let v = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        assert!((v - 1.0).abs() < 0.01, "should clamp: {}", v);
    }

    #[test]
    fn test_expression_selector_no_expr_is_zero() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: None,
            ..Default::default()
        };
        let v = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0);
        assert!((v - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_expression_selector_text_index_text_total() {
        let sel = RangeSelector {
            shape: SelectorShape::Expression,
            expression: Some("textIndex / textTotal".into()),
            ..Default::default()
        };
        let a = TextAnimatorEngine::compute_amount(0, 10, &sel, 0.0); // textIndex=1, textTotal=10 -> 0.1
        let b = TextAnimatorEngine::compute_amount(9, 10, &sel, 0.0); // textIndex=10, textTotal=10 -> 1.0
        assert!((a - 0.1).abs() < 0.01, "a={}", a);
        assert!((b - 1.0).abs() < 0.01, "b={}", b);
    }

}
