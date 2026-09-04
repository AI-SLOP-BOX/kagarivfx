#![allow(warnings)]

// ── CRITICAL: re-enable these lint categories as the codebase stabilizes ──
// Once the codebase is clean enough, replace the blanket allow above with:
//   #![warn(clippy::correctness)]
//   #![warn(clippy::panic)]
//   #![warn(clippy::suspicious)]
// and fix the resulting warnings. Then remove #![allow(warnings)] entirely.

pub mod core;

#[cfg(feature = "gui")]
pub mod ui;

#[cfg(feature = "gui")]
pub mod app_state;

#[cfg(feature = "gui")]
pub use app_state::KagariApp;
#[cfg(feature = "gui")]
pub use app_state::DragTransaction;

pub use core::automation;
pub use core::ffmpeg_export::ExportEvent;
pub use core::timeline::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewportMode {
    Comp2D,
    Camera3D,
}

#[derive(Debug, Clone)]
pub enum TrackerEvent {
    Progress(f32, String),
    Finished {
        layer_id: String,
        layer_idx: usize,
        tracker_idx: usize,
        keyframes: Vec<core::keyframe::Keyframe<[f32; 2]>>,
    },
    Error(String),
}
