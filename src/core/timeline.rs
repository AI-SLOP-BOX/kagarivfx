use crate::core::property::Animatable;
use crate::core::expression_engine;
use serde::{Deserialize, Serialize};

// Thread-local cached Rhai engine — building it is expensive (~1ms), reusing is free.
thread_local! {
    static RHAI_ENGINE: rhai::Engine = expression_engine::build_engine();
}

// ─── Tracker Point ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerPoint {
    pub id: String,
    pub name: String,
    pub position: Animatable<[f32; 2]>,
    pub search_size: f32,
    pub feature_size: f32,
    pub reference_pattern: Option<Vec<f32>>,
}

impl TrackerPoint {
    pub fn new(id: String, name: String, initial_pos: [f32; 2]) -> Self {
        Self {
            id,
            name,
            position: Animatable::new_constant(initial_pos),
            search_size: 25.0,
            feature_size: 10.0,
            reference_pattern: None,
        }
    }
}

// ─── Layer Type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerType {
    Solid {
        color: [f32; 4],
    },
    Image {
        path: String,
    },
    Text {
        text: String,
        font_size: u32,
        color: [f32; 4],
    },
    Shape {
        shape_type: ShapeType,
        color: [f32; 4],
    },
    Null,
    PreComp {
        comp_id: String,
    },
    Audio {
        path: String,
        volume: Animatable<f32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Star,
    Polygon,
}

// ─── Track Matte ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrackMatteMode {
    #[default]
    None,
    AlphaMatte,
    AlphaMatteInverted,
    LumaMatte,
    LumaMatteInverted,
}

// ─── AE Blend Modes ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
    Darken,
    Lighten,
}

// ─── Label Colors ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LabelColor {
    #[default]
    None,
    Red,
    Yellow,
    Aqua,
    Pink,
    Lavender,
    Peach,
    Sea,
    Blue,
    Purple,
}

impl LabelColor {
    pub fn to_rgb(self) -> [f32; 3] {
        match self {
            LabelColor::None      => [0.35, 0.35, 0.35],
            LabelColor::Red       => [0.90, 0.25, 0.25],
            LabelColor::Yellow    => [0.95, 0.85, 0.10],
            LabelColor::Aqua      => [0.20, 0.85, 0.80],
            LabelColor::Pink      => [0.95, 0.45, 0.70],
            LabelColor::Lavender  => [0.65, 0.55, 0.90],
            LabelColor::Peach     => [0.95, 0.70, 0.45],
            LabelColor::Sea       => [0.30, 0.70, 0.55],
            LabelColor::Blue      => [0.35, 0.55, 0.95],
            LabelColor::Purple    => [0.70, 0.35, 0.90],
        }
    }
}

// ─── Expression Engine ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Wiggle { frequency: f32, amplitude: f32 },
    LoopOut,
    PingPong,
    TimeDriver { multiplier: f32, offset: f32 },
    Raw(String),
}

impl Expression {
    pub fn evaluate_f32(&self, base: f32, frame: u32, fps: u32) -> f32 {
        let time = frame as f32 / fps.max(1) as f32;
        match self {
            Expression::Wiggle { frequency, amplitude } => {
                let seed = (frame as f32 * frequency * 1.618_034) % 6.283_185;
                let noise = seed.sin() * 0.7 + (seed * 2.1).sin() * 0.2 + (seed * 5.3).sin() * 0.1;
                base + noise * amplitude
            }
            Expression::TimeDriver { multiplier, offset } => {
                time * multiplier + offset
            }
            // Raw Rhai script — evaluated by the Rhai expression engine.
            // Supports full AE-compatible API: time, frame, fps, value, wiggle(), etc.
            Expression::Raw(script) => {
                RHAI_ENGINE.with(|engine| {
                    expression_engine::eval_f32(engine, script, base, frame, fps)
                })
            }
            _ => base,
        }
    }

    pub fn evaluate_v2(&self, base: [f32; 2], frame: u32, fps: u32) -> [f32; 2] {
        let time = frame as f32 / fps.max(1) as f32;
        match self {
            Expression::Wiggle { frequency, amplitude } => {
                let t = frame as f32 * frequency * 1.618_034;
                let nx = t.sin() * 0.7 + (t * 2.1).sin() * 0.2 + (t * 5.3).sin() * 0.1;
                let ny = (t + 100.0).sin() * 0.7 + ((t + 100.0) * 2.1).sin() * 0.2;
                [base[0] + nx * amplitude, base[1] + ny * amplitude]
            }
            Expression::TimeDriver { multiplier, offset } => {
                [base[0] + time * multiplier + offset, base[1]]
            }
            // Raw Rhai script — evaluated by the Rhai expression engine.
            Expression::Raw(script) => {
                RHAI_ENGINE.with(|engine| {
                    expression_engine::eval_v2(engine, script, base, frame, fps)
                })
            }
            _ => base,
        }
    }
}

// ─── Transform ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform2D {
    pub anchor_point: Animatable<[f32; 2]>,
    pub position: Animatable<[f32; 2]>,
    pub scale: Animatable<[f32; 2]>,
    pub rotation: Animatable<f32>,
    pub opacity: Animatable<f32>,

    pub position_expression: Option<Expression>,
    pub rotation_expression: Option<Expression>,
    pub scale_expression: Option<Expression>,
    pub opacity_expression: Option<Expression>,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            anchor_point: Animatable::new_constant([0.0, 0.0]),
            position: Animatable::new_constant([0.0, 0.0]),
            scale: Animatable::new_constant([100.0, 100.0]),
            rotation: Animatable::new_constant(0.0),
            opacity: Animatable::new_constant(100.0),
            position_expression: None,
            rotation_expression: None,
            scale_expression: None,
            opacity_expression: None,
        }
    }
}

/// Helper to map an extended frame number according to keyframe loop settings (LoopOut / PingPong).
pub fn remap_frame_for_loop(frame: u32, first_kf: u32, last_kf: u32, is_pingpong: bool) -> u32 {
    if first_kf >= last_kf || frame <= last_kf {
        return frame;
    }
    let span = last_kf - first_kf;
    let offset = frame - first_kf;
    let cycle_idx = offset / span;
    let rem = offset % span;

    if is_pingpong && (cycle_idx % 2 == 1) {
        last_kf - rem
    } else {
        first_kf + rem
    }
}

impl Transform2D {
    pub fn eval_position(&self, frame: u32, fps: u32) -> [f32; 2] {
        let eval_frame = match &self.position_expression {
            Some(Expression::LoopOut) => {
                if let Some(kfs) = self.position.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, false)
                    } else { frame }
                } else { frame }
            }
            Some(Expression::PingPong) => {
                if let Some(kfs) = self.position.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, true)
                    } else { frame }
                } else { frame }
            }
            _ => frame,
        };

        let base = self.position.evaluate(eval_frame);
        match &self.position_expression {
            Some(expr) => expr.evaluate_v2(base, eval_frame, fps),
            None => base,
        }
    }

    pub fn eval_rotation(&self, frame: u32, fps: u32) -> f32 {
        let eval_frame = match &self.rotation_expression {
            Some(Expression::LoopOut) => {
                if let Some(kfs) = self.rotation.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, false)
                    } else { frame }
                } else { frame }
            }
            Some(Expression::PingPong) => {
                if let Some(kfs) = self.rotation.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, true)
                    } else { frame }
                } else { frame }
            }
            _ => frame,
        };

        let base = self.rotation.evaluate(eval_frame);
        match &self.rotation_expression {
            Some(expr) => expr.evaluate_f32(base, eval_frame, fps),
            None => base,
        }
    }

    pub fn eval_scale(&self, frame: u32, fps: u32) -> [f32; 2] {
        let eval_frame = match &self.scale_expression {
            Some(Expression::LoopOut) => {
                if let Some(kfs) = self.scale.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, false)
                    } else { frame }
                } else { frame }
            }
            Some(Expression::PingPong) => {
                if let Some(kfs) = self.scale.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, true)
                    } else { frame }
                } else { frame }
            }
            _ => frame,
        };

        let base = self.scale.evaluate(eval_frame);
        match &self.scale_expression {
            Some(expr) => expr.evaluate_v2(base, eval_frame, fps),
            None => base,
        }
    }

    pub fn eval_opacity(&self, frame: u32, fps: u32) -> f32 {
        let eval_frame = match &self.opacity_expression {
            Some(Expression::LoopOut) => {
                if let Some(kfs) = self.opacity.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, false)
                    } else { frame }
                } else { frame }
            }
            Some(Expression::PingPong) => {
                if let Some(kfs) = self.opacity.keyframes() {
                    if !kfs.is_empty() {
                        remap_frame_for_loop(frame, kfs[0].frame, kfs.last().unwrap().frame, true)
                    } else { frame }
                } else { frame }
            }
            _ => frame,
        };

        let base = self.opacity.evaluate(eval_frame);
        match &self.opacity_expression {
            Some(expr) => expr.evaluate_f32(base, eval_frame, fps),
            None => base,
        }
    }
}

// ─── 3D Space Extension (Camera Suite) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform3D {
    pub position: Animatable<[f32; 3]>,
    pub rotation: Animatable<[f32; 3]>,
    pub scale: Animatable<[f32; 3]>,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Animatable::new_constant([0.0, 0.0, 0.0]),
            rotation: Animatable::new_constant([0.0, 0.0, 0.0]),
            scale: Animatable::new_constant([100.0, 100.0, 100.0]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera3D {
    pub name: String,
    pub active: bool,
    pub fov_degrees: f32,
    pub focus_distance: f32,
    pub aperture: f32,
    pub transform: Transform3D,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            name: "Active Camera".to_string(),
            active: true,
            fov_degrees: 50.0,
            focus_distance: 1000.0,
            aperture: 2.8,
            transform: Transform3D::default(),
        }
    }
}

// ─── Color Space Convert Modes ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorConversionMode {
    #[default]
    LogCToLinear,
    LinearToLogC,
    SLog3ToLinear,
    LinearToSLog3,
}

// ─── Effects ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    GaussianBlur {
        blur_radius: Animatable<f32>,
    },
    ColorTint {
        color: Animatable<[f32; 4]>,
        intensity: Animatable<f32>,
    },
    DropShadow {
        color: Animatable<[f32; 4]>,
        opacity: Animatable<f32>,
        direction: Animatable<f32>,
        distance: Animatable<f32>,
        softness: Animatable<f32>,
    },
    ChromaticAberration {
        shift_r: Animatable<f32>,
        shift_b: Animatable<f32>,
        edge_falloff: Animatable<f32>,
    },
    Vignette {
        intensity: Animatable<f32>,
        roundness: Animatable<f32>,
        feather: Animatable<f32>,
        color: Animatable<[f32; 4]>,
    },
    Levels {
        input_black: Animatable<f32>,
        input_white: Animatable<f32>,
        gamma: Animatable<f32>,
        output_black: Animatable<f32>,
        output_white: Animatable<f32>,
    },
    HueSaturation {
        hue_shift: Animatable<f32>,
        saturation: Animatable<f32>,
        lightness: Animatable<f32>,
    },
    Glow {
        threshold: Animatable<f32>,
        radius: Animatable<f32>,
        intensity: Animatable<f32>,
        color: Animatable<[f32; 4]>,
    },
    MotionBlur {
        shutter_angle: Animatable<f32>,
        samples: u32,
    },
    MeshWarp {
        top_left: Animatable<[f32; 2]>,
        top_right: Animatable<[f32; 2]>,
        bottom_left: Animatable<[f32; 2]>,
        bottom_right: Animatable<[f32; 2]>,
    },
    ColorGradeLUT {
        lut_path: String,
        intensity: Animatable<f32>,
    },
    ColorSpaceConvert {
        mode: ColorConversionMode,
    },
    FilmGrain {
        intensity: Animatable<f32>,
        grain_size: f32,
        color_film: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    pub name: String,
    pub effect_type: EffectType,
    pub enabled: bool,
}

// ─── Layer ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub layer_type: LayerType,
    pub in_frame: u32,
    pub out_frame: u32,
    pub transform: Transform2D,
    pub effects: Vec<Effect>,
    pub visible: bool,
    pub locked: bool,

    pub parent_id: Option<String>,
    pub track_matte: TrackMatteMode,
    pub solo: bool,
    pub motion_blur: bool,
    pub label: LabelColor,
    pub time_remap: Option<Animatable<f32>>,

    pub trackers: Vec<TrackerPoint>,

    pub is_3d: bool,
    pub transform_3d: Transform3D,
    
    // ── AE Blend Mode ──
    pub blend_mode: BlendMode,

    pub is_adjustment_layer: bool,
    pub is_guide_layer: bool,
    pub is_shy: bool,

    // ── AE Masking System ──
    pub masks: Vec<crate::core::mask::Mask>,
}

impl Layer {
    pub fn new(id: String, name: String, layer_type: LayerType, duration_frames: u32) -> Self {
        Self {
            id,
            name,
            layer_type,
            in_frame: 0,
            out_frame: duration_frames,
            transform: Transform2D::default(),
            effects: Vec::new(),
            visible: true,
            locked: false,
            parent_id: None,
            track_matte: TrackMatteMode::None,
            solo: false,
            motion_blur: false,
            label: LabelColor::None,
            time_remap: None,
            trackers: Vec::new(),
            is_3d: false,
            transform_3d: Transform3D::default(),
            blend_mode: BlendMode::Normal,
            is_adjustment_layer: false,
            is_guide_layer: false,
            is_shy: false,
            masks: Vec::new(),
        }
    }

    pub fn new_null(id: String, name: String, duration_frames: u32) -> Self {
        let mut l = Self::new(id, name, LayerType::Null, duration_frames);
        l.label = LabelColor::Red;
        l
    }

    pub fn new_adjustment(id: String, name: String, duration_frames: u32) -> Self {
        let mut l = Self::new(id, name, LayerType::Solid { color: [1.0, 1.0, 1.0, 0.0] }, duration_frames);
        l.is_adjustment_layer = true;
        l.label = LabelColor::Purple;
        l
    }

    pub fn is_active(&self, frame: u32) -> bool {
        self.visible && frame >= self.in_frame && frame <= self.out_frame
    }

    pub fn remap_frame(&self, frame: u32) -> u32 {
        match &self.time_remap {
            Some(anim) => anim.evaluate(frame) as u32,
            None => frame,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarker {
    pub frame: u32,
    pub label: String,
    pub color: [f32; 3],
}

// ─── Composition ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_frames: u32,
    pub layers: Vec<Layer>,

    pub motion_blur_shutter_angle: f32,
    pub background_color: [f32; 4],
    pub active_camera: Camera3D,
    pub markers: Vec<TimelineMarker>,
}

impl Composition {
    pub fn new(
        id: String,
        name: String,
        width: u32,
        height: u32,
        fps: u32,
        duration_frames: u32,
    ) -> Self {
        Self {
            id,
            name,
            width,
            height,
            fps,
            duration_frames,
            layers: Vec::new(),
            motion_blur_shutter_angle: 180.0,
            background_color: [0.05, 0.05, 0.08, 1.0],
            active_camera: Camera3D::default(),
            markers: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    pub fn resolve_world_transform(
        &self,
        layer: &Layer,
        frame: u32,
    ) -> ([f32; 2], [f32; 2], f32, f32) {
        let fps = self.fps;
        let pos = layer.transform.eval_position(frame, fps);
        let scale = layer.transform.eval_scale(frame, fps);
        let rot = layer.transform.eval_rotation(frame, fps);
        let opa = layer.transform.eval_opacity(frame, fps);

        if let Some(pid) = &layer.parent_id {
            if let Some(parent) = self.layers.iter().find(|l| &l.id == pid) {
                let (ppos, pscale, prot, popa) = self.resolve_world_transform(parent, frame);
                let rot_rad = prot.to_radians();
                let (s, c) = (rot_rad.sin(), rot_rad.cos());
                let world_x = pos[0] * pscale[0] / 100.0 * c - pos[1] * pscale[1] / 100.0 * s + ppos[0];
                let world_y = pos[0] * pscale[0] / 100.0 * s + pos[1] * pscale[1] / 100.0 * c + ppos[1];
                return (
                    [world_x, world_y],
                    [scale[0] * pscale[0] / 100.0, scale[1] * pscale[1] / 100.0],
                    rot + prot,
                    opa * popa / 100.0,
                );
            }
        }
        (pos, scale, rot, opa)
    }

    pub fn resolve_world_transform_3d(
        &self,
        layer: &Layer,
        frame: u32,
    ) -> [[f32; 4]; 4] {
        let pos3d = layer.transform_3d.position.evaluate(frame);
        let rot3d = layer.transform_3d.rotation.evaluate(frame);
        let scale3d = layer.transform_3d.scale.evaluate(frame);

        // Perspective Projection Matrix for Camera
        let fov_rad = self.active_camera.fov_degrees.to_radians();
        let aspect = self.width as f32 / self.height.max(1) as f32;
        let near = 0.1f32;
        let far = 10000.0f32;
        let f = 1.0 / (fov_rad * 0.5).tan();

        let proj_3d = [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, -f, 0.0, 0.0],
            [0.0, 0.0, far / (far - near), 1.0],
            [0.0, 0.0, -(far * near) / (far - near), 0.0],
        ];

        let rad_x = rot3d[0].to_radians();
        let rad_y = rot3d[1].to_radians();
        let rad_z = rot3d[2].to_radians();

        let cx = rad_x.cos(); let sx = rad_x.sin();
        let cy = rad_y.cos(); let sy = rad_y.sin();
        let cz = rad_z.cos(); let sz = rad_z.sin();

        // Euler rotation matrix Z * Y * X
        let rot_matrix = [
            [cy * cz, cy * sz, -sy, 0.0],
            [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy, 0.0],
            [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let scale_matrix = [
            [scale3d[0] / 100.0, 0.0, 0.0, 0.0],
            [0.0, scale3d[1] / 100.0, 0.0, 0.0],
            [0.0, 0.0, scale3d[2] / 100.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let pos_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [pos3d[0] / (self.width as f32 * 0.5), pos3d[1] / (self.height as f32 * 0.5), pos3d[2] / 1000.0, 1.0],
        ];

        fn m4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
            let mut out = [[0.0; 4]; 4];
            for r in 0..4 {
                for c in 0..4 {
                    out[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
                }
            }
            out
        }

        m4_mul(proj_3d, m4_mul(pos_matrix, m4_mul(rot_matrix, scale_matrix)))
    }
}

// ─── Project Item & Asset Management ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectItemType {
    Composition { comp_idx: usize },
    Image { path: String, width: u32, height: u32 },
    Audio { path: String, duration_sec: f32 },
    Solid { color: [f32; 4] },
    Folder { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub item_type: ProjectItemType,
}

impl ProjectItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>, item_type: ProjectItemType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            item_type,
        }
    }
}

// ─── Project ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub compositions: Vec<Composition>,
    pub active_composition_idx: usize,
    pub assets: Vec<ProjectItem>,
}

impl Default for Project {
    fn default() -> Self {
        let mut comp = Composition::new(
            "comp_main".to_string(),
            "Main Comp 1".to_string(),
            1920,
            1080,
            30,
            300,
        );

        // Background Solid
        let mut bg_layer = Layer::new(
            "layer_bg".to_string(),
            "Background".to_string(),
            LayerType::Solid {
                color: [0.08, 0.08, 0.12, 1.0],
            },
            300,
        );
        bg_layer.transform.scale = Animatable::new_constant([1920.0, 1080.0]);
        bg_layer.transform.position = Animatable::new_constant([960.0, 540.0]);
        bg_layer.label = LabelColor::Blue;
        comp.add_layer(bg_layer);

        // Main Title Text
        let mut text_layer = Layer::new(
            "layer_text".to_string(),
            "Main Title".to_string(),
            LayerType::Text {
                text: "After Effects OSS".to_string(),
                font_size: 64,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            300,
        );
        text_layer.transform.position = Animatable::new_constant([960.0, 500.0]);
        text_layer.label = LabelColor::Aqua;
        comp.add_layer(text_layer);

        // Null Object controller
        let mut null_ctrl = Layer::new_null(
            "layer_null_ctrl".to_string(),
            "Controller [NULL]".to_string(),
            300,
        );
        null_ctrl.transform.position = Animatable::new_constant([960.0, 540.0]);
        null_ctrl.label = LabelColor::Peach;
        comp.add_layer(null_ctrl);

        let default_assets = vec![
            ProjectItem::new("item_comp1", "Main Comp 1", ProjectItemType::Composition { comp_idx: 0 }),
            ProjectItem::new("item_bg_solid", "Dark Solid Background", ProjectItemType::Solid { color: [0.08, 0.08, 0.12, 1.0] }),
            ProjectItem::new("item_logo", "Logo_Vector.svg", ProjectItemType::Image { path: "assets/logo.svg".to_string(), width: 512, height: 512 }),
            ProjectItem::new("item_audio", "Intro_BGM.wav", ProjectItemType::Audio { path: "assets/audio.wav".to_string(), duration_sec: 10.0 }),
            ProjectItem::new("item_solids_folder", "Solids", ProjectItemType::Folder { name: "Solids".to_string() }),
        ];

        Self {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: default_assets,
        }
    }
}

impl Project {
    pub fn active_composition(&self) -> &Composition {
        &self.compositions[self.active_composition_idx]
    }

    pub fn active_composition_mut(&mut self) -> &mut Composition {
        &mut self.compositions[self.active_composition_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap_frame_for_loop_cycle() {
        // Keyframe interval: 0 to 30 (span = 30)
        // Frame 15 -> 15
        assert_eq!(remap_frame_for_loop(15, 0, 30, false), 15);
        // Frame 30 -> 30
        assert_eq!(remap_frame_for_loop(30, 0, 30, false), 30);
        // Frame 45 -> (45-0)%30 = 15
        assert_eq!(remap_frame_for_loop(45, 0, 30, false), 15);
    }

    #[test]
    fn test_remap_frame_for_loop_pingpong() {
        // Keyframe interval: 0 to 30 (span = 30)
        // Cycle 0 (0..30): frame 15 -> 15
        assert_eq!(remap_frame_for_loop(15, 0, 30, true), 15);
        // Cycle 1 (30..60, odd cycle = reverse): frame 45 -> 30 - 15 = 15
        assert_eq!(remap_frame_for_loop(45, 0, 30, true), 15);
        // Cycle 2 (60..90, even cycle = forward): frame 75 -> 0 + 15 = 15
        assert_eq!(remap_frame_for_loop(75, 0, 30, true), 15);
    }
}
