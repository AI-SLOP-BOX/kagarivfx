use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HorizontalPin {
    #[default]
    Left,
    Center,
    Right,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VerticalPin {
    #[default]
    Top,
    Center,
    Bottom,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LayerConstraints {
    pub horizontal: HorizontalPin,
    pub vertical: VerticalPin,
}

impl LayerConstraints {
    pub fn new(horizontal: HorizontalPin, vertical: VerticalPin) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    /// Calculate the repositioned [x, y] coordinates when the composition is resized
    /// from (old_w, old_h) to (new_w, new_h).
    pub fn remap_position(
        &self,
        current_pos: [f32; 2],
        old_w: f32,
        old_h: f32,
        new_w: f32,
        new_h: f32,
    ) -> [f32; 2] {
        if old_w <= 0.0 || old_h <= 0.0 || new_w <= 0.0 || new_h <= 0.0 {
            return current_pos;
        }

        let new_x = match self.horizontal {
            HorizontalPin::Left => current_pos[0],
            HorizontalPin::Right => new_w - (old_w - current_pos[0]),
            HorizontalPin::Center => current_pos[0] + (new_w - old_w) * 0.5,
            HorizontalPin::Scale => current_pos[0] * (new_w / old_w),
        };

        let new_y = match self.vertical {
            VerticalPin::Top => current_pos[1],
            VerticalPin::Bottom => new_h - (old_h - current_pos[1]),
            VerticalPin::Center => current_pos[1] + (new_h - old_h) * 0.5,
            VerticalPin::Scale => current_pos[1] * (new_h / old_h),
        };

        [new_x, new_y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_constraints_remap() {
        let pin_right = LayerConstraints::new(HorizontalPin::Right, VerticalPin::Bottom);
        // Resizing from 1920x1080 to 3840x2160 (4K)
        // Pos [1800, 1000] is 120px from right and 80px from bottom.
        let new_pos = pin_right.remap_position([1800.0, 1000.0], 1920.0, 1080.0, 3840.0, 2160.0);
        assert_eq!(new_pos[0], 3840.0 - 120.0);
        assert_eq!(new_pos[1], 2160.0 - 80.0);

        let pin_center = LayerConstraints::new(HorizontalPin::Center, VerticalPin::Center);
        let center_pos = pin_center.remap_position([960.0, 540.0], 1920.0, 1080.0, 3840.0, 2160.0);
        assert_eq!(center_pos[0], 1920.0);
        assert_eq!(center_pos[1], 1080.0);
    }
}
