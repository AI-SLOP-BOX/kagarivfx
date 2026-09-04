#![allow(dead_code)]
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

type RenderFrameFn = Arc<dyn Fn(&str, u32) -> Vec<u8> + Send + Sync>;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;

/// RAII guard that ensures a child process is killed and waited on drop.
/// Prevents zombie processes on all exit paths (error, cancel, early return).
struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    /// Take ownership and prevent auto-kill (e.g., on successful wait).
    fn take(&mut self) -> Option<Child> {
        self.child.take()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Drain stderr from a child process in a background thread, returning the output.
/// This prevents the pipe buffer from filling up and causing a deadlock when the
/// parent calls `child.wait()` before reading stderr.
fn drain_stderr(mut child: Child) -> (Child, std::thread::JoinHandle<String>) {
    let stderr = child.stderr.take();
    let handle = std::thread::spawn(move || {
        let mut output = String::new();
        if let Some(mut s) = stderr {
            let _ = std::io::Read::read_to_string(&mut s, &mut output);
        }
        output
    });
    (child, handle)
}

/// Events sent from the export thread to the UI thread.
#[derive(Debug, Clone)]
pub enum ExportEvent {
    Progress(f32, String),
    Finished(String),
    Error(String),
}

/// Configuration for the export job.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum VideoCodec {
    /// H.264 — universal compatibility, small files
    #[default]
    H264,
    /// Apple ProRes 422 — professional editing codec, large files
    ProRes422,
    /// Apple ProRes 4444 — highest quality, alpha channel support
    ProRes4444,
    /// HEVC / H.265 — next-gen compression, smaller files than H.264
    H265,
    /// GIF animation
    Gif,
    /// WebP animation
    WebP,
}

#[derive(Clone)]
pub struct ExportConfig {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_frames: u32,
    /// Optional WAV to mux as the audio track (from a video layer's import).
    /// When None, the export is video-only.
    pub audio_wav: Option<String>,
    /// Video codec selection (default: H264)
    pub codec: VideoCodec,
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Start an asynchronous FFmpeg export job with an optional cancellation flag.
/// Invokes a render closure with the cooperative cancel flag installed on the
/// current thread, so a cancelled export aborts mid-frame instead of waiting
/// for the frame to finish.
fn render_with_cancel<F>(
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    render_frame_fn: &F,
    frame_idx: u32,
) -> Vec<u8>
where
    F: Fn(u32) -> Vec<u8>,
{
    crate::core::software_renderer::set_render_cancel_flag(Some(cancel_flag.clone()));
    let pixels = render_frame_fn(frame_idx);
    crate::core::software_renderer::set_render_cancel_flag(None);
    pixels
}

pub fn start_export_cancelable<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<AtomicBool>,
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

    // Validate paths: reject anything starting with '-' (FFmpeg arg injection)
    if let Some(wav) = &config.audio_wav {
        if wav.starts_with('-') {
            let msg = format!("audio_wav path must not start with '-': {}", wav);
            let _ = tx.send(ExportEvent::Error(msg.clone()));
            return Err(msg);
        }
    }
    if config.output_path.starts_with('-') {
        let msg = format!("output_path must not start with '-': {}", config.output_path);
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
            // When an audio WAV is provided it is muxed as a second input with
            // AAC encoding, producing a complete AV file.
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-y");
            if let Some(wav) = &config.audio_wav {
                // Audio input first (input 0) so the WAV clock defines duration sync
                cmd.args(["-i", wav]);
            }
            cmd.args([
                "-f",
                "rawvideo", // Input format: raw video
                "-pix_fmt",
                "rgba", // 4 bytes per pixel
                "-s",
                &format!("{}x{}", config.width, config.height),
                "-r",
                &config.fps.to_string(), // Frame rate
                "-i",
                "pipe:0", // Read from stdin
            ]);
            // Codec-specific encoding args
            match &config.codec {
                VideoCodec::H264 => {
                    cmd.args([
                        "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
                    ]);
                }
                VideoCodec::ProRes422 => {
                    cmd.args([
                        "-c:v",
                        "prores_ks",
                        "-profile:v",
                        "3",
                        "-pix_fmt",
                        "yuv422p10le",
                    ]);
                }
                VideoCodec::ProRes4444 => {
                    cmd.args([
                        "-c:v",
                        "prores_ks",
                        "-profile:v",
                        "4",
                        "-pix_fmt",
                        "yuva444p10le",
                    ]);
                }
                VideoCodec::H265 => {
                    cmd.args([
                        "-c:v", "libx265", "-preset", "medium", "-crf", "23", "-pix_fmt", "yuv420p",
                    ]);
                }
                VideoCodec::Gif => { /* GIF uses separate pipeline */ }
                VideoCodec::WebP => {
                    cmd.args([
                        "-c:v",
                        "libwebp",
                        "-quality",
                        "80",
                        "-lossless",
                        "0",
                        "-pix_fmt",
                        "rgba",
                    ]);
                }
            }
            if config.audio_wav.is_some() {
                // Video is input 1 when audio present; encode audio to AAC
                cmd.args([
                    "-map",
                    "0:a",
                    "-map",
                    "1:v",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    "-shortest",
                ]);
            }
            cmd.arg("-movflags")
                .arg("+faststart")
                .arg("--")
                .arg(&config.output_path);

            let ffmpeg_result = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let mut child = match ffmpeg_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to launch FFmpeg: {}",
                        e
                    )));
                    return;
                }
            };

            // Drain stderr in a background thread to prevent pipe buffer deadlock
            let (child_with_stderr, stderr_handle) = drain_stderr(child);
            child = child_with_stderr;
            let mut guard = ChildGuard::new(child);

            let mut stdin = match guard.child.as_mut().and_then(|c| c.stdin.take()) {
                Some(s) => s,
                None => {
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened".to_string(),
                    ));
                    return;
                }
            };

            if config.total_frames == 0 {
                let _ = tx.send(ExportEvent::Finished(format!(
                    "Export complete (0 frames) → {}",
                    config.output_path
                )));
                return;
            }

            let Some(frame_bytes) =
                crate::core::software_renderer::rgba_buffer_size(config.width, config.height)
            else {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Invalid dimensions: {}x{}",
                    config.width, config.height
                )));
                return;
            };

            for frame_idx in 0..config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    log::info!("[FFmpegExport] export canceled by user — terminating process");
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    return; // guard.drop() kills and waits
                }

                // Render the frame to raw RGBA pixels (cancellable mid-frame)
                let pixels = render_with_cancel(&cancel_flag, &render_frame_fn, frame_idx);

                if pixels.len() != frame_bytes {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Frame {} pixel data mismatch: expected {} bytes, got {}",
                        frame_idx,
                        frame_bytes,
                        pixels.len()
                    )));
                    return;
                }

                // Write RGBA frame to FFmpeg stdin
                if let Err(e) = stdin.write_all(&pixels) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Pipe write error at frame {}: {}",
                        frame_idx, e
                    )));
                    return;
                }

                // Report progress (total_frames guaranteed > 0 here)
                let progress = (frame_idx + 1) as f32 / config.total_frames as f32;
                let _ = tx.send(ExportEvent::Progress(
                    progress,
                    format!("Encoding frame {}/{}", frame_idx + 1, config.total_frames),
                ));
            }

            // Close stdin to signal EOF to FFmpeg
            drop(stdin);

            // Take child from guard before manual wait (guard won't double-kill)
            let mut child = guard.take().expect("child must still be in guard");

            // Wait for FFmpeg to finish
            let stderr_output = stderr_handle.join().unwrap_or_default();
            match child.wait() {
                Ok(status) if status.success() => {
                    let _ = tx.send(ExportEvent::Finished(format!(
                        "Export complete → {}",
                        config.output_path
                    )));
                }
                Ok(status) => {
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

/// Start an asynchronous FFmpeg GIF export job with a two-pass palette approach.
///
/// Pass 1: Generate an optimal 256-color palette from all frames.
/// Pass 2: Encode the GIF using that palette with sierra2_4a dithering.
pub fn start_gif_export<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<AtomicBool>,
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

    // Validate paths: reject anything starting with '-' (FFmpeg arg injection)
    if config.output_path.starts_with('-') {
        let msg = format!("output_path must not start with '-': {}", config.output_path);
        let _ = tx.send(ExportEvent::Error(msg.clone()));
        return Err(msg);
    }

    let config_clone = config.clone();

    std::thread::Builder::new()
        .name("ffmpeg_gif_export".to_string())
        .spawn(move || {
            let config = config_clone;
            let palette_path = format!("{}.palette.png", config.output_path);

            let Some(frame_bytes) =
                crate::core::software_renderer::rgba_buffer_size(config.width, config.height)
            else {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Invalid dimensions: {}x{}",
                    config.width, config.height
                )));
                return;
            };

            if config.total_frames == 0 {
                let _ = tx.send(ExportEvent::Finished(format!(
                    "GIF export complete (0 frames) → {}",
                    config.output_path
                )));
                return;
            }

            // ── Pass 1: Generate palette ──────────────────────────────────
            let _ = tx.send(ExportEvent::Progress(
                0.0,
                "Generating GIF palette...".to_string(),
            ));

            let palette_result = Command::new("ffmpeg")
                .args([
                    "-y",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "rgba",
                    "-s",
                    &format!("{}x{}", config.width, config.height),
                    "-r",
                    &config.fps.to_string(),
                    "-i",
                    "pipe:0",
                    "-vf",
                    &format!(
                        "fps={},palettegen=max_colors=256:stats_mode=diff",
                        config.fps
                    ),
                    &palette_path,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let mut palette_child = match palette_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to launch FFmpeg (palette pass): {}",
                        e
                    )));
                    return;
                }
            };

            // Drain stderr to prevent pipe buffer deadlock
            let (palette_child, palette_stderr_handle) = drain_stderr(palette_child);
            let mut palette_guard = ChildGuard::new(palette_child);

            let mut palette_stdin = match palette_guard.child.as_mut().and_then(|c| c.stdin.take()) {
                Some(s) => s,
                None => {
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened (palette pass)".to_string(),
                    ));
                    return;
                }
            };

            for frame_idx in 0..config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    return; // guard drops, kills child
                }

                let pixels = render_with_cancel(&cancel_flag, &render_frame_fn, frame_idx);
                if pixels.len() != frame_bytes {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Frame {} pixel data mismatch: expected {} bytes, got {}",
                        frame_idx,
                        frame_bytes,
                        pixels.len()
                    )));
                    return;
                }
                if let Err(e) = palette_stdin.write_all(&pixels) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Pipe write error (palette pass) at frame {}: {}",
                        frame_idx, e
                    )));
                    return;
                }

                let progress = (frame_idx + 1) as f32 / config.total_frames as f32 * 0.5;
                let _ = tx.send(ExportEvent::Progress(
                    progress,
                    format!(
                        "Palette pass: frame {}/{}",
                        frame_idx + 1,
                        config.total_frames
                    ),
                ));
            }

            drop(palette_stdin);

            let mut palette_child = palette_guard.take().expect("palette child must be in guard");
            let stderr_output = palette_stderr_handle.join().unwrap_or_default();
            match palette_child.wait() {
                Ok(status) if status.success() => { /* ok */ }
                Ok(status) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg palette pass exited with code {:?}\n{}",
                        status.code(),
                        &stderr_output[..stderr_output.len().min(500)]
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg palette pass wait() error: {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
            }

            // ── Pass 2: Encode GIF using palette ──────────────────────────
            let _ = tx.send(ExportEvent::Progress(0.5, "Encoding GIF...".to_string()));

            let gif_result = Command::new("ffmpeg")
                .args([
                    "-y",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "rgba",
                    "-s",
                    &format!("{}x{}", config.width, config.height),
                    "-r",
                    &config.fps.to_string(),
                    "-i",
                    "pipe:0",
                    "-i",
                    &palette_path,
                    "-lavfi",
                    "paletteuse=dither=sierra2_4a",
                    "-loop",
                    "0",
                    &config.output_path,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let mut gif_child = match gif_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Failed to launch FFmpeg (gif pass): {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
            };

            // Drain stderr to prevent pipe buffer deadlock
            let (gif_child, gif_stderr_handle) = drain_stderr(gif_child);
            let mut gif_guard = ChildGuard::new(gif_child);

            let mut gif_stdin = match gif_guard.child.as_mut().and_then(|c| c.stdin.take()) {
                Some(s) => s,
                None => {
                    let _ = tx.send(ExportEvent::Error(
                        "FFmpeg stdin pipe was not opened (gif pass)".to_string(),
                    ));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
            };

            for frame_idx in 0..config.total_frames {
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = tx.send(ExportEvent::Error("Export canceled by user".to_string()));
                    let _ = std::fs::remove_file(&palette_path);
                    return; // guard drops, kills child
                }

                let pixels = render_with_cancel(&cancel_flag, &render_frame_fn, frame_idx);
                if pixels.len() != frame_bytes {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Frame {} pixel data mismatch: expected {} bytes, got {}",
                        frame_idx,
                        frame_bytes,
                        pixels.len()
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }
                if let Err(e) = gif_stdin.write_all(&pixels) {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "Pipe write error (gif pass) at frame {}: {}",
                        frame_idx, e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                    return;
                }

                let progress = 0.5 + (frame_idx + 1) as f32 / config.total_frames as f32 * 0.5;
                let _ = tx.send(ExportEvent::Progress(
                    progress,
                    format!("GIF pass: frame {}/{}", frame_idx + 1, config.total_frames),
                ));
            }

            drop(gif_stdin);
            let mut gif_child = gif_guard.take().expect("gif child must be in guard");
            let stderr_output = gif_stderr_handle.join().unwrap_or_default();
            match gif_child.wait() {
                Ok(status) if status.success() => {
                    let _ = std::fs::remove_file(&palette_path);
                    let _ = tx.send(ExportEvent::Finished(format!(
                        "GIF export complete → {}",
                        config.output_path
                    )));
                }
                Ok(status) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg gif pass exited with code {:?}\n{}",
                        status.code(),
                        &stderr_output[..stderr_output.len().min(500)]
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                }
                Err(e) => {
                    let _ = tx.send(ExportEvent::Error(format!(
                        "FFmpeg gif pass wait() error: {}",
                        e
                    )));
                    let _ = std::fs::remove_file(&palette_path);
                }
            }
        })
        .map_err(|e| format!("Failed to spawn GIF export thread: {}", e))?;

    Ok(())
}

pub fn start_export<F>(
    config: ExportConfig,
    tx: Sender<ExportEvent>,
    render_frame_fn: F,
) -> Result<(), String>
where
    F: Fn(u32) -> Vec<u8> + Send + 'static,
{
    start_export_cancelable(
        config,
        tx,
        Arc::new(AtomicBool::new(false)),
        render_frame_fn,
    )
}

/// Multi-comp parallel export using `ParallelRenderQueue`.
///
/// Each `RenderQueueItem` is rendered in parallel via rayon, with frames
/// within each item rendered sequentially. Progress is reported via the
/// `tx` channel as `ExportEvent::Progress` with the overall percentage.
pub fn start_parallel_export(
    items: Vec<crate::core::parallel_render::RenderQueueItem>,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<AtomicBool>,
    render_frame_fn: RenderFrameFn,
) -> Result<(), String> {
    if items.is_empty() {
        let _ = tx.send(ExportEvent::Error("No render queue items".to_string()));
        return Err("No items".to_string());
    }

    let total: u32 = items
        .iter()
        .map(|i| i.end_frame.saturating_sub(i.start_frame) + 1)
        .sum();
    if total == 0 {
        let _ = tx.send(ExportEvent::Error("Total frames is zero".to_string()));
        return Err("Zero frames".to_string());
    }

    std::thread::Builder::new()
        .name("parallel_export".to_string())
        .spawn(move || {
            let mut queue = crate::core::parallel_render::ParallelRenderQueue::new();
            for item in items {
                queue.add_item(item);
            }
            queue
                .cancelled
                .store(cancel_flag.load(Ordering::SeqCst), Ordering::Relaxed);

            let tx_clone = tx.clone();
            let total_frames = total;
            queue.set_progress_callback(move |_item_idx, done, _total| {
                let pct = done as f32 / total_frames.max(1) as f32;
                let _ = tx_clone.send(ExportEvent::Progress(
                    pct,
                    format!("Parallel: {}/{} frames", done, total_frames),
                ));
            });

            queue.render_all(|comp_name, frame| render_frame_fn(comp_name, frame));

            let _ = tx.send(ExportEvent::Finished(format!(
                "Parallel export complete: {} items, {} frames",
                queue.items.len(),
                total_frames
            )));
        })
        .map_err(|e| format!("Failed to spawn parallel export thread: {}", e))?;

    Ok(())
}

/// PNG image-sequence export: renders every frame and writes
/// `{stem}_{frame:04}.png` into `dir` using the `image` crate.
#[allow(clippy::too_many_arguments)]
pub fn start_png_sequence_export<F>(
    dir: std::path::PathBuf,
    stem: String,
    width: u32,
    height: u32,
    total_frames: u32,
    first_index: u32,
    tx: Sender<ExportEvent>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    mut render_frame: F,
) where
    F: FnMut(u32) -> Vec<u8> + Send + 'static,
{
    use std::sync::atomic::Ordering;
    std::thread::spawn(move || {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            let _ = tx.send(ExportEvent::Error(format!(
                "Cannot create dir {}: {}",
                dir.display(),
                e
            )));
            return;
        }
        let last = total_frames.saturating_sub(1);
        for i in 0..total_frames {
            // Absolute frame number (work-area offset aware) for file naming.
            let f = i + first_index;
            if cancel_flag.load(Ordering::SeqCst) {
                let _ = tx.send(ExportEvent::Error("Export canceled".to_string()));
                return;
            }
            let pixels = render_frame(f);
            let path = dir.join(format!("{}_{:04}.png", stem, f));
            if let Err(e) =
                image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8)
            {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Failed writing {}: {}",
                    path.display(),
                    e
                )));
                return;
            }
            if i % 2 == 0 || i == last {
                let _ = tx.send(ExportEvent::Progress(
                    (i + 1) as f32 / total_frames.max(1) as f32,
                    format!("Frame {} / {} → {}", f + 1, total_frames, path.display()),
                ));
            }
        }
        let _ = tx.send(ExportEvent::Finished(format!(
            "PNG sequence exported: {} frames → {}",
            total_frames,
            dir.join(format!("{}_", stem)).display()
        )));
    });
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
            audio_wav: None,
            codec: VideoCodec::H264,
            output_path: "test.mp4".to_string(),
            width: 1920,
            height: 1080,
            fps: 30,
            total_frames: 90,
        };
        let cfg2 = cfg.clone();
        assert_eq!(cfg.output_path, cfg2.output_path);
    }
}
