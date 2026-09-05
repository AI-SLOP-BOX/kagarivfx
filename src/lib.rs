#![warn(unused_imports)]
#![warn(unused_variables)]
#![warn(dead_code)]
#![warn(unreachable_code)]
#![warn(unreachable_patterns)]
// Style lints: allow at crate level — widespread patterns across 100+ files.
// Fix incrementally, not in a single hardening pass.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::unused_io_amount)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::single_match)]
#![allow(clippy::chunks_exact_to_as_chunks)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::obfuscated_if_else)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::identity_op)]
#![allow(clippy::useless_vec)]
#![allow(clippy::manual_range_contains)]

pub mod core;

#[cfg(feature = "gui")]
pub mod ui;

#[cfg(feature = "gui")]
pub mod app_state;

#[cfg(feature = "gui")]
pub use app_state::DragTransaction;
#[cfg(feature = "gui")]
pub use app_state::KagariApp;

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
