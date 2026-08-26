pub mod core;

#[cfg(feature = "gui")]
pub mod ui;

pub mod app_state;

#[cfg(feature = "gui")]
pub use app_state::AfterEffectsApp;
pub use app_state::DragTransaction;

pub use core::ffmpeg_export::ExportEvent;
pub use core::automation;
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
