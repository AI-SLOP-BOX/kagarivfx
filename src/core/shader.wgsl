struct Globals {
    viewport_size: vec2<f32>,
    exposure_ev: f32,
    lut_mode: u32,
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

    // Shape params: x = polygon sides, y = rectangle corner radius (normalized 0..1 of half-size)
    shape_params: vec4<f32>,

    // Mesh Warp / Corner Pin
    meshwarp_enabled: u32,
    corner_top_left: vec2<f32>,
    corner_top_right: vec2<f32>,
    corner_bottom_left: vec2<f32>,
    corner_bottom_right: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> layer: Layer;
@group(2) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(1) var s_diffuse: sampler;

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
    return alpha;
}

// Helper: sample layer color at a given local_pos and tex_coords
fn sample_layer_color(local_pos_in: vec2<f32>, tc_in: vec2<f32>, blur_extend: f32) -> vec4<f32> {
    // Mesh Warp / Corner Pin: bilinear corner-offset displacement field.
    // Corners are pixel offsets from each quad corner; normalize by viewport to get UV deltas.
    var tc = tc_in;
    var local_pos = local_pos_in;
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
            // --- Gaussian Blur (Single-Pass Box Approximation) ---
            // Current: 9-tap box kernel in a single render pass.
            //   Quality limit: banding artifacts appear at large blur_radius values
            //   because the sample footprint grows but sample count stays fixed at 9.
            //
            // Future roadmap: Replace with 2-pass Separable Gaussian Blur using
            //   intermediate offscreen textures (ping-pong buffers):
            //   Pass 1: horizontal kernel N taps  → offscreen texture A
            //   Pass 2: vertical kernel N taps    → final composite
            //   This gives O(2N) samples vs O(N²) for the same quality, and is
            //   how AE, Nuke and OBS all implement their Gaussian blur.
            let texel_size = 1.0 / globals.viewport_size;
            let offset = layer.effect_blur_radius * texel_size;
            var color_sum = textureSample(t_diffuse, s_diffuse, tc) * 0.227027;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(offset.x * 1.384615, 0.0)) * 0.3162162;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(offset.x * 1.384615, 0.0)) * 0.3162162;
            color_sum += textureSample(t_diffuse, s_diffuse, tc + vec2<f32>(0.0, offset.y * 3.230769)) * 0.0702702;
            color_sum += textureSample(t_diffuse, s_diffuse, tc - vec2<f32>(0.0, offset.y * 3.230769)) * 0.0702702;
            c = color_sum;
        } else {
            c = textureSample(t_diffuse, s_diffuse, tc);
        }
    } else if (layer.layer_type == 2u) {
        let alpha = shape_sdf_alpha(local_pos, blur_extend);
        c = vec4<f32>(layer.color.rgb, layer.color.a * alpha);
    } else if (layer.layer_type == 3u) {
        c = layer.color;
    } else if (layer.layer_type == 8u) {
        // Particle layers are simulated and rasterized on the CPU;
        // return fully transparent so fs_main's alpha check discards this quad.
        c = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    // ── Glow: screen-space bloom from bright areas ──
    if (layer.glow_enabled == 1u && c.a > 0.01) {
        let vp = max(globals.viewport_size, vec2<f32>(1.0, 1.0));
        let texel = vec2<f32>(1.0) / vp;
        let gr = layer.glow_radius * texel;

        var bloom = vec3<f32>(0.0);
        for (var s = 0; s < 8; s = s + 1) {
            let angle = f32(s) * 0.785398; // PI/4
            let sx = tc.x + cos(angle) * gr.x;
            let sy = tc.y + sin(angle) * gr.y;
            let sc = textureSample(t_diffuse, s_diffuse, vec2<f32>(sx, sy));
            let luma = dot(sc.rgb, vec3<f32>(0.299, 0.587, 0.114));
            let thresh = max(layer.glow_threshold, 0.001);
            if (luma > thresh) {
                bloom += sc.rgb * ((luma - thresh) / luma);
            }
        }
        bloom = bloom / 8.0 * layer.glow_intensity;

        // Tint the bloom
        let gc = layer.glow_color.rgb;
        let gc_lum = max(dot(gc, vec3<f32>(0.333)), 0.001);
        c = vec4<f32>(c.r + bloom.r * gc.r / gc_lum, c.g + bloom.g * gc.g / gc_lum, c.b + bloom.b * gc.b / gc_lum, c.a);
    }

    return c;
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

    // --- Physical Film Grain Noise ---
    if (layer.grain_enabled == 1u) {
        let grain_uv = in.tex_coords * globals.viewport_size / max(layer.grain_size, 0.1);
        let n = fract(sin(dot(grain_uv, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let grain_noise = (n - 0.5) * layer.grain_intensity;
        let luma = dot(final_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let luma_weight = 1.0 - abs(luma - 0.5) * 2.0;
        let final_grain = grain_noise * luma_weight;
        final_color = vec4<f32>(clamp(final_color.rgb + vec3<f32>(final_grain), vec3<f32>(0.0), vec3<f32>(1.0)), final_color.a);
    }

    // --- Layer Opacity ---
    final_color.a = final_color.a * (layer.opacity / 100.0);

    // --- Track Matte Masking ---
    if (layer.track_matte_mode > 0u) {
        let matte_tex = textureSample(t_diffuse, s_diffuse, in.tex_coords);
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
