#![allow(dead_code)]
/// Vulkan/DirectX12-inspired lazy evaluation render pipeline with atomic cancellation tokens.
///
/// The UI thread never blocks on GPU work. Instead, it enqueues `RenderCommand`
/// messages. A background `RenderWorker` thread drains the queue, performs GPU
/// rendering, and writes results to FrameCache.
/// Includes atomic `CancellationToken` checks so stale seek/scrub tasks abort
/// immediately before wasting CPU/GPU cycles.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

/// Commands the UI thread sends to the render worker.
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Render a specific frame at a specific cache version.
    RenderFrame { frame: u32, version: u64 },
    /// Render a contiguous range of frames (background pre-fetch).
    PrefetchRange { start: u32, end: u32, version: u64 },
    /// Invalidate the in-flight queue (e.g., project changed).
    Flush,
    /// Shut down the worker thread cleanly.
    Shutdown,
}

/// Notification from the worker back to the UI thread.
#[derive(Debug)]
pub enum RenderResult {
    /// A frame finished rendering and is now in the FrameCache.
    FrameReady { frame: u32, cache_version: u64 },
    /// A batch of frames is ready.
    BatchReady { start: u32, end: u32 },
}

/// Manages the command channel to the background render worker.
pub struct RenderPipeline {
    cmd_tx: Sender<RenderCommand>,
    result_rx: Receiver<RenderResult>,
    /// Atomic cancellation token sequence. Incremented on Flush/Scrub to abort stale tasks.
    cancellation_token: Arc<AtomicU64>,
}

impl RenderPipeline {
    /// Spawn the background render worker and return the pipeline handle.
    pub fn new<F>(render_fn: F) -> Self
    where
        F: Fn(RenderCommand, &Sender<RenderResult>) + Send + 'static,
    {
        let (cmd_tx, cmd_rx) = mpsc::channel::<RenderCommand>();
        let (result_tx, result_rx) = mpsc::channel::<RenderResult>();
        let cancellation_token = Arc::new(AtomicU64::new(1));
        let worker_token = Arc::clone(&cancellation_token);

        thread::Builder::new()
            .name("render_worker".to_string())
            .spawn(move || {
                log::info!("[RenderWorker] started");
                while let Ok(cmd) = cmd_rx.recv() {
                    match &cmd {
                        RenderCommand::Shutdown => {
                            log::info!("[RenderWorker] shutting down");
                            break;
                        }
                        RenderCommand::Flush => {
                            log::debug!("[RenderWorker] flush — draining queue");
                            while cmd_rx.try_recv().is_ok() {}
                        }
                        RenderCommand::RenderFrame { version, .. }
                        | RenderCommand::PrefetchRange { version, .. } => {
                            let cur_token = worker_token.load(Ordering::SeqCst);
                            if *version < cur_token {
                                log::debug!("[RenderWorker] aborting stale render command (ver {})", version);
                                continue;
                            }
                            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                render_fn(cmd, &result_tx);
                            }));
                            if let Err(e) = res {
                                log::error!("[RenderWorker] Caught panic during render task execution: {:?}", e);
                            }
                        }
                    }
                }
            })
            .expect("Failed to spawn render worker thread");

        Self {
            cmd_tx,
            result_rx,
            cancellation_token,
        }
    }

    /// Request an asynchronous render of `frame`.
    pub fn request_frame(&self, frame: u32, version: u64) {
        let _ = self
            .cmd_tx
            .send(RenderCommand::RenderFrame { frame, version });
    }

    /// Pre-fetch a range of frames in the background.
    pub fn prefetch_range(&self, start: u32, end: u32, version: u64) {
        let _ = self.cmd_tx.send(RenderCommand::PrefetchRange {
            start,
            end,
            version,
        });
    }

    /// Flush all pending commands and bump cancellation token.
    pub fn flush(&self) {
        self.cancellation_token.fetch_add(1, Ordering::SeqCst);
        let _ = self.cmd_tx.send(RenderCommand::Flush);
    }

    /// Poll for completed frames without blocking.
    pub fn poll_results(&self) -> Vec<RenderResult> {
        let mut results = Vec::new();
        while let Ok(r) = self.result_rx.try_recv() {
            results.push(r);
        }
        results
    }

    /// Gracefully shut down the worker thread.
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(RenderCommand::Shutdown);
    }
}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct LazyFrameEvaluator {
    in_flight: std::collections::HashSet<u32>,
}

impl Default for LazyFrameEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl LazyFrameEvaluator {
    pub fn new() -> Self {
        Self {
            in_flight: std::collections::HashSet::new(),
        }
    }

    pub fn needs_render(
        &mut self,
        frame: u32,
        cache: &crate::core::frame_cache::FrameCache,
    ) -> bool {
        if cache.is_cached(frame) {
            self.in_flight.remove(&frame);
            return false;
        }
        if self.in_flight.contains(&frame) {
            return false;
        }
        self.in_flight.insert(frame);
        true
    }

    pub fn mark_delivered(&mut self, frame: u32) {
        self.in_flight.remove(&frame);
    }
}
