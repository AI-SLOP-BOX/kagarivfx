#![allow(dead_code)]
//! Advanced Text Animator features completing the AE per-character model.
//!
//! Adds on top of [`crate::core::text_animator`]:
//!   * "Based On" selection units — Characters / Words / Lines
//!   * Fill & stroke color animation with mix factors
//!   * Skew / skew axis and anchor point offsets
//!   * Character offset (codepoint shifting)
//!   * Multi-animator stacking (`AnimatorStack::compose`) matching AE's
//!     sequential animator evaluation order

use serde::{Deserialize, Serialize};

use crate::core::text_animator::{CharacterTransform, RangeSelector, TextAnimatorEngine};

/// "Based On" granularity of the range selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectorUnit {
    #[default]
    Characters,
    Words,
    Lines,
}

/// Advanced animator properties (AE Animator > Advanced / color groups).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancedProperties {
    /// Anchor point offset at full amount (px).
    pub anchor_point: [f32; 2],
    /// Skew in degrees at full amount.
    pub skew: f32,
    /// Skew axis in degrees.
    pub skew_axis: f32,
    /// Target fill color; `None` = do not animate fill.
    pub fill_color: Option<[f32; 4]>,
    /// Target stroke color; `None` = do not animate stroke.
    pub stroke_color: Option<[f32; 4]>,
    /// Stroke width delta (px) added at full amount.
    pub stroke_width: f32,
    /// Character offset applied to codepoints at full amount.
    pub character_offset: i32,
}

impl Default for AdvancedProperties {
    fn default() -> Self {
        Self {
            anchor_point: [0.0, 0.0],
            skew: 0.0,
            skew_axis: 0.0,
            fill_color: None,
            stroke_color: None,
            stroke_width: 0.0,
            character_offset: 0,
        }
    }
}

/// One fully-specified advanced animator (selector + targets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnimatorAdvanced {
    pub enabled: bool,
    pub selector: RangeSelector,
    #[serde(default)]
    pub unit: SelectorUnit,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    /// Target opacity multiplier at full amount (1.0 = unchanged).
    pub opacity: f32,
    pub tracking: f32,
    pub rotation: f32,
    pub blur: f32,
    #[serde(default)]
    pub cumulative_tracking: bool,
    #[serde(default)]
    pub advanced: AdvancedProperties,
}

impl Default for TextAnimatorAdvanced {
    fn default() -> Self {
        Self {
            enabled: true,
            selector: RangeSelector::default(),
            unit: SelectorUnit::Characters,
            position: [0.0, 0.0],
            scale: [1.0, 1.0],
            opacity: 1.0,
            tracking: 0.0,
            rotation: 0.0,
            blur: 0.0,
            cumulative_tracking: false,
            advanced: AdvancedProperties::default(),
        }
    }
}

/// Per-character result including the advanced channels.
#[derive(Debug, Clone, Copy)]
pub struct AdvancedCharacterTransform {
    pub base: CharacterTransform,
    pub anchor_offset: [f32; 2],
    pub skew_deg: f32,
    pub skew_axis_deg: f32,
    /// Target fill color for this char (already resolved by the stack).
    pub fill_color: Option<[f32; 4]>,
    /// How strongly the fill target applies (0..1).
    pub fill_mix: f32,
    pub stroke_color: Option<[f32; 4]>,
    pub stroke_mix: f32,
    /// Additive stroke width for this char (px).
    pub stroke_width_add: f32,
    /// Codepoint shift for this char.
    pub character_offset: i32,
}

impl Default for AdvancedCharacterTransform {
    fn default() -> Self {
        Self {
            base: CharacterTransform::default(),
            anchor_offset: [0.0, 0.0],
            skew_deg: 0.0,
            skew_axis_deg: 0.0,
            fill_color: None,
            fill_mix: 0.0,
            stroke_color: None,
            stroke_mix: 0.0,
            stroke_width_add: 0.0,
            character_offset: 0,
        }
    }
}

/// Shift a character's codepoint deterministically, falling back to the
/// original char when the result is not a valid scalar.
pub fn shift_character(c: char, offset: i32) -> char {
    if offset == 0 {
        return c;
    }
    let base = c as u32;
    let shifted = if offset > 0 {
        base.saturating_add(offset as u32)
    } else {
        base.saturating_sub(offset.unsigned_abs())
    };
    char::from_u32(shifted).unwrap_or(c)
}

/// Compute inclusive-exclusive char ranges for each selection unit.
/// Characters not covered by any range (whitespace separators) receive
/// a zero amount.
fn unit_ranges(text: &str, unit: SelectorUnit) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut ranges = Vec::new();
    match unit {
        SelectorUnit::Characters => {
            for i in 0..chars.len() {
                ranges.push((i, i + 1));
            }
        }
        SelectorUnit::Words => {
            let mut start: Option<usize> = None;
            for (i, c) in chars.iter().enumerate() {
                if c.is_whitespace() {
                    if let Some(s) = start.take() {
                        ranges.push((s, i));
                    }
                } else if start.is_none() {
                    start = Some(i);
                }
            }
            if let Some(s) = start {
                ranges.push((s, chars.len()));
            }
        }
        SelectorUnit::Lines => {
            let mut start: Option<usize> = None;
            for (i, c) in chars.iter().enumerate() {
                if *c == '\n' {
                    if let Some(s) = start.take() {
                        ranges.push((s, i));
                    }
                } else if start.is_none() {
                    start = Some(i);
                }
            }
            if let Some(s) = start {
                ranges.push((s, chars.len()));
            }
        }
    }
    ranges
}

impl TextAnimatorAdvanced {
    /// Per-character selector amounts honoring the unit granularity.
    pub fn compute_amounts(&self, text: &str) -> Vec<f32> {
        let total_chars = text.chars().count();
        let ranges = unit_ranges(text, self.unit);
        let total_units = ranges.len();
        let mut amounts = vec![0.0f32; total_chars];
        if total_units == 0 {
            return amounts;
        }
        for (unit_idx, (s, e)) in ranges.iter().enumerate() {
            let amt = TextAnimatorEngine::compute_amount(unit_idx, total_units, &self.selector, 0.0);
            for slot in amounts.iter_mut().take(*e).skip(*s) {
                *slot = amt;
            }
        }
        amounts
    }

    /// Evaluate this single animator over `text`.
    pub fn eval(&self, text: &str) -> Vec<AdvancedCharacterTransform> {
        let amounts = self.compute_amounts(text);
        let bases = TextAnimatorEngine::eval_with_amounts(
            amounts.clone(),
            self.position,
            self.scale,
            self.opacity,
            self.tracking,
            self.rotation,
            self.blur,
            self.cumulative_tracking,
        );
        let a = &self.advanced;
        bases
            .into_iter()
            .zip(amounts)
            .map(|(base, amount)| AdvancedCharacterTransform {
                anchor_offset: [a.anchor_point[0] * amount, a.anchor_point[1] * amount],
                skew_deg: a.skew * amount,
                skew_axis_deg: a.skew_axis * amount,
                fill_color: a.fill_color,
                fill_mix: if a.fill_color.is_some() { amount } else { 0.0 },
                stroke_color: a.stroke_color,
                stroke_mix: if a.stroke_color.is_some() { amount } else { 0.0 },
                stroke_width_add: a.stroke_width * amount,
                character_offset: (a.character_offset as f32 * amount).round() as i32,
                base,
            })
            .collect()
    }
}

/// Ordered stack of animators, composed AE-style (later animators layer on top).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimatorStack {
    pub animators: Vec<TextAnimatorAdvanced>,
}

impl AnimatorStack {
    /// Compose all enabled animators into one transform per character.
    ///
    /// Combination rules:
    ///   * position / tracking / rotation / blur / skew / anchor / stroke width /
    ///     character offset: additive
    ///   * scale: multiplicative deltas, opacity: multiplicative
    ///   * colors: the last animator providing a target wins (with its own mix)
    pub fn compose(&self, text: &str) -> Vec<AdvancedCharacterTransform> {
        let n = text.chars().count();
        let mut out = vec![AdvancedCharacterTransform::default(); n];
        for animator in &self.animators {
            if !animator.enabled {
                continue;
            }
            let per = animator.eval(text);
            for (slot, t) in out.iter_mut().zip(per) {
                slot.base.position_offset[0] += t.base.position_offset[0];
                slot.base.position_offset[1] += t.base.position_offset[1];
                slot.base.scale_multiplier[0] *= t.base.scale_multiplier[0];
                slot.base.scale_multiplier[1] *= t.base.scale_multiplier[1];
                slot.base.opacity_multiplier *= t.base.opacity_multiplier;
                slot.base.tracking_offset += t.base.tracking_offset;
                slot.base.rotation_deg += t.base.rotation_deg;
                slot.base.blur += t.base.blur;
                slot.anchor_offset[0] += t.anchor_offset[0];
                slot.anchor_offset[1] += t.anchor_offset[1];
                slot.skew_deg += t.skew_deg;
                slot.skew_axis_deg += t.skew_axis_deg;
                slot.stroke_width_add += t.stroke_width_add;
                slot.character_offset += t.character_offset;
                if t.fill_color.is_some() {
                    slot.fill_color = t.fill_color;
                    slot.fill_mix = t.fill_mix;
                }
                if t.stroke_color.is_some() {
                    slot.stroke_color = t.stroke_color;
                    slot.stroke_mix = t.stroke_mix;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::text_animator::SelectorShape;

    fn square(start: f32, end: f32) -> RangeSelector {
        RangeSelector { shape: SelectorShape::Square, start, end, ..Default::default() }
    }

    #[test]
    fn test_characters_unit_matches_engine() {
        let anim = TextAnimatorAdvanced {
            selector: square(0.0, 100.0),
            ..Default::default()
        };
        let amounts = anim.compute_amounts("hello");
        assert_eq!(amounts.len(), 5);
        assert!(amounts.iter().all(|&a| a == 1.0));
    }

    #[test]
    fn test_words_unit_selects_whole_words() {
        // 3 words => unit pcts are 0%, 33.3%, 66.7%.
        let anim = TextAnimatorAdvanced {
            selector: square(0.0, 50.0),
            unit: SelectorUnit::Words,
            ..Default::default()
        };
        let amounts = anim.compute_amounts("alpha beta gamma");
        assert_eq!(amounts.len(), 16);
        // Word 0 ("alpha", chars 0..5): selected.
        assert!(amounts[0..5].iter().all(|&a| a == 1.0));
        // Space between word 0 and 1: separator gets 0.
        assert_eq!(amounts[5], 0.0);
        // Word 1 ("beta", chars 6..10): pct 33.3 <= 50 → selected.
        assert!(amounts[6..10].iter().all(|&a| a == 1.0));
        // Word 2 ("gamma"): pct 66.7 > 50 → excluded.
        assert!(amounts[11..16].iter().all(|&a| a == 0.0));
    }

    #[test]
    fn test_lines_unit_selects_per_line() {
        let anim = TextAnimatorAdvanced {
            selector: square(33.0, 100.0), // excludes line 0 (pct 0); line 1 sits at 33.3%
            unit: SelectorUnit::Lines,
            ..Default::default()
        };
        let amounts = anim.compute_amounts("one\ntwo\nthree");
        // Line 0 excluded.
        assert!(amounts[0..3].iter().all(|&a| a == 0.0));
        // Newline separator excluded.
        assert_eq!(amounts[3], 0.0);
        // Lines 1 and 2 included.
        assert!(amounts[4..7].iter().all(|&a| a == 1.0));
        assert!(amounts[8..13].iter().all(|&a| a == 1.0));
    }

    #[test]
    fn test_eval_applies_advanced_channels_by_amount() {
        let mut anim = TextAnimatorAdvanced {
            selector: RangeSelector { shape: SelectorShape::RampUp, ..Default::default() },
            ..Default::default()
        };
        anim.advanced.fill_color = Some([1.0, 0.0, 0.0, 1.0]);
        anim.advanced.skew = 20.0;
        anim.advanced.character_offset = 4;
        let out = anim.eval("abcd");
        assert_eq!(out.len(), 4);
        assert!((out[0].fill_mix - 0.0).abs() < 1e-5);
        assert!((out[3].fill_mix - 0.75).abs() < 1e-4);
        assert_eq!(out[3].fill_color, Some([1.0, 0.0, 0.0, 1.0]));
        assert!((out[3].skew_deg - 15.0).abs() < 1e-4); // 20 * 0.75
        assert_eq!(out[3].character_offset, 3); // round(4 * 0.75)
    }

    #[test]
    fn test_stack_compose_additive_and_override() {
        let mut first = TextAnimatorAdvanced {
            selector: square(0.0, 100.0),
            ..Default::default()
        };
        first.position = [10.0, 0.0];
        first.opacity = 0.5;
        first.advanced.fill_color = Some([1.0, 1.0, 1.0, 1.0]);

        let mut second = TextAnimatorAdvanced {
            selector: square(0.0, 100.0),
            ..Default::default()
        };
        second.position = [5.0, -5.0];
        second.advanced.fill_color = Some([0.0, 0.0, 1.0, 1.0]);

        let stack = AnimatorStack { animators: vec![first, second] };
        let composed = stack.compose("ab");
        assert_eq!(composed.len(), 2);
        let c = &composed[0];
        assert!((c.base.position_offset[0] - 15.0).abs() < 1e-5);
        assert!((c.base.position_offset[1] + 5.0).abs() < 1e-5);
        assert!((c.base.opacity_multiplier - 0.5).abs() < 1e-5);
        // Later animator's color wins.
        assert_eq!(c.fill_color, Some([0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_disabled_animators_are_skipped() {
        let mut anim = TextAnimatorAdvanced {
            selector: square(0.0, 100.0),
            ..Default::default()
        };
        anim.enabled = false;
        anim.position = [99.0, 99.0];
        let stack = AnimatorStack { animators: vec![anim] };
        let composed = stack.compose("xy");
        assert!(composed.iter().all(|t| t.base.position_offset == [0.0, 0.0]));
    }

    #[test]
    fn test_shift_character_deterministic_and_safe() {
        assert_eq!(shift_character('a', 0), 'a');
        assert_eq!(shift_character('a', 1), 'b');
        assert_eq!(shift_character('b', -1), 'a');
        // Invalid boundary falls back to the original char.
        assert_eq!(shift_character(char::MAX, 5), char::MAX);
        assert_eq!(shift_character('\u{0000}', -5), '\u{0000}');
    }

    #[test]
    fn test_serde_backward_compat() {
        let json = r#"{
            "enabled": true,
            "selector": {"start": 0.0, "end": 100.0, "offset": 0.0,
                         "shape": "Square", "ease_high": 0.0, "ease_low": 0.0,
                         "random_order": false},
            "position": [0.0, 0.0], "scale": [1.0, 1.0],
            "opacity": 1.0, "tracking": 0.0, "rotation": 0.0, "blur": 0.0
        }"#;
        let anim: TextAnimatorAdvanced = serde_json::from_str(json).unwrap_or_default();
        assert!(anim.enabled);
        assert_eq!(anim.unit, SelectorUnit::Characters);
        assert_eq!(anim.advanced, AdvancedProperties::default());
    }

    #[test]
    fn test_empty_text_is_safe() {
        let anim = TextAnimatorAdvanced::default();
        assert!(anim.eval("").is_empty());
        assert!(AnimatorStack::default().compose("").is_empty());
    }
}