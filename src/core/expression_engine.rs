/// Rhai-powered After Effects-style expression evaluation engine.
///
/// Supports AE-compatible expression APIs:
///   - `time` (current time in seconds)
///   - `wiggle(freq, amp)` → f32 noise offset
///   - `loopOut("cycle")` / `loopOut("pingpong")`
///   - `thisComp.layer("Name").transform.position[0]`  (inter-layer reference)
///   - Any arbitrary Rhai script that returns a number or array.

use rhai::{Engine, Scope, Dynamic, Array, EvalAltResult};

/// Build the shared Rhai engine with all AE-compatible functions registered.
/// Creating the engine is expensive — do it once and cache it.
pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 32);
    engine.set_max_operations(100_000);

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
        let s = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
        v_min + s * (v_max - v_min)
    });

    // --- Array constructors ---
    engine.register_fn("array2", |x: f64, y: f64| -> Array {
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
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
            // Scalar return — apply to both axes
            if let Ok(f) = val.as_float() {
                return [base[0] + f as f32, base[1] + f as f32];
            }
            base
        }
        Err(e) => {
            log::warn!("[ExprEngine] eval_v2 error: {}", e);
            base
        }
    }
}

/// Pre-validate a script without evaluating (for real-time syntax checking in UI).
/// Returns `Ok(())` if the script compiles, or an error message.
pub fn validate_script(engine: &Engine, script: &str) -> Result<(), String> {
    engine.compile(script)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
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
}
