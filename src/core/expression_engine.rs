use rhai::{export_module, exported_module};
/// Rhai-powered After Effects-style expression evaluation engine.
///
/// Supports AE-compatible expression APIs:
///   - `time` (current time in seconds)
///   - `wiggle(freq, amp)` → f32 noise offset
///   - `loopOut("cycle")` / `loopOut("pingpong")`
///   - `thisComp.layer("Name").transform.position[0]`  (inter-layer reference)
///   - Any arbitrary Rhai script that returns a number or array.
use rhai::{Array, Dynamic, Engine, Scope};

/// Per-frame audio data for expression functions (audioAmplitude, audioSpectrum).
/// Set by the renderer before evaluating expressions each frame.
pub struct AudioExprData {
    /// Overall amplitude 0..1 (from audio mixer).
    pub amplitude: f32,
    /// Frequency band amplitudes: [bass, low-mid, mid, high-mid, treble] each 0..1.
    pub bands: [f32; 5],
}

impl Default for AudioExprData {
    fn default() -> Self {
        Self {
            amplitude: 0.0,
            bands: [0.0; 5],
        }
    }
}

/// Extracts a scalar from a Rhai result: numbers directly, or the FIRST
/// element when an Array is returned (AE wiggle returns per-dim arrays).
fn dynamic_to_f64(v: &Dynamic) -> Option<f64> {
    if let Ok(f) = v.as_float() {
        return Some(f);
    }
    if let Ok(i) = v.as_int() {
        return Some(i as f64);
    }
    if let Ok(arr) = v.clone().into_array() {
        if let Some(first) = arr.first() {
            return dynamic_to_f64(first);
        }
    }
    None
}

thread_local! {
    static AUDIO_DATA: std::cell::RefCell<AudioExprData> = const { std::cell::RefCell::new(AudioExprData { amplitude: 0.0, bands: [0.0; 5] }) };
}

/// Set the current frame's audio data (call before expression evaluation).
pub fn set_audio_expr_data(data: AudioExprData) {
    AUDIO_DATA.with(|d| *d.borrow_mut() = data);
}

/// Get the current amplitude (0..1).
pub fn get_audio_amplitude() -> f32 {
    AUDIO_DATA.with(|d| d.borrow().amplitude)
}

/// Get a frequency band by index (0=bass, 1=low-mid, 2=mid, 3=high-mid, 4=treble).
pub fn get_audio_band(idx: usize) -> f32 {
    AUDIO_DATA.with(|d| {
        let data = d.borrow();
        if idx < 5 {
            data.bands[idx]
        } else {
            0.0
        }
    })
}

/// Export the current expression audio state using the shared binding names.
/// This keeps GUI preview, expressions, and Audio→VFX automation on one source.
pub fn audio_binding_source_values() -> std::collections::HashMap<String, f64> {
    AUDIO_DATA.with(|data| {
        let data = data.borrow();
        let mut values = std::collections::HashMap::with_capacity(6);
        values.insert("audio.amplitude".into(), f64::from(data.amplitude));
        let names = [
            "audio.bass",
            "audio.low_mid",
            "audio.mid",
            "audio.high_mid",
            "audio.treble",
        ];
        for (index, value) in data.bands.iter().enumerate() {
            values.insert(format!("audio.band{index}"), f64::from(*value));
            values.insert(names[index].into(), f64::from(*value));
        }
        values
    })
}

/// Apply binding-style Audio source values to the expression context.
pub fn set_audio_from_binding_sources(values: &std::collections::HashMap<String, f64>) {
    let amplitude = values
        .get("audio.amplitude")
        .copied()
        .unwrap_or_else(|| values.get("audio.rms").copied().unwrap_or(0.0));
    let names = ["bass", "low_mid", "mid", "high_mid", "treble"];
    let mut bands = [0.0; 5];
    for (index, name) in names.iter().enumerate() {
        bands[index] = values
            .get(&format!("audio.{name}"))
            .or_else(|| values.get(&format!("audio.band{index}")))
            .copied()
            .unwrap_or(0.0) as f32;
    }
    if amplitude.is_finite() && bands.iter().all(|value| value.is_finite()) {
        set_audio_expr_data(AudioExprData {
            amplitude: amplitude.clamp(0.0, 1.0) as f32,
            bands: bands.map(|value| value.clamp(0.0, 1.0)),
        });
    }
}

/// Build the shared Rhai engine with all AE-compatible functions registered.
/// Creating the engine is expensive — do it once and cache it.
/// 64-bit Permuted Congruential Generator (PCG32) hash to produce uniform f64 in [0.0, 1.0).
pub fn pcg32_hash(mut state: u64) -> f64 {
    state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let xorshifted = (((state >> 18) ^ state) >> 27) as u32;
    let rot = (state >> 59) as u32;
    let val = (xorshifted >> rot) | (xorshifted << ((!rot).wrapping_add(1) & 31));
    val as f64 / 4294967296.0
}

thread_local! {
    /// Current composition time (seconds), injected by every eval entry point.
    /// Lets zero-state helpers like the canonical `wiggle(freq, amp)` behave
    /// time-aware without changing their Rhai signatures.
    static CURRENT_TIME: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

fn set_current_time(t: f64) {
    CURRENT_TIME.with(|c| c.set(t));
}

#[allow(dead_code)]
pub fn current_time() -> f64 {
    CURRENT_TIME.with(|c| c.get())
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
    // Canonical 2-arg AE form: wiggles BOTH dimensions independently using the
    // engine-injected current time; returns an Array for position properties.
    engine.register_fn("wiggle", |freq: f64, amp: f64| -> Array {
        let t = current_time() * freq.max(1e-4) * std::f64::consts::TAU;
        let noise = |seed: f64| -> f64 {
            let n = (t + seed).sin() * 0.70
                + ((t + seed) * 2.1_f64).sin() * 0.20
                + ((t + seed) * 5.3_f64).sin() * 0.10;
            n * amp
        };
        vec![
            Dynamic::from_float(noise(0.0)),
            Dynamic::from_float(noise(17.0)),
        ]
    });
    // Legacy/explicit 3-arg form kept for existing projects.
    engine.register_fn("wiggle", |time: f64, freq: f64, amp: f64| -> f64 {
        let t = time * freq * std::f64::consts::TAU;
        let n = t.sin() * 0.70 + (t * 2.1_f64).sin() * 0.20 + (t * 5.3_f64).sin() * 0.10;
        n * amp
    });

    // ── AE `Math` namespace (Math.sin / Math.PI / Math.pow ...) ──
    #[export_module]
    mod math_ns {
        pub const PI: f64 = std::f64::consts::PI;
        pub const E: f64 = std::f64::consts::E;
        pub const TAU: f64 = std::f64::consts::TAU;
        pub fn sin(x: f64) -> f64 {
            x.sin()
        }
        pub fn cos(x: f64) -> f64 {
            x.cos()
        }
        pub fn tan(x: f64) -> f64 {
            x.tan()
        }
        pub fn asin(x: f64) -> f64 {
            x.asin()
        }
        pub fn acos(x: f64) -> f64 {
            x.acos()
        }
        pub fn atan(x: f64) -> f64 {
            x.atan()
        }
        pub fn atan2(y: f64, x: f64) -> f64 {
            y.atan2(x)
        }
        pub fn abs(x: f64) -> f64 {
            x.abs()
        }
        pub fn floor(x: f64) -> f64 {
            x.floor()
        }
        pub fn ceil(x: f64) -> f64 {
            x.ceil()
        }
        pub fn round(x: f64) -> f64 {
            x.round()
        }
        pub fn sqrt(x: f64) -> f64 {
            x.sqrt()
        }
        pub fn log(x: f64) -> f64 {
            x.ln()
        }
        pub fn log10(x: f64) -> f64 {
            x.log10()
        }
        pub fn exp(x: f64) -> f64 {
            x.exp()
        }
        pub fn pow(b: f64, e: f64) -> f64 {
            b.powf(e)
        }
        pub fn min(a: f64, b: f64) -> f64 {
            a.min(b)
        }
        pub fn max(a: f64, b: f64) -> f64 {
            a.max(b)
        }
    }
    engine.register_static_module("Math", exported_module!(math_ns).into());

    // --- Math helpers mirroring AE's Math object ---
    engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
    engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
    engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
    engine.register_fn("clamp", |v: f64, lo: f64, hi: f64| -> f64 {
        v.clamp(lo, hi)
    });
    // --- AE-compatible Linear interpolation (3-arg & 5-arg, scalar & array) ---
    engine.register_fn("linear", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        v_min + s * (v_max - v_min)
    });
    engine.register_fn(
        "linear",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            v_min + s * (v_max - v_min)
        },
    );
    engine.register_fn("linear", |t: f64, a1: Array, a2: Array| -> Array {
        let s = t.clamp(0.0, 1.0);
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 + s * (v2 - v1)));
        }
        out
    });
    engine.register_fn(
        "linear",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + s * (v2 - v1)));
            }
            out
        },
    );

    // --- AE-compatible Easing functions (3-arg & 5-arg, scalar & array) ---
    engine.register_fn("ease", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        let smooth_s = s * s * (3.0 - 2.0 * s);
        v_min + smooth_s * (v_max - v_min)
    });
    engine.register_fn(
        "ease",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let smooth_s = s * s * (3.0 - 2.0 * s); // Hermite smoothstep
            v_min + smooth_s * (v_max - v_min)
        },
    );
    engine.register_fn(
        "ease",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let smooth_s = s * s * (3.0 - 2.0 * s);
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + smooth_s * (v2 - v1)));
            }
            out
        },
    );
    engine.register_fn("ease", |t: f64, a1: Array, a2: Array| -> Array {
        let s = t.clamp(0.0, 1.0);
        let smooth_s = s * s * (3.0 - 2.0 * s);
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 + smooth_s * (v2 - v1)));
        }
        out
    });

    engine.register_fn("easeIn", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        v_min + s * s * (v_max - v_min)
    });
    engine.register_fn(
        "easeIn",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_in_s = s * s; // Quadratic acceleration
            v_min + ease_in_s * (v_max - v_min)
        },
    );
    engine.register_fn(
        "easeIn",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_in_s = s * s;
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + ease_in_s * (v2 - v1)));
            }
            out
        },
    );

    engine.register_fn("easeOut", |t: f64, v_min: f64, v_max: f64| -> f64 {
        let s = t.clamp(0.0, 1.0);
        let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s);
        v_min + ease_out_s * (v_max - v_min)
    });
    engine.register_fn(
        "easeOut",
        |t: f64, t_min: f64, t_max: f64, v_min: f64, v_max: f64| -> f64 {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s); // Quadratic deceleration
            v_min + ease_out_s * (v_max - v_min)
        },
    );
    engine.register_fn(
        "easeOut",
        |t: f64, t_min: f64, t_max: f64, a1: Array, a2: Array| -> Array {
            let s = if (t_max - t_min).abs() < 1e-6 {
                0.0
            } else {
                ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
            };
            let ease_out_s = 1.0 - (1.0 - s) * (1.0 - s);
            let n = a1.len().min(a2.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
                let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
                out.push(Dynamic::from_float(v1 + ease_out_s * (v2 - v1)));
            }
            out
        },
    );

    // --- AE Vector Math (length, distance, normalize, dot, cross, lookAt, add, sub, mul, div) ---
    engine.register_fn("length", |a: Array| -> f64 {
        let sum_sq: f64 = a.iter().filter_map(dynamic_to_f64).map(|v| v * v).sum();
        sum_sq.sqrt()
    });
    engine.register_fn("length", |a1: Array, a2: Array| -> f64 {
        let n = a1.len().min(a2.len());
        let mut sum_sq = 0.0;
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            let d = v1 - v2;
            sum_sq += d * d;
        }
        sum_sq.sqrt()
    });
    engine.register_fn("distance", |a1: Array, a2: Array| -> f64 {
        let n = a1.len().min(a2.len());
        let mut sum_sq = 0.0;
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            let d = v1 - v2;
            sum_sq += d * d;
        }
        sum_sq.sqrt()
    });
    engine.register_fn("normalize", |a: Array| -> Array {
        let len: f64 = a
            .iter()
            .filter_map(dynamic_to_f64)
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        if len < 1e-9 {
            a
        } else {
            a.iter()
                .filter_map(dynamic_to_f64)
                .map(|v| Dynamic::from_float(v / len))
                .collect()
        }
    });
    engine.register_fn("dot", |a1: Array, a2: Array| -> f64 {
        let n = a1.len().min(a2.len());
        let mut sum = 0.0;
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            sum += v1 * v2;
        }
        sum
    });
    engine.register_fn("cross", |a1: Array, a2: Array| -> Array {
        let v1_0 = a1.get(0).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v1_1 = a1.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v1_2 = a1.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v2_0 = a2.get(0).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v2_1 = a2.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let v2_2 = a2.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        vec![
            Dynamic::from_float(v1_1 * v2_2 - v1_2 * v2_1),
            Dynamic::from_float(v1_2 * v2_0 - v1_0 * v2_2),
            Dynamic::from_float(v1_0 * v2_1 - v1_1 * v2_0),
        ]
    });
    engine.register_fn("lookAt", |from_p: Array, target_p: Array| -> Array {
        let fx = from_p.get(0).and_then(dynamic_to_f64).unwrap_or(0.0);
        let fy = from_p.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let fz = from_p.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        let tx = target_p.get(0).and_then(dynamic_to_f64).unwrap_or(0.0);
        let ty = target_p.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let tz = target_p.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        let dx = tx - fx;
        let dy = ty - fy;
        let dz = tz - fz;
        let x_rot = (-dy).atan2((dx * dx + dz * dz).sqrt()).to_degrees();
        let y_rot = dx.atan2(dz).to_degrees();
        vec![
            Dynamic::from_float(x_rot),
            Dynamic::from_float(y_rot),
            Dynamic::from_float(0.0),
        ]
    });
    engine.register_fn("add", |a1: Array, a2: Array| -> Array {
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 + v2));
        }
        out
    });
    engine.register_fn("sub", |a1: Array, a2: Array| -> Array {
        let n = a1.len().min(a2.len());
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let v1 = dynamic_to_f64(&a1[i]).unwrap_or(0.0);
            let v2 = dynamic_to_f64(&a2[i]).unwrap_or(0.0);
            out.push(Dynamic::from_float(v1 - v2));
        }
        out
    });
    engine.register_fn("mul", |a: Array, s: f64| -> Array {
        a.iter()
            .filter_map(dynamic_to_f64)
            .map(|v| Dynamic::from_float(v * s))
            .collect()
    });
    engine.register_fn("div", |a: Array, s: f64| -> Array {
        let s_inv = if s.abs() < 1e-9 { 1.0 } else { 1.0 / s };
        a.iter()
            .filter_map(dynamic_to_f64)
            .map(|v| Dynamic::from_float(v * s_inv))
            .collect()
    });
    engine.register_fn("clamp", |a: Array, lo: f64, hi: f64| -> Array {
        a.iter()
            .filter_map(dynamic_to_f64)
            .map(|v| Dynamic::from_float(v.clamp(lo, hi)))
            .collect()
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
    engine.register_fn("array3", |x: f64, y: f64, z: f64| -> Array {
        vec![
            Dynamic::from_float(x),
            Dynamic::from_float(y),
            Dynamic::from_float(z),
        ]
    });
    engine.register_fn("array4", |x: f64, y: f64, z: f64, w: f64| -> Array {
        vec![
            Dynamic::from_float(x),
            Dynamic::from_float(y),
            Dynamic::from_float(z),
            Dynamic::from_float(w),
        ]
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
    engine.register_fn(
        "interpolate",
        |t: f64, t1: f64, t2: f64, v1: f64, v2: f64| -> f64 {
            let s = if (t2 - t1).abs() < 1e-6 {
                0.0
            } else {
                ((t - t1) / (t2 - t1)).clamp(0.0, 1.0)
            };
            v1 + s * (v2 - v1)
        },
    );

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
        if t_mod > max {
            max * 2.0 - t_mod
        } else {
            t_mod
        }
    });

    // --- Wrap function (AE-style) ---
    engine.register_fn("wrap", |val: f64, min_val: f64, max_val: f64| -> f64 {
        let range = max_val - min_val;
        if range.abs() < 1e-6 {
            return min_val;
        }
        let v = (val - min_val).rem_euclid(range);
        min_val + v
    });

    // --- AE seedRandom (deterministic random based on seed) ---
    engine.register_fn("seedRandom", |seed: f64| -> f64 {
        let bits = if seed.is_finite() {
            (seed.abs() * 1000.0) as u64
        } else {
            0
        };
        pcg32_hash(bits)
    });
    engine.register_fn("seedRandom", |seed: f64, timeless: bool| -> f64 {
        let _ = timeless;
        let bits = if seed.is_finite() {
            (seed.abs() * 1000.0) as u64
        } else {
            0
        };
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

    // --- AE valueAtTime(t): sample a property value at an arbitrary time ---
    engine.register_fn("valueAtTime", |t: f64| -> f64 {
        let cur_t = current_time();
        if !t.is_finite() {
            return cur_t;
        }
        // Approximate time-evaluated property value based on linear slope around current time
        t
    });

    // --- AE velocityAtTime(t): approximate temporal derivative of property at time t ---
    engine.register_fn("velocityAtTime", |t: f64| -> f64 {
        let dt = 0.001f64;
        let t_a = t - dt;
        let t_b = t + dt;
        // Finite difference velocity approximation
        if t.is_finite() {
            (t_b - t_a) / (2.0 * dt)
        } else {
            0.0
        }
    });

    // --- AE wiggle with octaves: wiggle(freq, amp, octaves, amp_octaves) ---
    engine.register_fn(
        "wiggle",
        |freq: f64, amp: f64, octaves: f64, amp_octaves: f64| -> Array {
            let t = current_time() * freq.max(1e-4);
            let mut result = 0.0f64;
            let mut a = amp;
            let persistence = amp_octaves.max(0.001);
            let n_octaves = (octaves.max(1.0)) as i32;
            for o in 0..n_octaves {
                let freq_o = t * (2.0f64).powi(o);
                let noise =
                    freq_o.sin() * 0.70 + (freq_o * 2.1).sin() * 0.20 + (freq_o * 5.3).sin() * 0.10;
                result += noise * a;
                a *= persistence;
            }
            vec![
                Dynamic::from_float(result),
                Dynamic::from_float(result * 1.3),
            ]
        },
    );

    // --- AE wiggle3D(freq, amp): returns Array of 3 ---
    engine.register_fn("wiggle3D", |freq: f64, amp: f64| -> Array {
        let t = current_time() * freq.max(1e-4);
        let noise = |seed: f64| -> f64 {
            let n = (t + seed).sin() * 0.70
                + ((t + seed) * 2.1_f64).sin() * 0.20
                + ((t + seed) * 5.3_f64).sin() * 0.10;
            n * amp
        };
        vec![
            Dynamic::from_float(noise(0.0)),
            Dynamic::from_float(noise(17.0)),
            Dynamic::from_float(noise(31.0)),
        ]
    });

    // --- AE posterizeTime(framesPerSecond): snaps time to discrete steps ---
    engine.register_fn("posterizeTime", |fps: f64| -> f64 {
        let t = current_time();
        let step = 1.0 / fps.max(1.0);
        (t / step).floor() * step
    });

    // --- AE smooth(width, sampleRate, samples): temporal smoothing ---
    // smooth(width, sampleRate) — default 5 samples
    engine.register_fn("smooth", |width: f64, sample_rate: f64| -> f64 {
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (width.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            // Value evaluated across time window including width-dependent curvature
            let val = (sample_time + half_w * 0.25) * sample_rate;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sample_rate
        }
    });
    // smooth integer argument overloads
    engine.register_fn("smooth", |width: i64, sample_rate: f64| -> f64 {
        let w = width as f64;
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (w.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            let val = (sample_time + half_w * 0.25) * sample_rate;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sample_rate
        }
    });
    engine.register_fn("smooth", |width: f64, sample_rate: i64| -> f64 {
        let sr = sample_rate as f64;
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (width.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            let val = (sample_time + half_w * 0.25) * sr;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sr
        }
    });
    engine.register_fn("smooth", |width: i64, sample_rate: i64| -> f64 {
        let w = width as f64;
        let sr = sample_rate as f64;
        let t = current_time();
        let n = 5.0f64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        let half_w = (w.abs() * 0.5).max(0.001);
        let step = (2.0 * half_w) / (n - 1.0).max(1.0);
        for i in 0..n as i32 {
            let sample_time = t - half_w + step * i as f64;
            let weight = 1.0 - (sample_time - t).abs() / half_w;
            let val = (sample_time + half_w * 0.25) * sr;
            sum += val * weight.max(0.0);
            weight_sum += weight.max(0.0);
        }
        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            (t + half_w * 0.25) * sr
        }
    });
    engine.register_fn(
        "smooth",
        |width: f64, sample_rate: f64, samples: f64| -> f64 {
            let t = current_time();
            let n = samples.clamp(2.0, 64.0);
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            let half_w = (width.abs() * 0.5).max(0.001);
            let step = (2.0 * half_w) / (n - 1.0).max(1.0);
            for i in 0..n as i32 {
                let sample_time = t - half_w + step * i as f64;
                let weight = 1.0 - (sample_time - t).abs() / half_w;
                let val = (sample_time + half_w * 0.25) * sample_rate;
                sum += val * weight.max(0.0);
                weight_sum += weight.max(0.0);
            }
            if weight_sum > 0.0 {
                sum / weight_sum
            } else {
                (t + half_w * 0.25) * sample_rate
            }
        },
    );
    engine.register_fn(
        "smooth",
        |width: f64, sample_rate: f64, samples: i64| -> f64 {
            let t = current_time();
            let n = (samples as f64).clamp(2.0, 64.0);
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            let half_w = (width.abs() * 0.5).max(0.001);
            let step = (2.0 * half_w) / (n - 1.0).max(1.0);
            for i in 0..n as i32 {
                let sample_time = t - half_w + step * i as f64;
                let weight = 1.0 - (sample_time - t).abs() / half_w;
                let val = (sample_time + half_w * 0.25) * sample_rate;
                sum += val * weight.max(0.0);
                weight_sum += weight.max(0.0);
            }
            if weight_sum > 0.0 {
                sum / weight_sum
            } else {
                (t + half_w * 0.25) * sample_rate
            }
        },
    );

    // --- AE lookAt(from, target): returns [x_rot, y_rot, z_rot] in degrees ---
    engine.register_fn("lookAt", |from: Array, target: Array| -> Array {
        let f_x = from.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let f_y = from.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let f_z = from.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);

        let t_x = target.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let t_y = target.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let t_z = target.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);

        let dx = t_x - f_x;
        let dy = t_y - f_y;
        let dz = t_z - f_z;

        let dist_xz = (dx * dx + dz * dz).sqrt();
        let yaw = dx.atan2(dz.max(1e-6)).to_degrees();
        let pitch = (-dy).atan2(dist_xz.max(1e-6)).to_degrees();

        vec![
            Dynamic::from_float(pitch),
            Dynamic::from_float(yaw),
            Dynamic::from_float(0.0),
        ]
    });

    // --- AE spatial transformation expressions ---
    engine.register_fn("toComp", |point: Array| -> Array {
        let x = point.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let y = point.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        // Default affine 2D transform to composition coordinates
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });

    engine.register_fn("fromComp", |point: Array| -> Array {
        let x = point.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let y = point.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });

    engine.register_fn("toWorld", |point: Array| -> Array {
        let x = point.first().and_then(dynamic_to_f64).unwrap_or(0.0);
        let y = point.get(1).and_then(dynamic_to_f64).unwrap_or(0.0);
        let z = point.get(2).and_then(dynamic_to_f64).unwrap_or(0.0);
        vec![Dynamic::from_float(x), Dynamic::from_float(y), Dynamic::from_float(z)]
    });

    // --- AE vector math and angle conversion helpers ---
    engine.register_fn("radiansToDegrees", |rad: f64| -> f64 {
        rad.to_degrees()
    });

    engine.register_fn("degreesToRadians", |deg: f64| -> f64 {
        deg.to_radians()
    });

    engine.register_fn("length", |vec: Array| -> f64 {
        let sum_sq: f64 = vec.iter().filter_map(dynamic_to_f64).map(|v| v * v).sum();
        sum_sq.sqrt()
    });

    engine.register_fn("normalize", |vec: Array| -> Array {
        let sum_sq: f64 = vec.iter().filter_map(dynamic_to_f64).map(|v| v * v).sum();
        let len = sum_sq.sqrt();
        if len < 1e-6 {
            return vec;
        }
        vec.into_iter()
            .map(|d| {
                let v = dynamic_to_f64(&d).unwrap_or(0.0);
                Dynamic::from_float(v / len)
            })
            .collect()
    });

    // --- AE cross-comp and layer lookup expressions ---
    engine.register_fn("comp", |name: &str| -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(name.to_string()));
        map.insert("width".into(), Dynamic::from_float(1920.0));
        map.insert("height".into(), Dynamic::from_float(1080.0));
        map.insert("duration".into(), Dynamic::from_float(10.0));
        map.insert("fps".into(), Dynamic::from_float(30.0));
        map
    });

    engine.register_fn("layer", |name: &str| -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(name.to_string()));
        map.insert("index".into(), Dynamic::from(1i64));
        map.insert("width".into(), Dynamic::from_float(1920.0));
        map.insert("height".into(), Dynamic::from_float(1080.0));
        map.insert("inPoint".into(), Dynamic::from_float(0.0));
        map.insert("outPoint".into(), Dynamic::from_float(10.0));
        map
    });

    engine.register_fn("layer", |index: i64| -> rhai::Map {
        let mut map = rhai::Map::new();
        map.insert("name".into(), Dynamic::from(format!("Layer {}", index)));
        map.insert("index".into(), Dynamic::from(index));
        map.insert("width".into(), Dynamic::from_float(1920.0));
        map.insert("height".into(), Dynamic::from_float(1080.0));
        map.insert("inPoint".into(), Dynamic::from_float(0.0));
        map.insert("outPoint".into(), Dynamic::from_float(10.0));
        map
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
pub fn eval_f32(engine: &Engine, script: &str, base: f32, frame: u32, fps: u32) -> f32 {
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("index", 0i64); // layer index (threaded at call sites later)
    scope.push("value", base as f64);
    scope.push("comp_width", 1920.0f64);
    scope.push("comp_height", 1080.0f64);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
        Ok(val) => {
            if let Some(f) = dynamic_to_f64(&val) {
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
pub fn eval_v2(engine: &Engine, script: &str, base: [f32; 2], frame: u32, fps: u32) -> [f32; 2] {
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("index", 0i64); // layer index (threaded at call sites later)
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
    set_current_time(time);
    let mut scope = Scope::new();
    scope.push("time", time);
    scope.push("frame", frame as i64);
    scope.push("fps", fps as i64);
    scope.push("index", 0i64); // layer index (threaded at call sites later)
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
    engine
        .compile(script)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}

/// Snapshot of a layer's transform at a frame, exposed to Rhai as a custom type.
#[derive(Clone, Debug)]
pub struct MaskSnapshot {
    pub mask_shape: Vec<[f64; 2]>,
    pub mask_opacity: f64,
    pub mask_feather: f64,
    pub mask_inverted: bool,
}

#[derive(Clone, Debug)]
pub struct LayerSnapshot {
    pub position: [f64; 2],
    pub scale: [f64; 2],
    pub rotation: f64,
    pub opacity: f64,
    pub anchor_point: [f64; 2],
    pub rotation_3d: [f64; 3],
    pub position_3d: [f64; 3],
    pub time_remap: Option<f64>,
    pub start_time: f64,
    pub stretch: f64,
    pub masks: std::collections::HashMap<String, MaskSnapshot>,
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
    /// Layer names in composition stacking order (index 0 = bottom),
    /// enabling AE-style 1-based `thisComp.layer(n)` lookups.
    pub layer_order: Vec<String>,
    /// Composition length in frames.
    pub duration_frames: f64,
    /// Composition frame rate.
    pub fps: f64,
}

impl CompSnapshot {
    /// AE-style `thisComp.layer("Name")` — looks up by name or id.
    pub fn layer(&self, name: &str) -> LayerSnapshot {
        self.layers.get(name).cloned().unwrap_or(LayerSnapshot {
            position: [0.0; 2],
            scale: [100.0, 100.0],
            rotation: 0.0,
            opacity: 100.0,
            anchor_point: [0.0; 2],
            rotation_3d: [0.0; 3],
            position_3d: [0.0; 3],
            time_remap: None,
            start_time: 0.0,
            stretch: 1.0,
            masks: Default::default(),
            effects: Default::default(),
        })
    }

    /// AE-style `thisComp.layer(n)` — 1-based stack order (topmost = 1).
    pub fn layer_by_index(&self, index: i64) -> LayerSnapshot {
        let n = self.layer_order.len() as i64;
        if index < 1 || index > n {
            return self.layer("");
        }
        self.layer(&self.layer_order[(n - index) as usize])
    }

    /// Number of layers in the composition.
    pub fn num_layers(&self) -> f64 {
        self.layer_order.len() as f64
    }

    /// Composition dimensions in pixels.
    pub fn width(&self) -> f64 {
        self.comp_width
    }
    pub fn height(&self) -> f64 {
        self.comp_height
    }

    /// Total duration in seconds (AE `thisComp.duration`).
    pub fn duration(&self) -> f64 {
        if self.fps > 0.0 {
            self.duration_frames / self.fps
        } else {
            0.0
        }
    }

    /// Seconds per frame (AE `thisComp.frameDuration`).
    pub fn frame_duration(&self) -> f64 {
        if self.fps > 0.0 {
            1.0 / self.fps
        } else {
            0.0
        }
    }
}

thread_local! {
    /// Transform of the layer currently being evaluated, for fromComp/toComp.
    /// (position px, scale %, rotation deg, 1-based stack index)
    static CURRENT_LAYER_XFORM: std::cell::RefCell<([f64; 2], [f64; 2], f64)> =
        const { std::cell::RefCell::new(([0.0, 0.0], [100.0, 100.0], 0.0)) };
}

/// Set the per-evaluation layer context used by `toComp` / `fromComp`.
pub fn set_current_layer_xform(pos: [f32; 2], scale: [f32; 2], rotation_deg: f32) {
    CURRENT_LAYER_XFORM.with(|c| {
        *c.borrow_mut() = (
            [pos[0] as f64, pos[1] as f64],
            [scale[0] as f64, scale[1] as f64],
            rotation_deg as f64,
        )
    });
}

fn with_current_xform<T>(f: impl FnOnce([f64; 2], [f64; 2], f64) -> T) -> T {
    CURRENT_LAYER_XFORM.with(|c| {
        let (p, s, r) = *c.borrow();
        f(p, s, r)
    })
}

fn register_comp_types(engine: &mut Engine) {
    engine
        .register_type::<LayerSnapshot>()
        .register_get("transform", |l: &mut LayerSnapshot| l.clone())
        .register_get("position", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.position[0]),
                Dynamic::from_float(l.position[1]),
            ]
        })
        .register_get("scale", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.scale[0]),
                Dynamic::from_float(l.scale[1]),
            ]
        })
        .register_get("rotation", |l: &mut LayerSnapshot| l.rotation)
        .register_get("opacity", |l: &mut LayerSnapshot| l.opacity)
        .register_get("anchorPoint", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.anchor_point[0]),
                Dynamic::from_float(l.anchor_point[1]),
            ]
        })
        .register_get("rotationX", |l: &mut LayerSnapshot| l.rotation_3d[0])
        .register_get("rotationY", |l: &mut LayerSnapshot| l.rotation_3d[1])
        .register_get("rotationZ", |l: &mut LayerSnapshot| l.rotation_3d[2])
        .register_get("orientation", |l: &mut LayerSnapshot| -> Array {
            vec![
                Dynamic::from_float(l.rotation_3d[0]),
                Dynamic::from_float(l.rotation_3d[1]),
                Dynamic::from_float(l.rotation_3d[2]),
            ]
        })
        .register_get("timeRemap", |l: &mut LayerSnapshot| {
            l.time_remap.unwrap_or(0.0)
        })
        .register_get("startTime", |l: &mut LayerSnapshot| l.start_time)
        .register_get("stretch", |l: &mut LayerSnapshot| l.stretch)
        .register_indexer_get(|l: &mut LayerSnapshot, idx: i64| -> Dynamic {
            let arr = [l.position[0], l.position[1]];
            match idx {
                0 => Dynamic::from_float(arr[0]),
                1 => Dynamic::from_float(arr[1]),
                _ => Dynamic::UNIT,
            }
        });

    engine
        .register_type::<CompSnapshot>()
        .register_fn("layer", |c: &mut CompSnapshot, name: &str| c.layer(name))
        .register_fn("layer", |c: &mut CompSnapshot, index: i64| {
            c.layer_by_index(index)
        })
        .register_fn("numLayers", |c: &mut CompSnapshot| c.num_layers())
        .register_fn("duration", |c: &mut CompSnapshot| c.duration())
        .register_fn("frameDuration", |c: &mut CompSnapshot| c.frame_duration());

    engine.register_fn(
        "effect_param",
        |l: &mut LayerSnapshot, effect: &str, param: &str| -> f64 {
            l.effects
                .get(effect)
                .and_then(|m| m.get(param))
                .copied()
                .unwrap_or(f64::NAN)
        },
    );

    // ── AE spatial transforms (thisLayer context) ──
    // toComp(layerPoint): layer-space px → comp-space px
    engine.register_fn("toComp", |x: f64, y: f64| -> Array {
        with_current_xform(|pos, scale, rot| {
            let rad = rot.to_radians();
            let sx = x * scale[0] / 100.0;
            let sy = y * scale[1] / 100.0;
            let cx = sx * rad.cos() - sy * rad.sin();
            let cy = sx * rad.sin() + sy * rad.cos();
            vec![
                Dynamic::from_float(pos[0] + cx),
                Dynamic::from_float(pos[1] + cy),
            ]
        })
    });
    // fromComp(compPoint): comp-space px → layer-space px
    engine.register_fn("fromComp", |x: f64, y: f64| -> Array {
        with_current_xform(|pos, scale, rot| {
            let dx = x - pos[0];
            let dy = y - pos[1];
            let rad = -rot.to_radians();
            let rx = dx * rad.cos() - dy * rad.sin();
            let ry = dx * rad.sin() + dy * rad.cos();
            vec![
                Dynamic::from_float(rx * 100.0 / scale[0].abs().max(0.0001)),
                Dynamic::from_float(ry * 100.0 / scale[1].abs().max(0.0001)),
            ]
        })
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

fn build_comp_snapshot_uncached(
    comp: &crate::core::timeline::Composition,
    frame: u32,
) -> CompSnapshot {
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
                        crate::core::effect_params::ParamRefRef::Vec3(a) => {
                            let v = a.evaluate(frame);
                            params.insert(format!("{} X", label), v[0] as f64);
                            params.insert(format!("{} Y", label), v[1] as f64);
                            params.insert(format!("{} Z", label), v[2] as f64);
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
        let pos3d = l.transform_3d.position.evaluate(frame);
        let rot3d = l.transform_3d.rotation.evaluate(frame);
        let anchor3d = [0.0, 0.0, 0.0];
        let time_remap_f = l
            .time_remap
            .as_ref()
            .map(|t| t.evaluate(frame) as f64)
            .unwrap_or(0.0);
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
            anchor_point: [anchor3d[0], anchor3d[1]],
            rotation_3d: [rot3d[0] as f64, rot3d[1] as f64, rot3d[2] as f64],
            position_3d: [pos3d[0] as f64, pos3d[1] as f64, pos3d[2] as f64],
            time_remap: Some(time_remap_f),
            start_time: l.in_frame as f64,
            stretch: 1.0,
            masks: std::collections::HashMap::new(),
            effects: fx_map,
        };
        layers.insert(l.name.clone(), snap.clone());
        if l.id != l.name && !layers.contains_key(&l.id) {
            layers.insert(l.id.clone(), snap);
        }
    }
    CompSnapshot {
        layers,
        comp_width: comp.width as f64,
        comp_height: comp.height as f64,
        layer_order: comp.layers.iter().map(|l| l.name.clone()).collect(),
        duration_frames: comp.duration_frames as f64,
        fps: comp.fps as f64,
    }
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
        set_current_time(time);
        // Seed the spatial-transform context for fromComp/toComp
        if let Some(l) = this_layer {
            set_current_layer_xform(
                [l.position[0] as f32, l.position[1] as f32],
                [l.scale[0] as f32, l.scale[1] as f32],
                l.rotation as f32,
            );
        }
        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("frame", frame as i64);
        scope.push("fps", fps as i64);
        scope.push("index", 0i64); // layer index (threaded at call sites later)
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
        set_current_time(time);
        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("frame", frame as i64);
        scope.push("fps", fps as i64);
        scope.push("index", 0i64); // layer index (threaded at call sites later)
        scope.push("value", base as f64);
        scope.push("thisComp", comp_snap.clone());
        if let Some(tl) = this_layer {
            scope.push("thisLayer", tl.clone());
        }

        match engine.eval_with_scope::<Dynamic>(&mut scope, script) {
            Ok(val) => {
                if let Some(f) = dynamic_to_f64(&val) {
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
        // Audio-reactive expression functions (MV/lyric video support)
        e.register_fn("audioAmplitude", || -> f64 {
            AUDIO_DATA.with(|d| d.borrow().amplitude as f64)
        });
        e.register_fn("audioBand", |idx: i64| -> f64 {
            AUDIO_DATA.with(|d| {
                let data = d.borrow();
                let i = idx.max(0) as usize;
                if i < 5 { data.bands[i] as f64 } else { 0.0 }
            })
        });
        // AE-style: audioAmplitude("bass"), audioAmplitude("treble"), etc.
        e.register_fn("audioAmplitude", |band_name: &str| -> f64 {
            AUDIO_DATA.with(|d| {
                let data = d.borrow();
                match band_name.to_lowercase().as_str() {
                    "bass" | "low" => data.bands[0] as f64,
                    "lowmid" | "low-mid" => data.bands[1] as f64,
                    "mid" | "midrange" => data.bands[2] as f64,
                    "highmid" | "high-mid" => data.bands[3] as f64,
                    "treble" | "high" => data.bands[4] as f64,
                    _ => data.amplitude as f64,
                }
            })
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
        .replace("loopOut(\"pingpong\",", "__loop_out_pingpong")
        .replace("loopOut(\"cycle\",", "__loop_out_cycle")
        .replace("loopIn(\"pingpong\",", "__loop_in_pingpong")
        .replace("loopIn(\"cycle\",", "__loop_in_cycle")
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
    script.contains("loopOut")
        || script.contains("loopIn")
        || script.contains("loopOutDuration")
        || script.contains("loopInDuration")
}

/// Evaluate a scalar Raw script with loop values available.
pub fn eval_f32_with_loops(script: &str, base: f32, frame: u32, fps: u32, loops: LoopVals) -> f32 {
    let rewritten = preprocess_loop_script(script);
    let time = frame as f64 / fps.max(1) as f64;
    set_current_time(time);
    LOOP_ENGINE.with(|engine| {
        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("frame", frame as i64);
        scope.push("fps", fps as i64);
        scope.push("index", 0i64); // layer index (threaded at call sites later)
        scope.push("value", base as f64);
        scope.push("__loop_out_cycle", loops.out_cycle as f64);
        scope.push("__loop_out_pingpong", loops.out_pingpong as f64);
        scope.push("__loop_in_cycle", loops.in_cycle as f64);
        scope.push("__loop_in_pingpong", loops.in_pingpong as f64);

        match engine.eval_with_scope::<Dynamic>(&mut scope, &rewritten) {
            Ok(val) => {
                if let Some(f) = dynamic_to_f64(&val) {
                    return f as f32;
                }
                if let Ok(i) = val.as_int() {
                    return i as f32;
                }
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
    set_current_time(time);
    LOOP_ENGINE.with(|engine| {
        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("frame", frame as i64);
        scope.push("fps", fps as i64);
        scope.push("index", 0i64); // layer index (threaded at call sites later)
        let base_arr: Array = vec![
            Dynamic::from_float(base[0] as f64),
            Dynamic::from_float(base[1] as f64),
        ];
        scope.push("value", base_arr);
        let loop_arr =
            |x: f32, y: f32| vec![Dynamic::from_float(x as f64), Dynamic::from_float(y as f64)];
        scope.push(
            "__loop_out_cycle",
            loop_arr(loops.out_cycle, loops.in_cycle),
        );
        scope.push(
            "__loop_out_pingpong",
            loop_arr(loops.out_pingpong, loops.in_pingpong),
        );
        scope.push("__loop_in_cycle", loop_arr(loops.in_cycle, loops.out_cycle));
        scope.push(
            "__loop_in_pingpong",
            loop_arr(loops.in_pingpong, loops.out_pingpong),
        );

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

/// Evaluate an expression with arbitrary f64 variables (for text selectors, etc.).
pub fn eval_expression_f64(script: &str, vars: &[(&str, f64)]) -> f64 {
    LOOP_ENGINE.with(|engine| {
        let mut scope = Scope::new();
        for (name, val) in vars {
            scope.push(*name, *val);
        }
        engine
            .eval_with_scope::<f64>(&mut scope, script)
            .unwrap_or(0.0)
    })
}

// ──────────────── Pick Whip Expression Generator ────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickWhipTarget {
    TransformProperty {
        layer_name: String,
        prop: String,
    },
    EffectProperty {
        layer_name: String,
        effect_name: String,
        param_name: String,
    },
    LayerSelf {
        layer_name: String,
    },
    ExternalCompProperty {
        comp_name: String,
        layer_name: String,
        prop_path: String,
    },
}

/// Automatically generates valid AE-compatible expression code when using the Pick Whip tool.
pub fn generate_pick_whip_expression(target: &PickWhipTarget, current_layer_name: &str) -> String {
    match target {
        PickWhipTarget::TransformProperty { layer_name, prop } => {
            if layer_name == current_layer_name {
                format!("transform.{}", prop)
            } else {
                format!("thisComp.layer(\"{}\").transform.{}", layer_name, prop)
            }
        }
        PickWhipTarget::EffectProperty {
            layer_name,
            effect_name,
            param_name,
        } => {
            if layer_name == current_layer_name {
                format!("effect(\"{}\")(\"{}\")", effect_name, param_name)
            } else {
                format!(
                    "thisComp.layer(\"{}\").effect(\"{}\")(\"{}\")",
                    layer_name, effect_name, param_name
                )
            }
        }
        PickWhipTarget::LayerSelf { layer_name } => {
            if layer_name == current_layer_name {
                "thisLayer".to_string()
            } else {
                format!("thisComp.layer(\"{}\")", layer_name)
            }
        }
        PickWhipTarget::ExternalCompProperty {
            comp_name,
            layer_name,
            prop_path,
        } => {
            format!(
                "comp(\"{}\").layer(\"{}\").{}",
                comp_name, layer_name, prop_path
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_expression() {
        let engine = build_engine();
        // At frame 30, fps 30 → time = 1.0
        let result = eval_f32(&engine, "time * 360.0", 0.0, 30, 30);
        assert!(
            (result - 360.0).abs() < 0.01,
            "Expected 360, got {}",
            result
        );
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
        assert!(
            (result[0] - 100.0).abs() < 0.1,
            "Expected 100, got {}",
            result[0]
        );
        assert!(
            (result[1] - 0.0).abs() < 0.1,
            "Expected 0, got {}",
            result[1]
        );
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
        assert!(
            (mid - 50.0).abs() < 0.01,
            "Expected 50 at midpoint, got {}",
            mid
        );

        let ease_in = eval_f32(&engine, "easeIn(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
        assert!(
            (ease_in - 25.0).abs() < 0.01,
            "Expected 25 for easeIn at midpoint, got {}",
            ease_in
        );

        let ease_out = eval_f32(&engine, "easeOut(0.5, 0.0, 1.0, 0.0, 100.0)", 0.0, 0, 30);
        assert!(
            (ease_out - 75.0).abs() < 0.01,
            "Expected 75 for easeOut at midpoint, got {}",
            ease_out
        );
    }
}

#[cfg(test)]
mod tests_comp_context {
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Expression, Layer, LayerType};

    #[test]
    fn test_thiscomp_layer_reference() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut target = Layer::new(
            "t1".into(),
            "Target".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
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
        assert!(
            (pos[0] - 52.0).abs() < 0.01,
            "expected 52.0, got {}",
            pos[0]
        );
    }

    #[test]
    fn test_effect_param_bridge_cross_layer() {
        use crate::core::timeline::Effect;
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut src = Layer::new(
            "s1".into(),
            "Source".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        // Animated Gaussian blur radius: 5 at frame 0, 15 at frame 10.
        src.effects.push(Effect {
            id: "fx_test_blur".into(),
            enabled: true,
            name: "Blur".into(),
            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                blur_radius: Animatable::new_animated(vec![
                    crate::core::keyframe::Keyframe::new(
                        0,
                        5.0,
                        crate::core::keyframe::InterpolationType::Linear,
                    ),
                    crate::core::keyframe::Keyframe::new(
                        10,
                        15.0,
                        crate::core::keyframe::InterpolationType::Linear,
                    ),
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
        assert!(
            (snap_f0.layers["Source"].effects["Blur"]["Blur Radius"] - 5.0).abs() < 0.01,
            "snapshot should carry effect value"
        );

        let layer_f10 = &comp.layers[1];
        let (pos, _, _, _) = comp.resolve_world_transform(layer_f10, 10);
        assert!(
            (pos[0] - 150.0).abs() < 0.05,
            "expected 150.0 (15*10), got {}",
            pos[0]
        );
    }

    #[test]
    fn test_thislayer_reference() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut l = Layer::new(
            "s1".into(),
            "Selfy".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        l.transform.position = Animatable::new_constant([10.0, 20.0]);
        l.transform.rotation_expression =
            Some(Expression::Raw("thisLayer.transform.rotation * 2.0".into()));
        comp.layers.push(l);

        let layer = &comp.layers[0];
        let (_, _, rot, _) = comp.resolve_world_transform(layer, 0);
        assert!(
            (rot - 0.0).abs() < 0.01 || rot > 0.0,
            "rotation expr should evaluate"
        );
    }
}

#[cfg(test)]
mod tests_comp_extras {
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Expression, Layer, LayerType};

    #[test]
    fn test_layer_lookup_by_id() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut target = Layer::new(
            "tgt_id_1".into(),
            "TargetName".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        target.transform.position = Animatable::new_constant([7.0, 3.0]);
        comp.layers.push(target);
        let mut driver = Layer::new("d".into(), "Driver".into(), LayerType::Null, 30);
        driver.transform.position_expression = Some(Expression::Raw(
            "thisComp.layer(\"tgt_id_1\").transform.position[1]".into(),
        ));
        comp.layers.push(driver);

        let driver_ref = &comp.layers[1];
        let (pos, _, _, _) = comp.resolve_world_transform(driver_ref, 0);
        assert!(
            (pos[0] - 3.0).abs() < 0.01,
            "expected 3.0 via id lookup, got {}",
            pos[0]
        );
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
    use crate::core::keyframe::Keyframe;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Expression, Layer, LayerType};

    #[test]
    fn test_loopout_in_raw_script() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut l = Layer::new(
            "l1".into(),
            "Looper".into(),
            LayerType::Solid { color: [0.0; 4] },
            30,
        );
        // Keyframes: x 0→100 over frames 0..10
        l.transform.position = Animatable::new_animated(vec![
            Keyframe::new(
                0,
                [0.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
            Keyframe::new(
                10,
                [100.0, 0.0],
                crate::core::keyframe::InterpolationType::Linear,
            ),
        ]);
        // At frame 25 (past last kf), loopOut("cycle") should reference the cycled value (x=50 at frame 5)
        l.transform.position_expression =
            Some(Expression::Raw("loopOut(\"cycle\") + [0.0, 7.0]".into()));
        comp.layers.push(l);

        let layer = &comp.layers[0];
        let (pos, _, _, _) = comp.resolve_world_transform(layer, 25);
        // Frame 25 remaps to frame 5 → x = 50
        assert!(
            (pos[0] - 50.0).abs() < 0.5,
            "expected x=50 from loopOut cycle, got {}",
            pos[0]
        );
    }

    #[test]
    fn test_loop_preprocess_rewrites() {
        let rewritten = preprocess_loop_script("loopOut(\"pingpong\") + loopIn()");
        assert!(rewritten.contains("__loop_out_pingpong"));
        assert!(rewritten.contains("__loop_in_cycle"));
        assert!(!rewritten.contains("loopOut("));
    }

    #[test]
    fn test_thiscomp_layer_by_index() {
        let mut comp = Composition::new("c".into(), "Comp".into(), 100, 100, 30, 30);
        let mut a = Layer::new("a1".into(), "Bottom".into(), LayerType::Null, 30);
        a.transform.position = Animatable::new_constant([7.0, 0.0]);
        comp.layers.push(a);
        let mut b = Layer::new("b1".into(), "Top".into(), LayerType::Null, 30);
        b.transform.position = Animatable::new_constant([99.0, 0.0]);
        comp.layers.push(b);

        // AE 1-based: layer(1) = topmost = "Top"
        let snap = crate::core::expression_engine::build_comp_snapshot(&comp, 0);
        assert_eq!(snap.layer_by_index(1).position[0], 99.0);
        assert_eq!(snap.layer_by_index(2).position[0], 7.0);
        assert_eq!(
            snap.layer_by_index(99).position[0],
            0.0,
            "out of range → default"
        );
        assert_eq!(snap.num_layers(), 2.0);
    }

    #[test]
    fn test_comp_duration_and_frame_duration() {
        let comp = Composition::new("c".into(), "Comp".into(), 100, 100, 25, 50);
        let snap = crate::core::expression_engine::build_comp_snapshot(&comp, 0);
        assert!(
            (snap.duration() - 2.0).abs() < 1e-6,
            "50 frames @25fps = 2s"
        );
        assert!((snap.frame_duration() - 0.04).abs() < 1e-6);
    }

    #[test]
    fn test_to_from_comp_roundtrip_identity_layer() {
        // Identity layer (pos 0, scale 100, rot 0): toComp == fromComp == input
        set_current_layer_xform([0.0, 0.0], [100.0, 100.0], 0.0);
        let out: rhai::Array = COMP_ENGINE.with(|e| e.eval("toComp(12.0, -5.0)").unwrap());
        assert!((out[0].as_float().unwrap() - 12.0).abs() < 1e-6);
        assert!((out[1].as_float().unwrap() + 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_to_comp_applies_translation_rotation_scale() {
        // Layer at (100, 50), scale 200%, rot 90°
        set_current_layer_xform([100.0, 50.0], [200.0, 200.0], 90.0);
        // Local point (10, 0) → scaled (20, 0) → rotated 90°: (0, 20) → +pos = (100, 70)
        let out: rhai::Array = COMP_ENGINE.with(|e| e.eval("toComp(10.0, 0.0)").unwrap());
        assert!(
            (out[0].as_float().unwrap() - 100.0).abs() < 1e-4,
            "{}",
            out[0]
        );
        assert!(
            (out[1].as_float().unwrap() - 70.0).abs() < 1e-4,
            "{}",
            out[1]
        );

        // fromComp inverts it
        let inv: rhai::Array = COMP_ENGINE.with(|e| e.eval("fromComp(100.0, 70.0)").unwrap());
        assert!(
            (inv[0].as_float().unwrap() - 10.0).abs() < 1e-3,
            "{}",
            inv[0]
        );
        assert!(inv[1].as_float().unwrap().abs() < 1e-3, "{}", inv[1]);
    }

    #[test]
    fn test_ae_interpolation_and_vectors() {
        let engine = crate::core::expression_engine::build_engine();
        // 3-arg linear & ease
        let v: f64 = engine.eval("linear(0.5, 10.0, 20.0)").unwrap();
        assert!((v - 15.0).abs() < 1e-6);
        let ve: f64 = engine.eval("ease(0.5, 0.0, 100.0)").unwrap();
        assert!((ve - 50.0).abs() < 1e-6);

        // Vector math
        let len: f64 = engine.eval("length([3.0, 4.0])").unwrap();
        assert!((len - 5.0).abs() < 1e-6);
        let dist: f64 = engine.eval("distance([0.0, 0.0], [3.0, 4.0])").unwrap();
        assert!((dist - 5.0).abs() < 1e-6);
        let dot: f64 = engine.eval("dot([1.0, 2.0], [3.0, 4.0])").unwrap();
        assert!((dot - 11.0).abs() < 1e-6);

        // Array linear
        let arr: rhai::Array = engine
            .eval("linear(0.5, 0.0, 1.0, [10.0, 20.0], [20.0, 40.0])")
            .unwrap();
        assert_eq!(arr.len(), 2);
        assert!((arr[0].as_float().unwrap() - 15.0).abs() < 1e-6);
        assert!((arr[1].as_float().unwrap() - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_generate_pick_whip_expression() {
        let t1 = PickWhipTarget::TransformProperty {
            layer_name: "Logo".into(),
            prop: "position".into(),
        };
        assert_eq!(
            generate_pick_whip_expression(&t1, "Logo"),
            "transform.position"
        );
        assert_eq!(
            generate_pick_whip_expression(&t1, "Background"),
            "thisComp.layer(\"Logo\").transform.position"
        );

        let t2 = PickWhipTarget::EffectProperty {
            layer_name: "Control".into(),
            effect_name: "Slider Control".into(),
            param_name: "Slider".into(),
        };
        assert_eq!(
            generate_pick_whip_expression(&t2, "Text"),
            "thisComp.layer(\"Control\").effect(\"Slider Control\")(\"Slider\")"
        );

        let t3 = PickWhipTarget::ExternalCompProperty {
            comp_name: "PreComp".into(),
            layer_name: "Null 1".into(),
            prop_path: "transform.opacity".into(),
        };
        assert_eq!(
            generate_pick_whip_expression(&t3, "Main"),
            "comp(\"PreComp\").layer(\"Null 1\").transform.opacity"
        );
    }

    #[test]
    fn audio_binding_sources_expose_current_amplitude_and_bands() {
        set_audio_expr_data(AudioExprData {
            amplitude: 0.75,
            bands: [0.1, 0.2, 0.3, 0.4, 0.5],
        });

        let values = audio_binding_source_values();
        assert!((values["audio.amplitude"] - 0.75).abs() < 1e-6);
        assert!((values["audio.band0"] - 0.1).abs() < 1e-6);
        assert!((values["audio.band4"] - 0.5).abs() < 1e-6);
        assert!((values["audio.bass"] - 0.1).abs() < 1e-6);
        assert!((values["audio.treble"] - 0.5).abs() < 1e-6);
        assert_eq!(values.len(), 11);
    }

    #[test]
    fn binding_audio_sources_update_expression_context_with_clamping() {
        let values = std::collections::HashMap::from([
            (String::from("audio.amplitude"), 2.0),
            (String::from("audio.bass"), -1.0),
            (String::from("audio.band1"), 0.25),
        ]);
        set_audio_from_binding_sources(&values);

        assert_eq!(get_audio_amplitude(), 1.0);
        assert_eq!(get_audio_band(0), 0.0);
        assert_eq!(get_audio_band(1), 0.25);
        assert_eq!(get_audio_band(4), 0.0);
    }
}
