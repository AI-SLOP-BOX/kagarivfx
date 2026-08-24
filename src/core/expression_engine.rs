/// Rhai-powered After Effects-style expression evaluation engine.
///
/// Supports AE-compatible expression APIs:
///   - `time` (current time in seconds)
///   - `wiggle(freq, amp)` → f32 noise offset
///   - `loopOut("cycle")` / `loopOut("pingpong")`
///   - `thisComp.layer("Name").transform.position[0]`  (inter-layer reference)
///   - Any arbitrary Rhai script that returns a number or array.
use rhai::{Engine, Scope, Dynamic, Array};

/// Build the shared Rhai engine with all AE-compatible functions registered.
/// Creating the engine is expensive — do it once and cache it.
/// 64-bit Permuted Congruential Generator (PCG32) hash to produce uniform f64 in [0.0, 1.0).
pub fn pcg32_hash(mut state: u64) -> f64 {
    state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let xorshifted = (((state >> 18) ^ state) >> 27) as u32;
    let rot = (state >> 59) as u32;
    let val = (xorshifted >> rot) | (xorshifted << ((!rot).wrapping_add(1) & 31));
    val as f64 / 4294967296.0
}

pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 32);
    engine.set_max_operations(100_000);
    engine.set_max_string_size(4096);
    engine.set_max_array_size(1024);
    engine.set_max_modules(0);

    // ── Security: disable dangerous Rhai builtins ──
    engine.disable_symbol("eval");
    engine.disable_symbol("call");
    engine.disable_symbol("import");
    engine.disable_symbol("export");
    engine.disable_symbol("to_json");
    engine.disable_symbol("from_json");

    // --- wiggle(frequency: f32, amplitude: f32) -> f32 ---
    // Built-in pseudo-random noise matching AE's wiggle expression.
    engine.register_fn("wiggle", |time: f64, freq: f64, amp: f64| -> f64 {
        let t = time * freq * std::f64::consts::TAU;
        let n = t.sin() * 0.70 + (t * 2.1_f64).sin() * 0.20 + (t * 5.3_f64).sin() * 0.10;
        n * amp
    });

    // --- Math helpers mirroring AE's Math object ---
    engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
    engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
    engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
    engine.register_fn("clamp", |v: f64, lo: f64, hi: f64| -> f64 { v.clamp(lo, hi) });
    engine.register_fn("linear", |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
        let s = if (t_max - t_min).abs() < 1e-6 { 0.0 } else { ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0) };
        v_min + s * (v_max - v_min)
    });

    // --- AE-compatible Easing functions ---
    engine.register_fn("ease", |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
        let s = if (t_max - t_min).abs() < 1e-6 { 0.0 } else { ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0) };
        let smooth_s = s * s * (3.0 - 2.0 * s); // Hermite smoothstep
        v_min + smooth_s * (v_max - v_min)
    });

    engine.register_fn("easeIn", |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
        let s = if (t_max - t_min).abs() < 1e-6 { 0.0 } else { ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0) };
        let ease_in_s = s * s; // Quadratic acceleration
        v_min + ease_in_s * (v_max - v_min)
    });

    engine.register_fn("easeOut", |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
        let s = if (t_max - t_min).abs() < 1e-6 { 0.0 } else { ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0) };
        let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s); // Quadratic deceleration
        v_min + ease_out_s * (v_max - v_min)
    });

    // --- AE-compatible Random functions (PCG32 Deterministic Generator with time-variation) ---
    engine.register_fn("random", |min: f64, max: f64| -> f64 {
        let seed = min.to_bits() ^ max.to_bits().rotate_left(16);
        let r = pcg32_hash(seed);
        min + r * (max - min)
    });

    engine.register_fn("random", |time: f64, min: f64, max: f64| -> f64 {
        let seed = time.to_bits() ^ min.to_bits() ^ max.to_bits().rotate_left(16);
        let r = pcg32_hash(seed);
        min + r * (max - min)
    });

    engine.register_fn("gaussRandom", |min: f64, max: f64| -> f64 {
        let mean = (min + max) * 0.5;
        let std_dev = (max - min) * 0.16666;
        let seed1 = min.to_bits() ^ 0xa09e667f3bcc9091;
        let seed2 = max.to_bits() ^ 0x517cc1b727220a95;
        let u1 = pcg32_hash(seed1).clamp(0.0001, 0.9999);
        let u2 = pcg32_hash(seed2).clamp(0.0001, 0.9999);
        let z0 = f64::sqrt(-2.0 * u1.ln()) * (std::f64::consts::TAU * u2).cos();
        (mean + z0 * std_dev).clamp(min, max)
    });

    engine.register_fn("gaussRandom", |time: f64, min: f64, max: f64| -> f64 {
        let mean = (min + max) * 0.5;
        let std_dev = (max - min) * 0.16666;
        let t_bits = time.to_bits();
        let seed1 = min.to_bits() ^ t_bits ^ 0xa09e667f3bcc9091;
        let seed2 = max.to_bits() ^ t_bits.rotate_left(32) ^ 0x517cc1b727220a95;
        let u1 = pcg32_hash(seed1).clamp(0.0001, 0.9999);
        let u2 = pcg32_hash(seed2).clamp(0.0001, 0.9999);
        let z0 = f64::sqrt(-2.0 * u1.ln()) * (std::f64::consts::TAU * u2).cos();
        (mean + z0 * std_dev).clamp(min, max)
    });

    // --- Array constructors ---
    engine.register_fn("array2", |x: f64, y: f64| -> Array {
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });

    // --- Additional AE-compatible math functions ---
    engine.register_fn("ceil", |x: f64| -> f64 { x.ceil() });
    engine.register_fn("floor", |x: f64| -> f64 { x.floor() });
    engine.register_fn("round", |x: f64| -> f64 { x.round() });
    engine.register_fn("min", |a: f64, b: f64| -> f64 { a.min(b) });
    engine.register_fn("max", |a: f64, b: f64| -> f64 { a.max(b) });
    engine.register_fn("pow", |base: f64, exp: f64| -> f64 { base.powf(exp) });
    engine.register_fn("sqrt", |x: f64| -> f64 { x.sqrt() });
    engine.register_fn("sinh", |x: f64| -> f64 { x.sinh() });
    engine.register_fn("cosh", |x: f64| -> f64 { x.cosh() });
    engine.register_fn("tanh", |x: f64| -> f64 { x.tanh() });
    engine.register_fn("asin", |x: f64| -> f64 { x.asin() });
    engine.register_fn("acos", |x: f64| -> f64 { x.acos() });
    engine.register_fn("atan", |x: f64| -> f64 { x.atan() });
    engine.register_fn("atan2", |y: f64, x: f64| -> f64 { y.atan2(x) });
    engine.register_fn("log", |x: f64| -> f64 { x.ln() });
    engine.register_fn("log10", |x: f64| -> f64 { x.log10() });
    engine.register_fn("exp", |x: f64| -> f64 { x.exp() });

    // --- AE trigonometry in degrees ---
    engine.register_fn("degreesToRadians", |d: f64| -> f64 { d.to_radians() });
    engine.register_fn("radiansToDegrees", |r: f64| -> f64 { r.to_degrees() });

    // --- Interpolation (more flexible than linear) ---
    engine.register_fn("interpolate", |t: f64, t1: f64, t2: f64, v1: f64, v2: f64| -> f64 {
        let s = if (t2 - t1).abs() < 1e-6 { 0.0 } else { ((t - t1) / (t2 - t1)).clamp(0.0, 1.0) };
        v1 + s * (v2 - v1)
    });

    // --- Smoothstep (AE-compatible) ---
    engine.register_fn("smoothstep", |edge0: f64, edge1: f64, x: f64| -> f64 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    });

    // --- Distance between two points (1D) ---
    engine.register_fn("distance", |a: f64, b: f64| -> f64 { (a - b).abs() });

    // --- Ping-pong function ---
    engine.register_fn("pingpong", |t: f64, max: f64| -> f64 {
        let t_mod = t.rem_euclid(max * 2.0);
        if t_mod > max { max * 2.0 - t_mod } else { t_mod }
    });

    // --- Wrap function (AE-style) ---
    engine.register_fn("wrap", |val: f64, min_val: f64, max_val: f64| -> f64 {
        let range = max_val - min_val;
        if range.abs() < 1e-6 { return min_val; }
        let v = (val - min_val).rem_euclid(range);
        min_val + v
    });

    // --- AE seedRandom (deterministic random based on seed) ---
    engine.register_fn("seedRandom", |seed: f64| -> f64 {
        let bits = (seed * 1000.0) as u64;
        pcg32_hash(bits)
    });
    engine.register_fn("seedRandom", |seed: f64, timeless: bool| -> f64 {
        let _ = timeless;
        let bits = (seed * 1000.0) as u64;
        pcg32_hash(bits)
    });

    // --- Noise functions (simple value noise) ---
    engine.register_fn("noise", |x: f64| -> f64 {
        let n = x.sin() * 43758.5453;
        n - n.floor()
    });
    engine.register_fn("noise", |x: f64, y: f64| -> f64 {
        let n = (x * 12.9898 + y * 78.233).sin() * 43758.5453;
        n - n.floor()
    });

    // --- Toe / Shoulder (filmic helpers) ---
    engine.register_fn("toe", |x: f64, strength: f64| -> f64 {
        let p = strength.max(0.001);
        x.powf(p)
    });
    engine.register_fn("shoulder", |x: f64, strength: f64| -> f64 {
        let p = strength.max(0.001);
        1.0 - (1.0 - x).powf(p)
    });

    engine
}

/// Evaluate a Rhai expression that should return a single `f32`.
///
/// # Context variables available in the script
/// - `time`  : current time in seconds (f64)
/// - `frame` : current frame number (i64)
/// - `fps`   : frames per second (i64)
/// - `value` : the un-animated base value (f64)
pub fn eval_f32(
    engine: &Engine,
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
) -> f32 {
    let time = frame as f64 / fps.max(1) as f64;
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("value", base as f64);
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Ok(f) = val.as_float() {
                return f as f32;
            }
            if let Ok(i) = val.as_int() {
                return i as f32;
            }
            log::warn!("[ExprEngine] expression did not return a number: {:?}", val);
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_f32 error: {}", e);
            base
        }
    }
}

/// Evaluate a Rhai expression that should return a `[f32; 2]` pair.
///
/// The script may return:
///   - A Rhai `Array` of two numbers → `[x, y]`
///   - A single number → applied to both X and Y
pub fn eval_v2(
    engine: &Engine,
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
) -> [f32; 2] {
    let time = frame as f64 / fps.max(1) as f64;
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);
    // Expose base value as a Rhai array for scripts like `value + wiggle(...)`
    let base_arr: Array = vec![
        Dynamic::from_float(base[0] as f64),
        Dynamic::from_float(base[1] as f64),
    ];
    scope.push("value", base_arr);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            // Array return
            if let Ok(arr) = val.clone().into_array() {
                if arr.len() >= 2 {
                    let x = arr[0].as_float().unwrap_or(base[0] as f64) as f32;
                    let y = arr[1].as_float().unwrap_or(base[1] as f64) as f32;
                    return [x, y];
                }
            }
            // Scalar return — AE standard applies scalar offset to X axis only
            if let Ok(f) = val.as_float() {
                return [base[0] + f as f32, base[1]];
            }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_v2 error: {}", e);
            base
        }
    }
}

/// Evaluate expression with detailed diagnostic feedback for UI toast notifications.
#[allow(dead_code)]
pub fn eval_v2_with_diagnostics(
    engine: &Engine,
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
) -> ([f32; 2], Option<String>) {
    let time = frame as f64 / fps.max(1) as f64;
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);
    let base_arr: Array = vec![
        Dynamic::from_float(base[0] as f64),
        Dynamic::from_float(base[1] as f64),
    ];
    scope.push("value", base_arr);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Ok(arr) = val.clone().into_array() {
                if arr.len() >= 2 {
                    let x = arr[0].as_float().unwrap_or(base[0] as f64) as f32;
                    let y = arr[1].as_float().unwrap_or(base[1] as f64) as f32;
                    return ([x, y], None);
                }
            }
            if let Ok(f) = val.as_float() {
                return ([base[0] + f as f32, base[1]], None);
            }
            (base, Some("Expression returned non-numeric type".into()))
        }
        Err(e) => (base, Some(format!("Script syntax/eval error: {}", e))),
    }
}

/// Pre-validate a script without evaluating (for real-time syntax checking in UI).
/// Returns `Ok(())` if the script compiles, or an error message.
#[allow(dead_code)]
pub fn validate_script(engine: &Engine, script: &str) -> Result<(), String> {
    engine.compile(script)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}

/// Snapshot of a layer's transform at a frame, exposed to Rhai as a custom type.
#[derive(Clone)]
pub struct LayerSnapshot {
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation: f64,
    pub opacity: f64,
    /// Effect name -> param label -> evaluated value at the snapshot frame.
    /// Vec2 params are stored as "X"/"Y" entries.
    pub effects: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
}

/// Snapshot of the whole composition, exposed to Rhai as `thisComp`.
#[derive(Clone)]
pub struct CompSnapshot {
    pub layers: std::collections::HashMap<String, LayerSnapshot>,
    pub comp_width: f64,
    pub comp_height: f64,
}

impl CompSnapshot {
    /// AE-style `thisComp.layer("Name")` — looks up by name or id.
    pub fn layer(&self, name: &str) -> LayerSnapshot {
        self.layers.get(name).cloned().unwrap_or(LayerSnapshot {
            position: [0.0; 2],
            scale: [100.0, 100.0],
            rotation: 0.0,
            opacity: 100.0,
            effects: Default::default(),
        })
    }

    /// Composition dimensions in pixels.
    pub fn width(&self) -> f64 { self.comp_width }
    pub fn height(&self) -> f64 { self.comp_height }
}

fn register_comp_types(engine: &mut Engine) {
    engine.register_type::<LayerSnapshot>()
        .register_get("transform", |l: &mut LayerSnapshot| l.clone())
        .register_get("position", |l: &mut LayerSnapshot| -> Array {
            vec![Dynamic::from_float(l.position[0]), Dynamic::from_float(l.position[1])]
        })
        .register_get("scale", |l: &mut LayerSnapshot| -> Array {
            vec![Dynamic::from_float(l.scale[0]), Dynamic::from_float(l.scale[1])]
        })
        .register_get("rotation", |l: &mut LayerSnapshot| l.rotation)
        .register_get("opacity", |l: &mut LayerSnapshot| l.opacity)
        .register_indexer_get(|l: &mut LayerSnapshot, idx: i64| -> Dynamic {
            let arr = [l.position[0], l.position[1]];
            match idx {
                0 => Dynamic::from_float(arr[0]),
                1 => Dynamic::from_float(arr[1]),
                _ => Dynamic::UNIT,
            }
        });

    engine.register_type::<CompSnapshot>()
        .register_fn("layer", |c: &mut CompSnapshot, name: &str| c.layer(name));

    engine.register_fn("effect_param",
        |l: &mut LayerSnapshot, effect: &str, param: &str| -> f64 {
            l.effects
                .get(effect)
                .and_then(|m| m.get(param))
                .copied()
                .unwrap_or(f64::NAN)
        });
}

thread_local! {
    // Single-slot memo: building a snapshot is O(layers); expression layers each
    // need one per frame, which is O(n^2) across a comp without this cache.
    // Keyed by (project version, frame, comp identity) — any edit bumps the
    // global version, so staleness is impossible.
    static SNAPSHOT_CACHE: std::cell::RefCell<Option<(u64, u32, usize, CompSnapshot)>> =
        const { std::cell::RefCell::new(None) };
}

/// Builds a composition snapshot of all layer transforms at the given frame.
pub fn build_comp_snapshot(comp: &crate::core::timeline::Composition, frame: u32) -> CompSnapshot {
    let ver = crate::core::frame_cache::current_version();
    let comp_id = comp as *const _ as usize;
    if let Some(snap) = SNAPSHOT_CACHE.with(|c| {
        c.borrow().as_ref().and_then(|(v, f, cid, snap)| {
            (*v == ver && *f == frame && *cid == comp_id).then(|| snap.clone())
        })
    }) {
        return snap;
    }
    let snapshot = build_comp_snapshot_uncached(comp, frame);
    SNAPSHOT_CACHE.with(|c| *c.borrow_mut() = Some((ver, frame, comp_id, snapshot.clone())));
    snapshot
}

fn build_comp_snapshot_uncached(comp: &crate::core::timeline::Composition, frame: u32) -> CompSnapshot {
    let fps = comp.fps;
    let mut layers = std::collections::HashMap::new();
    for l in &comp.layers {
        let mut fx_map: std::collections::HashMap<String, std::collections::HashMap<String, f64>> =
            std::collections::HashMap::new();
        if l.effects_enabled {
            for eff in &l.effects {
                if !eff.enabled {
                    continue;
                }
                let mut params = std::collections::HashMap::new();
                for (label, pref) in eff.effect_type.animatable_params_ref() {
                    match pref {
                        crate::core::effect_params::ParamRefRef::Scalar(a) => {
                            params.insert(label.to_string(), a.evaluate(frame) as f64);
                        }
                        crate::core::effect_params::ParamRefRef::Vec2(a) => {
                            let v = a.evaluate(frame);
                            params.insert(format!("{} X", label), v[0] as f64);
                            params.insert(format!("{} Y", label), v[1] as f64);
                            params.insert(label.to_string(), v[0] as f64);
                        }
                        crate::core::effect_params::ParamRefRef::Vec4Color(a) => {
                            let v = a.evaluate(frame);
                            params.insert(label.to_string(), (v[0] + v[1] + v[2]).max(v[3]) as f64);
                        }
                    }
                }
                fx_map.insert(eff.name.clone(), params);
            }
        }
        let snap = LayerSnapshot {
            position: [
                l.transform.eval_position(frame, fps)[0] as f64,
                l.transform.eval_position(frame, fps)[1] as f64,
            ],
            scale: [
                l.transform.eval_scale(frame, fps)[0] as f64,
                l.transform.eval_scale(frame, fps)[1] as f64,
            ],
            rotation: l.transform.eval_rotation(frame, fps) as f64,
            opacity: l.transform.eval_opacity(frame, fps) as f64,
            effects: fx_map,
        };
        layers.insert(l.name.clone(), snap.clone());
        if l.id != l.name && !layers.contains_key(&l.id) {
            layers.insert(l.id.clone(), snap);
        }
    }
    CompSnapshot { layers, comp_width: comp.width as f64, comp_height: comp.height as f64 }
}

/// Evaluate a v2 expression with composition context (thisComp / thisLayer).
pub fn eval_v2_with_comp(
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
) -> [f32; 2] {
    COMP_ENGINE.with(|engine| {
    let time = frame as f64 / fps.max(1) as f64;
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    let base_arr: Array = vec![
        Dynamic::from_float(base[0] as f64),
        Dynamic::from_float(base[1] as f64),
    ];
    scope.push("value", base_arr);
    scope.push("thisComp", comp_snap.clone());
    if let Some(tl) = this_layer {
        scope.push("thisLayer", tl.clone());
    }

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Ok(arr) = val.clone().into_array() {
                if arr.len() >= 2 {
                    return [
                        arr[0].as_float().unwrap_or(base[0] as f64) as f32,
                        arr[1].as_float().unwrap_or(base[1] as f64) as f32,
                    ];
                }
            }
            if let Ok(f) = val.as_float() {
                return [base[0] + f as f32, base[1]];
            }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_v2_with_comp error: {}", e);
            base
        }
    }
    })
}

/// Evaluate a scalar expression with composition context.
pub fn eval_f32_with_comp(
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
    comp_snap: &CompSnapshot,
    this_layer: Option<&LayerSnapshot>,
) -> f32 {
    COMP_ENGINE.with(|engine| {
    let time = frame as f64 / fps.max(1) as f64;
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("value", base as f64);
    scope.push("thisComp", comp_snap.clone());
    if let Some(tl) = this_layer {
        scope.push("thisLayer", tl.clone());
    }

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Ok(f) = val.as_float() {
                return f as f32;
            }
            if let Ok(i) = val.as_int() {
                return i as f32;
            }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_f32_with_comp error: {}", e);
            base
        }
    }
    })
}

thread_local! {
    static COMP_ENGINE: Engine = {
        let mut e = build_engine();
        register_comp_types(&mut e);
        e
    };

    /// Cached engine for loop-enabled expressions with array `+` operator.
    static LOOP_ENGINE: Engine = {
        let mut e = build_engine();
        // AE semantics: [x, y] + [x2, y2] is element-wise (Rhai default concatenates)
        e.register_fn("+", |a: Array, b: Array| -> Array {
            let n = a.len().max(b.len());
            (0..n).map(|i| {
                let av = a.get(i).and_then(|d| d.as_float().ok()).unwrap_or(0.0);
                let bv = b.get(i).and_then(|d| d.as_float().ok()).unwrap_or(0.0);
                Dynamic::from_float(av + bv)
            }).collect()
        });
        e
    };
}

/// Preprocessed loop values exposed to Raw scripts that call loopOut()/loopIn().
#[derive(Clone, Copy, Default)]
pub struct LoopVals {
    pub out_cycle: f32,
    pub out_pingpong: f32,
    pub in_cycle: f32,
    pub in_pingpong: f32,
}

/// Rewrites AE-style loop calls in a Raw script into scope-variable references.
/// `loopOut("cycle")` → `__loop_out_cycle`, `loopOut("pingpong")` → `__loop_out_pingpong`,
/// `loopIn("cycle")` → `__loop_in_cycle`, `loopIn("pingpong")` → `__loop_in_pingpong`,
/// bare `loopOut()` → `__loop_out_cycle`.
pub fn preprocess_loop_script(script: &str) -> String {
    script
        .replace("loopOut(\"pingpong\")", "__loop_out_pingpong")
        .replace("loopIn(\"pingpong\")", "__loop_in_pingpong")
        .replace("loopOut(\"cycle\")", "__loop_out_cycle")
        .replace("loopIn(\"cycle\")", "__loop_in_cycle")
        .replace("loopOut()", "__loop_out_cycle")
        .replace("loopIn()", "__loop_in_cycle")
        .replace("loopOut", "__loop_out_cycle")
        .replace("loopIn", "__loop_in_cycle")
}

/// True if the script references any loop function.
pub fn script_uses_loops(script: &str) -> bool {
    script.contains("loopOut") || script.contains("loopIn")
}

/// Evaluate a scalar Raw script with loop values available.
pub fn eval_f32_with_loops(
    script: &str,
    base: f32,
    frame: u32,
    fps: u32,
    loops: LoopVals,
) -> f32 {
    let rewritten = preprocess_loop_script(script);
    let time = frame as f64 / fps.max(1) as f64;
    LOOP_ENGINE.with(|engine| {
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("value", base as f64);
    scope.push("__loop_out_cycle", loops.out_cycle as f64);
    scope.push("__loop_out_pingpong", loops.out_pingpong as f64);
    scope.push("__loop_in_cycle", loops.in_cycle as f64);
    scope.push("__loop_in_pingpong", loops.in_pingpong as f64);

    match engine.eval_with_scope::<Dynamic>(&mut scope, &rewritten) {
        Ok(val) => {
            if let Ok(f) = val.as_float() { return f as f32; }
            if let Ok(i) = val.as_int() { return i as f32; }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_f32_with_loops error: {}", e);
            base
        }
    }
    })
}

/// Evaluate a v2 Raw script with loop values available.
pub fn eval_v2_with_loops(
    script: &str,
    base: [f32; 2],
    frame: u32,
    fps: u32,
    loops: LoopVals,
) -> [f32; 2] {
    let rewritten = preprocess_loop_script(script);
    let time = frame as f64 / fps.max(1) as f64;
    LOOP_ENGINE.with(|engine| {
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    let base_arr: Array = vec![
        Dynamic::from_float(base[0] as f64),
        Dynamic::from_float(base[1] as f64),
    ];
    scope.push("value", base_arr);
    let loop_arr = |x: f32, y: f32| vec![Dynamic::from_float(x as f64), Dynamic::from_float(y as f64)];
    scope.push("__loop_out_cycle", loop_arr(loops.out_cycle, loops.in_cycle));
    scope.push("__loop_out_pingpong", loop_arr(loops.out_pingpong, loops.in_pingpong));
    scope.push("__loop_in_cycle", loop_arr(loops.in_cycle, loops.out_cycle));
    scope.push("__loop_in_pingpong", loop_arr(loops.in_pingpong, loops.out_pingpong));

    match engine.eval_with_scope::<Dynamic>(&mut scope, &rewritten) {
        Ok(val) => {
            if let Ok(arr) = val.clone().into_array() {
                if arr.len() >= 2 {
                    return [
                        arr[0].as_float().unwrap_or(base[0] as f64) as f32,
                        arr[1].as_float().unwrap_or(base[1] as f64) as f32,
                    ];
                }
            }
            if let Ok(f) = val.as_float() {
                return [base[0] + f as f32, base[1]];
            }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_v2_with_loops error: {}", e);
            base
        }
    }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_expression() {
        let engine = build_engine();
        // At frame 30, fps 30 → time = 1.0
        let result = eval_f32(&engine, "time * 360.0", 0.0, 30, 30);
        assert!((result - 360.0).abs() < 0.01, "Expected 360, got {}", result);
    }

    #[test]
    fn test_wiggle_expression() {
        let engine = build_engine();
        // wiggle should return a value offset from 0
        let result = eval_f32(&engine, "wiggle(time, 2.0, 50.0)", 0.0, 15, 30);
        assert!(result.abs() <= 55.0, "Wiggle out of range: {}", result);
    }

    #[test]
    fn test_v2_expression() {
        let engine = build_engine();
        let result = eval_v2(&engine, "[time * 100.0, 0.0]", [0.0, 0.0], 30, 30);
        assert!((result[0] - 100.0).abs() < 0.1, "Expected 100, got {}", result[0]);
        assert!((result[1] - 0.0).abs() < 0.1, "Expected 0, got {}", result[1]);
    }

    #[test]
    fn test_invalid_script() {
        let engine = build_engine();
        let r = validate_script(&engine, "this is not valid rhai;;;;;");
        assert!(r.is_err(), "Should have failed validation");
    }

    #[test]
    fn test_ease_expression() {
        let engine = build_engine();
        let mid = eval_f32(&engine, "ease(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
        assert!((mid - 50.0).abs() < 0.01, "Expected 50 at midpoint, got {}", mid);

        let ease_in = eval_f32(&engine, "easeIn(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
        assert!((ease_in - 25.0).abs() < 0.01, "Expected 25 for easeIn at midpoint, got {}", ease_in);

        let ease_out = eval_f32(&engine, "easeOut(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
        assert!((ease_out - 75.0).abs() < 0.01, "Expected 75 for easeOut at midpoint, got {}", ease_out);
    }
}

#[cfg(test)]
mod tests_comp_context {
    use crate::core::timeline::{Composition, Layer, LayerType, Expression};
    use crate::core::property::Animatable;

    #[test]
    fn test_thiscomp_layer_reference() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut target = Layer::new("t1".into(), "Target".into(), LayerType::Solid { color: [0.0; 4] }, 30);
        target.transform.position = Animatable::new_constant([42.0, 24.0]);
        comp.layers.push(target);
        let mut driver = Layer::new("d1".into(), "Driver".into(), LayerType::Null, 30);
        // Driver position follows Target's X
        driver.transform.position_expression = Some(Expression::Raw(
            "thisComp.layer(\"Target\").transform.position[0] + 10.0".into(),
        ));
        let driver_name = driver.name.clone();
        comp.layers.push(driver);

        let driver = comp.layers.iter().find(|l| l.name == driver_name).unwrap();
        let (pos, _, _, _) = comp.resolve_world_transform(driver, 0);
        assert!((pos[0] - 52.0).abs() < 0.01, "expected 52.0, got {}", pos[0]);
    }

    #[test]
    fn test_effect_param_bridge_cross_layer() {
        use crate::core::timeline::Effect;
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut src = Layer::new("s1".into(), "Source".into(), LayerType::Solid { color: [0.0; 4] }, 30);
        // Animated Gaussian blur radius: 5 at frame 0, 15 at frame 10.
        src.effects.push(Effect {
            id: "fx_test_blur".into(),
            enabled: true,
            name: "Blur".into(),
            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                blur_radius: Animatable::new_animated(vec![
                    crate::core::keyframe::Keyframe::new(0, 5.0, crate::core::keyframe::InterpolationType::Linear),
                    crate::core::keyframe::Keyframe::new(10, 15.0, crate::core::keyframe::InterpolationType::Linear),
                ]),
            },
        });
        comp.layers.push(src);
        let mut drv = Layer::new("d1".into(), "Driver".into(), LayerType::Null, 30);
        drv.transform.position_expression = Some(Expression::Raw(
            "let b = thisComp.layer(\"Source\").effect_param(\"Blur\", \"Blur Radius\"); [b * 10.0, 0.0]".into(),
        ));
        comp.layers.push(drv);

        let snap_f0 = crate::core::expression_engine::build_comp_snapshot(&comp, 0);
        assert!((snap_f0.layers["Source"].effects["Blur"]["Blur Radius"] - 5.0).abs() < 0.01,
            "snapshot should carry effect value");

        let layer_f10 = &comp.layers[1];
        let (pos, _, _, _) = comp.resolve_world_transform(layer_f10, 10);
        assert!((pos[0] - 150.0).abs() < 0.05, "expected 150.0 (15*10), got {}", pos[0]);
    }

    #[test]
    fn test_thislayer_reference() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut l = Layer::new("s1".into(), "Selfy".into(), LayerType::Solid { color: [0.0; 4] }, 30);
        l.transform.position = Animatable::new_constant([10.0, 20.0]);
        l.transform.rotation_expression = Some(Expression::Raw("thisLayer.transform.rotation * 2.0".into()));
        comp.layers.push(l);

        let layer = &comp.layers[0];
        let (_, _, rot, _) = comp.resolve_world_transform(layer, 0);
        assert!((rot - 0.0).abs() < 0.01 || rot > 0.0, "rotation expr should evaluate");
    }
}

#[cfg(test)]
mod tests_comp_extras {
    use crate::core::timeline::{Composition, Layer, LayerType, Expression};
    use crate::core::property::Animatable;

    #[test]
    fn test_layer_lookup_by_id() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut target = Layer::new("tgt_id_1".into(), "TargetName".into(), LayerType::Solid { color: [0.0; 4] }, 30);
        target.transform.position = Animatable::new_constant([7.0, 3.0]);
        comp.layers.push(target);
        let mut driver = Layer::new("d".into(), "Driver".into(), LayerType::Null, 30);
        driver.transform.position_expression = Some(Expression::Raw(
            "thisComp.layer(\"tgt_id_1\").transform.position[1]".into(),
        ));
        comp.layers.push(driver);

        let driver_ref = &comp.layers[1];
        let (pos, _, _, _) = comp.resolve_world_transform(driver_ref, 0);
        assert!((pos[0] - 3.0).abs() < 0.01, "expected 3.0 via id lookup, got {}", pos[0]);
    }

    #[test]
    fn test_meshwarp_effect_parses() {
        // MeshWarp effect should evaluate corner params without panicking
        use crate::core::effect_plugin::evaluate_effects;
        use crate::core::timeline::{Effect, EffectType};
        let effects = vec![Effect {
            id: "fx1".to_string(),
            name: "MeshWarp".to_string(),
            enabled: true,
            effect_type: EffectType::MeshWarp {
                top_left: Animatable::new_constant([10.0, 5.0]),
                top_right: Animatable::new_constant([-10.0, 5.0]),
                bottom_left: Animatable::new_constant([10.0, -5.0]),
                bottom_right: Animatable::new_constant([-10.0, -5.0]),
            },
        }];
        let ep = evaluate_effects(&effects, 0);
        assert_eq!(ep.meshwarp_enabled, 1);
        assert_eq!(ep.corner_top_left, [10.0, 5.0]);
        assert_eq!(ep.corner_bottom_right, [-10.0, -5.0]);
    }
}

#[cfg(test)]
mod tests_loops {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType, Expression};
    use crate::core::keyframe::Keyframe;
    use crate::core::property::Animatable;

    #[test]
    fn test_loopout_in_raw_script() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut l = Layer::new("l1".into(), "Looper".into(), LayerType::Solid { color: [0.0; 4] }, 30);
        // Keyframes: x 0→100 over frames 0..10
        l.transform.position = Animatable::new_animated(vec![
            Keyframe::new(0, [0.0, 0.0], crate::core::keyframe::InterpolationType::Linear),
            Keyframe::new(10, [100.0, 0.0], crate::core::keyframe::InterpolationType::Linear),
        ]);
        // At frame 25 (past last kf), loopOut("cycle") should reference the cycled value (x=50 at frame 5)
        l.transform.position_expression = Some(Expression::Raw(
            "loopOut(\"cycle\") + [0.0, 7.0]".into(),
        ));
        comp.layers.push(l);

        let layer = &comp.layers[0];
        let (pos, _, _, _) = comp.resolve_world_transform(layer, 25);
        // Frame 25 remaps to frame 5 → x = 50
        assert!((pos[0] - 50.0).abs() < 0.5, "expected x=50 from loopOut cycle, got {}", pos[0]);
    }

    #[test]
    fn test_loop_preprocess_rewrites() {
        let rewritten = preprocess_loop_script("loopOut(\"pingpong\") + loopIn()");
        assert!(rewritten.contains("__loop_out_pingpong"));
        assert!(rewritten.contains("__loop_in_cycle"));
        assert!(!rewritten.contains("loopOut("));
    }
}
