use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use rayon::prelude::*;

/// Render queue item representing a composition to be rendered.
#[derive(Debug, Clone)]
pub struct RenderQueueItem {
    pub comp_name: String,
    pub start_frame: u32,
    pub end_frame: u32,
    pub output_path: String,
    pub status: RenderStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStatus {
    Pending,
    Rendering,
    Done,
    Failed,
}

/// Progress callback: (item_index, frames_done, total_frames).
type ProgressCallback = dyn Fn(usize, u32, u32) + Send + Sync;

/// Parallel render queue that processes multiple compositions and/or frames
/// using rayon's thread pool.
pub struct ParallelRenderQueue {
    pub items: Vec<RenderQueueItem>,
    /// Total frames rendered across all items.
    pub total_frames_rendered: AtomicU32,
    /// Total frames to render across all items.
    pub total_frames: u32,
    /// Whether the entire queue has been cancelled.
    pub cancelled: AtomicBool,
    progress: Option<Arc<ProgressCallback>>,
}

impl ParallelRenderQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_frames_rendered: AtomicU32::new(0),
            total_frames: 0,
            cancelled: AtomicBool::new(false),
            progress: None,
        }
    }

    pub fn set_progress_callback<F: Fn(usize, u32, u32) + Send + Sync + 'static>(&mut self, cb: F) {
        self.progress = Some(Arc::new(cb));
    }

    pub fn add_item(&mut self, item: RenderQueueItem) {
        self.total_frames += item.end_frame.saturating_sub(item.start_frame) + 1;
        self.items.push(item);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Render all queue items in parallel. Each item's frames are rendered
    /// sequentially within the item, but different items run in parallel.
    pub fn render_all<F>(&self, render_frame: F)
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let items_done = AtomicU32::new(0);

        self.items.par_iter().enumerate().for_each(|(item_idx, item)| {
            if self.is_cancelled() { return; }

            for frame in item.start_frame..=item.end_frame {
                if self.is_cancelled() { return; }

                let _pixels = render_frame(&item.comp_name, frame);
                self.total_frames_rendered.fetch_add(1, Ordering::Relaxed);

                if let Some(cb) = &self.progress {
                    let done = self.total_frames_rendered.load(Ordering::Relaxed);
                    cb(item_idx, done, self.total_frames);
                }
            }
            items_done.fetch_add(1, Ordering::Relaxed);
        });
    }

    /// Multi-frame rendering (MFR): render all frames of all items in parallel
    /// across both items AND frames within each item. This maximizes CPU core
    /// utilization for batch rendering.
    pub fn render_all_mfr<F>(&self, render_frame: F)
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        self.items.par_iter().enumerate().for_each(|(item_idx, item)| {
            if self.is_cancelled() { return; }

            let frames: Vec<u32> = (item.start_frame..=item.end_frame).collect();
            frames.par_iter().for_each(|&frame| {
                if self.is_cancelled() { return; }

                let _pixels = render_frame(&item.comp_name, frame);
                self.total_frames_rendered.fetch_add(1, Ordering::Relaxed);

                if let Some(cb) = &self.progress {
                    let done = self.total_frames_rendered.load(Ordering::Relaxed);
                    cb(item_idx, done, self.total_frames);
                }
            });
        });
    }
}

impl Default for ParallelRenderQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-frame render statistics for the status bar.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub frames_rendered: u32,
    pub total_frames: u32,
    pub elapsed_ms: f64,
    pub avg_frame_ms: f64,
    pub active_threads: usize,
}

impl RenderStats {
    pub fn progress_pct(&self) -> f32 {
        if self.total_frames == 0 { 0.0 } else { self.frames_rendered as f32 / self.total_frames as f32 * 100.0 }
    }

    pub fn fps(&self) -> f32 {
        if self.elapsed_ms <= 0.0 { 0.0 } else { self.frames_rendered as f32 / (self.elapsed_ms as f32 / 1000.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_parallel_render_queue_basics() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Comp 1".into(),
            start_frame: 0,
            end_frame: 9,
            output_path: "/tmp/out1.mp4".into(),
            status: RenderStatus::Pending,
        });
        queue.add_item(RenderQueueItem {
            comp_name: "Comp 2".into(),
            start_frame: 0,
            end_frame: 4,
            output_path: "/tmp/out2.mp4".into(),
            status: RenderStatus::Pending,
        });
        assert_eq!(queue.total_frames, 15);
        assert!(!queue.is_cancelled());
    }

    #[test]
    fn test_cancel() {
        let queue = ParallelRenderQueue::new();
        assert!(!queue.is_cancelled());
        queue.cancel();
        assert!(queue.is_cancelled());
    }

    #[test]
    fn test_render_stats() {
        let stats = RenderStats {
            frames_rendered: 50,
            total_frames: 100,
            elapsed_ms: 1000.0,
            avg_frame_ms: 20.0,
            active_threads: 4,
        };
        assert_eq!(stats.progress_pct(), 50.0);
        assert!((stats.fps() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_parallel_render_executes() {
        let counter = AtomicUsize::new(0);
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Test".into(),
            start_frame: 0,
            end_frame: 3,
            output_path: "/tmp/test.mp4".into(),
            status: RenderStatus::Pending,
        });
        queue.render_all(|_comp, _frame| {
            counter.fetch_add(1, Ordering::Relaxed);
            vec![0u8; 4]
        });
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }
}
