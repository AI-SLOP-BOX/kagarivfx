use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::{ExportEvent, TrackerEvent, ViewportMode};
use crate::core::timeline::Project;

/// Mixer channel state for the audio mixer panel.
#[derive(Debug, Clone, Copy)]
pub struct MixerChannel {
    /// Gain in dB (-60..+12)
    pub gain_db: f32,
    /// Pan position (-100 left .. 0 center .. +100 right)
    pub pan: f32,
    /// Mute toggle — when true, this channel is silenced in the mix
    pub mute: bool,
    /// Solo toggle — when any channel is soloed, only soloed channels are heard
    pub solo: bool,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self { gain_db: 0.0, pan: 0.0, mute: false, solo: false }
    }
}

/// Playback domain state: transport, current frame, work area bounds, volume.
#[derive(Debug, Clone)]
pub struct PlaybackDomainState {
    pub is_playing: bool,
    /// Adaptive preview quality: multiplier (0.125..=1.0) applied to the viewport
    /// render width while playing. When frames take longer than the playback
    /// budget, the factor drops so playback stays smooth; it recovers when fast.
    /// This mirrors AE's automatic resolution reduction during RAM preview.
    pub adaptive_preview_factor: f32,
    /// Viewport pan offset in screen pixels (used with zoom != Fit).
    pub viewport_pan: eframe::egui::Vec2,
    /// Exponential moving average of GPU render time in milliseconds.
    pub preview_render_ema_ms: f32,
    pub current_frame: u32,
    pub master_volume: f32,
    pub work_area_in: Option<u32>,
    pub work_area_out: Option<u32>,
}

impl Default for PlaybackDomainState {
    fn default() -> Self {
        Self {
            is_playing: false,
            adaptive_preview_factor: 1.0,
            viewport_pan: eframe::egui::Vec2::ZERO,
            preview_render_ema_ms: 0.0,
            current_frame: 0,
            master_volume: 1.0,
            work_area_in: None,
            work_area_out: None,
        }
    }
}

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

#[derive(Clone)]
pub struct DragTransaction {
    pub snapshot: Project,
    pub label: &'static str,
}

impl DragTransaction {
    pub fn new(snapshot: Project, label: &'static str) -> Self {
        Self { snapshot, label }
    }
}

#[cfg(feature = "gui")]
pub struct AfterEffectsApp {
    pub history: crate::core::history::ProjectHistory,
    /// Crash-recovery autosave manager
    pub autosave: crate::core::autosave::AutosaveManager,
    /// Show the crash-recovery restore prompt at startup
    pub show_recovery_dialog: bool,
    /// Ensures the startup recovery check runs only once
    pub recovery_checked: bool,
    /// Human-readable timestamp of the recovery snapshot (for the dialog)
    pub recovery_snapshot_time: Option<String>,
    pub is_playing: bool,
    /// Loop playback at the work area / comp end (Preview panel toggle)
    pub loop_playback: bool,
    /// Adaptive preview quality: multiplier (0.125..=1.0) applied to the viewport
    /// render width while playing. When frames take longer than the playback
    /// budget, the factor drops so playback stays smooth; it recovers when fast.
    /// This mirrors AE's automatic resolution reduction during RAM preview.
    pub adaptive_preview_factor: f32,
    /// Viewport pan offset in screen pixels (used with zoom != Fit).
    pub viewport_pan: eframe::egui::Vec2,
    /// Exponential moving average of GPU render time in milliseconds.
    pub preview_render_ema_ms: f32,
    pub current_frame: u32,
    pub playback_speed: i32,
    pub u_key_last_press: Option<f64>,
    pub selected_layer_idx: Option<usize>,
    pub selected_layers: std::collections::HashSet<usize>,
    /// Selected keyframes: (layer_idx, property key, frame).
    pub selected_keyframes: std::collections::HashSet<(usize, String, u32)>,
    pub drag_tx: Option<DragTransaction>,
    /// Effect being dragged from the Effects & Presets library.
    /// Stores (effect_name, create_fn_index) while dragging, None otherwise.
    pub dragging_effect: Option<(String, usize)>,
    #[cfg(feature = "wgpu")]
    pub renderer: Option<crate::core::renderer::WgpuRenderer>,
    #[cfg(feature = "wgpu")]
    pub wgpu_state: Option<eframe::egui_wgpu::RenderState>,
    pub viewport_texture_id: Option<eframe::egui::TextureId>,
    /// RAM preview: (frame -> egui texture id) entries for pre-rendered frames.
    pub ram_texture_ids: Vec<(u32, eframe::egui::TextureId)>,
    /// True if the previous frame had playback active — used to detect playback
    /// start and kick off the RAM preview pre-pass.
    pub was_playing_last_frame: bool,
    /// Internal clipboard for copied keyframes: (prop_key, offset-from-anchor frame, value+interpolation JSON).
    /// Values are stored as serde_json::Value to stay type-erased across tracks.
    pub kf_clipboard: Vec<(String, i32, serde_json::Value)>,
    /// Frame the clipboard anchor was at when copied (paste preserves relative spacing).
    pub kf_clipboard_anchor: u32,
    /// Incremental RAM preview pre-pass state: next frame to pre-render.
    pub ram_prepass_cursor: u32,
    /// Last frame (inclusive) of the current pre-pass.
    pub ram_prepass_end: u32,
    pub viewport_snapshot_texture_id: Option<eframe::egui::TextureId>,
    pub rx_frame: Option<std::sync::mpsc::Receiver<u32>>,
    pub rx_connection: Option<std::sync::mpsc::Receiver<Option<String>>>,
    pub connected_app: Option<String>,
    pub project_path: String,
    pub otio_path: String,
    pub expanded_layers: std::collections::HashSet<usize>,
    pub expanded_waveform_layers: std::collections::HashSet<usize>,
    /// Breadcrumb trail of composition indices visited via PreComp
    /// double-click navigation (most recent = last). Back = pop.
    pub comp_nav_stack: Vec<usize>,
    pub timeline_fit_to_selection: bool,
    pub timeline_fit_all: bool,
    /// Index of the layer currently being drag-reordered in the timeline.
    pub dragging_layer: Option<usize>,
    pub show_grid: bool,
    pub show_guides: bool,
    pub show_handles: bool,
    pub show_comp_settings: bool,
    pub show_shortcuts_dialog: bool,
    pub show_precompose_dialog: bool,
    pub show_sequence_layers: bool,
    pub show_the_smoother: bool,
    pub show_the_wiggler: bool,
    pub show_motion_sketch: bool,
    pub precompose_name: String,
    pub precompose_move_attributes: bool,
    pub show_command_palette: bool,
    /// 📊 Vectorscope / RGB Parade floating scope window (Shift+F4)
    pub show_vectorscope: bool,
    pub show_history_panel: bool,
    pub show_welcome: bool,
    /// Skill-level UI mode: Beginner hides advanced panels/menus.
    pub ui_mode: crate::ui::mode::UiMode,
    /// Interactive tutorial state (None = closed).
    pub tutorial: Option<crate::ui::tutorial::TutorialState>,
    pub show_new_comp_dialog: bool,
    pub show_preferences: bool,
    /// Audio preview during playback (Preferences).
    pub audio_preview_enabled: bool,
    /// Handle to the running egui Context (set each frame; used by prefs).
    pub ui_ctx: Option<eframe::egui::Context>,
    pub command_palette_search: String,
    pub command_palette_selected_idx: usize,
    pub snap_to_keyframes: bool,
    pub show_graph_editor: bool,
    pub linked_tangent: bool,
    pub timeline_zoom: f32,
    /// Left edge (frame) of the visible timeline window. Stays fixed while
    /// scrubbing so the ruler does not slide under the cursor; only re-centers
    /// when the playhead leaves the visible range or via navigation keys.
    pub timeline_view_start: u32,
    pub timeline_scroll: f32,
    pub viewport_mode: ViewportMode,
    pub camera_orbit: (f32, f32, f32),
    pub orbit_drag_start: Option<eframe::egui::Pos2>,
    pub active_tool: crate::ui::toolbar::ActiveTool,
    pub viewport_drag_state: Option<(usize, [f32; 2], eframe::egui::Pos2)>,
    /// One-shot request: frame this comp-space bbox (min,max) in the viewport.
    pub viewport_focus_bbox: Option<([f32; 2], [f32; 2])>,
    pub viewport_mask_drag_state: Option<(usize, usize, usize, [f32; 2], eframe::egui::Pos2)>,
    /// Spatial drag of a position keyframe dot on the motion path:
    /// (layer_idx, keyframe_frame, start_value, start_pointer)
    pub viewport_pos_kf_drag_state: Option<(usize, u32, [f32; 2], eframe::egui::Pos2)>,
    /// (layer_idx, kf_frame, handle_type: 0=out/1=in, start_bezier_pt, start_pointer)
    pub viewport_tangent_drag_state: Option<(usize, u32, u8, [f32; 2], eframe::egui::Pos2)>,
    pub viewport_linked_tangent: bool,
    /// Pick whip mode: when true, clicking a layer sets it as parent
    pub pick_whip_mode: bool,
    /// Layer index being picked as parent
    pub pick_whip_target: Option<usize>,
    /// Corner-handle scale drag: (layer_idx, start_scale, start_pointer_distance)
    pub viewport_scale_drag: Option<(usize, [f32; 2], f32)>,
    /// Multi-layer group drag: (per-layer start positions, start pointer)
    #[allow(clippy::type_complexity)]
    pub viewport_multi_drag: Option<(Vec<(usize, [f32; 2])>, eframe::egui::Pos2)>,
    /// Rectangle tool rubber-band: (start pointer position in screen space)
    pub rect_drag_start: Option<eframe::egui::Pos2>,
    /// Layer index whose text is being edited inline from the viewport (double-click)
    pub inline_text_edit_layer: Option<usize>,
    /// Pen tool: in-progress mask vertices in composition coordinates
    pub pen_points: Vec<[f32; 2]>,
    /// Motion Sketch: records position keyframes while playing + dragging.
    pub motion_sketch_active: bool,
    /// Motion Sketch recording buffer: (frame, [x, y]) pairs captured during drag.
    pub motion_sketch_recording: Vec<(u32, [f32; 2])>,
    pub selected_property: Option<String>,
    pub show_export_dialog: bool,
    pub export_status: Option<String>,
    pub export_progress: f32,
    pub export_fps: u32,
    pub export_output_path: String,
    pub is_exporting: bool,
    pub export_format_preset: usize,
    /// Video codec selection shared by Export dialog + Render presets (0=H264,1=ProRes422,2=ProRes4444)
    pub export_codec_idx: usize,
    pub export_resolution_scale: usize,
    pub export_rx: Option<std::sync::mpsc::Receiver<ExportEvent>>,
    pub import_rx: Option<std::sync::mpsc::Receiver<crate::ui::drop_import::ImportResult>>,
    /// Path of the most recent export (for the "Reveal in Finder" button).
    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub last_export_path: Option<String>,
    pub export_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub tracker_rx: Option<std::sync::mpsc::Receiver<TrackerEvent>>,
    pub tracker_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub master_volume: f32,
    pub master_eq_highpass: f32,
    pub master_eq_lowpass: f32,
    pub master_eq_mid_gain: f32,
    pub master_eq_mid_freq: f32,
    pub master_comp_threshold: f32,
    pub master_comp_ratio: f32,
    pub master_comp_attack: f32,
    pub master_comp_release: f32,
    pub master_comp_makeup: f32,
    pub left_tab_idx: usize,
    pub right_tab_idx: usize,
    pub viewport_mag_ratio: f32,
    pub viewport_cam_view: usize,
    pub viewport_render_resolution: usize,
    pub viewport_color_channel: usize,
    pub viewport_fast_preview: usize,
    pub work_area_in: Option<u32>,
    pub work_area_out: Option<u32>,
    pub layer_filter_text: String,
    pub bottom_dock_tab: usize,
    pub show_switches_pane: bool,
    pub global_shy_active: bool,
    pub effects_search_query: String,
    pub project_search_query: String,
    pub cc_libraries_search: String,
    pub audio_mixer_channels: Vec<MixerChannel>,
    /// Synced audio playback for AV preview (gui builds with an audio device).
    #[cfg(feature = "gui")]
    pub audio_playback: Option<crate::core::audio_playback::AudioPlayback>,
    /// Live audio meter (linear 0..1) from the mix, updated each UI frame.
    pub audio_meter: (f32, f32),
    /// Whether the last viewport render used GPU (updated each frame by viewport)
    pub gpu_rendered: bool,
    /// Layer index being renamed (inline edit), None = not renaming
    pub renaming_layer: Option<usize>,
    /// Tracker panel: target layer index for Apply Motion.
    pub tracker_apply_target: Option<usize>,
    /// Real render queue entries (composition names awaiting export).
    pub render_queue_items: Vec<String>,
    /// Sequential batch export: remaining comps to render after the current one.
    pub batch_queue: Vec<String>,
    /// Index of the comp currently rendering within the original queue snapshot.
    pub batch_idx: usize,
    pub camera_view_layout: usize,
    pub camera_view_angle: usize,
    pub font_family_idx: usize,
    pub faux_font_switches: (bool, bool, bool, bool),
    pub toasts: crate::ui::notification::ToastManager,
    pub frame_cache: crate::core::frame_cache::FrameCache,
    pub lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator,
    /// Panel animation states for smooth open/close transitions
    pub inspector_animation: crate::ui::panel_animation::PanelAnimation,
    pub effects_animation: crate::ui::panel_animation::PanelAnimation,
    /// Scripting console state
    pub script_console_output: Option<Vec<String>>,
    pub script_console_history: Option<Vec<String>>,
    pub script_console_command: String,
    /// Color management settings
    pub color_space_idx: usize,
    pub bit_depth_idx: usize,
    pub display_sim_idx: usize,
    /// Custom saved workspaces
    pub custom_workspaces: Vec<crate::ui::workspace_manager::SavedWorkspace>,
    /// Selected expression property index
    pub selected_expression_prop_idx: usize,
}

#[cfg(feature = "gui")]
impl Default for AfterEffectsApp {
    fn default() -> Self {
        
        Self {
            history: crate::core::history::ProjectHistory::new(Project::default()),
            autosave: crate::core::autosave::AutosaveManager::new(
                std::env::temp_dir().join("aevfx_recovery"),
            ),
            show_recovery_dialog: false,
            recovery_checked: false,
            recovery_snapshot_time: None,
            is_playing: false,
            loop_playback: true,
            adaptive_preview_factor: 1.0,
            viewport_pan: eframe::egui::Vec2::ZERO,
            preview_render_ema_ms: 0.0,
            current_frame: 0,
            playback_speed: 1,
            u_key_last_press: None,
            selected_layer_idx: Some(1),
            selected_layers: vec![1].into_iter().collect(),
            selected_keyframes: std::collections::HashSet::new(),
            drag_tx: None,
            dragging_effect: None,
            #[cfg(feature = "wgpu")]
            renderer: None,
            #[cfg(feature = "wgpu")]
            wgpu_state: None,
            viewport_texture_id: None,
            ram_texture_ids: Vec::new(),
            was_playing_last_frame: false,
            kf_clipboard: Vec::new(),
            kf_clipboard_anchor: 0,
            ram_prepass_cursor: u32::MAX,
            ram_prepass_end: u32::MAX,
            viewport_snapshot_texture_id: None,
            rx_frame: None,
            rx_connection: None,
            connected_app: None,
            project_path: "project.aevfx.json".to_string(),
            otio_path: "timeline.otio.json".to_string(),
            expanded_layers: std::collections::HashSet::new(),
            expanded_waveform_layers: std::collections::HashSet::new(),
            comp_nav_stack: Vec::new(),
            timeline_fit_to_selection: false,
            timeline_fit_all: false,
            show_grid: false,
            show_guides: true,
            show_handles: true,
            show_comp_settings: false,
            show_shortcuts_dialog: false,
            show_precompose_dialog: false,
            show_sequence_layers: false,
            show_the_smoother: false,
            show_the_wiggler: false,
            show_motion_sketch: false,
            precompose_name: String::new(),
            precompose_move_attributes: true,
            snap_to_keyframes: true,
            dragging_layer: None,
            show_graph_editor: false,
            linked_tangent: true,
            timeline_zoom: 1.0,
            timeline_view_start: 0,
            timeline_scroll: 0.0,
            viewport_mode: ViewportMode::Comp2D,
            camera_orbit: (30.0, 20.0, 800.0),
            orbit_drag_start: None,
            active_tool: crate::ui::toolbar::ActiveTool::default(),
            viewport_drag_state: None,
            viewport_focus_bbox: None,
            viewport_mask_drag_state: None,
            viewport_pos_kf_drag_state: None,
            viewport_tangent_drag_state: None,
            viewport_linked_tangent: true,
            pick_whip_mode: false,
            pick_whip_target: None,
            viewport_scale_drag: None,
            viewport_multi_drag: None,
            rect_drag_start: None,
            inline_text_edit_layer: None,
            pen_points: Vec::new(),
            motion_sketch_active: false,
            motion_sketch_recording: vec![],
            selected_property: None,
            show_export_dialog: false,
            export_status: None,
            export_progress: 0.0,
            export_fps: 30,
            export_output_path: "output.mp4".to_string(),
            is_exporting: false,
            export_format_preset: 0,
            export_codec_idx: 0,
            export_resolution_scale: 0,
            export_rx: None,
            export_cancel_flag: None,
            tracker_rx: None,
            tracker_cancel_flag: None,
            master_volume: 0.8,
            master_eq_highpass: 30.0,
            master_eq_lowpass: 18000.0,
            master_eq_mid_gain: 0.0,
            master_eq_mid_freq: 1000.0,
            master_comp_threshold: -12.0,
            master_comp_ratio: 2.0,
            master_comp_attack: 10.0,
            master_comp_release: 100.0,
            master_comp_makeup: 0.0,
            left_tab_idx: 0,
            right_tab_idx: 0,
            viewport_mag_ratio: 1.0,
            viewport_cam_view: 0,
            viewport_render_resolution: 0,
            viewport_color_channel: 0,
            viewport_fast_preview: 0,
            work_area_in: None,
            work_area_out: None,
            layer_filter_text: String::new(),
            bottom_dock_tab: 0,
            show_switches_pane: true,
            global_shy_active: false,
            effects_search_query: String::new(),
            project_search_query: String::new(),
            cc_libraries_search: String::new(),
            audio_mixer_channels: Vec::new(),
            audio_playback: crate::core::audio_playback::AudioPlayback::new().ok(),
            audio_meter: (0.0, 0.0),
            gpu_rendered: false,
            renaming_layer: None,
            tracker_apply_target: None,
            render_queue_items: Vec::new(),
            batch_queue: Vec::new(),
            batch_idx: 0,
            camera_view_layout: 0,
            camera_view_angle: 0,
            font_family_idx: 0,
            faux_font_switches: (false, false, false, false),
            show_command_palette: false,
            show_vectorscope: false,
            show_history_panel: false,
            show_welcome: true,
            ui_mode: crate::ui::mode::load_mode(),
            tutorial: None,
            show_new_comp_dialog: false,
            show_preferences: false,
            audio_preview_enabled: true,
            ui_ctx: None,
            import_rx: None,
            last_export_path: None,
            command_palette_search: String::new(),
            command_palette_selected_idx: 0,
            toasts: crate::ui::notification::ToastManager::new(),
            frame_cache: crate::core::frame_cache::FrameCache::new(256),
            lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator::new(),
            inspector_animation: crate::ui::panel_animation::PanelAnimation::new(true),
            effects_animation: crate::ui::panel_animation::PanelAnimation::new(true),
            script_console_output: None,
            script_console_history: None,
            script_console_command: String::new(),
            color_space_idx: 0,
            bit_depth_idx: 2,
            display_sim_idx: 0,
            custom_workspaces: Vec::new(),
            selected_expression_prop_idx: 0,
        }
    }
}

#[cfg(feature = "gui")]
impl AfterEffectsApp {
    pub fn modify_project(&mut self, f: impl FnOnce(&mut Project)) {
        let mut next_project = self.history.current().clone();
        f(&mut next_project);
        self.history.commit(next_project);
        self.autosave.mark_dirty();
        crate::core::frame_cache::bump_version();
        self.frame_cache.collect_garbage();
    }

    pub fn begin_drag(&mut self, label: &'static str) {
        if self.drag_tx.is_some() {
            // Unbalanced re-begin: seal the previous gesture as its own undo
            // entry instead of letting one transaction swallow multiple
            // gestures (which would produce a single giant undo step).
            self.commit_drag();
        }
        self.drag_tx = Some(DragTransaction::new(
            self.history.current().clone(),
            label,
        ));
    }

    /// Whether a drag transaction is currently open.
    pub fn drag_active(&self) -> bool {
        self.drag_tx.is_some()
    }

    pub fn commit_drag(&mut self) {
        if self.drag_tx.take().is_some() {
            let current = self.history.current().clone();
            self.history.commit(current);
            crate::core::frame_cache::bump_version();
            self.frame_cache.collect_garbage();
        }
    }

    pub fn cancel_drag(&mut self) {
        if let Some(tx) = self.drag_tx.take() {
            self.history.commit(tx.snapshot);
            crate::core::frame_cache::bump_version();
        }
    }
}

#[cfg(feature = "gui")]
impl eframe::App for AfterEffectsApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use eframe::egui;

        // Re-assert the dark AE theme every frame (cheap, and guards against
        // eframe's system-theme following resetting visuals)
        crate::ui::theme::configure_ae_theme(ctx);

        // Global drag safety net: any pointer release while a transaction is
        // open seals it, even if the owning widget missed the release event
        // (prevents permanently-open transactions from swallowing edits).
        if self.drag_active() && ctx.input(|i| i.pointer.any_released()) {
            self.commit_drag();
        }

        // One-time startup check for crash recovery snapshots.
        // Only prompt if a "dirty exit" marker exists — written on startup and
        // removed on clean shutdown. This prevents false positives when the
        // user simply closed the app normally with autosave files still present.
        if !self.recovery_checked {
            self.recovery_checked = true;
            let dirty_exit_marker = std::env::temp_dir().join("aevfx_dirty_exit");
            let had_crash = dirty_exit_marker.exists();
            // Write the marker for THIS session
            let _ = std::fs::write(&dirty_exit_marker, std::process::id().to_string());
            if had_crash && self.autosave.has_recovery() {
                self.recovery_snapshot_time = std::fs::metadata(
                    std::env::temp_dir().join("aevfx_recovery").join("recovery_0.json"),
                )
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    format!("#{}", secs)
                });
                self.show_recovery_dialog = true;
            }
        }

        // Crash-recovery autosave: write a rotating snapshot when dirty and due
        if let Some(path) = self.autosave.tick(self.history.current()) {
            log::info!("[Autosave] Recovery snapshot written: {:?}", path);
        }

        // ── Synced audio playback: follow the playhead while playing ──
        {
            // First video layer's extracted WAV drives the preview audio
            let wav = self
                .history
                .current()
                .active_composition()
                .layers
                .iter()
                .find_map(|l| match &l.layer_type {
                    crate::core::timeline::LayerType::Video { audio_wav, .. } => audio_wav.clone(),
                    _ => None,
                });
            let audio_enabled = self.audio_preview_enabled;
            let fps = self.history.current().active_composition().fps.max(1);
            let playhead_sec = self.current_frame as f32 / fps as f32;

            // Live metering: mix a tiny buffer at the playhead with mixer gains
            {
                let project = self.history.current();
                let comp = project.active_composition();
                let dsp = crate::core::audio_engine::MasterDspParams {
                    eq_highpass: self.master_eq_highpass,
                    eq_lowpass: self.master_eq_lowpass,
                    eq_mid_gain: self.master_eq_mid_gain,
                    eq_mid_freq: self.master_eq_mid_freq,
                    comp_threshold: self.master_comp_threshold,
                    comp_ratio: self.master_comp_ratio,
                    comp_attack: self.master_comp_attack,
                    comp_release: self.master_comp_release,
                    comp_makeup: self.master_comp_makeup,
                };
                let (_mix, meter) = crate::core::audio_engine::mix_audio_sources_for_frame(
                    comp,
                    self.current_frame,
                    48000,
                    64, // small buffer — metering only
                    Some(&self.audio_mixer_channels),
                    &dsp,
                );
                let db_to_lin = |db: f32| 10.0f32.powf(db / 20.0).min(1.0);
                self.audio_meter = (
                    db_to_lin(meter.peak_db_left),
                    db_to_lin(meter.peak_db_right),
                );
            }

            match (self.is_playing, audio_enabled, wav, &mut self.audio_playback) {
                (true, true, Some(wav), Some(playback)) => {
                    if let Err(e) = playback.play(&std::path::PathBuf::from(wav), playhead_sec) {
                        log::warn!("[Audio] playback error: {}", e);
                    }
                }
                _ => {
                    if let Some(playback) = &mut self.audio_playback {
                        if playback.is_playing() {
                            playback.pause();
                        }
                    }
                }
            }
            // Master volume applies to the sink output
            if let Some(playback) = &self.audio_playback {
                playback.set_volume(self.master_volume);
            }
        }

        if let Some(rx_frame) = &self.rx_frame {
            while let Ok(frame) = rx_frame.try_recv() {
                self.current_frame = frame;
            }
        }
        if let Some(rx_connection) = &self.rx_connection {
            while let Ok(conn) = rx_connection.try_recv() {
                self.connected_app = conn;
            }
        }

        let total_frames = self.history.current().active_composition().duration_frames;
        let mut current_frame = self.current_frame;

        let wa_start = self.work_area_in.unwrap_or(0);
        let wa_end = self.work_area_out.unwrap_or(total_frames.saturating_sub(1)).min(total_frames.saturating_sub(1));

        if self.is_playing {
            let speed = self.playback_speed.abs().max(1) as u32;
            if self.playback_speed >= 0 {
                let next = current_frame.saturating_add(speed);
                current_frame = if next >= wa_end {
                    if self.loop_playback {
                        // Wrap to work-area start (or comp start)
                        if self.work_area_in.is_some() || self.work_area_out.is_some() { wa_start } else { 0 }
                    } else {
                        // Stop at the end when looping is off
                        self.is_playing = false;
                        self.motion_sketch_active = false;
                        wa_end
                    }
                } else {
                    next
                };
            } else {
                current_frame = if current_frame == wa_start {
                    if self.loop_playback {
                        wa_end.saturating_sub(1)
                    } else {
                        self.is_playing = false;
                        self.motion_sketch_active = false;
                        wa_start
                    }
                } else {
                    current_frame.saturating_sub(speed)
                };
            }
            let fps = self.history.current().active_composition().fps.max(1);
            let effective_fps = (fps as f32 / speed.max(1) as f32).max(1.0);
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.0 / effective_fps));
        }

        self.ui_ctx = Some(ctx.clone());
        if !self.recovery_checked {
            self.recovery_checked = true;
            crate::ui::preferences_dialog::apply_loaded(self);
        }
        crate::ui::shortcuts::handle_global_shortcuts(self, ctx, &mut current_frame, total_frames);
        crate::ui::menu::draw(self, ctx);
        crate::ui::toolbar::draw(self, ctx);
        crate::ui::inspector::draw(self, ctx, &mut current_frame);
        crate::ui::effects_library::draw(self, ctx, &mut current_frame);
        crate::ui::timeline::draw(self, ctx, &mut current_frame, total_frames);

        let status_frame = egui::Frame::none()
            .fill(crate::ui::theme::colors::BG_DARKEST)
            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
            .stroke(egui::Stroke::new(1.0, crate::ui::theme::colors::BORDER_SUBTLE));

        egui::TopBottomPanel::bottom("ae_status_bar")
            .frame(status_frame)
            .default_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 6.0;
                    let (gpu_label, gpu_color) = if self.gpu_rendered {
                        ("● Metal GPU Render Engine", crate::ui::theme::colors::ACCENT_GREEN)
                    } else {
                        ("○ CPU Software Renderer", crate::ui::theme::colors::ACCENT_ORANGE)
                    };
                    ui.label(egui::RichText::new(gpu_label).small().color(gpu_color));
                    ui.separator();
                    // Timecode
                    let fps = self.history.current().active_composition().fps.max(1);
                    let cf = self.current_frame;
                    ui.label(egui::RichText::new(format!(
                        "TC: {:02}:{:02}:{:02}:{:02}",
                        cf / (fps * 3600), (cf / fps) / 60 % 60,
                        (cf / fps) % 60, cf % fps
                    )).monospace().small().color(crate::ui::theme::colors::ACCENT_YELLOW));
                    ui.separator();
                    ui.label(egui::RichText::new(format!("Frame: {} / {}", cf, total_frames)).small().color(crate::ui::theme::colors::TEXT_SECONDARY));
                    ui.separator();
                    ui.label(egui::RichText::new("16-bpc | Rec.709").small().color(crate::ui::theme::colors::TEXT_MUTED));
                    ui.separator();
                    let cached_cnt = (0..total_frames).filter(|&f| self.frame_cache.is_cached(f)).count();
                    ui.label(egui::RichText::new(format!("RAM Preview: {}/{} frames cached", cached_cnt, total_frames)).small().color(crate::ui::theme::colors::TEXT_MUTED));
                    ui.separator();
                    // Selection summary: layers + keyframes
                    let kf_count = self.selected_keyframes.len();
                    let layer_count = self.selected_layers.len();
                    if kf_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("{} keyframes selected (, . move | Del delete | Cmd+C/V)", kf_count))
                                .small()
                                .color(crate::ui::theme::colors::ACCENT_ORANGE),
                        );
                    } else if layer_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("{} layer{} selected", layer_count, if layer_count > 1 { "s" } else { "" }))
                                .small()
                                .color(crate::ui::theme::colors::ACCENT_BLUE),
                        );
                    }
                    let pointer_pos = ctx.pointer_hover_pos().unwrap_or(egui::pos2(960.0, 540.0));
                    ui.separator();
                    ui.label(egui::RichText::new(format!("X: {:.0} Y: {:.0} px", pointer_pos.x, pointer_pos.y)).small().color(egui::Color32::from_rgb(0, 180, 255)));
                    ui.separator();
                    let pixel_rgba = {
                        let comp = self.history.current().active_composition();
                        let px = pointer_pos.x as i32;
                        let py = pointer_pos.y as i32;
                        if px >= 0 && py >= 0 && (px as u32) < comp.width && (py as u32) < comp.height {
                            let layer_indices: Vec<usize> = comp.layers.iter().enumerate()
                                .filter(|(_, l)| l.is_active(self.current_frame))
                                .map(|(i, _)| i)
                                .collect();
                            if let Some(entry) = self.frame_cache.get_with_layers(self.current_frame, &layer_indices) {
                                let idx = ((py as u32 * comp.width + px as u32) * 4) as usize;
                                if idx + 3 < entry.pixels.len() {
                                    Some([entry.pixels[idx], entry.pixels[idx+1], entry.pixels[idx+2], entry.pixels[idx+3]])
                                } else { None }
                            } else { None }
                        } else { None }
                    };
                    if let Some([r, g, b, a]) = pixel_rgba {
                        ui.label(egui::RichText::new(format!("R: {} G: {} B: {} A: {}", r, g, b, a)).small().color(egui::Color32::from_rgb(255, 200, 100)));
                    } else {
                        ui.label(egui::RichText::new("R: – G: – B: – A: –").small().color(egui::Color32::from_rgb(255, 200, 100)));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("AE OSS v0.1.0-parity").small().color(egui::Color32::from_gray(120)));
                        ui.separator();
                        ui.label(egui::RichText::new("Tool: Selection (V)").small().color(egui::Color32::from_rgb(255, 230, 0)));
                        ui.separator();
                        ui.label(egui::RichText::new("Dynamic Link: Active").small().color(egui::Color32::from_rgb(100, 180, 255)));
                        ui.separator();
                        ui.label(egui::RichText::new("RAM: 1.4 GB / 32 GB").small().color(egui::Color32::from_gray(160)));
                    });
                });
            });

        crate::ui::viewport::draw(self, ctx, current_frame);
        crate::ui::history_panel::draw_history_panel(self, ctx);
        crate::ui::drop_import::handle_dropped_files(self, ctx);
        crate::ui::welcome::draw(self, ctx);
        // Auto-open the walkthrough once per session for beginners on first run
        if self.show_welcome && self.ui_mode.is_beginner() && self.tutorial.is_none() {
            self.tutorial = Some(crate::ui::tutorial::TutorialState::default());
        }
        crate::ui::tutorial::draw(self, ctx);
        crate::ui::new_comp_dialog::draw_new_comp_dialog(self, ctx);
        crate::ui::preferences_dialog::draw_preferences_dialog(self, ctx);
        crate::ui::export_dialog::draw(self, ctx);
        crate::ui::comp_settings_dialog::draw_comp_settings_dialog(self, ctx);

        let cmd_k_pressed = ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::K));
        if cmd_k_pressed {
            self.show_command_palette = !self.show_command_palette;
            if self.show_command_palette {
                self.command_palette_search.clear();
                self.command_palette_selected_idx = 0;
            }
        }

        crate::ui::command_palette::draw_command_palette(self, ctx);
        crate::ui::vectorscope::draw_vectorscope_window(self, ctx);
        crate::ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);
        self.toasts.draw(ctx);
        self.current_frame = current_frame;
    }

    fn on_exit(&mut self) {
        // Clean exit — remove the dirty-exit marker so next launch doesn't
        // show the recovery dialog
        let _ = std::fs::remove_file(std::env::temp_dir().join("aevfx_dirty_exit"));
        if let Some(ref flag) = self.export_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(ref flag) = self.tracker_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        log::info!("Cleaned up background thread lifecycle flags on exit");
    }
}
