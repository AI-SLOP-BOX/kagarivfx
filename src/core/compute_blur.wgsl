// Compute-shader separable Gaussian blur (ping-pong storage buffers).
//
// Buffer-based instead of texture-based so it stays inside core WebGPU
// features (write-only storage is guaranteed; no adapter-specific format
// features needed) and works identically on Metal / Vulkan / DX12.
//
// Pass model:
//   dispatch 1: horizontal pass, src -> mid
//   dispatch 2: vertical pass,   mid -> dst
// The Rust host alternates which buffers are bound for repeated passes.
//
// Colors are premultiplied before weighting and unpremultiplied after, so
// transparent pixels never bleed their color into the blurred result.

struct Params {
    width: u32,
    height: u32,
    radius: u32,
    horizontal: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> src: array<u32>;
@group(0) @binding(2) var<storage, read_write> dst: array<u32>;
@group(0) @binding(3) var<storage, read> kernel: array<f32>;

fn unpack(px: u32) -> vec4<f32> {
    let r = f32((px >> 24u) & 0xFFu) / 255.0;
    let g = f32((px >> 16u) & 0xFFu) / 255.0;
    let b = f32((px >> 8u) & 0xFFu) / 255.0;
    let a = f32(px & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

fn pack(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.r, 0.0, 1.0) * 255.0 + 0.5);
    let g = u32(clamp(c.g, 0.0, 1.0) * 255.0 + 0.5);
    let b = u32(clamp(c.b, 0.0, 1.0) * 255.0 + 0.5);
    let a = u32(clamp(c.a, 0.0, 1.0) * 255.0 + 0.5);
    return (r << 24u) | (g << 16u) | (b << 8u) | a;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= params.width || y >= params.height) {
        return;
    }

    let r = params.radius;
    let w = params.width;
    let h = params.height;

    // Premultiplied accumulators
    var acc = vec4<f32>(0.0);
    var wsum = 0.0;

    for (var i: i32 = -i32(r); i <= i32(r); i = i + 1) {
        let weight = kernel[u32(i + i32(r))];
        var sx: u32;
        var sy: u32;
        if (params.horizontal == 1u) {
            let xi = i32(x) + i;
            sx = u32(clamp(xi, 0, i32(w - 1u)));
            sy = y;
        } else {
            sx = x;
            let yi = i32(y) + i;
            sy = u32(clamp(yi, 0, i32(h - 1u)));
        }
        let c = unpack(src[sy * w + sx]);
        let a = c.a;
        acc = acc + vec4<f32>(c.rgb * a, a) * weight;
        wsum = wsum + weight;
    }

    let inv = 1.0 / max(wsum, 0.0001);
    let pm = acc * inv;
    let out_a = clamp(pm.a, 0.0, 1.0);
    let rgb = select(vec3<f32>(0.0), pm.rgb / max(out_a, 0.0001), out_a > 0.0001);
    dst[y * w + x] = pack(vec4<f32>(rgb, out_a));
}
