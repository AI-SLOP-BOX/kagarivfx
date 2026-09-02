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
    mode: u32,
    angle: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    hue_shift: f32,
    param_f3: f32,
    param_f4: f32,
    param_f5: f32,
    param_f6: f32,
    param_f7: f32,
    param_f8: f32,
    _pad: f32,
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

fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
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

    // ── Mode 12: ColorTint ──
    if (params.mode == 12u) {
        let c = unpack(src[y * w + x]);
        let tint_rgb = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let intensity = clamp(params.hue_shift, 0.0, 1.0);
        let rgb = mix(c.rgb, tint_rgb, intensity);
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 13: DropShadow (offset + directional blur) ──
    if (params.mode == 13u) {
        let shadow_c = vec4<f32>(params.brightness, params.contrast, params.saturation, params.hue_shift);
        let dx = params.param_f3;
        let dy = params.param_f4;
        let blur_r = params.radius;
        var acc = vec4<f32>(0.0);
        var wsum = 0.0;
        let n = max(blur_r, 1u);
        for (var i: i32 = -i32(n); i <= i32(n); i = i + 1) {
            let sx = u32(clamp(i32(x) + i, 0, i32(w - 1u)));
            let sy = u32(clamp(i32(y) + i, 0, i32(h - 1u)));
            let c = unpack(src[sy * w + sx]);
            acc = acc + vec4<f32>(c.rgb * c.a, c.a);
            wsum = wsum + 1.0;
        }
        let inv = 1.0 / max(wsum, 0.0001);
        let blurred = acc * inv;
        let blur_a = clamp(blurred.a, 0.0, 1.0);
        let blur_rgb = select(vec3<f32>(0.0), blurred.rgb / max(blur_a, 0.0001), blur_a > 0.0001);
        let shadow_pixel = vec4<f32>(blur_rgb * shadow_c.a, blur_a * shadow_c.a);
        let c0 = unpack(src[y * w + x]);
        let osx = u32(clamp(i32(x) + i32(dx), 0, i32(w - 1u)));
        let osy = u32(clamp(i32(y) + i32(dy), 0, i32(h - 1u)));
        let original = unpack(src[osy * w + osx]);
        let out_a = clamp(original.a + shadow_pixel.a * (1.0 - original.a), 0.0, 1.0);
        var out_rgb: vec3<f32>;
        if (out_a > 0.0001) {
            out_rgb = (original.rgb * original.a + shadow_pixel.rgb * (1.0 - original.a)) / out_a;
        } else {
            out_rgb = vec3<f32>(0.0);
        }
        dst[y * w + x] = pack(vec4<f32>(out_rgb, out_a));
        return;
    }

    // ── Mode 14: Glow (threshold bright pixels + blur + composite) ──
    if (params.mode == 14u) {
        let c0 = unpack(src[y * w + x]);
        let threshold = params.param_f3;
        let intensity = params.param_f4;
        let lum = dot(c0.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        if (lum > threshold) {
            let blur_r = params.radius;
            var acc = vec4<f32>(0.0);
            var wsum = 0.0;
            let n = max(blur_r, 1u);
            for (var i: i32 = -i32(n); i <= i32(n); i = i + 1) {
                let sx = u32(clamp(i32(x) + i, 0, i32(w - 1u)));
                let sy = u32(clamp(i32(y) + i, 0, i32(h - 1u)));
                let c = unpack(src[sy * w + sx]);
                acc = acc + vec4<f32>(c.rgb * c.a, c.a);
                wsum = wsum + 1.0;
            }
            let inv = 1.0 / max(wsum, 0.0001);
            let blurred = acc * inv;
            let blur_a = clamp(blurred.a, 0.0, 1.0);
            let blur_rgb = select(vec3<f32>(0.0), blurred.rgb / max(blur_a, 0.0001), blur_a > 0.0001);
            let glow = blur_rgb * intensity;
            let out_rgb = clamp(c0.rgb + glow, vec3<f32>(0.0), vec3<f32>(1.0));
            dst[y * w + x] = pack(vec4<f32>(out_rgb, c0.a));
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return;
    }

    // ── Mode 15: Levels (input range + gamma + output range) ──
    if (params.mode == 15u) {
        let c = unpack(src[y * w + x]);
        let in_black = params.brightness;
        let in_white = params.contrast;
        let gamma = max(params.saturation, 0.01);
        let out_black = params.hue_shift;
        let out_white = params.param_f3;
        let range = max(in_white - in_black, 0.001);
        let t = clamp((c.r - in_black) / range, 0.0, 1.0);
        let r = mix(out_black, out_white, pow(t, 1.0 / gamma));
        let tg = clamp((c.g - in_black) / range, 0.0, 1.0);
        let g = mix(out_black, out_white, pow(tg, 1.0 / gamma));
        let tb = clamp((c.b - in_black) / range, 0.0, 1.0);
        let b = mix(out_black, out_white, pow(tb, 1.0 / gamma));
        dst[y * w + x] = pack(vec4<f32>(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), c.a));
        return;
    }

    // ── Mode 16: Hue/Saturation/Lightness ──
    if (params.mode == 16u) {
        let c = unpack(src[y * w + x]);
        let hsv = rgb_to_hsv(c.rgb);
        var h = hsv.x + params.brightness / 6.2831853;
        if (h < 0.0) { h = h + 1.0; }
        if (h > 1.0) { h = h - 1.0; }
        let s = clamp(hsv.y * (1.0 + params.contrast), 0.0, 1.0);
        let v = clamp(hsv.z + params.saturation, 0.0, 1.0);
        let rgb = hsv_to_rgb(vec3<f32>(h, s, v));
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 17: Offset (wrap) ──
    if (params.mode == 17u) {
        let ox = i32(x) + i32(params.param_f3);
        let oy = i32(y) + i32(params.param_f4);
        let sx = u32(((ox % i32(w) + i32(w)) % i32(w)));
        let sy = u32(((oy % i32(h) + i32(h)) % i32(h)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 18: Twirl ──
    if (params.mode == 18u) {
        let cx = params.param_f5;
        let cy = params.param_f6;
        let strength = params.brightness;
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let dist = sqrt(dx * dx + dy * dy);
        let max_r = max(f32(min(w, h)) * 0.5, 1.0);
        let t = clamp(1.0 - dist / max_r, 0.0, 1.0);
        let angle = strength * t * t;
        let cos_a = cos(angle);
        let sin_a = sin(angle);
        let rx = dx * cos_a - dy * sin_a + cx;
        let ry = dx * sin_a + dy * cos_a + cy;
        let sx = u32(clamp(i32(rx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(ry), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 19: Bulge ──
    if (params.mode == 19u) {
        let cx = params.param_f5;
        let cy = params.param_f6;
        let strength = params.brightness;
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let dist = sqrt(dx * dx + dy * dy);
        let max_r = max(f32(min(w, h)) * 0.5, 1.0);
        let t = clamp(dist / max_r, 0.0, 1.0);
        let scale = 1.0 + strength * (1.0 - t * t);
        let rx = cx + dx / scale;
        let ry = cy + dy / scale;
        let sx = u32(clamp(i32(rx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(ry), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 20: Spherize ──
    if (params.mode == 20u) {
        let cx = params.param_f5;
        let cy = params.param_f6;
        let strength = params.brightness;
        let dx = (f32(x) - cx) / (f32(w) * 0.5);
        let dy = (f32(y) - cy) / (f32(h) * 0.5);
        let d2 = dx * dx + dy * dy;
        let r2 = sqrt(d2);
        if (r2 < 1.0) {
            let z = sqrt(1.0 - d2);
            let rx = cx + dx * z * strength * f32(w) * 0.5;
            let ry = cy + dy * z * strength * f32(h) * 0.5;
            let sx = u32(clamp(i32(rx), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(ry), 0, i32(h - 1u)));
            dst[y * w + x] = src[sy * w + sx];
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return;
    }

    // ── Mode 21: Wave Warp ──
    if (params.mode == 21u) {
        let amplitude = params.brightness;
        let frequency = params.contrast;
        let phase = params.angle;
        let direction = params.param_f5;
        let dx = cos(direction);
        let dy = sin(direction);
        let d = f32(x) * dx + f32(y) * dy;
        let offset = sin(d * frequency + phase) * amplitude;
        let sx = u32(clamp(i32(f32(x) + offset * dy), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(f32(y) - offset * dx), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 22: Turbulent Displace ──
    if (params.mode == 22u) {
        let amplitude = params.brightness;
        let scale = max(params.contrast, 0.001);
        // Simple hash-based noise
        let nx = f32(x) * scale / f32(w);
        let ny = f32(y) * scale / f32(h);
        let ix = floor(nx);
        let iy = floor(ny);
        let fx = fract(nx);
        let fy = fract(ny);
        let n00 = hash2(vec2<f32>(ix, iy));
        let n10 = hash2(vec2<f32>(ix + 1.0, iy));
        let n01 = hash2(vec2<f32>(ix, iy + 1.0));
        let n11 = hash2(vec2<f32>(ix + 1.0, iy + 1.0));
        let nx_interp = mix(mix(n00, n10, fx), mix(n01, n11, fx), fy);
        let angle = nx_interp * 6.2831853;
        let disp = amplitude;
        let sx = u32(clamp(i32(f32(x) + cos(angle) * disp), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(f32(y) + sin(angle) * disp), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 23: Chromatic Aberration ──
    if (params.mode == 23u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let amount = params.brightness;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let dist2 = dx * dx + dy * dy;
        let offset = dist2 * amount;
        let r_x = u32(clamp(i32(f32(x) + dx * offset * 2.0), 0, i32(w - 1u)));
        let r_y = u32(clamp(i32(f32(y) + dy * offset * 2.0), 0, i32(h - 1u)));
        let g_x = x;
        let g_y = y;
        let b_x = u32(clamp(i32(f32(x) - dx * offset * 2.0), 0, i32(w - 1u)));
        let b_y = u32(clamp(i32(f32(y) - dy * offset * 2.0), 0, i32(h - 1u)));
        let r = unpack(src[r_y * w + r_x]).r;
        let g = unpack(src[g_y * w + g_x]).g;
        let b = unpack(src[b_y * w + b_x]).b;
        let a = unpack(src[y * w + x]).a;
        dst[y * w + x] = pack(vec4<f32>(r, g, b, a));
        return;
    }

    // ── Mode 24: Vignette ──
    if (params.mode == 24u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let radius = params.brightness;
        let softness = max(params.contrast, 0.01);
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let v = smoothstep(radius, radius + softness, d);
        let c = unpack(src[y * w + x]);
        let rgb = c.rgb * (1.0 - v);
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 25: Minimax (dilate/erode) ──
    if (params.mode == 25u) {
        let radius = max(params.radius, 1u);
        let mode_flag = params.param_f5;
        var extreme: f32 = select(0.0, 1.0, mode_flag > 0.5);
        let c0 = unpack(src[y * w + x]);
        var result = c0;
        for (var dy: i32 = -i32(radius); dy <= i32(radius); dy++) {
            for (var dx: i32 = -i32(radius); dx <= i32(radius); dx++) {
                let sx = u32(clamp(i32(x) + dx, 0, i32(w - 1u)));
                let sy = u32(clamp(i32(y) + dy, 0, i32(h - 1u)));
                let c = unpack(src[sy * w + sx]);
                if (mode_flag > 0.5) {
                    result.r = max(result.r, c.r);
                    result.g = max(result.g, c.g);
                    result.b = max(result.b, c.b);
                } else {
                    result.r = min(result.r, c.r);
                    result.g = min(result.g, c.g);
                    result.b = min(result.b, c.b);
                }
            }
        }
        dst[y * w + x] = pack(result);
        return;
    }

    // ── Mode 26: LinearWipe ──
    if (params.mode == 26u) {
        let c0 = unpack(src[y * w + x]);
        let completion = clamp(params.brightness, 0.0, 1.0);
        let angle_rad = params.angle;
        let nx = cos(angle_rad);
        let ny = sin(angle_rad);
        let px = f32(x) / f32(w) - 0.5;
        let py = f32(y) / f32(h) - 0.5;
        let d = px * nx + py * ny + 0.5;
        let edge = completion;
        let feather = max(params.contrast, 0.001);
        let a = clamp((edge - d) / feather, 0.0, 1.0);
        dst[y * w + x] = pack(vec4<f32>(c0.rgb, c0.a * a));
        return;
    }

    // ── Mode 27: SimpleChoker (alpha expand/contract) ──
    if (params.mode == 27u) {
        let c0 = unpack(src[y * w + x]);
        let amount = params.brightness;
        var max_a: f32 = 0.0;
        let r = u32(abs(amount)) + 1u;
        for (var dy: i32 = -i32(r); dy <= i32(r); dy++) {
            for (var dx: i32 = -i32(r); dx <= i32(r); dx++) {
                let sx = u32(clamp(i32(x) + dx, 0, i32(w - 1u)));
                let sy = u32(clamp(i32(y) + dy, 0, i32(h - 1u)));
                let a = unpack(src[sy * w + sx]).a;
                max_a = max(max_a, a);
            }
        }
        let new_a = select(max_a, 1.0 - max_a, amount < 0.0);
        dst[y * w + x] = pack(vec4<f32>(c0.rgb, clamp(new_a, 0.0, 1.0)));
        return;
    }

    // ── Mode 28: ShiftChannels ──
    if (params.mode == 28u) {
        let c = unpack(src[y * w + x]);
        let take_r = u32(params.brightness);
        let take_g = u32(params.contrast);
        let take_b = u32(params.saturation);
        let take_a = u32(params.hue_shift);
        var channels = array<f32, 4>(c.r, c.g, c.b, c.a);
        let r = channels[clamp(take_r, 0u, 3u)];
        let g = channels[clamp(take_g, 0u, 3u)];
        let b = channels[clamp(take_b, 0u, 3u)];
        let a = channels[clamp(take_a, 0u, 3u)];
        dst[y * w + x] = pack(vec4<f32>(r, g, b, a));
        return;
    }

    // ── Mode 29: ColorBalance ──
    if (params.mode == 29u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let shadows_w = clamp(1.0 - lum * 2.0, 0.0, 1.0);
        let highlights_w = clamp(lum * 2.0 - 1.0, 0.0, 1.0);
        let midtones_w = 1.0 - shadows_w - highlights_w;
        let shift = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let rgb = c.rgb + shift * shadows_w + shift * midtones_w * 0.5 + shift * highlights_w;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return;
    }

    // ── Mode 30: Vibrance ──
    if (params.mode == 30u) {
        let c = unpack(src[y * w + x]);
        let amount = params.brightness;
        let hsv = rgb_to_hsv(c.rgb);
        let boost = amount * (1.0 - hsv.y);
        let s = clamp(hsv.y + boost, 0.0, 1.0);
        dst[y * w + x] = pack(vec4<f32>(hsv_to_rgb(vec3<f32>(hsv.x, s, hsv.z)), c.a));
        return;
    }

    // ── Mode 31: WhiteBalance ──
    if (params.mode == 31u) {
        let c = unpack(src[y * w + x]);
        let temperature = params.brightness;
        let tint = params.contrast;
        var rgb = c.rgb;
        rgb.r = rgb.r + temperature * 0.1;
        rgb.b = rgb.b - temperature * 0.1;
        rgb.g = rgb.g + tint * 0.05;
        rgb.r = rgb.r + tint * 0.05;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return;
    }

    // ── Mode 32: HslAdjust ──
    if (params.mode == 32u) {
        let c = unpack(src[y * w + x]);
        let hsv = rgb_to_hsv(c.rgb);
        var h = hsv.x + params.brightness / 360.0;
        if (h < 0.0) { h = h + 1.0; }
        if (h > 1.0) { h = h - 1.0; }
        let s = clamp(hsv.y * (1.0 + params.contrast), 0.0, 1.0);
        let v = clamp(hsv.z + params.saturation, 0.0, 1.0);
        dst[y * w + x] = pack(vec4<f32>(hsv_to_rgb(vec3<f32>(h, s, v)), c.a));
        return;
    }

    // ── Mode 33: CrtScanlines ──
    if (params.mode == 33u) {
        let c = unpack(src[y * w + x]);
        let spacing = max(params.brightness, 1.0);
        let intensity = params.contrast;
        let mask = select(1.0, 1.0 - intensity, (f32(y) % spacing) < 1.0);
        dst[y * w + x] = pack(vec4<f32>(c.rgb * mask, c.a));
        return;
    }

    // ── Mode 34: AlphaFromLuminance ──
    if (params.mode == 34u) {
        let c = unpack(src[y * w + x]);
        var lum = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        if (params.brightness > 0.5) { lum = 1.0 - lum; }
        dst[y * w + x] = pack(vec4<f32>(c.rgb, lum));
        return;
    }

    // ── Mode 35: LumaKeyRange ──
    if (params.mode == 35u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let low = params.brightness;
        let high = params.contrast;
        var key: f32;
        if (params.saturation > 0.5) {
            key = select(0.0, 1.0, lum < low || lum > high);
        } else {
            key = select(0.0, 1.0, lum >= low && lum <= high);
        }
        dst[y * w + x] = pack(vec4<f32>(c.rgb, c.a * key));
        return;
    }

    // ── Mode 36: VenetianBlinds ──
    if (params.mode == 36u) {
        let c0 = unpack(src[y * w + x]);
        let completion = clamp(params.brightness, 0.0, 1.0);
        let blind_w = max(params.contrast, 2.0);
        let pos = f32(y) % blind_w;
        let blind = step(blind_w * (1.0 - completion), pos);
        dst[y * w + x] = pack(vec4<f32>(c0.rgb, c0.a * blind));
        return;
    }

    // ── Mode 37: Tritone ──
    if (params.mode == 37u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let shadow = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let mid = vec3<f32>(params.hue_shift, params.param_f3, params.param_f4);
        let highlight = vec3<f32>(params.param_f5, params.param_f6, params.param_f7);
        var rgb: vec3<f32>;
        if (lum < 0.5) {
            rgb = mix(shadow, mid, lum * 2.0);
        } else {
            rgb = mix(mid, highlight, (lum - 0.5) * 2.0);
        }
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 38: GradientMap ──
    if (params.mode == 38u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let low = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let mid = vec3<f32>(params.hue_shift, params.param_f3, params.param_f4);
        let high = vec3<f32>(params.param_f5, params.param_f6, params.param_f7);
        var rgb: vec3<f32>;
        if (lum < 0.5) {
            rgb = mix(low, mid, lum * 2.0);
        } else {
            rgb = mix(mid, high, (lum - 0.5) * 2.0);
        }
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 39: Letterbox ──
    if (params.mode == 39u) {
        let frac = params.brightness;
        let py = f32(y) / f32(h);
        if (py < frac || py > 1.0 - frac) {
            dst[y * w + x] = pack(vec4<f32>(0.0, 0.0, 0.0, 1.0));
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return;
    }

    // ── Mode 40: CcLens ──
    if (params.mode == 40u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let convergence = params.brightness;
        let zoom = params.contrast;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = d / max_d;
        let new_d = pow(t, max(convergence / 100.0, 0.01)) * max_d * zoom;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 41: PolarCoordinates ──
    if (params.mode == 41u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let to_polar = params.brightness > 0.5;
        if (to_polar) {
            let dx = (f32(x) - cx) / cx;
            let dy = (f32(y) - cy) / cy;
            let r = sqrt(dx * dx + dy * dy);
            let theta = (atan2(dy, dx) + 3.14159265) / 6.2831853;
            let sx = u32(clamp(i32(theta * f32(w)), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(r * f32(h)), 0, i32(h - 1u)));
            dst[y * w + x] = src[sy * w + sx];
        } else {
            let theta = (f32(x) / f32(w) - 0.5) * 6.2831853;
            let r = f32(y) / f32(h);
            let sx = u32(clamp(i32(cx + cos(theta) * r * cx), 0, i32(w - 1u)));
            let sy = u32(clamp(i32(cy + sin(theta) * r * cy), 0, i32(h - 1u)));
            dst[y * w + x] = src[sy * w + sx];
        }
        return;
    }

    // ── Mode 42: OpticsCompensation ──
    if (params.mode == 42u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let fov = params.brightness;
        let reverse = params.contrast > 0.5;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d2 = dx * dx + dy * dy;
        let k = fov / 100.0;
        let distortion = 1.0 + k * d2;
        let dist = select(distortion, 1.0 / max(distortion, 0.001), reverse);
        let sx = u32(clamp(i32(cx + dx * dist * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * dist * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 43: Fisheye ──
    if (params.mode == 43u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let strength = params.brightness;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = d / max_d;
        let new_d = sin(t * 3.14159265 * 0.5 * strength) * max_d;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 44: LensCorrection ──
    if (params.mode == 44u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let k1 = params.brightness;
        let k2 = params.contrast;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let r2 = dx * dx + dy * dy;
        let r4 = r2 * r2;
        let distortion = 1.0 + k1 * r2 + k2 * r4;
        let sx = u32(clamp(i32(cx + dx * distortion * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * distortion * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 45: Vortex ──
    if (params.mode == 45u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let angle = params.brightness;
        let radius_param = max(params.contrast, 1.0);
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_r = f32(min(w, h)) * 0.5;
        let t = clamp(d / (max_r * radius_param / 100.0), 0.0, 1.0);
        let swirl = angle * t * t;
        let cos_a = cos(swirl);
        let sin_a = sin(swirl);
        let rx = dx * cos_a - dy * sin_a + cx;
        let ry = dx * sin_a + dy * cos_a + cy;
        let sx = u32(clamp(i32(rx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(ry), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 46: PinchPunch ──
    if (params.mode == 46u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let strength = params.brightness;
        let dx = (f32(x) - cx) / (f32(w) * 0.5);
        let dy = (f32(y) - cy) / (f32(h) * 0.5);
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = clamp(d / max_d, 0.0, 1.0);
        let new_d = pow(t, 1.0 + strength) * max_d;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * f32(w) * 0.5), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * f32(h) * 0.5), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 47: RefractionLens ──
    if (params.mode == 47u) {
        let cx = f32(w) * 0.5;
        let cy = f32(h) * 0.5;
        let ior = params.brightness;
        let dx = (f32(x) - cx) / cx;
        let dy = (f32(y) - cy) / cy;
        let d = sqrt(dx * dx + dy * dy);
        let max_d = sqrt(2.0);
        let t = clamp(d / max_d, 0.0, 1.0);
        let refracted = 1.0 / max(ior, 0.001);
        let new_d = t * refracted * max_d;
        let ratio = select(new_d / max(d, 0.0001), 0.0, d < 0.0001);
        let sx = u32(clamp(i32(cx + dx * ratio * cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy + dy * ratio * cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 48: BendIt ──
    if (params.mode == 48u) {
        let top_off = params.brightness;
        let bot_off = params.contrast;
        let py = f32(y) / f32(h);
        let offset = mix(top_off, bot_off, py);
        let sx = u32(clamp(i32(f32(x) + offset), 0, i32(w - 1u)));
        dst[y * w + x] = src[y * w + sx];
        return;
    }

    // ── Mode 49: Tiler ──
    if (params.mode == 49u) {
        let scale = max(params.brightness, 0.01);
        let tx = (f32(x) / f32(w)) * scale;
        let ty = (f32(y) / f32(h)) * scale;
        let sx = u32(clamp(i32(tx * f32(w)) % i32(w), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(ty * f32(h)) % i32(h), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 50: DirectionalSharpen ──
    if (params.mode == 50u) {
        let ix = i32(x);
        let iy = i32(y);
        let angle = params.brightness;
        let dx = cos(angle);
        let dy = sin(angle);
        let c0 = unpack(src[y * w + x]);
        let s1 = unpack(src[u32(clamp(iy + i32(dy), 0, i32(h-1u))) * w + u32(clamp(ix + i32(dx), 0, i32(w-1u)))]);
        let s2 = unpack(src[u32(clamp(iy - i32(dy), 0, i32(h-1u))) * w + u32(clamp(ix - i32(dx), 0, i32(w-1u)))]);
        let sharpen = max(params.contrast, 0.0);
        let rgb = c0.rgb + (c0.rgb * 2.0 - s1.rgb - s2.rgb) * sharpen;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c0.a));
        return;
    }

    // ── Mode 51: Halftone ──
    if (params.mode == 51u) {
        let c = unpack(src[y * w + x]);
        let cell = max(params.brightness, 2.0);
        let cx = (floor(f32(x) / cell) + 0.5) * cell;
        let cy = (floor(f32(y) / cell) + 0.5) * cell;
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let d = sqrt(dx * dx + dy * dy);
        let r = cell * 0.5 * sqrt(lum);
        let v = select(1.0, 0.0, d < r);
        dst[y * w + x] = pack(vec4<f32>(v, v, v, c.a));
        return;
    }

    // ── Mode 52: Mosaic ──
    if (params.mode == 52u) {
        let bw = max(params.brightness, 1.0);
        let bh = max(params.contrast, 1.0);
        let cx = (floor(f32(x) / bw) + 0.5) * bw;
        let cy = (floor(f32(y) / bh) + 0.5) * bh;
        let sx = u32(clamp(i32(cx), 0, i32(w - 1u)));
        let sy = u32(clamp(i32(cy), 0, i32(h - 1u)));
        dst[y * w + x] = src[sy * w + sx];
        return;
    }

    // ── Mode 53: CrossHatch ──
    if (params.mode == 53u) {
        let c = unpack(src[y * w + x]);
        let gap = max(params.brightness, 2.0);
        let threshold = params.contrast;
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        var v: f32 = 1.0;
        if (lum < threshold) {
            if ((f32(x) + f32(y)) % gap < 1.0) { v = 0.0; }
            if ((f32(x) - f32(y) + f32(h)) % gap < 1.0) { v = 0.0; }
        }
        dst[y * w + x] = pack(vec4<f32>(c.rgb * v, c.a));
        return;
    }

    // ── Mode 54: Colorama (luminance to cyclic palette) ──
    if (params.mode == 54u) {
        let c = unpack(src[y * w + x]);
        let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
        let h = lum + params.brightness;
        let rgb = hsv_to_rgb(vec3<f32>(fract(h), params.contrast, params.saturation));
        dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        return;
    }

    // ── Mode 55: BevelAlpha ──
    if (params.mode == 55u) {
        let ix = i32(x);
        let iy = i32(y);
        let depth = params.brightness;
        let angle = params.contrast;
        let a_l = unpack(src[y * w + u32(clamp(ix - 1, 0, i32(w-1u)))]).a;
        let a_r = unpack(src[y * w + u32(clamp(ix + 1, 0, i32(w-1u)))]).a;
        let a_t = unpack(src[u32(clamp(iy - 1, 0, i32(h-1u))) * w + x]).a;
        let a_b = unpack(src[u32(clamp(iy + 1, 0, i32(h-1u))) * w + x]).a;
        let dx = a_r - a_l;
        let dy = a_b - a_t;
        let light = cos(angle) * dx + sin(angle) * dy;
        let bevel = clamp(light * depth + 0.5, 0.0, 1.0);
        let c0 = unpack(src[y * w + x]);
        let rgb = mix(c0.rgb, vec3<f32>(bevel), abs(light) * depth);
        dst[y * w + x] = pack(vec4<f32>(rgb, c0.a));
        return;
    }

    // ── Mode 56: ChromaKey ──
    if (params.mode == 56u) {
        let c = unpack(src[y * w + x]);
        let key = vec3<f32>(params.brightness, params.contrast, params.saturation);
        let gain = params.hue_shift;
        let d = distance(c.rgb, key);
        let a = smoothstep(gain, gain + params.param_f3, d);
        dst[y * w + x] = pack(vec4<f32>(c.rgb, c.a * a));
        return;
    }

    // ── Mode 57: ColorSpaceConvert (RGB<->HSV) ──
    if (params.mode == 57u) {
        let c = unpack(src[y * w + x]);
        let mode = u32(params.brightness);
        if (mode == 0u) {
            let hsv = rgb_to_hsv(c.rgb);
            dst[y * w + x] = pack(vec4<f32>(hsv, c.a));
        } else {
            let rgb = hsv_to_rgb(c.rgb);
            dst[y * w + x] = pack(vec4<f32>(rgb, c.a));
        }
        return;
    }

    // ── Mode 58: FilmGrain ──
    if (params.mode == 58u) {
        let c = unpack(src[y * w + x]);
        let intensity = params.brightness;
        let seed = params.param_f5;
        let noise = hash2(vec2<f32>(f32(x) + seed, f32(y) + seed * 1.7)) * 2.0 - 1.0;
        let rgb = c.rgb + noise * intensity;
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a));
        return;
    }

    // ── Mode 59: FractalNoise ──
    if (params.mode == 59u) {
        let scale = max(params.brightness, 0.001);
        let evolution = params.contrast;
        let nx = f32(x) * scale / f32(w) + evolution;
        let ny = f32(y) * scale / f32(h) + evolution * 0.7;
        var v = 0.0;
        var amp = 0.5;
        var freq = 1.0;
        for (var i: u32 = 0u; i < 6u; i++) {
            v += amp * hash2(vec2<f32>(nx * freq, ny * freq));
            freq *= 2.0;
            amp *= 0.5;
        }
        let c0 = unpack(src[y * w + x]);
        let rgb = mix(c0.rgb, vec3<f32>(v), params.saturation);
        dst[y * w + x] = pack(vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c0.a));
        return;
    }

    // ── Mode 60: GlitchDisplacement ──
    if (params.mode == 60u) {
        let seed = params.param_f5;
        let amount = params.brightness;
        let block_h = 8.0;
        let row = floor(f32(y) / block_h);
        let h_val = hash2(vec2<f32>(row, seed));
        if (h_val > (1.0 - amount)) {
            let offset = i32((hash2(vec2<f32>(row + 1.0, seed)) - 0.5) * amount * f32(w) * 0.3);
            let sx = u32(clamp(i32(x) + offset, 0, i32(w - 1u)));
            dst[y * w + x] = src[y * w + sx];
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return;
    }

    // ── Mode 61: ScanlineGlitch ──
    if (params.mode == 61u) {
        let seed = params.param_f5;
        let amount = params.brightness;
        let h_val = hash2(vec2<f32>(f32(y), seed));
        if (h_val > (1.0 - amount * 0.3)) {
            let offset = i32((hash2(vec2<f32>(f32(y) + 3.0, seed)) - 0.5) * amount * f32(w) * 0.1);
            let sx = u32(clamp(i32(x) + offset, 0, i32(w - 1u)));
            dst[y * w + x] = src[y * w + sx];
        } else {
            dst[y * w + x] = src[y * w + x];
        }
        return;
    }

    // ── Mode 62: ReflectionMap ──
    if (params.mode == 62u) {
        let reflect_y = params.brightness > 0.5;
        let fade = params.contrast;
        var sy: u32;
        var fade_factor: f32;
        if (reflect_y) {
            sy = u32(clamp(i32(f32(h) - f32(y)), 0, i32(h - 1u)));
            fade_factor = 1.0 - fade * (f32(y) / f32(h));
        } else {
            sy = y;
            fade_factor = 1.0 - fade * (f32(x) / f32(w));
        }
        let c = unpack(src[sy * w + x]);
        dst[y * w + x] = pack(vec4<f32>(c.rgb * clamp(fade_factor, 0.0, 1.0), c.a));
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
