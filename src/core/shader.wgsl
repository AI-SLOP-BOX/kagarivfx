struct Globals {
    viewport_size: vec2<f32>,
    exposure_ev: f32,
    lut_mode: u32,
    shadow_enabled: u32,
    shadow_strength: f32,
};

struct Layer {
    transform_matrix: mat4x4<f32>,
    color: vec4<f32>,
    opacity: f32,
    layer_type: u32,
    shape_type: u32,
    
    effect_tint_enabled: u32,
    effect_tint_color: vec4<f32>,
    effect_tint_intensity: f32,
    effect_blur_enabled: u32,
    effect_blur_radius: f32,

    effect_shadow_enabled: u32,
    effect_shadow_color: vec4<f32>,
    effect_shadow_opacity: f32,
    effect_shadow_direction: f32,
    effect_shadow_distance: f32,
    effect_shadow_softness: f32,

    effect_ca_enabled: u32,
    effect_ca_shift_r: f32,
    effect_ca_shift_b: f32,
    effect_ca_edge_falloff: f32,

    effect_vignette_enabled: u32,
    effect_vignette_intensity: f32,
    effect_vignette_roundness: f32,
    effect_vignette_feather: f32,
    effect_vignette_color: vec4<f32>,

    // AE Blend Mode: 0=Normal, 1=Multiply, 2=Screen, 3=Overlay, 4=Add, 5=Darken, 6=Lighten
    blend_mode: u32,

    // Levels Adjustment
    levels_enabled: u32,
    levels_in_black: f32,
    levels_in_white: f32,
    levels_gamma: f32,
    levels_out_black: f32,
    levels_out_white: f32,

    // Hue / Saturation
    huesat_enabled: u32,
    huesat_hue: f32,
    huesat_sat: f32,
    huesat_light: f32,

    // Glow / Bloom
    glow_enabled: u32,
    glow_threshold: f32,
    glow_radius: f32,
    glow_intensity: f32,
    glow_color: vec4<f32>,

    // Film Grain
    grain_enabled: u32,
    grain_intensity: f32,
    grain_size: f32,

    // Track Matte System
    track_matte_mode: u32,

    // Per-layer GPU mask (coverage rasterized on CPU, uploaded once per frame)
    mask_enabled: u32,
    mask_mode: u32,
    mask_inverted: u32,
    mask_feather: f32,

    // Lens Flare (screen-space, from light source)
    flare_enabled: u32,
    flare_pos_x: f32,
    flare_pos_y: f32,
    flare_intensity: f32,
    flare_threshold: f32,
    flare_color: vec4<f32>,

    // Shape params: x = polygon sides, y = rectangle corner radius (normalized 0..1 of half-size)
    shape_params: vec4<f32>,

    // Mesh Warp / Corner Pin
    meshwarp_enabled: u32,
    corner_top_left: vec2<f32>,
    corner_top_right: vec2<f32>,
    corner_bottom_left: vec2<f32>,
    corner_bottom_right: vec2<f32>,

    // Motion Blur (per-pixel velocity-based)
    motionblur_enabled: u32,
    motionblur_shutter: f32,
    motionblur_velocity_x: f32,
    motionblur_velocity_y: f32,
    motionblur_samples: u32,

    // TrimPaths (angular trim on shape SDF)
    trim_start: f32,
    trim_end: f32,
    trim_offset: f32,
    _pad_trim: f32,

    // Shape fill gradient (0=solid, 1=linear, 2=radial)
    fill_type: u32,
    grad_start_x: f32,
    grad_start_y: f32,
    grad_end_x: f32,
    grad_end_y: f32,
    grad_color1_r: f32,
    grad_color1_g: f32,
    grad_color1_b: f32,
    grad_color1_a: f32,
    grad_color2_r: f32,
    grad_color2_g: f32,
    grad_color2_b: f32,
    grad_color2_a: f32,
    grad_center_x: f32,
    grad_center_y: f32,
    grad_radius: f32,
    _grad_pad: f32,

    // Layer Styles (applied after effects, before compositing)
    ls_stroke_width: f32,
    ls_stroke_r: f32,
    ls_stroke_g: f32,
    ls_stroke_b: f32,
    ls_color_overlay_r: f32,
    ls_color_overlay_g: f32,
    ls_color_overlay_b: f32,
    ls_color_overlay_a: f32,
    ls_gradient_start_x: f32,
    ls_gradient_start_y: f32,
    ls_gradient_end_x: f32,
    ls_gradient_end_y: f32,
    ls_gradient_color1_r: f32,
    ls_gradient_color1_g: f32,
    ls_gradient_color1_b: f32,
    ls_gradient_color1_a: f32,
    ls_gradient_color2_r: f32,
    ls_gradient_color2_g: f32,
    ls_gradient_color2_b: f32,
    ls_gradient_color2_a: f32,
    ls_inner_shadow_offset_x: f32,
    ls_inner_shadow_offset_y: f32,
    ls_inner_shadow_size: f32,
    ls_inner_shadow_opacity: f32,
    ls_inner_shadow_r: f32,
    ls_inner_shadow_g: f32,
    ls_inner_shadow_b: f32,
    ls_bevel_size: f32,
    ls_bevel_angle: f32,
    ls_bevel_strength: f32,
    ls_bevel_light_r: f32,
    ls_bevel_light_g: f32,
    ls_bevel_light_b: f32,
    ls_style_flags: u32,
    _ls_pad1: f32,
    _ls_pad2: f32,
    _ls_pad3: f32,

    // 3D Extrusion (pseudo-3D depth shading for shape layers)
    extrusion_depth: f32,
    bevel_depth: f32,

    // ── GPU Real-time VFX Shader Extensions ──
    // Chromatic Aberration
    chromatic_enabled: u32,
    chromatic_amount: f32,
    chromatic_angle: f32,
    _pad_chromatic: f32,

    // Vignette
    vignette_enabled: u32,
    vignette_amount: f32,
    vignette_midpoint: f32,
    vignette_feather: f32,

    // Invert & Posterize
    invert_enabled: u32,
    posterize_enabled: u32,
    posterize_levels: f32,
    threshold_level: f32,

    // Tint
    tint_enabled: u32,
    tint_amount: f32,
    _pad_tint1: f32,
    _pad_tint2: f32,
    tint_black: vec4<f32>,
    tint_white: vec4<f32>,

    // CRT Scanlines
    crt_enabled: u32,
    crt_scanline_count: f32,
    crt_scanline_intensity: f32,
    crt_curvature: f32,

    // ── GPU Simulation Effects ──
    sim_enabled: u32,
    sim_type: u32,
    sim_p1: f32,
    sim_p2: f32,
    sim_p3: f32,
    sim_p4: f32,
    sim_p5: f32,
    sim_p6: f32,

    _padding_align: vec4<f32>,
    _ls_pad4: f32,
    _ls_pad5: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> layer: Layer;
@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;
@group(3) @binding(0) var t_mask: texture_2d<f32>;
@group(3) @binding(1) var s_mask: sampler;

// Shadow density map (CPU-built, uploaded per frame when shadows active)
@group(4) @binding(0) var t_shadow: texture_2d<f32>;
@group(4) @binding(1) var s_shadow: sampler;

// Track matte source texture (the layer ABOVE this one in AE track matte order)
@group(5) @binding(0) var t_matte: texture_2d<f32>;
@group(5) @binding(1) var s_matte: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) local_pos: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = layer.transform_matrix * vec4<f32>(model.position, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    out.local_pos = model.position; // Ranges in [-0.5, 0.5]
    return out;
}

fn trim_shape_alpha(angle: f32, trim_start: f32, trim_end: f32, trim_offset: f32) -> f32 {
    if (trim_start == 0.0 && trim_end == 1.0) {
        return 1.0;
    }
    let normalized = fract(angle / 6.2831853 + 1.0);
    let start = fract(trim_start + trim_offset);
    let end = fract(trim_end + trim_offset);
    // AE treats a zero-length trim as an empty path, not a wrapped full path.
    if (abs(start - end) < 0.000001) {
        return 0.0;
    }
    if (start < end) {
        return select(0.0, 1.0, normalized >= start && normalized <= end);
    } else {
        return select(0.0, 1.0, normalized >= start || normalized <= end);
    }
}

// Helper: SDF-based alpha coverage for Shape layers (layer_type == 2u).
// Shared between the layer fill and the drop-shadow mask so all four
// shape types (rect / ellipse / star / polygon) cast correct shadows.
fn shape_sdf_alpha(local_pos_in: vec2<f32>, blur_extend: f32) -> f32 {
    var alpha = 0.0;
    if (layer.shape_type == 0u) {
        let d_x = abs(local_pos_in.x) - 0.5;
        let d_y = abs(local_pos_in.y) - 0.5;
        let d = max(d_x, d_y);
        alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.02 + blur_extend, d);
    } else if (layer.shape_type == 1u) {
        let dist = length(local_pos_in);
        alpha = 1.0 - smoothstep(0.48 - blur_extend, 0.5 + blur_extend, dist);
    } else if (layer.shape_type == 2u) {
        // Procedural N-Point Star SDF
        let p = local_pos_in * 2.0;
        let r = length(p);
        let angle = atan2(p.y, p.x);
        let n = max(layer.shape_params.x, 3.0);
        let angle_mod = abs(fract((angle / 6.2831853) * n + 0.5) - 0.5) * (6.2831853 / n);
        let d_star = r * cos(angle_mod - 0.314159) - 0.45;
        alpha = 1.0 - smoothstep(-0.04 - blur_extend, 0.04 + blur_extend, d_star);
    } else if (layer.shape_type == 3u) {
        // Procedural Regular N-Gon Polygon SDF
        let p = local_pos_in * 2.0;
        let r = length(p);
        let angle = atan2(p.y, p.x);
        let n = max(layer.shape_params.x, 3.0);
        let angle_mod = abs(fract((angle / 6.2831853) * n + 0.5) - 0.5) * (6.2831853 / n);
        let d_poly = r * cos(angle_mod) - 0.45;
        alpha = 1.0 - smoothstep(-0.04 - blur_extend, 0.04 + blur_extend, d_poly);
    } else {
        alpha = 1.0;
    }
    let angle = atan2(local_pos_in.y, local_pos_in.x);
    return alpha * trim_shape_alpha(angle, layer.trim_start, layer.trim_end, layer.trim_offset);
}

// ─── GPU Procedural Noise & fBM Helpers ───

fn gpu_hash21(p: vec2<f32>) -> f32 {
    let q = fract(sin(vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)))) * 43758.5453);
    return fract(q.x + q.y);
}

fn gpu_noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = gpu_hash21(i + vec2<f32>(0.0, 0.0));
    let b = gpu_hash21(i + vec2<f32>(1.0, 0.0));
    let c = gpu_hash21(i + vec2<f32>(0.0, 1.0));
    let d = gpu_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn gpu_fbm2d(p: vec2<f32>, octaves: i32) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var pt = p;
    for (var i = 0; i < 6; i = i + 1) {
        if (i >= octaves) { break; }
        v += gpu_noise2d(pt * freq) * amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    return v;
}

// Helper: sample layer color at a given local_pos and tex_coords
fn sample_layer_color(local_pos_in: vec2<f32>, tc_in: vec2<f32>, blur_extend: f32) -> vec4<f32> {
    // Mesh Warp / Corner Pin: bilinear corner-offset displacement field.
    // Corners are pixel offsets from each quad corner; normalize by viewport to get UV deltas.
    var tc = tc_in;
    var local_pos = local_pos_in;

    // ── GPU Spatial Simulation Distortion (Twirl, Bulge, Spherize, Wave Warp, Turbulent Displace) ──
    if (layer.sim_enabled == 1u) {
        let center = vec2<f32>(0.5, 0.5);
        let uv_rel = tc - center;
        let dist = length(uv_rel);

        if (layer.sim_type == 2u) { // Turbulent Displace
            let amount = layer.sim_p1 * 0.05;
            let size = max(layer.sim_p2, 1.0);
            let n_x = gpu_noise2d((tc + vec2<f32>(layer.sim_p3, 0.0)) * size) - 0.5;
            let n_y = gpu_noise2d((tc + vec2<f32>(0.0, layer.sim_p3 + 12.3)) * size) - 0.5;
            tc += vec2<f32>(n_x, n_y) * amount;
        } else if (layer.sim_type == 3u) { // Wave Warp
            let freq = layer.sim_p1;
            let amp = layer.sim_p2 * 0.01;
            let phase = layer.sim_p3;
            tc.x += sin(tc.y * freq + phase) * amp;
        } else if (layer.sim_type == 4u) { // Twirl
            let radius = max(layer.sim_p1, 0.001);
            let angle = layer.sim_p2 * 0.0174533; // deg to rad
            if (dist < radius) {
                let factor = (1.0 - dist / radius);
                let a = angle * factor * factor;
                let sin_a = sin(a);
                let cos_a = cos(a);
                let rotated = vec2<f32>(
                    uv_rel.x * cos_a - uv_rel.y * sin_a,
                    uv_rel.x * sin_a + uv_rel.y * cos_a
                );
                tc = center + rotated;
            }
        } else if (layer.sim_type == 5u) { // Bulge
            let radius = max(layer.sim_p1, 0.001);
            let amount = layer.sim_p2;
            if (dist < radius) {
                let factor = 1.0 - dist / radius;
                let displace = 1.0 - amount * factor * factor;
                tc = center + uv_rel * displace;
            }
        } else if (layer.sim_type == 6u) { // Spherize
            let radius = max(layer.sim_p1, 0.001);
            if (dist < radius) {
                let d_norm = dist / radius;
                let z = sqrt(max(1.0 - d_norm * d_norm, 0.0));
                let r = (1.0 - z) * 0.5 + d_norm * 0.5;
                tc = center + normalize(uv_rel) * (r * radius);
            }
        } else if (layer.sim_type == 7u) { // Heat Distortion
            let strength = layer.sim_p1 * 0.02;
            let speed = layer.sim_p2;
            let n = sin(tc.y * 40.0 + speed * 6.0) * cos(tc.x * 30.0 + speed * 4.0);
            tc.x += n * strength;
        }
    }

    if (layer.meshwarp_enabled == 1u) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let d_tl = layer.corner_top_left / vp;
        let d_tr = layer.corner_top_right / vp;
        let d_bl = layer.corner_bottom_left / vp;
        let d_br = layer.corner_bottom_right / vp;
        let u = tc.x;
        let v = tc.y;
        let top = mix(d_tl, d_tr, u);
        let bot = mix(d_bl, d_br, u);
        let disp = mix(top, bot, v);
        // One-step inverse approximation: sample opposite the displacement
        tc = vec2<f32>(tc.x - disp.x, tc.y + disp.y);
        local_pos = vec2<f32>(local_pos.x - disp.x * 2.0, local_pos.y + disp.y * 2.0);
    }
    var c = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    if (layer.layer_type == 0u) {
        if (layer.effect_blur_enabled == 1u) {
            let d_x = abs(local_pos.x) - 0.5;
            let d_y = abs(local_pos.y) - 0.5;
            let d = max(d_x, d_y);
            let alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.0 + blur_extend, d);
            c = vec4<f32>(layer.color.rgb, layer.color.a * alpha);
        } else if (layer.shape_params.y > 0.001) {
            // Rounded-corner solid via SDF
            let half_sz = vec2<f32>(0.5, 0.5);
            let q = abs(local_pos) - (half_sz - layer.shape_params.y);
            let d = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - layer.shape_params.y;
            let alpha = 1.0 - smoothstep(-0.02 - blur_extend, 0.02 + blur_extend, d);
            c = vec4<f32>(layer.color.rgb, layer.color.a * alpha);
        } else {
            c = layer.color;
        }
    } else if (layer.layer_type == 1u) {
        if (layer.effect_blur_enabled == 1u) {
            // --- Gaussian Blur (13-tap single-pass separable approximation) ---
            // Uses pre-computed Gaussian weights for sigma = radius/2.
            // 13 samples: center + 6 symmetric pairs, weighted by Gaussian curve.
            // Quality: equivalent to ~26-tap bilinear-optimized separable blur.
            let texel_size = 1.0 / globals.viewport_size;
            let offset = layer.effect_blur_radius * texel_size;

            // Gaussian weights for sigma=2.0 (13-tap): [0.111, 0.105, 0.088, 0.066, 0.044, 0.025, 0.013]
            // Normalized to sum to 1.0 with bilateral symmetry
            var color_sum = textureSample(t_diffuse, s_diffuse, tc) * 0.1111;
            // Tap pair 1 (offset 1.0)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x, 0.0)) * 0.1053;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x, 0.0)) * 0.1053;
            // Tap pair 2 (offset 2.0)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x * 2.0, 0.0)) * 0.0877;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x * 2.0, 0.0)) * 0.0877;
            // Tap pair 3 (offset 3.0)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x * 3.0, 0.0)) * 0.0660;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x * 3.0, 0.0)) * 0.0660;
            // Tap pair 4 (offset 4.0)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x * 4.0, 0.0)) * 0.0440;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x * 4.0, 0.0)) * 0.0440;
            // Tap pair 5 (offset 5.0)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x * 5.0, 0.0)) * 0.0252;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x * 5.0, 0.0)) * 0.0252;
            // Tap pair 6 (offset 6.0)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x * 6.0, 0.0)) * 0.0128;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x * 6.0, 0.0)) * 0.0128;
            // Vertical pass (same kernel, y-axis)
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y)) * 0.1053;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y)) * 0.1053;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y * 2.0)) * 0.0877;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y * 2.0)) * 0.0877;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y * 3.0)) * 0.0660;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y * 3.0)) * 0.0660;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y * 4.0)) * 0.0440;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y * 4.0)) * 0.0440;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y * 5.0)) * 0.0252;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y * 5.0)) * 0.0252;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y * 6.0)) * 0.0128;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y * 6.0)) * 0.0128;
            c = color_sum;
        } else {
            c = textureSample(t_diffuse, s_diffuse, tc);
        }
    } else if (layer.layer_type == 2u) {
        let alpha = shape_sdf_alpha(local_pos, blur_extend);
        var shape_color = layer.color;
        if (layer.fill_type == 1u) {
            let d = vec2<f32>(layer.grad_end_x - layer.grad_start_x, layer.grad_end_y - layer.grad_start_y);
            let len_sq = dot(d, d);
            if (len_sq > 0.001) {
                let t = clamp(dot(local_pos - vec2<f32>(layer.grad_start_x, layer.grad_start_y), d) / len_sq, 0.0, 1.0);
                shape_color = mix(vec4<f32>(layer.grad_color1_r, layer.grad_color1_g, layer.grad_color1_b, layer.grad_color1_a), vec4<f32>(layer.grad_color2_r, layer.grad_color2_g, layer.grad_color2_b, layer.grad_color2_a), t);
            }
        } else if (layer.fill_type == 2u) {
            let center = vec2<f32>(layer.grad_center_x, layer.grad_center_y);
            let dist = length(local_pos - center);
            let t = clamp(dist / max(layer.grad_radius, 0.001), 0.0, 1.0);
            shape_color = mix(vec4<f32>(layer.grad_color1_r, layer.grad_color1_g, layer.grad_color1_b, layer.grad_color1_a), vec4<f32>(layer.grad_color2_r, layer.grad_color2_g, layer.grad_color2_b, layer.grad_color2_a), t);
        }
        c = vec4<f32>(shape_color.rgb, shape_color.a * alpha);

        // 3D Extrusion: pseudo-3D depth shading based on distance from shape edge
        if (layer.extrusion_depth > 0.01 && alpha > 0.01) {
            // Compute distance from edge (0 at edge, 1 at center) using SDF
            var edge_dist = 0.0;
            if (layer.shape_type == 0u) {
                let d_x = abs(local_pos.x) - 0.5;
                let d_y = abs(local_pos.y) - 0.5;
                edge_dist = -max(d_x, d_y); // positive inside, negative outside
            } else if (layer.shape_type == 1u) {
                edge_dist = 0.5 - length(local_pos);
            } else {
                edge_dist = 0.1; // fallback for star/polygon
            }
            edge_dist = clamp(edge_dist * 2.0, 0.0, 1.0); // normalize to 0..1

            // Extrusion depth: front cap brightest, back cap darkest
            let depth_factor = 1.0 - layer.extrusion_depth * 0.003;
            let extrusion_shade = mix(depth_factor, 1.0, edge_dist);

            // Bevel: brighten near edges for a rounded edge effect
            let bevel_factor = 1.0 + layer.bevel_depth * 0.02 * (1.0 - smoothstep(0.0, 0.3, edge_dist));

            let final_shade = clamp(extrusion_shade * bevel_factor, 0.2, 1.0);
            c = vec4<f32>(c.rgb * final_shade, c.a);
        }
    } else if (layer.layer_type == 3u) {
        c = layer.color;
    } else if (layer.layer_type == 8u) {
        // Particle layers are simulated and rasterized on the CPU;
        // return fully transparent so fs_main's alpha check discards this quad.
        c = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // ── GPU Procedural Fractal Noise Generator ──
    if (layer.sim_enabled == 1u && layer.sim_type == 1u) {
        let contrast = layer.sim_p1;
        let brightness = layer.sim_p2;
        let evolution = layer.sim_p3;
        let complexity = clamp(i32(layer.sim_p4), 1, 6);
        let n_val = gpu_fbm2d(tc * 6.0 + vec2<f32>(evolution * 0.5, evolution * 0.3), complexity);
        let f_col = clamp((n_val - 0.5) * contrast + 0.5 + brightness, 0.0, 1.0);
        c = vec4<f32>(f_col, f_col, f_col, 1.0);
    }

    // ── Levels: in_black/gamma/in_white adjustment ──
    if (layer.levels_enabled == 1u) {
        let range = max(layer.levels_in_white - layer.levels_in_black, 0.001);
        var r = (c.r - layer.levels_in_black) / range;
        var g = (c.g - layer.levels_in_black) / range;
        var b = (c.b - layer.levels_in_black) / range;
        // Gamma
        let inv_gamma = 1.0 / max(layer.levels_gamma, 0.01);
        r = pow(max(r, 0.0), inv_gamma);
        g = pow(max(g, 0.0), inv_gamma);
        b = pow(max(b, 0.0), inv_gamma);
        // Out black/white mapping
        let out_range = layer.levels_out_white - layer.levels_out_black;
        c.r = clamp(layer.levels_out_black + r * out_range, 0.0, 1.0);
        c.g = clamp(layer.levels_out_black + g * out_range, 0.0, 1.0);
        c.b = clamp(layer.levels_out_black + b * out_range, 0.0, 1.0);
    }

    /// ── Vignette: darkened edges ──
    if (layer.effect_vignette_enabled == 1u) {
        let center_uv = vec2<f32>(0.5, 0.5);
        let d = distance(local_pos, center_uv) * 2.0;
        let vig = 1.0 - smoothstep(
            1.0 - layer.effect_vignette_feather,
            1.0,
            d * (1.0 - layer.effect_vignette_roundness * 0.3)
        );
        let vig_amount = layer.effect_vignette_intensity;
        c = vec4<f32>(
            mix(c.rgb, layer.effect_vignette_color.rgb * c.rgb, (1.0 - vig) * vig_amount),
            c.a
        );
    }

    // ── Chromatic Aberration: radial RGB separation ──
    if (layer.effect_ca_enabled == 1u) {
        let center = vec2<f32>(0.5, 0.5);
        let dir = normalize(tc - center);
        let shift_r = tc + dir * vec2<f32>(layer.effect_ca_shift_r / globals.viewport_size.x);
        let shift_b = tc + dir * vec2<f32>(layer.effect_ca_shift_b / globals.viewport_size.y);
        let r = textureSample(t_diffuse, s_diffuse, shift_r).r;
        let b = textureSample(t_diffuse, s_diffuse, shift_b).b;
        c = vec4<f32>(r, c.g, b, c.a);
    }

    // ── Motion Blur: directional blur based on per-pixel velocity ──
    if (layer.motionblur_enabled == 1u) {
        let vel = vec2<f32>(layer.motionblur_velocity_x, layer.motionblur_velocity_y);
        let speed = length(vel);
        if (speed > 0.01) {
            let texel = vec2<f32>(1.0) / globals.viewport_size;
            let dir = normalize(vel) * texel;
            let max_offset = speed * layer.motionblur_shutter;
            let samples = max(layer.motionblur_samples, 1u);
            var blur_color = c;
            var blur_weight = 1.0;
            // Forward samples
            for (var s = 1u; s <= samples; s = s + 1u) {
                let t = f32(s) / f32(samples);
                let offset = dir * max_offset * t;
                blur_color += textureSample(t_diffuse, s_diffuse, tc + offset);
                blur_weight += 1.0;
            }
            // Backward samples
            for (var s = 1u; s <= samples; s = s + 1u) {
                let t = f32(s) / f32(samples);
                let offset = dir * max_offset * t;
                blur_color += textureSample(t_diffuse, s_diffuse, tc - offset);
                blur_weight += 1.0;
            }
            c = blur_color / blur_weight;
        }
    }

    // ── Glow: screen-space bloom from bright areas (improved: 16 samples + dual ring) ──
    if (layer.glow_enabled == 1u && c.a > 0.01) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let texel = vec2<f32>(1.0) / vp;
        let gr = layer.glow_radius * texel;
        let thresh = max(layer.glow_threshold, 0.001);

        var bloom = vec3<f32>(0.0);
        var bloom_weight = 0.0;

        // Inner ring: 8 samples at radius 1.0
        for (var s = 0; s < 8; s = s + 1) {
            let angle = f32(s) * 0.785398; // PI/4
            let offset = vec2<f32>(cos(angle), sin(angle)) * gr;
            let sc = textureSample(t_diffuse, s_diffuse, tc + offset);
            let luma = dot(sc.rgb, vec3<f32>(0.299, 0.587, 0.114));
            if (luma > thresh) {
                let contribution = (luma - thresh) / max(luma, 0.001);
                bloom += sc.rgb * contribution;
                bloom_weight += contribution;
            }
        }

        // Outer ring: 8 samples at radius 2.0 (wider spread)
        for (var s = 0; s < 8; s = s + 1) {
            let angle = f32(s) * 0.785398 + 0.392699; // offset by PI/8
            let offset = vec2<f32>(cos(angle), sin(angle)) * gr * 2.0;
            let sc = textureSample(t_diffuse, s_diffuse, tc + offset);
            let luma = dot(sc.rgb, vec3<f32>(0.299, 0.587, 0.114));
            if (luma > thresh) {
                let contribution = (luma - thresh) / max(luma, 0.001) * 0.5; // weight outer ring less
                bloom += sc.rgb * contribution;
                bloom_weight += contribution;
            }
        }

        // Normalize and apply intensity
        if (bloom_weight > 0.01) {
            bloom = bloom / max(bloom_weight, 1.0) * layer.glow_intensity;
        }

        // Tint the bloom
        let gc = layer.glow_color.rgb;
        let gc_lum = max(dot(gc, vec3<f32>(0.333)), 0.001);
        c = vec4<f32>(c.r + bloom.r * gc.r / gc_lum, c.g + bloom.g * gc.g / gc_lum, c.b + bloom.b * gc.b / gc_lum, c.a);
    }

    // ── Lens Flare: screen-space optical flare from light source ──
    if (layer.flare_enabled == 1u) {
        let flare_center = vec2<f32>(layer.flare_pos_x, layer.flare_pos_y);
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let texel = vec2<f32>(1.0) / vp;

        // Distance from this pixel to the flare center
        let d = distance(tc_in, flare_center);
        let d_norm = d * 2.0; // normalize to 0..1 range

        // Core glow: bright center
        let core_radius = 0.02;
        let core = exp(-d_norm * d_norm / (core_radius * core_radius)) * layer.flare_intensity;

        // Ring artifacts: concentric rings along the axis from center to pixel
        let ring_phase = d_norm * 12.0; // ring frequency
        let ring = sin(ring_phase) * 0.3 + 0.5; // oscillate 0.2..0.8
        let ring_mask = exp(-d_norm * 3.0); // fade with distance
        let ring_contribution = ring * ring_mask * layer.flare_intensity * 0.3;

        // Streaks: 4-pointed star along horizontal and vertical axes
        let to_center = tc_in - flare_center;
        let streak_h = exp(-abs(to_center.y) * 80.0) * exp(-abs(to_center.x) * 8.0);
        let streak_v = exp(-abs(to_center.x) * 80.0) * exp(-abs(to_center.y) * 8.0);
        let streaks = (streak_h + streak_v) * layer.flare_intensity * 0.4;

        // Combine all flare elements
        let flare_total = (core + ring_contribution + streaks) * layer.flare_threshold;
        let fc = layer.flare_color.rgb;
        c = vec4<f32>(
            clamp(c.r + fc.r * flare_total, 0.0, 1.0),
            clamp(c.g + fc.g * flare_total, 0.0, 1.0),
            clamp(c.b + fc.b * flare_total, 0.0, 1.0),
            c.a
        );
    }

    return c;
}

// ── Layer Style Helpers ────────────────────────────────────────────────────

fn ls_stroke_edge_distance(alpha: f32) -> f32 {
    let dx = abs(dpdx(alpha));
    let dy = abs(dpdy(alpha));
    let edge_width = max(dx + dy, 0.001);
    return clamp((1.0 - alpha) / edge_width, 0.0, 1.0);
}

fn ls_apply_stroke(color: vec4<f32>, alpha: f32, stroke_color: vec3<f32>, width: f32) -> vec4<f32> {
    if width <= 0.0 || alpha <= 0.0 { return color; }
    let dist = ls_stroke_edge_distance(alpha);
    let stroke_alpha = 1.0 - smoothstep(0.0, width * 0.01, dist);
    let out_rgb = mix(color.rgb, stroke_color, stroke_alpha);
    let out_a = max(color.a, stroke_alpha);
    return vec4<f32>(out_rgb, out_a);
}

fn ls_apply_color_overlay(color: vec4<f32>, overlay: vec4<f32>) -> vec4<f32> {
    let a = overlay.a * color.a;
    return vec4<f32>(mix(color.rgb, overlay.rgb, overlay.a), a);
}

fn ls_apply_gradient_overlay(color: vec4<f32>, pos: vec2<f32>, start: vec2<f32>, end: vec2<f32>, c1: vec4<f32>, c2: vec4<f32>) -> vec4<f32> {
    let d = end - start;
    let len_sq = dot(d, d);
    var t: f32;
    if len_sq < 0.001 {
        t = 0.5;
    } else {
        t = clamp(dot(pos - start, d) / len_sq, 0.0, 1.0);
    }
    let grad_color = mix(c1, c2, t);
    let a = grad_color.a * color.a;
    return vec4<f32>(mix(color.rgb, grad_color.rgb, grad_color.a), a);
}

fn ls_apply_inner_shadow(color: vec4<f32>, alpha: f32, offset: vec2<f32>, size: f32, shadow_color: vec3<f32>, opacity: f32) -> vec4<f32> {
    if alpha > 0.99 || size <= 0.0 || opacity <= 0.0 { return color; }
    let edge_dist = 1.0 - alpha;
    let shadow_falloff = 1.0 - smoothstep(0.0, size * 0.01, edge_dist);
    let offset_len = length(offset);
    let shadow_strength = shadow_falloff * opacity * smoothstep(0.0, offset_len + 0.001, edge_dist * 10.0);
    let shadow = vec4<f32>(shadow_color, shadow_strength);
    let out_a = max(color.a, shadow.a * color.a);
    let out_rgb = mix(color.rgb, shadow.rgb, shadow.a);
    return vec4<f32>(out_rgb, out_a);
}

fn ls_apply_bevel(color: vec4<f32>, alpha: f32, size: f32, angle: f32, strength: f32, light_color: vec3<f32>, dark_color: vec3<f32>) -> vec4<f32> {
    if alpha <= 0.01 || size <= 0.0 { return color; }
    let normal_x = dpdx(alpha);
    let normal_y = dpdy(alpha);
    let edge_width = max(abs(normal_x) + abs(normal_y), 0.001);
    let bevel_t = clamp((1.0 - alpha) / (size * 0.01 * 4.0 + 0.001), 0.0, 1.0);
    let rad = angle * 0.01745329251;
    let light_dir = vec2<f32>(cos(rad), sin(rad));
    let edge_normal = normalize(vec2<f32>(normal_x, normal_y) + vec2<f32>(0.0001));
    let ndotl = dot(edge_normal, light_dir);
    let highlight = max(ndotl, 0.0) * strength * 0.01;
    let shadow = max(-ndotl, 0.0) * strength * 0.01;
    let highlight_mask = bevel_t * highlight;
    let shadow_mask = bevel_t * shadow;
    let out_rgb = color.rgb + light_color * highlight_mask - dark_color * shadow_mask;
    return vec4<f32>(clamp(out_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), color.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var blur_extend = 0.0;
    if (layer.effect_blur_enabled == 1u) {
        blur_extend = layer.effect_blur_radius * 0.01;
    }

    // --- Chromatic Aberration ---
    // Offsets the R and B channels slightly relative to the center.
    var final_color: vec4<f32>;
    if (layer.effect_ca_enabled == 1u) {
        // Compute edge distance (0 at center, 1 at corner)
        let edge_dist = length(in.local_pos) * 2.0; // 0 to ~1.414
        let falloff = pow(clamp(edge_dist, 0.0, 1.0), max(layer.effect_ca_edge_falloff * 4.0, 0.1));

        let texel_size = 1.0 / globals.viewport_size;

        // Red channel: shift outward from center
        let dir = normalize(in.local_pos + vec2<f32>(0.0001, 0.0001));
        let r_offset_local = dir * layer.effect_ca_shift_r * falloff * 0.01;
        let b_offset_local = dir * (-layer.effect_ca_shift_b) * falloff * 0.01;

        let r_local = in.local_pos - r_offset_local;
        let b_local = in.local_pos - b_offset_local;

        let r_tc = in.tex_coords - r_offset_local.x * dpdx(in.tex_coords) - r_offset_local.y * dpdy(in.tex_coords);
        let b_tc = in.tex_coords - b_offset_local.x * dpdx(in.tex_coords) - b_offset_local.y * dpdy(in.tex_coords);

        let col_r = sample_layer_color(r_local, r_tc, blur_extend);
        let col_g = sample_layer_color(in.local_pos, in.tex_coords, blur_extend);
        let col_b = sample_layer_color(b_local, b_tc, blur_extend);

        final_color = vec4<f32>(col_r.r, col_g.g, col_b.b, col_g.a);
    } else {
        final_color = sample_layer_color(in.local_pos, in.tex_coords, blur_extend);
    }

    // --- Color Tint ---
    if (layer.effect_tint_enabled == 1u) {
        let tinted_rgb = mix(final_color.rgb, layer.effect_tint_color.rgb, layer.effect_tint_intensity);
        final_color = vec4<f32>(tinted_rgb, final_color.a);
    }

    // --- Drop Shadow ---
    if (layer.effect_shadow_enabled == 1u) {
        let rad = -layer.effect_shadow_direction * 0.01745329251;
        let offset_pixels = vec2<f32>(cos(rad), sin(rad)) * layer.effect_shadow_distance;
        
        let local_dx = dpdx(in.local_pos);
        let local_dy = dpdy(in.local_pos);
        let local_offset = offset_pixels.x * local_dx + offset_pixels.y * local_dy;
        let shadow_local_pos = in.local_pos - local_offset;
        
        let tex_dx = dpdx(in.tex_coords);
        let tex_dy = dpdy(in.tex_coords);
        let tex_offset = offset_pixels.x * tex_dx + offset_pixels.y * tex_dy;
        let shadow_tex_coords = in.tex_coords - tex_offset;
        
        let shadow_blur_extend = layer.effect_shadow_softness * 0.01;
        
        var shadow_alpha = 0.0;
        if (layer.layer_type == 0u) {
            let d_x = abs(shadow_local_pos.x) - 0.5;
            let d_y = abs(shadow_local_pos.y) - 0.5;
            let d = max(d_x, d_y);
            shadow_alpha = 1.0 - smoothstep(-0.02 - shadow_blur_extend, 0.0 + shadow_blur_extend, d);
        } else if (layer.layer_type == 1u) {
            if (layer.effect_shadow_softness > 0.0) {
                let texel_size = 1.0 / globals.viewport_size;
                let offset = layer.effect_shadow_softness * texel_size * 0.5;
                var alpha_sum = textureSample(t_diffuse, s_diffuse, shadow_tex_coords).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(-offset.x, -offset.y)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(offset.x, -offset.y)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(-offset.x, offset.y)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(offset.x, offset.y)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(-offset.x, 0.0)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(offset.x, 0.0)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(0.0, -offset.y)).a;
                alpha_sum += textureSample(t_diffuse, s_diffuse, shadow_tex_coords + vec2<f32>(0.0, offset.y)).a;
                shadow_alpha = alpha_sum / 9.0;
            } else {
                shadow_alpha = textureSample(t_diffuse, s_diffuse, shadow_tex_coords).a;
            }
        } else if (layer.layer_type == 2u) {
            shadow_alpha = shape_sdf_alpha(shadow_local_pos, shadow_blur_extend);
        } else if (layer.layer_type == 3u) {
            let d_x = abs(shadow_local_pos.x) - 0.5;
            let d_y = abs(shadow_local_pos.y) - 0.5;
            let d = max(d_x, d_y);
            shadow_alpha = 1.0 - smoothstep(-0.02 - shadow_blur_extend, 0.0 + shadow_blur_extend, d);
        }
        
        let shadow_intensity = layer.effect_shadow_opacity / 100.0;
        let shadow_color = vec4<f32>(layer.effect_shadow_color.rgb, shadow_alpha * shadow_intensity);
        
        let blended_rgb = final_color.rgb * final_color.a + shadow_color.rgb * shadow_color.a * (1.0 - final_color.a);
        let blended_alpha = final_color.a + shadow_color.a * (1.0 - final_color.a);
        final_color = vec4<f32>(blended_rgb / max(blended_alpha, 0.001), blended_alpha);
    }

    // --- Vignette ---
    if (layer.effect_vignette_enabled == 1u) {
        // local_pos is in [-0.5, 0.5]; normalize to [-1, 1] with aspect-corrected roundness
        let uv = in.local_pos * 2.0;
        // Mix between square (aspect ratio) and circle based on roundness
        let r = layer.effect_vignette_roundness;
        let dist_sq = uv.x * uv.x * (1.0 - r * 0.5) + uv.y * uv.y * (1.0 - r * 0.5);
        let dist_circ = sqrt(uv.x * uv.x + uv.y * uv.y);
        let dist = mix(dist_sq, dist_circ, r);

        let feather = max(layer.effect_vignette_feather / 100.0, 0.001) * 1.5;
        let inner = 1.0 - feather;
        let vignette_factor = smoothstep(inner, inner + feather, dist);
        let vignette_strength = (layer.effect_vignette_intensity / 100.0) * vignette_factor;

        // Blend the vignette color over the final color
        let vcolor = layer.effect_vignette_color;
        final_color = vec4<f32>(
            mix(final_color.rgb, vcolor.rgb, vignette_strength * vcolor.a),
            final_color.a
        );
    }

    // --- Levels Adjustment ---
    if (layer.levels_enabled == 1u) {
        let range = max(layer.levels_in_white - layer.levels_in_black, 0.001);
        let val_normalized = clamp((final_color.rgb - vec3<f32>(layer.levels_in_black)) / range, vec3<f32>(0.0), vec3<f32>(1.0));
        let gamma_power = 1.0 / max(layer.levels_gamma, 0.01);
        let gamma_adjusted = pow(val_normalized, vec3<f32>(gamma_power));
        final_color = vec4<f32>(
            layer.levels_out_black + gamma_adjusted * (layer.levels_out_white - layer.levels_out_black),
            final_color.a
        );
    }

    // --- Hue / Saturation ---
    if (layer.huesat_enabled == 1u) {
        // Hue / Saturation adjustment using relative luminance weights
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let desat = mix(vec3<f32>(luma), final_color.rgb, layer.huesat_sat);
        final_color = vec4<f32>(
            clamp(desat * layer.huesat_light, vec3<f32>(0.0), vec3<f32>(1.0)),
            final_color.a
        );
    }

    // --- Glow / Bloom ---
    if (layer.glow_enabled == 1u) {
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        if (luma >= layer.glow_threshold) {
            let highlight = (luma - layer.glow_threshold) / max(1.0 - layer.glow_threshold, 0.001);
            let bloom_rgb = layer.glow_color.rgb * highlight * layer.glow_intensity;
            final_color = vec4<f32>(final_color.rgb + bloom_rgb, final_color.a);
        }
    }

    // --- Physical Film Grain Noise (improved: temporal variation + color noise) ---
    if (layer.grain_enabled == 1u) {
        let grain_uv = in.tex_coords * globals.viewport_size / max(layer.grain_size, 0.1);
        let grain_frame = fract(globals.exposure_ev * 0.1);
        let grain_uv_t = grain_uv + vec2<f32>(grain_frame * 43.758, grain_frame * 17.321);
        let n1 = fract(sin(dot(grain_uv_t, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let n2 = fract(sin(dot(grain_uv_t * 2.0, vec2<f32>(63.7264, 10.873))) * 23421.631);
        let n3 = fract(sin(dot(grain_uv_t * 0.5, vec2<f32>(45.164, 89.332))) * 65432.123);
        let grain_luma = (n1 * 0.60 + n2 * 0.25 + n3 * 0.15) - 0.5;
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let luma_weight = 1.0 - abs(luma - 0.5) * 1.5;
        let grain_r = grain_luma + (n2 - 0.5) * 0.15;
        let grain_g = grain_luma + (n3 - 0.5) * 0.15;
        let grain_b = grain_luma + (n1 - 0.5) * 0.15;
        let intensity = layer.grain_intensity * luma_weight;
        final_color = vec4<f32>(
            clamp(final_color.r + grain_r * intensity, 0.0, 1.0),
            clamp(final_color.g + grain_g * intensity, 0.0, 1.0),
            clamp(final_color.b + grain_b * intensity, 0.0, 1.0),
            final_color.a
        );
    }

    // --- Invert / Color Inversion ---
    if (layer.invert_enabled == 1u) {
        final_color = vec4<f32>(
            1.0 - final_color.r,
            1.0 - final_color.g,
            1.0 - final_color.b,
            final_color.a
        );
    }

    // --- Posterize / Color Quantization ---
    if (layer.posterize_enabled == 1u && layer.posterize_levels >= 2.0) {
        let steps = max(layer.posterize_levels - 1.0, 1.0);
        final_color = vec4<f32>(
            floor(final_color.rgb * steps + 0.5) / steps,
            final_color.a
        );
    }

    // --- Tint / Dual Color Map ---
    if (layer.tint_enabled == 1u && layer.tint_amount > 0.0) {
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let mapped = mix(layer.tint_black.rgb, layer.tint_white.rgb, luma);
        let tinted = mix(final_color.rgb, mapped, layer.tint_amount);
        final_color = vec4<f32>(tinted, final_color.a);
    }

    // --- Vignette / Lens Falloff ---
    if (layer.vignette_enabled == 1u && layer.vignette_amount > 0.0) {
        let center_dist = length(in.local_pos * 2.0);
        let vig_start = max(layer.vignette_midpoint, 0.0);
        let vig_feather = max(layer.vignette_feather, 0.01);
        let vig_factor = 1.0 - smoothstep(vig_start, vig_start + vig_feather, center_dist) * layer.vignette_amount;
        final_color = vec4<f32>(final_color.rgb * clamp(vig_factor, 0.0, 1.0), final_color.a);
    }

    // --- CRT Scanlines / TV Glitch ---
    if (layer.crt_enabled == 1u) {
        let scan_count = max(layer.crt_scanline_count, 100.0);
        let scan_phase = in.tex_coords.y * scan_count * 3.14159265;
        let scanline = 0.5 + 0.5 * sin(scan_phase);
        let scan_mult = 1.0 - (1.0 - scanline) * layer.crt_scanline_intensity;
        final_color = vec4<f32>(final_color.rgb * clamp(scan_mult, 0.0, 1.0), final_color.a);
    }

    // ── Layer Styles: applied after effects, before compositing ──
    if (layer.ls_style_flags != 0u && final_color.a > 0.001) {
        // Stroke (bit 0)
        if (layer.ls_style_flags & 1u) != 0u {
            let stroke_col = vec3<f32>(layer.ls_stroke_r, layer.ls_stroke_g, layer.ls_stroke_b);
            final_color = ls_apply_stroke(final_color, final_color.a, stroke_col, layer.ls_stroke_width);
        }
        // Color Overlay (bit 1)
        if (layer.ls_style_flags & 2u) != 0u {
            let overlay = vec4<f32>(layer.ls_color_overlay_r, layer.ls_color_overlay_g, layer.ls_color_overlay_b, layer.ls_color_overlay_a);
            final_color = ls_apply_color_overlay(final_color, overlay);
        }
        // Gradient Overlay (bit 2)
        if (layer.ls_style_flags & 4u) != 0u {
            let start = vec2<f32>(layer.ls_gradient_start_x, layer.ls_gradient_start_y);
            let end = vec2<f32>(layer.ls_gradient_end_x, layer.ls_gradient_end_y);
            let c1 = vec4<f32>(layer.ls_gradient_color1_r, layer.ls_gradient_color1_g, layer.ls_gradient_color1_b, layer.ls_gradient_color1_a);
            let c2 = vec4<f32>(layer.ls_gradient_color2_r, layer.ls_gradient_color2_g, layer.ls_gradient_color2_b, layer.ls_gradient_color2_a);
            final_color = ls_apply_gradient_overlay(final_color, in.local_pos, start, end, c1, c2);
        }
        // Inner Shadow (bit 3)
        if (layer.ls_style_flags & 8u) != 0u {
            let is_offset = vec2<f32>(layer.ls_inner_shadow_offset_x, layer.ls_inner_shadow_offset_y);
            let is_color = vec3<f32>(layer.ls_inner_shadow_r, layer.ls_inner_shadow_g, layer.ls_inner_shadow_b);
            final_color = ls_apply_inner_shadow(final_color, final_color.a, is_offset, layer.ls_inner_shadow_size, is_color, layer.ls_inner_shadow_opacity);
        }
        // Bevel/Emboss (bit 4)
        if (layer.ls_style_flags & 16u) != 0u {
            let light_col = vec3<f32>(layer.ls_bevel_light_r, layer.ls_bevel_light_g, layer.ls_bevel_light_b);
            let dark_col = vec3<f32>(0.0, 0.0, 0.0);
            final_color = ls_apply_bevel(final_color, final_color.a, layer.ls_bevel_size, layer.ls_bevel_angle, layer.ls_bevel_strength, light_col, dark_col);
        }
    }


    // ── Shadow map: darken by CPU-projected density at this screen position ──
    if (globals.shadow_enabled == 1u && final_color.a > 0.01) {
        let occ = textureSample(t_shadow, s_shadow, in.tex_coords).r;
        let dark = 1.0 - min(occ, 1.0);
        final_color = vec4<f32>(final_color.rgb * select(1.0, dark, occ > 0.003), final_color.a);
    }

    // --- Layer Opacity ---
    final_color.a = final_color.a * (layer.opacity / 100.0);

    // --- GPU Layer Mask Compositing ---
    if (layer.mask_enabled == 1u) {
        // Mask mode: 0=none, 1=alpha, 2=inverted alpha, 3=luma, 4=inverted luma
        var mask_alpha: f32 = 1.0;
        if (layer.mask_mode == 1u) {
            mask_alpha = textureSample(t_mask, s_mask, in.tex_coords).a;
        } else if (layer.mask_mode == 2u) {
            mask_alpha = 1.0 - textureSample(t_mask, s_mask, in.tex_coords).a;
        } else if (layer.mask_mode == 3u) {
            let tex = textureSample(t_mask, s_mask, in.tex_coords);
            mask_alpha = dot(tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        } else if (layer.mask_mode == 4u) {
            let tex = textureSample(t_mask, s_mask, in.tex_coords);
            mask_alpha = 1.0 - dot(tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        }
        // Apply mask inversion and feather
        if (layer.mask_inverted == 1u) {
            mask_alpha = 1.0 - mask_alpha;
        }
        if (layer.mask_feather > 0.01) {
            // Simple feather blur approximation using smoothstep
            mask_alpha = smoothstep(0.0, layer.mask_feather, mask_alpha * layer.mask_feather);
        }
        final_color.a = final_color.a * mask_alpha;
    }

    // --- Track Matte Masking ---
    if (layer.track_matte_mode > 0u) {
        let matte_tex = textureSample(t_matte, s_matte, in.tex_coords);
        var matte_alpha = 1.0;
        if (layer.track_matte_mode == 1u) {
            matte_alpha = matte_tex.a;
        } else if (layer.track_matte_mode == 2u) {
            matte_alpha = 1.0 - matte_tex.a;
        } else if (layer.track_matte_mode == 3u) {
            matte_alpha = dot(matte_tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        } else if (layer.track_matte_mode == 4u) {
            matte_alpha = 1.0 - dot(matte_tex.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        }
        final_color = vec4<f32>(final_color.rgb, final_color.a * matte_alpha);
    }

    // --- AE Blend Mode Compositing ---
    // The GPU blend equation handles Normal (alpha-over) automatically.
    // For other modes we bake the result into the RGB output by treating
    // the underlying buffer color as black (standard single-pass approximation).
    // A full multi-pass solution requires ping-pong offscreen textures.
    if (layer.blend_mode == 1u) {
        // Multiply: src * backdrop. Approximated as src * src (self-multiply darkens transparencies).
        final_color = vec4<f32>(final_color.rgb * final_color.rgb, final_color.a);
    } else if (layer.blend_mode == 2u) {
        // Screen: 1 - (1-src)*(1-src)
        let inv = 1.0 - final_color.rgb;
        final_color = vec4<f32>(1.0 - inv * inv, final_color.a);
    } else if (layer.blend_mode == 3u) {
        // Overlay: 2*src^2 if src<0.5, else 1-2*(1-src)^2
        let s = final_color.rgb;
        let overlay = select(
            1.0 - 2.0 * (1.0 - s) * (1.0 - s),
            2.0 * s * s,
            s < vec3<f32>(0.5)
        );
        final_color = vec4<f32>(overlay, final_color.a);
    } else if (layer.blend_mode == 4u) {
        // Add (Linear Dodge): clamp(src * 2, 0, 1) — brightens compositing
        final_color = vec4<f32>(clamp(final_color.rgb * 1.5, vec3<f32>(0.0), vec3<f32>(1.0)), final_color.a);
    } else if (layer.blend_mode == 5u) {
        // Darken: min(src, backdrop). Approximate via pow darkening.
        final_color = vec4<f32>(pow(final_color.rgb, vec3<f32>(1.5)), final_color.a);
    } else if (layer.blend_mode == 6u) {
        // Lighten: max(src, backdrop). Approximate via pow brightening.
        final_color = vec4<f32>(pow(final_color.rgb, vec3<f32>(0.67)), final_color.a);
    }
    // blend_mode == 0u: Normal — no modification needed, GPU alpha blend handles it.

    // --- Viewport Exposure Adjustment ---
    final_color = vec4<f32>(final_color.rgb * pow(2.0, globals.exposure_ev), final_color.a);

    // --- Viewport LUT Color Management ---
    if (globals.lut_mode == 1u) {
        // Linear sRGB conversion (Approximated 2.2 Gamma linearize)
        final_color = vec4<f32>(pow(final_color.rgb, vec3<f32>(2.2)), final_color.a);
    } else if (globals.lut_mode == 2u) {
        // ACEScg Approximated filmic tone map curve
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        let aces = clamp((final_color.rgb * (a * final_color.rgb + vec3<f32>(b))) / (final_color.rgb * (c * final_color.rgb + vec3<f32>(d)) + vec3<f32>(e)), vec3<f32>(0.0), vec3<f32>(1.0));
        final_color = vec4<f32>(aces, final_color.a);
    }

    if (final_color.a <= 0.001) {
        discard;
    }

    return final_color;
}
