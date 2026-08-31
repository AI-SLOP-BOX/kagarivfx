//! Dynamic Custom WGSL Shader Hot-Reloading & Validation Engine.
//!
//! Allows users to write or load custom WGSL fragment shaders at runtime,
//! validate syntax via naga with precise line/column errors, and compile
//! pipelines for real-time GPU rendering.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Compilation result for a custom WGSL shader.
#[derive(Debug, Clone)]
pub struct ShaderCompileStatus {
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub line_number: Option<usize>,
}

/// Global cache for compiled custom WGSL shaders.
pub struct CustomShaderRegistry {
    cache: Mutex<HashMap<String, ShaderCompileStatus>>,
}

static REGISTRY: OnceLock<CustomShaderRegistry> = OnceLock::new();

impl CustomShaderRegistry {
    pub fn global() -> &'static Self {
        REGISTRY.get_or_init(|| Self {
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// Validates and compiles a WGSL snippet, returning syntax errors if any.
    pub fn validate_wgsl(&self, source: &str) -> ShaderCompileStatus {
        if source.trim().is_empty() {
            return ShaderCompileStatus {
                is_valid: true,
                error_message: None,
                line_number: None,
            };
        }

        // Cache lookup
        if let Ok(guard) = self.cache.lock() {
            if let Some(cached) = guard.get(source) {
                return cached.clone();
            }
        }

        // Wrap code in a standard fragment shader test harness if user just wrote a fragment body
        let full_source = if source.contains("@fragment") {
            source.to_string()
        } else {
            format!(
                r#"
struct CustomUniforms {{
    time: f32,
    width: f32,
    height: f32,
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    _pad: f32,
}};
@group(0) @binding(0) var<uniform> u: CustomUniforms;
@group(0) @binding(1) var s_tex: texture_2d<f32>;
@group(0) @binding(2) var s_samp: sampler;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {{
    let src = textureSample(s_tex, s_samp, uv);
    {}
}}
"#,
                source
            )
        };

        // Parse with naga WGSL frontend
        match naga::front::wgsl::parse_str(&full_source) {
            Ok(_) => {
                let status = ShaderCompileStatus {
                    is_valid: true,
                    error_message: None,
                    line_number: None,
                };
                if let Ok(mut guard) = self.cache.lock() {
                    guard.insert(source.to_string(), status.clone());
                }
                status
            }
            Err(e) => {
                let msg = e.emit_to_string(&full_source);
                let line = msg
                    .lines()
                    .find(|l| l.contains("-->") || l.contains(':'))
                    .and_then(|l| {
                        l.split(':')
                            .nth(1)
                            .and_then(|num_str| num_str.trim().parse::<usize>().ok())
                    });

                let status = ShaderCompileStatus {
                    is_valid: false,
                    error_message: Some(msg),
                    line_number: line,
                };
                if let Ok(mut guard) = self.cache.lock() {
                    guard.insert(source.to_string(), status.clone());
                }
                status
            }
        }
    }

    /// Clear compiled shader cache (e.g. on hot reload or project reset).
    pub fn clear_cache(&self) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.clear();
        }
    }
}

/// Helper to get default template for custom WGSL shaders.
pub fn default_wgsl_template() -> String {
    r#"// Custom WGSL VFX Shader Template
// Available: src (input color vec4), uv (vec2), u.time, u.p0..p3
let t = u.time;
let dist = distance(uv, vec2<f32>(0.5, 0.5));
let wave = sin(dist * 20.0 - t * 4.0) * 0.5 + 0.5;
let tint = vec4<f32>(wave * u.p0, (1.0 - wave) * u.p1, wave * u.p2, 1.0);
return mix(src, tint, u.p3);"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_wgsl_validation() {
        let registry = CustomShaderRegistry::global();
        let valid_code = "return vec4<f32>(src.r, src.g * u.p0, src.b, src.a);";
        let status = registry.validate_wgsl(valid_code);
        assert!(status.is_valid, "Valid WGSL failed: {:?}", status.error_message);
    }

    #[test]
    fn test_invalid_wgsl_validation() {
        let registry = CustomShaderRegistry::global();
        let invalid_code = "return vec4<f32>(src.r, undefined_variable + 10.0);";
        let status = registry.validate_wgsl(invalid_code);
        assert!(!status.is_valid);
        assert!(status.error_message.is_some());
    }
}
