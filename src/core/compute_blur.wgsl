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
    // 0=gauss H, 1=directional, 2=gauss V, 3=radial zoom,
    // 4=color_correct, 5=sharpen, 6=threshold, 7=emboss,
    // 8=edge_detect, 9=invert, 10=solarize, 11=posterize
    mode: u32,
    // Directional angle / color correction params
    angle: f32,
    // color_correct: brightness (-1..1), contrast (0..4)
    brightness: f32,
    contrast: f32,
    // color_correct: saturation (0..4), hue_shift (radians)
    saturation: f32,
    hue_shift: f32,
    // threshold: cutoff (0..1), posterize: levels (2..32)
    param_f3: f32,
    param_f4: f32,
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

// ── Color correction kernel ──
fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let d = mx - mn;
    var h: f32 = 0.0;
    let s = select(0.0, d / max(mx, 0.0001), mx > 0.0001);
    let v = mx;
    if d > 0.0001 {
        if mx == c.r { h = (c.g - c.b) / d; }
        else if mx == c.g { h = 2.0 + (c.b - c.r) / d; }
        else { h = 4.0 + (c.r - c.g) / d; }
        h = h / 6.0;
        if h < 0.0 { h = h + 1.0; }
    }
    return vec3<f32>(h, s, v);
}

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;
    let c = v * s;
    let x = c * (1.0 - abs(h % 2.0 - 1.0));
    let m = v - c;
    var rgb: vec3<f32>;
    if h < 1.0 { rgb = vec3<f32>(c, x, 0.0); }
    else if h < 2.0 { rgb = vec3<f32>(x, c, 0.0); }
    else if h < 3.0 { rgb = vec3<f32>(0.0, c, x); }
    else if h < 4.0 { rgb = vec3<f32>(0.0, x, c); }
    else if h < 5.0 { rgb = vec3<f32>(x, 0.0, c); }
    else { rgb = vec3<f32>(c, 0.0, x); }
    return rgb + vec3<f32>(m);
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

    // ── Mode 9: Invert ──
    if (params.mode == 9u) {
        let c = unpack(src[y * w + x]);
        dst[y * w + x] = pack(vec4<f32>(1.0 - c.r, 1.0 - c.g, 1.0 - c.b, c.a));
        return;
    }

    // ── Mode 6: Threshold ──
    if (params.mode == 6u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let v = select(0.0, 1.0, lum >= params.param_f3);
        dst[y * w + x] = pack(vec4<f32>(v, v, v, c.a));
        return;
    }

    // ── Mode 10: Solarize ──
    if (params.mode == 10u) {
        let c = unpack(src[y * w + x]);
        let t = params.param_f3;
        var rgb = c.rgb;
        if (rgb.r > t) { rgb.r = 1.0 - rgb.r; }
        if (rgb.g > t) { rgb.g = 1.0 - rgb.g; }
        if (rgb.b > t) { rgb.b = 1.0 - rgb.b; }
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 11: Posterize ──
    if (params.mode == 11u) {
        let c = unpack(src[y * w + x]);
        let lv = max(params.param_f4, 2.0);
        let s = 1.0 / (lv - 1.0);
        let r = round(c.r / s) * s;
        let g = round(c.g / s) * s;
        let b = round(c.b / s) * s;
        dst[y * w + x] = pack(vec4<f32>(r, g, b, c.a));
        return;
    }

    // ── Mode 5: Sharpen (3x3 unsharp mask) ──
    if (params.mode == 5u) {
        let ix = i32(x);
        let iy = i32(y);
        var sum = vec3<f32>(0.0);
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                let sx = u32(clamp(ix + dx, 0, i32(w - 1u)));
                let sy = u32(clamp(iy + dy, 0, i32(h - 1u)));
                let c = unpack(src[sy * w + sx]);
                // Sharpen kernel: center=5, cross=-1, corner=0
                var kw: f32;
                if (dx == 0 && dy == 0) { kw = 5.0; }
                else if (dx == 0 || dy == 0) { kw = -1.0; }
                else { kw = 0.0; }
                sum = sum + c.rgb * kw;
            }
        }
        let c0 = unpack(src[y * w + x]);
        let sharp = max(params.brightness, 0.5);
        let rgb = mix(c0.rgb, sum, clamp(sharp - 0.5, 0.0, 1.0));
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c0.a));
        return;
    }

    // ── Mode 7: Emboss (directional) ──
    if (params.mode == 7u) {
        let ix = i32(x);
        let iy = i32(y);
        let c_tl = unpack(src[u32(clamp(iy - 1, 0, i32(h-1u))) * w + u32(clamp(ix - 1, 0, i32(w-1u)))]);
        let c_br = unpack(src[u32(clamp(iy + 1, 0, i32(h-1u))) * w + u32(clamp(ix + 1, 0, i32(w-1u)))]);
        let c0 = unpack(src[y * w + x]);
        let d = c_br.rgb - c_tl.rgb;
        let emboss = c0.rgb + d * params.brightness;
        dst[y * w + x] = pack(vec4<f32>(clamp(emboss, vec3<f32>(0.0), vec3<f32>(1.0)), c0.a));
        return;
    }

    // ── Mode 8: Edge detect (Sobel 3x3, no array indexing) ──
    if (params.mode == 8u) {
        let ix = i32(x);
        let iy = i32(y);
        var gx = vec3<f32>(0.0);
        var gy = vec3<f32>(0.0);
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                let sx = u32(clamp(ix + dx, 0, i32(w - 1u)));
                let sy = u32(clamp(iy + dy, 0, i32(h - 1u)));
                let c = unpack(src[sy * w + sx]);
                let gx_w: f32 = f32(dx) * select(-2.0, 2.0, abs(dy) == 0);
                let gy_w: f32 = f32(dy) * select(-2.0, 2.0, abs(dx) == 0);
                let corner_w = f32(dx) * f32(dy);
                gx = gx + c.rgb * (gx_w + corner_w * 0.0);
                gy = gy + c.rgb * (gy_w + corner_w * 0.0);
            }
        }
        let mag = sqrt(gx * gx + gy * gy);
        let c0 = unpack(src[y * w + x]);
        let blend = min(max(params.brightness, 0.0), 1.0);
        let rgb = mix(c0.rgb, clamp(mag, vec3<f32>(0.0), vec3<f32>(1.0)), blend);
        dst[y * w + x] = pack(vec4<f32>(rgb, c0.a));
        return;
    }

    // ── Mode 4: Color correction (brightness/contrast/saturation/hue) ──
    if (params.mode == 4u) {
        let c = unpack(src[y * w + x]);
        var rgb = c.rgb;
        // Brightness
        rgb = rgb + vec3<f32>(params.brightness);
        // Contrast
        let ct = params.contrast;
        rgb = (rgb - vec3<f32>(0.5)) * ct + vec3<f32>(0.5);
        // Saturation + Hue shift via HSV
        let hsv = rgb_to_hsv(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
        var h = hsv.x + params.hue_shift / 6.2831853;
        if (h < 0.0) { h = h + 1.0; }
        if (h > 1.0) { h = h - 1.0; }
        let s = clamp(hsv.y * params.saturation, 0.0, 1.0);
        rgb = hsv_to_rgb(vec3<f32>(h, s, hsv.z));
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return;
    }

    // ── Gaussian separable pass ──    // ── Gaussian separable pass ──
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
