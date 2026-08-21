#![allow(dead_code)]
use serde_json::json;
use crate::core::timeline::Composition;

/// Exporter for converting Aura Composition into industry-standard Lottie / Bodymovin JSON format.
pub struct LottieExporter;

impl LottieExporter {
    /// Serializes a Composition into a valid Lottie JSON string for web/mobile playback.
    pub fn export_to_json(comp: &Composition) -> String {
        let lottie_json = json!({
            "v": "5.7.4",
            "fr": comp.fps,
            "ip": 0,

            "op": comp.duration_frames,
            "w": comp.width,
            "h": comp.height,
            "nm": comp.name,
            "ddd": 0,
            "assets": [],
            "layers": comp.layers.iter().map(|layer| {
                json!({
                    "ddd": 0,
                    "ind": 1,
                    "ty": 4, // Shape Layer type
                    "nm": layer.name,
                    "sr": 1,
                    "ks": {
                        "o": { "a": 0, "k": 100 },
                        "r": { "a": 0, "k": 0 },
                        "p": { "a": 0, "k": [comp.width as f32 * 0.5, comp.height as f32 * 0.5, 0] },
                        "a": { "a": 0, "k": [0, 0, 0] },
                        "s": { "a": 0, "k": [100, 100, 100] }
                    },
                    "ao": 0,
                    "shapes": [],
                    "ip": 0,
                    "op": comp.duration_frames,
                    "st": 0,
                    "bm": 0
                })
            }).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&lottie_json).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lottie_export() {
        let comp = Composition::new("c1".into(), "TestComp".into(), 1920, 1080, 30, 300);
        let json_str = LottieExporter::export_to_json(&comp);

        assert!(json_str.contains("\"v\": \"5.7.4\""));
        assert!(json_str.contains("\"TestComp\""));
    }
}
