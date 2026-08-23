// Multi-layer composite compute shader
// Blends up to 16 layer textures with per-layer opacity and blend mode

struct CompositeParams {
    layer_count: u32,
    width: u32,
    height: u32,
    blend_mode: u32, // 0=normal for now (per-layer modes in future)
};

@group(0) @binding(0) var base_texture: texture_2d<f32>;
@group(0) @binding(1) var base_sampler: sampler;
@group(0) @binding(2) var<uniform> params: CompositeParams;
@group(0) @binding(3) var output_texture: texture_storage_2d<rgba8unorm, write>;

// Per-layer data uploaded as uniform arrays
struct LayerInfo {
    opacity: f32,
    blend_mode: u32,
    visible: u32,
    _pad: u32,
};
@group(0) @binding(4) var<storage, read> layer_infos: array<LayerInfo>;
@group(0) @binding(5) var layer_textures: binding_array<texture_2d<f32>, 16>;
@group(0) @binding(6) var layer_samplers: binding_array<sampler, 16>;

fn apply_blend_mode(base: vec3<f32>, src: vec3<f32>, mode: u32) -> vec3<f32> {
    switch mode {
        case 1u { return base * src; } // Multiply
        case 2u { return base + src - base * src; } // Screen
        case 3u { // Overlay
            let lum = dot(base, vec3<f32>(0.299, 0.587, 0.114));
            let blended = select(
                2.0 * base * src,
                base + src - 2.0 * (1.0 - base) * (1.0 - src),
                lum > 0.5
            );
            return blended;
        }
        case 4u { return base + src; } // Add
        case 5u { return min(base, src); } // Darken
        case 6u { return max(base, src); } // Lighten
        default { return mix(src, base, 0.0); } // Normal
    }
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) { return; }
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(f32(params.width), f32(params.height));

    // Start with the background/base
    var result = textureSampleLevel(base_texture, base_sampler, uv, 0.0);

    // Composite each layer on top
    for (var i = 0u; i < params.layer_count; i = i + 1u) {
        if (layer_infos[i].visible == 0u || layer_infos[i].opacity < 0.01) { continue; }

        let layer_color = textureSampleLevel(layer_textures[i], layer_samplers[i], uv, 0.0);
        let alpha = layer_color.a * layer_infos[i].opacity;

        if (alpha < 0.01) { continue; }

        let blended_rgb = apply_blend_mode(result.rgb, layer_color.rgb, layer_infos[i].blend_mode);

        // Standard alpha-over compositing
        let final_alpha = alpha + result.a * (1.0 - alpha);
        result = vec4<f32>(
            mix(result.rgb, blended_rgb, alpha),
            final_alpha
        );
    }

    let pos = vec2<i32>(i32(gid.x), i32(gid.y));
    textureStore(output_texture, pos, result);
}
