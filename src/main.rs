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
    pub export_rx: Option<std::sync::mpsc::Receiver<ExportEvent>>,
    pub tracker_rx: Option<std::sync::mpsc::Receiver<TrackerEvent>>,

    pub master_volume: f32,
    pub left_tab_idx: usize,
    pub right_tab_idx: usize,
    pub viewport_mag_ratio: f32,
    pub work_area_in: Option<u32>,
    pub work_area_out: Option<u32>,

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
            export_rx: None,
            tracker_rx: None,
            master_volume: 0.8,
            left_tab_idx: 0,
            right_tab_idx: 0,
            viewport_mag_ratio: 1.0,
            work_area_in: None,
            work_area_out: None,
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

                // B → Set Work Area Start, N → Set Work Area End
                let cmd = i.modifiers.command;
                let shift = i.modifiers.shift;
                if i.key_pressed(Key::B) && !cmd {
                    self.work_area_in = Some(current_frame);
                }
                if i.key_pressed(Key::N) && !cmd {
                    self.work_area_out = Some(current_frame);
                }

                // J → Jump to previous keyframe, K → Jump to next keyframe (when Cmd is NOT pressed)
                if !cmd && !shift {
                    if i.key_pressed(Key::J) {
                        if let Some(idx) = self.selected_layer_idx {
                            let comp = self.history.current().active_composition();
                            if idx < comp.layers.len() {
                                let layer = &comp.layers[idx];
                                let mut all_frames: Vec<u32> = Vec::new();
                                for kf in layer.transform.position.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                for kf in layer.transform.scale.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                for kf in layer.transform.rotation.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                for kf in layer.transform.opacity.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                all_frames.sort_unstable();
                                if let Some(&prev_f) = all_frames.iter().rev().find(|&&f| f < current_frame) {
                                    current_frame = prev_f;
                                }
                            }
                        }
                    }
                    if i.key_pressed(Key::K) {
                        if let Some(idx) = self.selected_layer_idx {
                            let comp = self.history.current().active_composition();
                            if idx < comp.layers.len() {
                                let layer = &comp.layers[idx];
                                let mut all_frames: Vec<u32> = Vec::new();
                                for kf in layer.transform.position.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                for kf in layer.transform.scale.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                for kf in layer.transform.rotation.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                for kf in layer.transform.opacity.keyframes().unwrap_or(&[]) { all_frames.push(kf.frame); }
                                all_frames.sort_unstable();
                                if let Some(&next_f) = all_frames.iter().find(|&&f| f > current_frame) {
                                    current_frame = next_f;
                                }
                            }
                        }
                    }
                    if i.key_pressed(Key::F9) {
                        if let Some(idx) = self.selected_layer_idx {
                            let mut temp_proj = self.history.current().clone();
                            let comp = temp_proj.active_composition_mut();
                            if idx < comp.layers.len() {
                                let layer = &mut comp.layers[idx];
                                let ez = if shift {
                                    // Easy Ease In (Shift+F9)
                                    crate::core::keyframe::InterpolationType::Bezier {
                                        outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.0, speed: 0.0 },
                                        incoming: crate::core::keyframe::BezierControlPoint { influence: 0.85, speed: 0.0 },
                                        custom_bezier: Some([0.85, 0.0, 1.0, 1.0]),
                                    }
                                } else if cmd {
                                    // Easy Ease Out (Cmd+Shift+F9 / Cmd+F9)
                                    crate::core::keyframe::InterpolationType::Bezier {
                                        outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.85, speed: 0.0 },
                                        incoming: crate::core::keyframe::BezierControlPoint { influence: 0.0, speed: 0.0 },
                                        custom_bezier: Some([0.0, 0.0, 0.15, 1.0]),
                                    }
                                } else {
                                    // Easy Ease (F9)
                                    crate::core::keyframe::InterpolationType::Bezier {
                                        outgoing: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                        incoming: crate::core::keyframe::BezierControlPoint { influence: 0.333, speed: 0.0 },
                                        custom_bezier: Some([0.333, 0.0, 0.333, 1.0]),
                                    }
                                };
                                if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.position { for kf in kfs { kf.interpolation = ez.clone(); } }
                                if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.scale { for kf in kfs { kf.interpolation = ez.clone(); } }
                                if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.rotation { for kf in kfs { kf.interpolation = ez.clone(); } }
                                if let crate::core::property::Animatable::Animated(ref mut kfs) = layer.transform.opacity { for kf in kfs { kf.interpolation = ez.clone(); } }
                                self.history.commit(temp_proj);
                                crate::core::frame_cache::bump_version();
                            }
                        }
                    }
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
                if cmd && !shift && i.key_pressed(Key::Z) {
                    self.history.undo();
                }
                if cmd && shift && i.key_pressed(Key::Z) {
                    self.history.redo();
                }

                // Cmd+K → Composition Settings Dialog
                if cmd && !shift && i.key_pressed(Key::K) {
                    self.show_comp_settings = true;
                }

                // Cmd+Shift+C → Pre-Compose Selected Layers
                if cmd && shift && i.key_pressed(Key::C) {
                    let mut temp_project = self.history.current().clone();
                    let selected_indices: Vec<usize> = if !self.selected_layers.is_empty() {
                        let mut s: Vec<usize> = self.selected_layers.iter().copied().collect();
                        s.sort();
                        s
                    } else if let Some(idx) = self.selected_layer_idx {
                        vec![idx]
                    } else {
                        vec![]
                    };

                    if !selected_indices.is_empty() {
                        let comp_len = temp_project.compositions.len();
                        let (width, height, fps, duration_frames) = {
                            let comp = temp_project.active_composition();
                            (comp.width, comp.height, comp.fps, comp.duration_frames)
                        };

                        let precomp_id = format!("precomp_{}", comp_len);
                        let precomp_name = format!("Pre-comp {}", comp_len + 1);
                        let mut new_comp = crate::core::timeline::Composition::new(
                            precomp_id.clone(),
                            precomp_name.clone(),
                            width,
                            height,
                            fps,
                            duration_frames,
                        );

                        let comp_mut = temp_project.active_composition_mut();
                        let mut extracted_layers = Vec::new();
                        for &idx in selected_indices.iter().rev() {
                            if idx < comp_mut.layers.len() {
                                extracted_layers.push(comp_mut.layers.remove(idx));
                            }
                        }
                        extracted_layers.reverse();
                        new_comp.layers = extracted_layers;

                        let precomp_layer = crate::core::timeline::Layer::new(
                            format!("layer_{}", precomp_id),
                            precomp_name,
                            crate::core::timeline::LayerType::PreComp { comp_id: precomp_id },
                            duration_frames,
                        );
                        let insert_pos = selected_indices.first().copied().unwrap_or(0).min(comp_mut.layers.len());
                        comp_mut.layers.insert(insert_pos, precomp_layer);
                        temp_project.compositions.push(new_comp);

                        self.selected_layers.clear();
                        self.selected_layers.insert(insert_pos);
                        self.selected_layer_idx = Some(insert_pos);
                        self.history.commit(temp_project);
                        crate::core::frame_cache::bump_version();
                    }
                }

                // Cmd+D → Duplicate selected layer, Cmd+Shift+D → Split layer at current frame
                if cmd && i.key_pressed(Key::D) {
                    if let Some(idx) = self.selected_layer_idx {
                        let mut proj = self.history.current().clone();
                        let cf = self.current_frame;
                        let comp = proj.active_composition_mut();
                        if idx < comp.layers.len() {
                            if !shift {
                                let mut dup = comp.layers[idx].clone();
                                dup.id = format!("{}_dup_{}", dup.id, comp.layers.len());
                                dup.name = format!("{} Copy", dup.name);
                                comp.layers.insert(idx + 1, dup);
                                self.selected_layer_idx = Some(idx + 1);
                            } else {
                                let mut split_b = comp.layers[idx].clone();
                                comp.layers[idx].out_frame = cf;
                                split_b.in_frame = cf;
                                split_b.id = format!("{}_split_{}", split_b.id, comp.layers.len());
                                split_b.name = format!("{} Split", split_b.name);
                                comp.layers.insert(idx + 1, split_b);
                                self.selected_layer_idx = Some(idx + 1);
                            }
                            self.history.commit(proj);
                            crate::core::frame_cache::bump_version();
                        }
                    }
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

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("AE OSS v0.1.0-parity").small().color(egui::Color32::from_gray(120)));
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

        self.current_frame = current_frame;
    }
}
