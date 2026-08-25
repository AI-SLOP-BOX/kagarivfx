use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreset {
    pub name: String,
    pub format: ExportFormat,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: u32,
    pub codec: String,
    pub audio_enabled: bool,
    pub audio_bitrate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Mp4,
    Webm,
    Gif,
    PngSequence,
    ProRes,
    Exr,
}

impl Default for ExportPreset {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            format: ExportFormat::Mp4,
            width: 1920,
            height: 1080,
            fps: 30,
            quality: 80,
            codec: "h264".into(),
            audio_enabled: true,
            audio_bitrate: 192,
        }
    }
}

pub fn builtin_presets() -> Vec<ExportPreset> {
    vec![
        ExportPreset {
            name: "YouTube 1080p".into(),
            format: ExportFormat::Mp4,
            width: 1920, height: 1080, fps: 30, quality: 85,
            codec: "h264".into(), audio_enabled: true, audio_bitrate: 192,
        },
        ExportPreset {
            name: "YouTube 4K".into(),
            format: ExportFormat::Mp4,
            width: 3840, height: 2160, fps: 30, quality: 90,
            codec: "h264".into(), audio_enabled: true, audio_bitrate: 320,
        },
        ExportPreset {
            name: "TikTok / Reels (9:16)".into(),
            format: ExportFormat::Mp4,
            width: 1080, height: 1920, fps: 30, quality: 80,
            codec: "h264".into(), audio_enabled: true, audio_bitrate: 128,
        },
        ExportPreset {
            name: "Instagram Square".into(),
            format: ExportFormat::Mp4,
            width: 1080, height: 1080, fps: 30, quality: 80,
            codec: "h264".into(), audio_enabled: true, audio_bitrate: 128,
        },
        ExportPreset {
            name: "ProRes 422 Master".into(),
            format: ExportFormat::ProRes,
            width: 1920, height: 1080, fps: 30, quality: 100,
            codec: "prores_422".into(), audio_enabled: true, audio_bitrate: 320,
        },
        ExportPreset {
            name: "GIF (Animated)".into(),
            format: ExportFormat::Gif,
            width: 640, height: 480, fps: 15, quality: 70,
            codec: "gif".into(), audio_enabled: false, audio_bitrate: 0,
        },
        ExportPreset {
            name: "PNG Sequence".into(),
            format: ExportFormat::PngSequence,
            width: 1920, height: 1080, fps: 30, quality: 100,
            codec: "png".into(), audio_enabled: false, audio_bitrate: 0,
        },
    ]
}

fn presets_path() -> PathBuf {
    let mut p = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push(".aevfx");
    p.push("export_presets.json");
    p
}

pub fn load_user_presets() -> Vec<ExportPreset> {
    let path = presets_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_user_presets(presets: &[ExportPreset]) -> Result<(), String> {
    let path = presets_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(presets).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn add_user_preset(preset: ExportPreset) -> Result<(), String> {
    let mut presets = load_user_presets();
    presets.push(preset);
    save_user_presets(&presets)
}

pub fn delete_user_preset(name: &str) -> Result<(), String> {
    let mut presets = load_user_presets();
    presets.retain(|p| p.name != name);
    save_user_presets(&presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_presets_count() {
        let presets = builtin_presets();
        assert!(presets.len() >= 7);
    }

    #[test]
    fn test_default_preset() {
        let p = ExportPreset::default();
        assert_eq!(p.width, 1920);
        assert_eq!(p.height, 1080);
        assert_eq!(p.format, ExportFormat::Mp4);
    }

    #[test]
    fn test_preset_serialization() {
        let p = ExportPreset::default();
        let json = serde_json::to_string(&p).unwrap();
        let back: ExportPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, p.name);
        assert_eq!(back.width, p.width);
    }

    #[test]
    fn test_format_equality() {
        assert_eq!(ExportFormat::Mp4, ExportFormat::Mp4);
        assert_ne!(ExportFormat::Mp4, ExportFormat::Gif);
    }
}
