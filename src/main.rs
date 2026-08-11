use eframe::egui;

mod core;
mod ui;

use crate::core::timeline::Project;

fn main() -> eframe::Result<()> {
    env_logger::init(); // Initialize logger

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("After Effects OSS Alternative"),
        ..Default::default()
    };

    eframe::run_native(
        "After Effects OSS Alternative",
        options,
        Box::new(|cc| {
            let mut app = AfterEffectsApp::default();

            // Start TCP Synchronization Server for NLE Dynamic Link integration
            let (frame_tx, frame_rx) = std::sync::mpsc::channel();
            let (conn_tx, conn_rx) = std::sync::mpsc::channel();
            crate::core::integration::start_sync_server(9000, frame_tx, conn_tx);

            app.rx_frame = Some(frame_rx);
            app.rx_connection = Some(conn_rx);

            #[cfg(feature = "wgpu")]
            if let Some(wgpu_state) = &cc.wgpu_render_state {
                let renderer = crate::core::renderer::WgpuRenderer::new(
                    wgpu_state.device.clone(),
                    wgpu_state.queue.clone(),
                );
                app.renderer = Some(renderer);
                app.wgpu_state = Some(wgpu_state.clone());
            }

            crate::ui::icons::init_image_loaders(&cc.egui_ctx);

            Box::new(app) as Box<dyn eframe::App>
        }),
    )
}

pub struct AfterEffectsApp {
    pub history: crate::core::history::ProjectHistory,

    // Playback state
    pub is_playing: bool,
    pub current_frame: u32,

    // UI state
    pub selected_layer_idx: Option<usize>,
    pub selected_layers: std::collections::HashSet<usize>,

    // GPU Renderer State
    #[cfg(feature = "wgpu")]
    pub renderer: Option<crate::core::renderer::WgpuRenderer>,
    #[cfg(feature = "wgpu")]
    pub wgpu_state: Option<eframe::egui_wgpu::RenderState>,

    // Cache for registered GPU texture in egui
    pub viewport_texture_id: Option<egui::TextureId>,

    // Dynamic Link channels & states
    pub rx_frame: Option<std::sync::mpsc::Receiver<u32>>,
    pub rx_connection: Option<std::sync::mpsc::Receiver<Option<String>>>,
    pub connected_app: Option<String>,

    pub project_path: String,
    pub otio_path: String,
    pub expanded_layers: std::collections::HashSet<usize>,
    pub show_grid: bool,
    pub show_guides: bool,
    pub show_handles: bool,
    pub show_comp_settings: bool,
    pub snap_to_keyframes: bool,
    pub show_graph_editor: bool,
    pub timeline_zoom: f32,
    pub timeline_scroll: f32,

    /// Viewport display mode: "2D" | "3D"
    pub viewport_mode: ViewportMode,
    /// Camera orbit state for 3D preview: (yaw_deg, pitch_deg, zoom)
    pub camera_orbit: (f32, f32, f32),
    /// Last mouse pos for orbit drag delta tracking
    pub orbit_drag_start: Option<egui::Pos2>,

    /// Active AE Tool (Selection, Hand, Zoom, Rotation, Anchor, Shape, Pen, Text)
    pub active_tool: crate::ui::toolbar::ActiveTool,

    /// Viewport drag state: (layer_idx, start_pos, start_pointer_pos)
    pub viewport_drag_state: Option<(usize, [f32; 2], egui::Pos2)>,
    /// Which property is selected in inspector (for graph editor)
    pub selected_property: Option<String>,

    // ── Export Dialog State ─────────────────────────────────────
    pub show_export_dialog: bool,
    pub export_status: Option<String>,
    pub export_progress: f32,
    pub export_fps: u32,
    pub export_output_path: String,
    pub is_exporting: bool,
    pub export_rx: Option<std::sync::mpsc::Receiver<ExportEvent>>,
    pub tracker_rx: Option<std::sync::mpsc::Receiver<TrackerEvent>>,

    // ── Explicit UI Panel & Audio States (Issue #2) ────────────
    pub master_volume: f32,
    pub left_tab_idx: usize,
    pub right_tab_idx: usize,
    pub viewport_mag_ratio: f32,

    // ── MVCC Frame Cache (#15) ─────────────────────────────────
    /// Versioned per-frame pixel cache. Stale entries auto-invalidate on project change.
    pub frame_cache: crate::core::frame_cache::FrameCache,
    /// Lazy demand evaluator: prevents duplicate render requests.
    pub lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewportMode {
    Comp2D,
    Camera3D,
}

pub use crate::core::ffmpeg_export::ExportEvent;

#[derive(Debug, Clone)]
pub enum TrackerEvent {
    Progress(f32, String),
    Finished {
        layer_idx: usize,
        tracker_idx: usize,
        keyframes: Vec<crate::core::keyframe::Keyframe<[f32; 2]>>,
    },
    Error(String),
}

impl Default for AfterEffectsApp {
    fn default() -> Self {
        Self {
            history: crate::core::history::ProjectHistory::new(Project::default()),
            is_playing: false,
            current_frame: 0,
            selected_layer_idx: Some(1), // Select Text Layer by default
            selected_layers: vec![1].into_iter().collect(),
            #[cfg(feature = "wgpu")]
            renderer: None,
            #[cfg(feature = "wgpu")]
            wgpu_state: None,
            viewport_texture_id: None,
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
            snap_to_keyframes: true,
            show_graph_editor: false,
            timeline_zoom: 1.0,
            timeline_scroll: 0.0,
            viewport_mode: ViewportMode::Comp2D,
            camera_orbit: (30.0, 20.0, 800.0), // yaw, pitch, zoom
            orbit_drag_start: None,
            active_tool: crate::ui::toolbar::ActiveTool::default(),
            viewport_drag_state: None,
            selected_property: None,
            show_export_dialog: false,
            export_status: None,
            export_progress: 0.0,
            export_fps: 30,
            export_output_path: "output.mp4".to_string(),
            is_exporting: false,
            export_rx: None,
            tracker_rx: None,
            master_volume: 0.8,
            left_tab_idx: 0,
            right_tab_idx: 0,
            viewport_mag_ratio: 1.0,
            // 256 frame entries max before GC kicks in
            frame_cache: crate::core::frame_cache::FrameCache::new(256),
            lazy_evaluator: crate::core::render_pipeline::LazyFrameEvaluator::new(),
        }
    }
}

impl AfterEffectsApp {
    /// Clone the current project state, execute the mutation function, and commit a new state.
    /// Automatically bumps the MVCC cache version so stale frames are invalidated.
    pub fn modify_project(&mut self, f: impl FnOnce(&mut Project)) {
        let mut next_project = self.history.current().clone();
        f(&mut next_project);
        self.history.commit(next_project);
        // ── MVCC: every project change increments the cache version ──
        crate::core::frame_cache::bump_version();
        // GC old cache entries to keep memory usage bounded
        self.frame_cache.collect_garbage();
    }
}

impl eframe::App for AfterEffectsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll sync messages from TCP connection
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

        // Frame progression when playing
        if self.is_playing {
            current_frame = (current_frame + 1) % total_frames;
            ctx.request_repaint();
        }

        // ── AE Keyboard Shortcuts ──────────────────────────────────────
        // Only fire when no text input widget has keyboard focus.
        let no_text_focus = !ctx.memory(|m| m.focused().is_some());
        if no_text_focus {
            ctx.input(|i| {
                use egui::Key;

                // Space → Play / Pause
                if i.key_pressed(Key::Space) {
                    self.is_playing = !self.is_playing;
                }

                // Home → first frame, End → last frame
                if i.key_pressed(Key::Home) { current_frame = 0; }
                if i.key_pressed(Key::End)  { current_frame = total_frames.saturating_sub(1); }

                // Page Up / ← → frame step backward/forward
                if i.key_pressed(Key::PageUp) || i.key_pressed(Key::ArrowLeft) {
                    current_frame = current_frame.saturating_sub(1);
                }
                if i.key_pressed(Key::PageDown) || i.key_pressed(Key::ArrowRight) {
                    current_frame = (current_frame + 1).min(total_frames.saturating_sub(1));
                }

                // Cmd+Z → Undo, Cmd+Shift+Z → Redo
                let cmd = i.modifiers.command;
                let shift = i.modifiers.shift;
                if cmd && !shift && i.key_pressed(Key::Z) {
                    self.history.undo();
                }
                if cmd && shift && i.key_pressed(Key::Z) {
                    self.history.redo();
                }

                // Delete / Backspace → Delete all selected layers (Multi-selection aware)
                if (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && !shift {
                    if !self.selected_layers.is_empty() {
                        let mut proj = self.history.current().clone();
                        let comp = proj.active_composition_mut();
                        let mut indices: Vec<usize> = self.selected_layers.iter().copied().collect();
                        indices.sort_unstable_by(|a, b| b.cmp(a));
                        for idx in indices {
                            if idx < comp.layers.len() {
                                comp.layers.remove(idx);
                            }
                        }
                        self.selected_layers.clear();
                        self.selected_layer_idx = if comp.layers.is_empty() { None } else { Some(0) };
                        self.history.commit(proj);
                        crate::core::frame_cache::bump_version();
                    }
                }

                // P → select Position property in graph editor
                if i.key_pressed(Key::P) && !cmd {
                    self.selected_property = Some("Position X".to_string());
                }
                // S → select Scale property
                if i.key_pressed(Key::S) && !cmd {
                    self.selected_property = Some("Scale X".to_string());
                }
                // T → select Opacity (Transparency)
                if i.key_pressed(Key::T) && !cmd {
                    self.selected_property = Some("Opacity".to_string());
                }
                // R → select Rotation property
                if i.key_pressed(Key::R) && !cmd {
                    self.selected_property = Some("Rotation".to_string());
                }

                // J → jump to previous keyframe of selected property
                // K → jump to next keyframe of selected property
                if i.key_pressed(Key::J) || i.key_pressed(Key::K) {
                    if let Some(idx) = self.selected_layer_idx {
                        let comp = self.history.current().active_composition();
                        if idx < comp.layers.len() {
                            let layer = &comp.layers[idx];
                            // Collect all keyframe positions from the selected property
                            let frames: Vec<u32> = {
                                let prop_name = self.selected_property.as_deref().unwrap_or("Position X");
                                match prop_name {
                                    "Position X" | "Position Y" => {
                                        layer.transform.position.keyframes()
                                            .map(|kfs| kfs.iter().map(|kf| kf.frame).collect())
                                            .unwrap_or_default()
                                    }
                                    "Scale X" | "Scale Y" => {
                                        layer.transform.scale.keyframes()
                                            .map(|kfs| kfs.iter().map(|kf| kf.frame).collect())
                                            .unwrap_or_default()
                                    }
                                    "Rotation" => {
                                        layer.transform.rotation.keyframes()
                                            .map(|kfs| kfs.iter().map(|kf| kf.frame).collect())
                                            .unwrap_or_default()
                                    }
                                    "Opacity" => {
                                        layer.transform.opacity.keyframes()
                                            .map(|kfs| kfs.iter().map(|kf| kf.frame).collect())
                                            .unwrap_or_default()
                                    }
                                    _ => vec![],
                                }
                            };
                            if i.key_pressed(Key::J) {
                                // Previous keyframe
                                if let Some(&prev) = frames.iter().rev().find(|&&f| f < current_frame) {
                                    current_frame = prev;
                                }
                            } else {
                                // Next keyframe
                                if let Some(&next) = frames.iter().find(|&&f| f > current_frame) {
                                    current_frame = next;
                                }
                            }
                        }
                    }
                }

                // Delete / Backspace → remove selected layers
                if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    let mut indices: Vec<usize> = self.selected_layers.iter().copied().collect();
                    if indices.is_empty() {
                        if let Some(s) = self.selected_layer_idx {
                            indices.push(s);
                        }
                    }
                    if !indices.is_empty() {
                        indices.sort_unstable_by(|a, b| b.cmp(a)); // sort descending to remove safely
                        self.modify_project(|project| {
                            let comp = project.active_composition_mut();
                            for idx in indices {
                                if idx < comp.layers.len() {
                                    comp.layers.remove(idx);
                                }
                            }
                        });
                        self.selected_layers.clear();
                        self.selected_layer_idx = None;
                    }
                }
            });
        }

        // Draw Menu Bar
        ui::menu::draw(self, ctx);

        // Draw Main Tools Bar (AE Tool Palette)
        ui::toolbar::draw(self, ctx);

        // Draw Left Panel: Properties Inspector
        ui::inspector::draw(self, ctx, &mut current_frame);

        // Draw Right Side Audio VU Meter Panel
        ui::audio_meter::draw(self, ctx);

        // Draw Right Panel: Effects Library & External Links
        ui::effects_library::draw(self, ctx, &mut current_frame);

        // Draw Bottom Panel: Timeline Editor
        ui::timeline::draw(self, ctx, &mut current_frame, total_frames);

        // Draw Central Panel: Viewport (GPU or CPU render)
        ui::viewport::draw(self, ctx, current_frame);

        // Draw Export Video Modal Dialog
        ui::export_dialog::draw(self, ctx);

        self.current_frame = current_frame;
    }
}
