use crate::core::timeline::EffectType;
use crate::ui::theme::colors;
use eframe::egui;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    _current_frame: u32,
    project_changed: &mut bool,
    _next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::CustomShader {
            wgsl_source,
            uniform_values,
        } => {
            ui.label(
                egui::RichText::new("⚡ Custom WGSL Shader Plugin")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );

            // Templates dropdown
            ui.horizontal(|ui| {
                ui.label("Templates:");
                if ui.small_button("📺 CRT Scanlines").clicked() {
                    *wgsl_source = r#"// CRT Scanlines Shader
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> u_params: vec4<f32>; // x: density, y: opacity

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, uv);
    let scanline = sin(uv.y * (u_params.x * 400.0 + 100.0)) * 0.5 + 0.5;
    color = vec4<f32>(color.rgb * mix(1.0, scanline, u_params.y), color.a);
    return color;
}"#
                    .to_string();
                    if uniform_values.len() < 2 {
                        *uniform_values = vec![1.0, 0.4];
                    }
                    *project_changed = true;
                }
                if ui.small_button("🌀 Chromatic Twist").clicked() {
                    *wgsl_source = r#"// Chromatic Twist Shader
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> u_params: vec4<f32>; // x: strength, y: radius

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let delta = uv - center;
    let dist = length(delta);
    let angle = atan2(delta.y, delta.x) + (1.0 - smoothstep(0.0, u_params.y, dist)) * u_params.x;
    let twisted_uv = center + vec2<f32>(cos(angle), sin(angle)) * dist;
    return textureSample(t_diffuse, s_diffuse, clamp(twisted_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}"#
                    .to_string();
                    if uniform_values.len() < 2 {
                        *uniform_values = vec![1.5, 0.5];
                    }
                    *project_changed = true;
                }
                if ui.small_button("⚡ Neon Edge").clicked() {
                    *wgsl_source = r#"// Neon Edge Glow
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
@group(0) @binding(2) var<uniform> u_params: vec4<f32>; // x: edge_thresh, y: intensity

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let col = textureSample(t_diffuse, s_diffuse, uv);
    let right = textureSample(t_diffuse, s_diffuse, uv + vec2<f32>(0.002, 0.0));
    let down = textureSample(t_diffuse, s_diffuse, uv + vec2<f32>(0.0, 0.002));
    let diff = length(col.rgb - right.rgb) + length(col.rgb - down.rgb);
    let neon = smoothstep(u_params.x * 0.1, 0.5, diff) * u_params.y * vec3<f32>(0.1, 0.9, 1.0);
    return vec4<f32>(col.rgb + neon, col.a);
}"#
                    .to_string();
                    if uniform_values.len() < 2 {
                        *uniform_values = vec![0.5, 2.0];
                    }
                    *project_changed = true;
                }
            });

            ui.add_space(4.0);
            if ui
                .add(
                    egui::TextEdit::multiline(wgsl_source)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(8),
                )
                .changed()
            {
                *project_changed = true;
            }

            // Real-time naga WGSL validation & hot-reload status
            let status = crate::core::custom_shader_runtime::CustomShaderRegistry::global()
                .validate_wgsl(wgsl_source);
            ui.horizontal(|ui| {
                if status.is_valid {
                    ui.label(
                        egui::RichText::new("✅ Naga WGSL Valid")
                            .small()
                            .color(colors::ACCENT_GREEN),
                    );
                } else {
                    let err_snippet = status
                        .error_message
                        .as_deref()
                        .unwrap_or("WGSL Syntax Error");
                    let short_err = err_snippet.lines().next().unwrap_or("Syntax Error");
                    ui.label(
                        egui::RichText::new(format!("❌ {}", short_err))
                            .small()
                            .color(colors::ACCENT_RED),
                    )
                    .on_hover_text(err_snippet);
                }

                if ui
                    .small_button("📁 Load .wgsl File")
                    .on_hover_text("Load and hot-reload shader from disk")
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("WGSL Shader", &["wgsl", "frag"])
                        .pick_file()
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            *wgsl_source = content;
                            *project_changed = true;
                        }
                    }
                }

                if ui.small_button("+ Add Float").clicked() {
                    uniform_values.push(1.0);
                    *project_changed = true;
                }
                if !uniform_values.is_empty() && ui.small_button("- Remove").clicked() {
                    uniform_values.pop();
                    *project_changed = true;
                }
            });

            // Interactive dynamic uniform sliders
            for (i, v) in uniform_values.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Param [{}] (u_params.{}):",
                        i,
                        match i {
                            0 => "x",
                            1 => "y",
                            2 => "z",
                            3 => "w",
                            _ => "?",
                        }
                    ));
                    if ui.add(egui::DragValue::new(v).speed(0.01)).changed() {
                        *project_changed = true;
                    }
                });
            }
        }
        _ => {}
    }
}
