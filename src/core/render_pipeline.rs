#![allow(dead_code)]
/// Vulkan/DirectX12-inspired lazy evaluation render pipeline.
///
/// The UI thread never blocks on GPU work. Instead, it enqueues `RenderCommand`
/// messages. A background `RenderWorker` thread drains the queue, performs GPU
/// rendering (via the existing WgpuRenderer), and writes results to FrameCache.
/// The viewport reads from FrameCache — if a frame is cached it's instant;
/// if not, the worker delivers it asynchronously while the UI stays responsive.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::sync::{Arc, Mutex};

/// Commands the UI thread sends to the render worker.
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Render a specific frame at the current cache version.
    RenderFrame { frame: u32 },
    /// Render a contiguous range of frames (background pre-fetch).
    PrefetchRange { start: u32, end: u32 },
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
    /// Sequence number for pending commands; used to detect stale results.
    pending: Arc<Mutex<u32>>,
}

impl RenderPipeline {
    /// Spawn the background render worker and return the pipeline handle.
    pub fn new<F>(render_fn: F) -> Self
    where
        F: Fn(RenderCommand, &Sender<RenderResult>) + Send + 'static,
    {
        let (cmd_tx, cmd_rx) = mpsc::channel::<RenderCommand>();
        let (result_tx, result_rx) = mpsc::channel::<RenderResult>();

        thread::Builder::new()
            .name("render_worker".to_string())
            .spawn(move || {
                log::info!("[RenderWorker] started");
                loop {
                    let cmd = match cmd_rx.recv() {
                        Ok(c) => c,
                        Err(_) => break, // channel closed
                    };
                    match &cmd {
                        RenderCommand::Shutdown => {
                            log::info!("[RenderWorker] shutting down");
                            break;
                        }
                        RenderCommand::Flush => {
                            log::debug!("[RenderWorker] flush — draining queue");
                            // Drain remaining commands immediately
                            while cmd_rx.try_recv().is_ok() {}
                        }
                        _ => {
                            render_fn(cmd, &result_tx);
                        }
                    }
                }
            })
            .expect("Failed to spawn render worker thread");

        Self {
            cmd_tx,
            result_rx,
            pending: Arc::new(Mutex::new(0)),
        }
    }

    /// Request an asynchronous render of `frame`.
    /// Returns immediately — the UI can continue drawing cached frames.
    pub fn request_frame(&self, frame: u32) {
        let _ = self.cmd_tx.send(RenderCommand::RenderFrame { frame });
    }

    /// Pre-fetch a range of frames in the background (e.g. during playback).
    pub fn prefetch_range(&self, start: u32, end: u32) {
        let _ = self.cmd_tx.send(RenderCommand::PrefetchRange { start, end });
    }

    /// Flush all pending commands (call after a project state change).
    pub fn flush(&self) {
        let _ = self.cmd_tx.send(RenderCommand::Flush);
    }

    /// Poll for completed frames without blocking. Call once per UI frame.
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

/// A lightweight "lazy" evaluator that decides whether a frame needs rendering
/// based on the current FrameCache state. Implements the demand-driven
/// scheduling strategy from Vulkan's lazy resource evaluation.
pub struct LazyFrameEvaluator {
    /// Frames that have been requested but not yet delivered.
    in_flight: std::collections::HashSet<u32>,
}

impl LazyFrameEvaluator {
    pub fn new() -> Self {
        Self {
            in_flight: std::collections::HashSet::new(),
        }
    }

    /// Should we request a render for this frame right now?
    /// Returns `true` only if the frame isn't cached and isn't already in-flight.
    pub fn needs_render(&mut self, frame: u32, cache: &crate::core::frame_cache::FrameCache) -> bool {
        if cache.is_cached(frame) {
            self.in_flight.remove(&frame);
            return false;
        }
        if self.in_flight.contains(&frame) {
            return false; // Already requested, wait for the worker
        }
        self.in_flight.insert(frame);
        true
    }

    /// Mark a frame as delivered (remove from in-flight set).
    pub fn mark_delivered(&mut self, frame: u32) {
        self.in_flight.remove(&frame);
    }
}
