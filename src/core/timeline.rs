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

impl Transform2D {
    pub fn eval_position(&self, frame: u32, fps: u32) -> [f32; 2] {
        let base = self.position.evaluate(frame);
        match &self.position_expression {
            Some(expr) => expr.evaluate_v2(base, frame, fps),
            None => base,
        }
    }

    pub fn eval_rotation(&self, frame: u32, fps: u32) -> f32 {
        let base = self.rotation.evaluate(frame);
        match &self.rotation_expression {
            Some(expr) => expr.evaluate_f32(base, frame, fps),
            None => base,
        }
    }

    pub fn eval_scale(&self, frame: u32, fps: u32) -> [f32; 2] {
        let base = self.scale.evaluate(frame);
        match &self.scale_expression {
            Some(expr) => expr.evaluate_v2(base, frame, fps),
            None => base,
        }
    }

    pub fn eval_opacity(&self, frame: u32, fps: u32) -> f32 {
        let base = self.opacity.evaluate(frame);
        match &self.opacity_expression {
            Some(expr) => expr.evaluate_f32(base, frame, fps),
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
}

// ─── Project ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub compositions: Vec<Composition>,
    pub active_composition_idx: usize,
}

impl Default for Project {
    fn default() -> Self {
        let mut comp = Composition::new(
            "comp1".to_string(),
            "Main Comp".to_string(),
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

        Self {
            compositions: vec![comp],
            active_composition_idx: 0,
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
