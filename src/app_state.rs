use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::{ExportEvent, TrackerEvent, ViewportMode};
use crate::core::timeline::Project;

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
    /// Index of the layer currently being drag-reordered in the timeline.
    pub dragging_layer: Option<usize>,
    pub show_grid: bool,
    pub show_guides: bool,
    pub show_handles: bool,
    pub show_comp_settings: bool,
    pub show_shortcuts_dialog: bool,
    pub show_precompose_dialog: bool,
    pub precompose_name: String,
    pub precompose_move_attributes: bool,
    pub show_command_palette: bool,
    pub command_palette_search: String,
    pub command_palette_selected_idx: usize,
    pub snap_to_keyframes: bool,
    pub show_graph_editor: bool,
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
    pub viewport_mask_drag_state: Option<(usize, usize, usize, [f32; 2], eframe::egui::Pos2)>,
    pub selected_property: Option<String>,
    pub show_export_dialog: bool,
    pub export_status: Option<String>,
    pub export_progress: f32,
    pub export_fps: u32,
    pub export_output_path: String,
    pub is_exporting: bool,
    pub export_format_preset: usize,
    pub export_resolution_scale: usize,
    pub export_rx: Option<std::sync::mpsc::Receiver<ExportEvent>>,
    pub export_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub tracker_rx: Option<std::sync::mpsc::Receiver<TrackerEvent>>,
    pub tracker_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub master_volume: f32,
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
    pub audio_mixer_channels: Vec<(f32, f32)>,
    /// Synced audio playback for AV preview (gui builds with an audio device).
    #[cfg(feature = "gui")]
    pub audio_playback: Option<crate::core::audio_playback::AudioPlayback>,
    pub camera_view_layout: usize,
    pub camera_view_angle: usize,
    pub font_family_idx: usize,
    pub faux_font_switches: (bool, bool, bool, bool),
    pub toasts: crate::ui::notification::ToastManager,
    pub frame_cache: crate::core::frame_cache::FrameCache,
    pub lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator,
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
            show_grid: false,
            show_guides: true,
            show_handles: true,
            show_comp_settings: false,
            show_shortcuts_dialog: false,
            show_precompose_dialog: false,
            precompose_name: String::new(),
            precompose_move_attributes: true,
            snap_to_keyframes: true,
            dragging_layer: None,
            show_graph_editor: false,
            timeline_zoom: 1.0,
            timeline_view_start: 0,
            timeline_scroll: 0.0,
            viewport_mode: ViewportMode::Comp2D,
            camera_orbit: (30.0, 20.0, 800.0),
            orbit_drag_start: None,
            active_tool: crate::ui::toolbar::ActiveTool::default(),
            viewport_drag_state: None,
            viewport_mask_drag_state: None,
            selected_property: None,
            show_export_dialog: false,
            export_status: None,
            export_progress: 0.0,
            export_fps: 30,
            export_output_path: "output.mp4".to_string(),
            is_exporting: false,
            export_format_preset: 0,
            export_resolution_scale: 0,
            export_rx: None,
            export_cancel_flag: None,
            tracker_rx: None,
            tracker_cancel_flag: None,
            master_volume: 0.8,
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
            camera_view_layout: 0,
            camera_view_angle: 0,
            font_family_idx: 0,
            faux_font_switches: (false, false, false, false),
            show_command_palette: false,
            command_palette_search: String::new(),
            command_palette_selected_idx: 0,
            toasts: crate::ui::notification::ToastManager::new(),
            frame_cache: crate::core::frame_cache::FrameCache::new(256),
            lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator::new(),
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
        if self.drag_tx.is_none() {
            self.drag_tx = Some(DragTransaction::new(
                self.history.current().clone(),
                label,
            ));
        }
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

        // One-time startup check for crash recovery snapshots
        if !self.recovery_checked {
            self.recovery_checked = true;
            if self.autosave.has_recovery() {
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
            let audio_enabled = ctx.data_mut(|d| {
                *d.get_temp_mut_or_insert_with(egui::Id::new("ae_audio_preview"), || true)
            });
            let fps = self.history.current().active_composition().fps.max(1);
            let playhead_sec = self.current_frame as f32 / fps as f32;

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
                    if self.work_area_in.is_some() || self.work_area_out.is_some() { wa_start } else { 0 }
                } else {
                    next
                };
            } else {
                current_frame = if current_frame == wa_start {
                    wa_end.saturating_sub(1)
                } else {
                    current_frame.saturating_sub(speed)
                };
            }
            let fps = self.history.current().active_composition().fps.max(1);
            let effective_fps = (fps as f32 / speed.max(1) as f32).max(1.0);
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.0 / effective_fps));
        }

        crate::ui::shortcuts::handle_global_shortcuts(self, ctx, &mut current_frame, total_frames);
        crate::ui::menu::draw(self, ctx);
        crate::ui::toolbar::draw(self, ctx);
        crate::ui::inspector::draw(self, ctx, &mut current_frame);
        crate::ui::effects_library::draw(self, ctx, &mut current_frame);
        crate::ui::timeline::draw(self, ctx, &mut current_frame, total_frames);

        let status_frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(20, 20, 20))
            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 38, 38)));

        egui::TopBottomPanel::bottom("ae_status_bar")
            .frame(status_frame)
            .default_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("● Metal GPU Render Engine").small().color(egui::Color32::from_rgb(0, 200, 120)));
                    ui.separator();
                    ui.label(egui::RichText::new("16-bpc | Rec.709 (Linear)").small().color(egui::Color32::from_gray(180)));
                    ui.separator();
                    let cached_cnt = (0..total_frames).filter(|&f| self.frame_cache.is_cached(f)).count();
                    ui.label(egui::RichText::new(format!("RAM Preview: {}/{} frames cached", cached_cnt, total_frames)).small().color(egui::Color32::from_gray(180)));
                    ui.separator();
                    // Selection summary: layers + keyframes
                    let kf_count = self.selected_keyframes.len();
                    let layer_count = self.selected_layers.len();
                    if kf_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("{} keyframes selected (, . move | Del delete | Cmd+C/V)", kf_count))
                                .small()
                                .color(egui::Color32::from_rgb(255, 200, 80)),
                        );
                    } else if layer_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("{} layer{} selected", layer_count, if layer_count > 1 { "s" } else { "" }))
                                .small()
                                .color(egui::Color32::from_rgb(0, 180, 255)),
                        );
                    }
                    let pointer_pos = ctx.pointer_hover_pos().unwrap_or(egui::pos2(960.0, 540.0));
                    ui.separator();
                    ui.label(egui::RichText::new(format!("X: {:.0} Y: {:.0} px", pointer_pos.x, pointer_pos.y)).small().color(egui::Color32::from_rgb(0, 180, 255)));
                    ui.separator();
                    ui.label(egui::RichText::new("R: 128 G: 128 B: 128 A: 255").small().color(egui::Color32::from_rgb(255, 200, 100)));
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
        crate::ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);
        self.toasts.draw(ctx);
        self.current_frame = current_frame;
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(ref flag) = self.export_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if let Some(ref flag) = self.tracker_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        log::info!("Cleaned up background thread lifecycle flags on exit");
    }
}
