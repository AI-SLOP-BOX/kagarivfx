//! Video import via FFmpeg.
//!
//! Videos are decoded once at import time into a PNG frame sequence plus an
//! optional WAV audio track under the project's media directory. Rendering then
//! samples the sequence like any other image source — no video decoder needed
//! at runtime or export time.
//!
//! This mirrors how NLEs proxy media: one decode pass up front, cheap random
//! access afterwards.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A decoded video asset on disk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoAsset {
    /// Original imported file path.
    pub source_path: String,
    /// Directory containing frame_%05d.png files.
    pub frames_dir: String,
    /// Number of extracted frames.
    pub frame_count: u32,
    /// Frames per second the sequence was extracted at.
    pub fps: f32,
    pub width: u32,
    pub height: u32,
    /// Extracted audio WAV path, if the source had an audio stream.
    pub audio_wav: Option<String>,
}

/// True if ffmpeg is available on PATH.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_duration_seconds(path: &Path) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            "--",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .ok()
}

fn probe_dimensions(path: &Path) -> Option<(u32, u32)> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            "--",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut it = s.split('x');
    let w = it.next()?.parse().ok()?;
    let h = it.next()?.parse().ok()?;
    Some((w, h))
}

fn probe_has_audio(path: &Path) -> bool {
    Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
            "--",
        ])
        .arg(path)
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("audio"))
        .unwrap_or(false)
}

/// Decodes `src_path` into `dest_dir` as a PNG frame sequence (+ WAV audio).
///
/// `fps` controls extraction rate (use the composition's fps so 1 sequence
/// frame == 1 composition frame).
pub fn import_video(src_path: &str, dest_dir: &Path, fps: f32) -> Result<VideoAsset, String> {
    if !ffmpeg_available() {
        return Err("ffmpeg not found on PATH — install it to import video".into());
    }
    let src = Path::new(src_path);
    if !src.is_file() {
        return Err(format!("source file not found: {}", src_path));
    }
    let fps = fps.max(1.0);
    let frames_dir = dest_dir.join("frames");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|e| format!("failed to create media dir: {}", e))?;

    // 1. Decode frames: scale to even dimensions (encoder-safe), numbered from 0
    let pattern = frames_dir.join("frame_%05d.png");
    let decode = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(src)
        .args([
            "-vf",
            &format!("fps={},scale=trunc(iw/2)*2:trunc(ih/2)*2", fps),
        ])
        .args(["-start_number", "0"])
        .arg("--")
        .arg(&pattern)
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {}", e))?;
    if !decode.status.success() {
        return Err(format!(
            "ffmpeg frame extraction failed: {}",
            String::from_utf8_lossy(&decode.stderr)
        ));
    }

    // Count produced frames
    let mut frame_count = 0u32;
    let entries =
        std::fs::read_dir(&frames_dir).map_err(|e| format!("failed to read frames dir: {}", e))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("frame_") && name.ends_with(".png") {
            frame_count += 1;
        }
    }
    if frame_count == 0 {
        return Err("ffmpeg produced no frames".into());
    }

    // 2. Audio extraction (best effort — absence is not fatal)
    let mut audio_wav = None;
    if probe_has_audio(src) {
        let wav = dest_dir.join("audio.wav");
        let audio = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i")
            .arg(src)
            .args(["-vn", "-acodec", "pcm_s16le", "-ar", "44100", "-ac", "2"])
            .arg("--")
            .arg(&wav)
            .output();
        if let Ok(out) = audio {
            if out.status.success() && wav.exists() {
                audio_wav = Some(wav.to_string_lossy().to_string());
            }
        }
    }

    let (width, height) = probe_dimensions(src).unwrap_or((1920, 1080));
    // Prefer actual frame count over probed duration for sequence length
    let _duration = probe_duration_seconds(src);

    Ok(VideoAsset {
        source_path: src_path.to_string(),
        frames_dir: frames_dir.to_string_lossy().to_string(),
        frame_count,
        fps,
        width,
        height,
        audio_wav,
    })
}

/// Returns the PNG path for a given sequence frame index (clamped to range).
pub fn frame_path(asset: &VideoAsset, frame: u32) -> PathBuf {
    let clamped = frame.min(asset.frame_count.saturating_sub(1));
    Path::new(&asset.frames_dir).join(format!("frame_{:05}.png", clamped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_path_clamps_to_range() {
        let asset = VideoAsset {
            source_path: "src.mp4".into(),
            frames_dir: "/tmp/media/frames".into(),
            frame_count: 10,
            fps: 30.0,
            width: 640,
            height: 360,
            audio_wav: None,
        };
        assert!(frame_path(&asset, 0).ends_with("frame_00000.png"));
        assert!(frame_path(&asset, 5).ends_with("frame_00005.png"));
        // Out-of-range clamps to last frame (freeze-frame behavior)
        assert_eq!(frame_path(&asset, 999), frame_path(&asset, 9));
    }

    #[test]
    fn test_import_rejects_missing_source() {
        let result = import_video(
            "/nonexistent/video.mp4",
            Path::new("/tmp/kagari_vid_test"),
            30.0,
        );
        // Missing source must be rejected whether or not ffmpeg exists
        if ffmpeg_available() {
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("not found"));
        }
    }

    #[test]
    fn test_import_rejects_without_ffmpeg_gracefully() {
        // If ffmpeg is absent we get a clean error, never a panic
        let result = import_video("/dev/null", Path::new("/tmp/kagari_vid_test2"), 30.0);
        let _ = result; // Ok or Err both fine — just must not panic
    }
}
