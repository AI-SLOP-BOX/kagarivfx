#![allow(dead_code)]
/// Line Cap styles for Shape Stroke matching After Effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCapStyle {
    Butt,
    Round,
    Square,
}

/// Line Join styles for Shape Stroke matching After Effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoinStyle {
    Miter,
    Round,
    Bevel,
}

/// Shape Stroke Modifier configuration.
#[derive(Debug, Clone)]
pub struct StrokeModifierOptions {
    pub width: f32,
    pub line_cap: LineCapStyle,
    pub line_join: LineJoinStyle,
    pub miter_limit: f32,
    pub dash_pattern: Vec<f32>, // [dash_len, gap_len, ...]
    pub dash_offset: f32,
}

impl Default for StrokeModifierOptions {
    fn default() -> Self {
        Self {
            width: 2.0,
            line_cap: LineCapStyle::Butt,
            line_join: LineJoinStyle::Miter,
            miter_limit: 4.0,
            dash_pattern: Vec::new(),
            dash_offset: 0.0,
        }
    }
}

/// Evaluates dash segment state (solid stroke vs gap) at distance `d` along path.
pub fn is_dash_solid_at_distance(distance: f32, options: &StrokeModifierOptions) -> bool {
    if options.dash_pattern.is_empty() {
        return true;
    }

    let pattern_total: f32 = options.dash_pattern.iter().sum();
    if pattern_total <= 0.001 {
        return true;
    }

    let d_wrapped = (distance + options.dash_offset).rem_euclid(pattern_total);
    let mut accumulated = 0.0f32;

    for (idx, &len) in options.dash_pattern.iter().enumerate() {
        accumulated += len;
        if d_wrapped < accumulated {
            return idx % 2 == 0; // Even index = Solid Dash, Odd index = Gap
        }
    }

    true
}

/// Evaluates Miter Join offset vertex for two joining segment normal vectors, enforcing Miter Limit clamping.
pub fn calculate_miter_join_offset(
    n0: [f32; 2],
    n1: [f32; 2],
    half_width: f32,
    miter_limit: f32,
) -> ([f32; 2], LineJoinStyle) {
    let miter_vec = [n0[0] + n1[0], n0[1] + n1[1]];
    let len_sq = miter_vec[0] * miter_vec[0] + miter_vec[1] * miter_vec[1];

    if len_sq < 0.0001 {
        return (
            [n0[0] * half_width, n0[1] * half_width],
            LineJoinStyle::Bevel,
        );
    }

    let miter_len = len_sq.sqrt();
    let miter_dir = [miter_vec[0] / miter_len, miter_vec[1] / miter_len];

    // Miter length ratio = 2.0 / (1.0 + dot(n0, n1))
    let dot = n0[0] * n1[0] + n0[1] * n1[1];
    let miter_ratio = 2.0 / (1.0 + dot).max(0.001);

    if miter_ratio > miter_limit * miter_limit {
        // Fall back to Bevel join if Miter Limit exceeded
        (
            [n0[0] * half_width, n0[1] * half_width],
            LineJoinStyle::Bevel,
        )
    } else {
        let dist = half_width * (miter_ratio.sqrt());
        (
            [miter_dir[0] * dist, miter_dir[1] * dist],
            LineJoinStyle::Miter,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dash_solid_alternation() {
        let options = StrokeModifierOptions {
            width: 2.0,
            line_cap: LineCapStyle::Butt,
            line_join: LineJoinStyle::Miter,
            miter_limit: 4.0,
            dash_pattern: vec![10.0, 5.0], // 10px dash, 5px gap
            dash_offset: 0.0,
        };

        assert!(is_dash_solid_at_distance(2.0, &options)); // Solid
        assert!(!is_dash_solid_at_distance(12.0, &options)); // Gap
        assert!(is_dash_solid_at_distance(17.0, &options)); // Solid (next cycle)
    }

    #[test]
    fn test_miter_join_limit_clamping() {
        let n0 = [1.0, 0.0];
        let n1 = [0.0, 1.0]; // 90 degree sharp corner
        let (offset, style) = calculate_miter_join_offset(n0, n1, 5.0, 4.0);

        assert_eq!(style, LineJoinStyle::Miter);
        assert!((offset[0] - 5.0).abs() < 1.0);
    }
}
