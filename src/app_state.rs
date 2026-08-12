use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::{ExportEvent, TrackerEvent};

/// Playback domain state: transport, current frame, work area bounds, volume.
#[derive(Debug, Clone)]
pub struct PlaybackDomainState {
    pub is_playing: bool,
    pub current_frame: u32,
    pub master_volume: f32,
    pub work_area_in: Option<u32>,
    pub work_area_out: Option<u32>,
}

impl Default for PlaybackDomainState {
    fn default() -> Self {
        Self {
            is_playing: false,
            current_frame: 0,
            master_volume: 1.0,
            work_area_in: None,
            work_area_out: None,
        }
    }
}

/// Selection domain state: layer multi-selection, selected property path, expanded hierarchy.
#[derive(Debug, Clone, Default)]
pub struct SelectionDomainState {
    pub selected_layer_idx: Option<usize>,
    pub selected_layers: HashSet<usize>,
    pub selected_property: Option<String>,
    pub expanded_layers: HashSet<usize>,
}

impl SelectionDomainState {
    pub fn select_single(&mut self, idx: usize) {
        self.selected_layers.clear();
        self.selected_layers.insert(idx);
        self.selected_layer_idx = Some(idx);
    }

    pub fn toggle_select(&mut self, idx: usize) {
        if self.selected_layers.contains(&idx) {
            self.selected_layers.remove(&idx);
            if self.selected_layer_idx == Some(idx) {
                self.selected_layer_idx = self.selected_layers.iter().next().copied();
            }
        } else {
            self.selected_layers.insert(idx);
            self.selected_layer_idx = Some(idx);
        }
    }

    pub fn clear(&mut self) {
        self.selected_layers.clear();
        self.selected_layer_idx = None;
    }
}

/// UI Tabs domain state: tab indices, dock panels, shy filter, search query.
#[derive(Debug, Clone)]
pub struct UiTabsDomainState {
    pub left_tab_idx: usize,
    pub right_tab_idx: usize,
    pub bottom_dock_tab: usize,
    pub viewport_mag_ratio: f32,
    pub show_switches_pane: bool,
    pub global_shy_active: bool,
    pub layer_filter_text: String,
    pub effects_search_query: String,
}

impl Default for UiTabsDomainState {
    fn default() -> Self {
        Self {
            left_tab_idx: 0,
            right_tab_idx: 0,
            bottom_dock_tab: 0,
            viewport_mag_ratio: 1.0,
            show_switches_pane: true,
            global_shy_active: false,
            layer_filter_text: String::new(),
            effects_search_query: String::new(),
        }
    }
}

/// Export domain state: async render background progress, output path, settings.
pub struct ExportDomainState {
    pub show_export_dialog: bool,
    pub export_status: Option<String>,
    pub export_progress: f32,
    pub export_fps: u32,
    pub export_output_path: String,
    pub is_exporting: bool,
    pub export_rx: Option<Receiver<ExportEvent>>,
    pub tracker_rx: Option<Receiver<TrackerEvent>>,
}

impl Default for ExportDomainState {
    fn default() -> Self {
        Self {
            show_export_dialog: false,
            export_status: None,
            export_progress: 0.0,
            export_fps: 30,
            export_output_path: "output.mp4".to_string(),
            is_exporting: false,
            export_rx: None,
            tracker_rx: None,
        }
    }
}
