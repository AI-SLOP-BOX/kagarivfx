/// FFmpeg-based async video export pipeline.
///
/// Spawns an FFmpeg subprocess in a background thread to avoid blocking the UI.
/// Progress is communicated back to the UI via an `mpsc` channel of `ExportEvent`.
///
/// # How it works
/// 1. UI calls `start_export(...)` → a `std::thread` is spawned.
/// 2. The thread renders each frame from the composition (CPU pixel data) and
///    pipes raw RGBA frames into FFmpeg's stdin.
/// 3. FFmpeg encodes to H.264 MP4 using:
///    `ffmpeg -f rawvideo -pix_fmt rgba -s WxH -r FPS -i pipe:0 -c:v libx264 -pix_fmt yuv420p output.mp4`
/// 4. Progress events are sent back to the UI via the mpsc sender.
/// 5. The UI polls the receiver each frame (non-blocking `try_recv`).

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

/// Events sent from the export thread to the UI thread.
#[derive(Debug, Clone)]
pub enum ExportEvent {
    Progress(f32, String),
    Finished(String),
    Error(String),
}

/// Configuration for the export job.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_frames: u32,
}

/// Check whether `ffmpeg` is available in PATH.
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Start an asynchronous FFmpeg export job.
///
/// `render_frame_fn` is called for each frame and must return `width * height * 4` RGBA bytes.
/// The function is called on the background thread, so the closure must be `Send + 'static`.
///
/// Returns `Err(String)` immediately if FFmpeg is not found.
pub fn start_export<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    render_frame_fn: F,
) -> Result<(), String>
where
    F: Fn(u32) -> Vec<u8> + Send + 'static,
{
    if !is_ffmpeg_available() {
        let msg = "FFmpeg not found. Install it via `brew install ffmpeg` (macOS) or your package manager.".to_string();
        let _ = tx.send(ExportEvent::Error(msg.clone()));
        return Err(msg);
    }

    let config_clone = config.clone();
    std::thread::Builder::new()
        .name("ffmpeg_export".to_string())
        .spawn(move || {
            let config = config_clone;

            // Build FFmpeg command:
            // Read raw RGBA frames from stdin, encode to H.264 yuv420p MP4.
            let ffmpeg_result = Command::new("ffmpeg")
                .args([
                    "-y",                           // Overwrite output without prompt
                    "-f", "rawvideo",               // Input format: raw video
                    "-pix_fmt", "rgba",             // 4 bytes per pixel
                    "-s", &format!("{}x{}", config.width, config.height),
                    "-r", &config.fps.to_string(),  // Frame rate
                    "-i", "pipe:0",                 // Read from stdin
                    "-c:v", "libx264",              // H.264 encoder
                    "-preset", "fast",              // Encoding speed/quality tradeoff
                    "-crf", "18",                   // Constant Rate Factor (quality: 0=lossless, 51=worst)
                    "-pix_fmt", "yuv420p",          // Required for broad compatibility (iOS, browsers)
                    "-movflags", "+faststart",      // MP4 faststart for streaming
                    &config.output_path,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let mut child = match ffmpeg_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!("Failed to launch FFmpeg: {}", e)));
                    return;
                }
            };

            let mut stdin = child.stdin.take().expect("FFmpeg stdin pipe failed");

            let frame_bytes = (config.width * config.height * 4) as usize;

            for frame_idx in 0..config.total_frames {
                // Render the frame to raw RGBA pixels
                let pixels = render_frame_fn(frame_idx);

                if pixels.len() != frame_bytes {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Frame {} pixel data mismatch: expected {} bytes, got {}",
                        frame_idx, frame_bytes, pixels.len()
                    )));
                    return;
                }

                // Write RGBA frame to FFmpeg stdin
                if let Err(e) = stdin.write_all(&pixels) {
                    let _ = tx.send(ExportEvent::Error(format!("Pipe write error at frame {}: {}", frame_idx, e)));
                    return;
                }

                // Report progress
                let progress = (frame_idx + 1) as f32 / config.total_frames as f32;
                let _ = tx.send(ExportEvent::Progress(
                    progress,
                    format!("Encoding frame {}/{}", frame_idx + 1, config.total_frames),
                ));
            }

            // Close stdin to signal EOF to FFmpeg
            drop(stdin);

            // Wait for FFmpeg to finish
            match child.wait() {
                Ok(status) if status.success() => {
                    let _ = tx.send(ExportEvent::Finished(format!(
                        "Export complete → {}",
                        config.output_path
                    )));
                }
                Ok(status) => {
                    // Collect stderr for diagnostics
                    let stderr_output = child.stderr
                        .take()
                        .map(|mut s| {
                            let mut buf = String::new();
                            std::io::Read::read_to_string(&mut s, &mut buf).ok();
                            buf
                        })
                        .unwrap_or_default();
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg exited with code {:?}\n{}",
                        status.code(),
                        &stderr_output[..stderr_output.len().min(500)]
                    )));
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!("FFmpeg wait() error: {}", e)));
                }
            }
        })
        .map_err(|e| format!("Failed to spawn export thread: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_availability_check() {
        // Just test that the function runs without panicking.
        let _available = is_ffmpeg_available();
    }

    #[test]
    fn test_export_config_clone() {
        let cfg = ExportConfig {
            output_path: "test.mp4".to_string(),
            width: 1920, height: 1080, fps: 30, total_frames: 90,
        };
        let cfg2 = cfg.clone();
        assert_eq!(cfg.output_path, cfg2.output_path);
    }
}
