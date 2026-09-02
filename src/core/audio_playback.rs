//! Synced audio playback for AV preview.
//!
//! Owns the rodio output stream and a sink playing the active composition's
//! mixed audio (all audio layers blended via mix_composition_to_wav). Playback
//! position is periodically reconciled against the playhead so audio stays in
//! sync even if the UI drops frames.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct AudioPlayback {
    #[allow(dead_code)]
    stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
    sink: Option<rodio::Sink>,
    /// Path currently loaded into the sink.
    loaded_path: Option<PathBuf>,
    /// Set by the watchdog when the sink dies unexpectedly.
    pub failed: Arc<AtomicBool>,
}

impl AudioPlayback {
    pub fn new() -> Result<Self, String> {
        let (stream, handle) =
            rodio::OutputStream::try_default().map_err(|e| format!("audio device: {}", e))?;
        Ok(Self {
            stream,
            handle,
            sink: None,
            loaded_path: None,
            failed: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Starts (or restarts) playback of `wav_path` at `offset_sec`.
    /// No-op if the same file is already loaded and playing.
    pub fn play(&mut self, wav_path: &PathBuf, offset_sec: f32) -> Result<(), String> {
        // Restart required if the file changed or the sink died
        let needs_reload = match (&self.sink, &self.loaded_path) {
            (Some(sink), Some(p)) => p != wav_path || sink.empty(),
            _ => true,
        };
        if needs_reload {
            self.stop();
            let sink =
                rodio::Sink::try_new(&self.handle).map_err(|e| format!("audio sink: {}", e))?;
            let file =
                std::fs::File::open(wav_path).map_err(|e| format!("cannot open audio: {}", e))?;
            let source = rodio::Decoder::new(std::io::BufReader::new(file))
                .map_err(|e| format!("audio decode: {}", e))?;
            sink.append(source);
            self.failed.store(false, Ordering::Relaxed);
            self.sink = Some(sink);
            self.loaded_path = Some(wav_path.clone());
        }
        if let Some(sink) = &self.sink {
            // Reconcile position with the playhead
            let pos = sink.get_pos();
            let want = Duration::from_secs_f32(offset_sec.max(0.0));
            if pos.abs_diff(want) > Duration::from_millis(120) {
                let _ = sink.try_seek(want);
            }
            if sink.is_paused() {
                sink.play();
            }
        }
        Ok(())
    }

    /// Applies master output volume (0.0..1.0) to the live sink.
    pub fn set_volume(&self, volume: f32) {
        if let Some(sink) = &self.sink {
            sink.set_volume(volume.clamp(0.0, 1.0));
        }
    }

    /// Pauses playback while keeping the current position.
    pub fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    /// Stops and unloads the current audio.
    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.loaded_path = None;
    }

    /// Current playback position in seconds, if a sink is live.
    pub fn position_sec(&self) -> Option<f32> {
        self.sink
            .as_ref()
            .and_then(|s| (!s.is_paused() || !s.empty()).then(|| s.get_pos().as_secs_f32()))
    }

    pub fn is_playing(&self) -> bool {
        self.sink
            .as_ref()
            .is_some_and(|s| !s.is_paused() && !s.empty())
    }
}
