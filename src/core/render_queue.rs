//! Render Queue & Background Batch Export Manager.
//!
//! Provides After Effects-style batch render management for multiple compositions,
//! output presets (ProRes, H.264, PNG Sequence, GIF, Lottie), frame ranges,
//! multi-threaded rendering queues, and progress tracking.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported export codec / container formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderExportFormat {
    Mp4H264,
    ProRes422,
    ProRes4444,
    Gif,
    PngSequence,
    LottieJson,
}

impl RenderExportFormat {
    pub fn default_extension(&self) -> &'static str {
        match self {
            Self::Mp4H264 => "mp4",
            Self::ProRes422 | Self::ProRes4444 => "mov",
            Self::Gif => "gif",
            Self::PngSequence => "png",
            Self::LottieJson => "json",
        }
    }
}

/// Status of an individual render queue item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderStatus {
    Queued,
    Rendering { progress: f32, current_frame: u32, total_frames: u32 },
    Completed { elapsed_sec: f64 },
    Failed(String),
    Paused,
}

/// An individual render job queued for batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderQueueItem {
    pub id: String,
    pub comp_id: String,
    pub comp_name: String,
    pub output_path: PathBuf,
    pub format: RenderExportFormat,
    pub start_frame: u32,
    pub end_frame: u32,
    pub resolution_scale: f32,
    pub status: RenderStatus,
    pub time_created: u64,
}

impl RenderQueueItem {
    pub fn new(
        comp_id: String,
        comp_name: String,
        output_path: PathBuf,
        format: RenderExportFormat,
        start_frame: u32,
        end_frame: u32,
    ) -> Self {
        let id = format!("job_{}_{}", comp_id, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
        Self {
            id,
            comp_id,
            comp_name,
            output_path,
            format,
            start_frame,
            end_frame,
            resolution_scale: 1.0,
            status: RenderStatus::Queued,
            time_created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        }
    }

    pub fn total_frames(&self) -> u32 {
        if self.end_frame >= self.start_frame {
            self.end_frame - self.start_frame + 1
        } else {
            1
        }
    }
}

/// Central Render Queue managing active and pending batch exports.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderQueue {
    pub items: Vec<RenderQueueItem>,
    pub is_running: bool,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            is_running: false,
        }
    }

    /// Add a new render job to the queue.
    pub fn add_item(&mut self, item: RenderQueueItem) {
        self.items.push(item);
    }

    /// Remove a render job by ID.
    pub fn remove_item(&mut self, item_id: &str) -> bool {
        let len_before = self.items.len();
        self.items.retain(|i| i.id != item_id);
        self.items.len() < len_before
    }

    /// Clear all completed or failed items.
    pub fn clean_completed(&mut self) {
        self.items.retain(|i| matches!(i.status, RenderStatus::Queued | RenderStatus::Rendering { .. } | RenderStatus::Paused));
    }

    /// Reset failed items back to Queued state for retry.
    pub fn retry_failed(&mut self) {
        for item in &mut self.items {
            if matches!(item.status, RenderStatus::Failed(_)) {
                item.status = RenderStatus::Queued;
            }
        }
    }

    /// Calculate aggregate queue progress (0.0 to 1.0).
    pub fn aggregate_progress(&self) -> f32 {
        if self.items.is_empty() {
            return 0.0;
        }
        let mut total_prog = 0.0f32;
        for item in &self.items {
            match &item.status {
                RenderStatus::Completed { .. } => total_prog += 1.0,
                RenderStatus::Rendering { progress, .. } => total_prog += progress.clamp(0.0, 1.0),
                _ => {},
            }
        }
        (total_prog / self.items.len() as f32).clamp(0.0, 1.0)
    }

    /// Count items in Queued state.
    pub fn pending_count(&self) -> usize {
        self.items.iter().filter(|i| matches!(i.status, RenderStatus::Queued)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_queue_lifecycle() {
        let mut queue = RenderQueue::new();
        assert_eq!(queue.pending_count(), 0);

        let item1 = RenderQueueItem::new(
            "comp1".into(),
            "Main Comp".into(),
            PathBuf::from("/tmp/out1.mp4"),
            RenderExportFormat::Mp4H264,
            0,
            100,
        );
        let item1_id = item1.id.clone();
        queue.add_item(item1);
        assert_eq!(queue.pending_count(), 1);
        assert_eq!(queue.aggregate_progress(), 0.0);

        // Simulate item completion
        queue.items[0].status = RenderStatus::Completed { elapsed_sec: 12.5 };
        assert_eq!(queue.aggregate_progress(), 1.0);
        assert_eq!(queue.pending_count(), 0);

        // Add second item rendering at 50%
        let item2 = RenderQueueItem::new(
            "comp2".into(),
            "Teaser".into(),
            PathBuf::from("/tmp/out2.mov"),
            RenderExportFormat::ProRes422,
            0,
            50,
        );
        queue.add_item(item2);
        queue.items[1].status = RenderStatus::Rendering { progress: 0.5, current_frame: 25, total_frames: 50 };

        // Average progress of 100% and 50% across 2 items is 75%
        assert!((queue.aggregate_progress() - 0.75).abs() < 1e-4);

        // Remove item1
        assert!(queue.remove_item(&item1_id));
        assert_eq!(queue.items.len(), 1);
    }
}
