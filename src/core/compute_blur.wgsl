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
    // Gaussian: kernel tap count per side. Directional: total tap count.
    radius: u32,
    // 0 = gaussian horizontal, 2 = gaussian vertical, 1 = directional,
    // 3 = radial zoom blur (radius taps toward center)
    mode: u32,
    // Directional angle (radians); radial: unused
    angle: f32,
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

    let w = params.width;
    let h = params.height;

    if (params.mode == 3u) {
        // ── Radial zoom blur: N taps along the ray to the frame center ──
        let n = max(params.radius, 1u);
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let rx = f32(x) - cx;
        let ry = f32(y) - cy;
        var acc = vec4<f32>(0.0);
        var wsum = 0.0;
        for (var i: i32 = 0; i < i32(n); i = i + 1) {
            let t = 1.0 - f32(i) / f32(n);
            let sx = u32(clamp(i32(cx + rx * t), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(cy + ry * t), 0, i32(h - 1u)));
            let c = unpack(src[sy * w + sx]);
            acc = acc + vec4<f32>(c.rgb * c.a, c.a);
            wsum = wsum + 1.0;
        }
        let inv = 1.0 / max(wsum, 0.0001);
        let pm = acc * inv;
        let out_a = clamp(pm.a, 0.0, 1.0);
        let rgb = select(vec3<f32>(0.0), pm.rgb / max(out_a, 0.0001), out_a > 0.0001);
        dst[y * w + x] = pack(vec4<f32>(rgb, out_a));
        return;
    }

    if (params.mode == 1u) {
        // ── Directional blur: N taps along the motion vector ──
        let n = max(params.radius, 1u);
        let dx = cos(params.angle);
        let dy = sin(params.angle);
        var acc = vec4<f32>(0.0);
        var wsum = 0.0;
        for (var i: i32 = 0; i < i32(n); i = i + 1) {
            // Centered spread across the motion vector
            let off = f32(i) - f32(n - 1u) * 0.5;
            let sx = u32(clamp(i32(x) + i32(off * dx), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(y) + i32(off * dy), 0, i32(h - 1u)));
            let c = unpack(src[sy * w + sx]);
            acc = acc + vec4<f32>(c.rgb * c.a, c.a);
            wsum = wsum + 1.0;
        }
        let inv = 1.0 / max(wsum, 0.0001);
        let pm = acc * inv;
        let out_a = clamp(pm.a, 0.0, 1.0);
        let rgb = select(vec3<f32>(0.0), pm.rgb / max(out_a, 0.0001), out_a > 0.0001);
        dst[y * w + x] = pack(vec4<f32>(rgb, out_a));
        return;
    }

    // ── Gaussian separable pass ──
    let r = params.radius;

    // Premultiplied accumulators
    var acc = vec4<f32>(0.0);
    var wsum = 0.0;

    for (var i: i32 = -i32(r); i <= i32(r); i = i + 1) {
        let weight = kernel[u32(i + i32(r))];
        var sx: u32;
        var sy: u32;
        if (params.mode == 2u) {
            // Second pass marker piggybacks on angle>=PI (set by host): vertical
            let yi = i32(y) + i;
            sx = x;
            sy = u32(clamp(yi, 0, i32(h - 1u)));
        } else {
            let xi = i32(x) + i;
            sx = u32(clamp(xi, 0, i32(w - 1u)));
            sy = y;
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
