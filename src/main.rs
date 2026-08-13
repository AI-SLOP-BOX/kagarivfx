use eframe::egui;

mod core;
mod ui;
pub mod app_state;

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

            crate::ui::theme::configure_ae_theme(&cc.egui_ctx);
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
    pub viewport_snapshot_texture_id: Option<egui::TextureId>,

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
    pub show_shortcuts_dialog: bool,
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
    /// Viewport vector mask vertex drag state: (layer_idx, mask_idx, vertex_idx, start_vertex_pos, start_pointer_pos)
    pub viewport_mask_drag_state: Option<(usize, usize, usize, [f32; 2], egui::Pos2)>,
    /// Which property is selected in inspector (for graph editor)
    pub selected_property: Option<String>,

    // ── Export Dialog State ─────────────────────────────────────
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

    // ── Structured UI Component State (Replaces fragile ctx.data_mut string keys) ──
    pub layer_filter_text: String,
    pub bottom_dock_tab: usize,
    pub show_switches_pane: bool,
    pub global_shy_active: bool,
    pub effects_search_query: String,

    // ── Toast Notification System (#8) ──
    pub toasts: crate::ui::notification::ToastManager,

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
        layer_id: String,
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
            snap_to_keyframes: true,
            show_graph_editor: false,
            timeline_zoom: 1.0,
            timeline_scroll: 0.0,
            viewport_mode: ViewportMode::Comp2D,
            camera_orbit: (30.0, 20.0, 800.0), // yaw, pitch, zoom
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
            toasts: crate::ui::notification::ToastManager::new(),
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

        let wa_start = self.work_area_in.unwrap_or(0);
        let wa_end = self.work_area_out.unwrap_or(total_frames.saturating_sub(1)).min(total_frames.saturating_sub(1));

        // Frame progression when playing (constrained to Work Area)
        if self.is_playing {
            current_frame = if current_frame < wa_start || current_frame >= wa_end {
                wa_start
            } else {
                current_frame + 1
            };
            let fps = self.history.current().active_composition().fps.max(1);
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.0 / fps as f32));
        }

        // ── AE Centralized Keyboard Shortcut Manager ─────────────────────────
        crate::ui::shortcuts::handle_global_shortcuts(self, ctx, &mut current_frame, total_frames);

        // Draw Menu Bar
        ui::menu::draw(self, ctx);

        // Draw Main Tools Bar (AE Tool Palette)
        ui::toolbar::draw(self, ctx);

        // Draw Left Panel: Project Bin & Properties Inspector
        ui::inspector::draw(self, ctx, &mut current_frame);

        // Draw Right Panel: Effects & Presets, Info, Audio VU Meter, Character, Paragraph, Align, Tracker
        ui::effects_library::draw(self, ctx, &mut current_frame);

        // Draw Bottom Panel: Timeline Editor & Render Queue
        ui::timeline::draw(self, ctx, &mut current_frame, total_frames);

        // ── AE Professional Status Bar (BOTTOM OF WINDOW) ─────────────────────
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

                    // AE Info Sampler: Pointer Pos & Pixel Color
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

        // Draw Central Panel: Viewport (GPU or CPU render)
        ui::viewport::draw(self, ctx, current_frame);

        // Draw Export Video Modal Dialog
        ui::export_dialog::draw(self, ctx);

        // Draw Composition Settings Modal Dialog
        ui::comp_settings_dialog::draw_comp_settings_dialog(self, ctx);

        // Draw Keyboard Shortcuts Reference Window
        ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);

        // Render Toast Notifications Overlay (#8)
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
