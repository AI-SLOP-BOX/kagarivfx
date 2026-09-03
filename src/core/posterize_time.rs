#![allow(dead_code)]
/// Posterize Time Engine: Quantizes frame evaluation timing to match target reduced frame rate (e.g. 12fps stop-motion).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PosterizeTimeSettings {
    pub target_fps: f32,
    pub enabled: bool,
}

impl Default for PosterizeTimeSettings {
    fn default() -> Self {
        Self {
            target_fps: 12.0,
            enabled: true,
        }
    }
}

/// Quantizes the active timeline frame number to match the specified posterize frame rate.
pub fn quantize_frame_posterize(
    current_frame: u32,
    comp_fps: u32,
    settings: &PosterizeTimeSettings,
) -> u32 {
    if !settings.enabled || settings.target_fps <= 0.0 || comp_fps == 0 {
        return current_frame;
    }

    let comp_fps_f = comp_fps as f32;
    if settings.target_fps >= comp_fps_f {
        return current_frame;
    }

    let _step_interval = (comp_fps_f / settings.target_fps).max(1.0);
    let current_sec = current_frame as f32 / comp_fps_f;
    let posterized_sec = (current_sec * settings.target_fps).floor() / settings.target_fps;

    (posterized_sec * comp_fps_f).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posterize_time_12fps_in_60fps_comp() {
        let settings = PosterizeTimeSettings {
            target_fps: 12.0,
            enabled: true,
        };
        let comp_fps = 60;

        // Frames 0..4 should all quantize to frame 0
        assert_eq!(quantize_frame_posterize(0, comp_fps, &settings), 0);
        assert_eq!(quantize_frame_posterize(1, comp_fps, &settings), 0);
        assert_eq!(quantize_frame_posterize(4, comp_fps, &settings), 0);

        // Frame 5 should quantize to frame 5 (1/12th of a second at 60fps)
        assert_eq!(quantize_frame_posterize(5, comp_fps, &settings), 5);
        assert_eq!(quantize_frame_posterize(7, comp_fps, &settings), 5);
    }
}
