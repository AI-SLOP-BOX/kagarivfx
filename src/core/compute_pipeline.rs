//! GPU Compute pipeline (wgpu): parallel per-pixel effects off the CPU.
//!
//! Architecture:
//! - Buffer-based ping-pong storage (core WebGPU only — no adapter-specific
//!   texture format features), portable across Metal / Vulkan / DX12.
//! - One lazily-initialized global [`GpuComputeContext`] guarded by a mutex;
//!   every call creates its own encoder so no command state is shared.
//! - Graceful headless degradation: [`global()`] returns `None` when no
//!   adapter exists, and callers fall back to the CPU path unchanged.
//!
//! Determinism: kernels are fixed-math (no atomics, no cooperative ops), so
//! output is byte-stable across runs on the same hardware. Cross-vendor
//! results may differ by ±1 LSB from the CPU reference, which is why GPU
//! effects are opt-in via [`set_gpu_effects_enabled`].

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const SHADER: &str = include_str!("compute_blur.wgsl");

/// Max supported blur radius (kernel buffer sized for 2*64+1 weights).
pub const MAX_BLUR_RADIUS: u32 = 64;

static GPU_EFFECTS_ENABLED: AtomicBool = AtomicBool::new(false);

/// Enable or disable GPU effect execution globally (default: disabled).
/// When disabled, all entry points return false immediately so callers
/// fall back to CPU implementations — preserving byte-determinism.
pub fn set_gpu_effects_enabled(on: bool) {
    GPU_EFFECTS_ENABLED.store(on, Ordering::Relaxed);
}

/// Whether GPU effects are currently enabled.
pub fn gpu_effects_enabled() -> bool {
    GPU_EFFECTS_ENABLED.load(Ordering::Relaxed)
}

struct Inner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    /// Cached ping-pong buffers keyed by byte length (recreated on resize).
    bufs: Option<BufSet>,
}

/// Last-run timing stats for the HUD (nanoseconds, atomic for cross-thread read).
pub struct GpuTimings {
    /// Total GPU wall time of the most recent blur call
    pub last_nanos: AtomicU64,
    /// Exponential moving average of blur wall time
    pub avg_nanos: AtomicU64,
    pub calls: AtomicU64,
}

pub static TIMINGS: GpuTimings = GpuTimings {
    last_nanos: AtomicU64::new(0),
    avg_nanos: AtomicU64::new(0),
    calls: AtomicU64::new(0),
};

fn record_timing(start: std::time::Instant) {
    let nanos = start.elapsed().as_nanos() as u64;
    TIMINGS.last_nanos.store(nanos, Ordering::Relaxed);
    let prev_avg = TIMINGS.avg_nanos.load(Ordering::Relaxed);
    let n_calls = TIMINGS.calls.fetch_add(1, Ordering::Relaxed);
    // EMA with alpha ~0.25 (or exact mean for early samples)
    let new_avg = if n_calls < 4 {
        (prev_avg * n_calls + nanos) / (n_calls + 1)
    } else {
        (prev_avg * 3 + nanos) / 4
    };
    TIMINGS.avg_nanos.store(new_avg, Ordering::Relaxed);
}

/// Human-readable HUD line, e.g. "GPU blur 1.2ms (avg 1.4ms, 12 calls)".
pub fn timing_hud_line() -> String {
    let last_us = TIMINGS.last_nanos.load(Ordering::Relaxed) / 1000;
    let avg_us = TIMINGS.avg_nanos.load(Ordering::Relaxed) / 1000;
    let calls = TIMINGS.calls.load(Ordering::Relaxed);
    if calls == 0 {
        "GPU compute: idle".to_string()
    } else {
        format!("GPU compute: {last_us}µs last, {avg_us}µs avg, {calls} calls")
    }
}

struct BufSet {
    len: u64,
    src: wgpu::Buffer,
    mid: wgpu::Buffer,
    dst: wgpu::Buffer,
    kernel: wgpu::Buffer,
    staging_out: wgpu::Buffer,
    params: wgpu::Buffer,
}

/// A live wgpu compute context. Obtain via [`global()`].
pub struct GpuComputeContext {
    inner: Mutex<Inner>,
    label: String,
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ParamsUniform {
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
}

impl GpuComputeContext {
    /// Request an adapter + device. Returns None when no suitable adapter
    /// exists (headless CI, missing drivers) — never panics.
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("aevfx-compute"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .ok()?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_blur.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gaussian_blur"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });
        let info = adapter.get_info();
        Some(Self {
            inner: Mutex::new(Inner {
                device,
                queue,
                pipeline,
                bufs: None,
            }),
            label: format!("{} ({})", info.name.trim(), info.backend.to_str()),
        })
    }

    pub fn backend_label(&self) -> &str {
        &self.label
    }

    /// Separable Gaussian blur over an RGBA8 buffer (straight alpha).
    /// Colors are premultiplied inside the shader, so transparency does not
    /// bleed into blurred edges.
    ///
    /// Flag-agnostic: callers gate execution via [`try_gpu_gaussian_blur`]
    /// or [`gpu_effects_enabled`] themselves, which keeps unit tests free
    /// of global-state races.
    pub fn gaussian_blur(&self, pixels: &mut [u8], width: u32, height: u32, radius: u32) -> bool {
        if pixels.is_empty() || width == 0 || height == 0 {
            return false;
        }
        self.gaussian_blur_inner(pixels, width, height, radius)
            .is_some()
    }

    fn gaussian_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        radius_in: u32,
    ) -> Option<()> {
        let _t = std::time::Instant::now();
        let radius = radius_in
            .min(MAX_BLUR_RADIUS)
            .min(width.max(1) / 2)
            .min(height.max(1) / 2);
        if radius == 0 {
            return Some(()); // nothing to do; caller's result already correct
        }
        let mut inner = self.inner.lock().ok()?;

        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }

        // (Re)allocate cached buffers when size changes
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;

        // Gaussian kernel (sigma = radius / 2, matching CPU reference feel)
        let sigma = (radius as f32 / 2.0).max(0.5);
        let mut weights = Vec::with_capacity((2 * radius + 1) as usize);
        for i in 0..=(2 * radius) {
            let x = i as f32 - radius as f32;
            weights.push((-0.5 * (x / sigma) * (x / sigma)).exp());
        }
        let ksum: f32 = weights.iter().sum();
        for wk in weights.iter_mut() {
            *wk /= ksum;
        }

        // Upload input + kernel + params
        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner
            .queue
            .write_buffer(&bufs.kernel, 0, bytemuck::cast_slice(&weights));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform {
                width,
                height,
                radius,
                mode: 0,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.5,
                param_f4: 8.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            }),
        );

        let bg_h = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.mid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_h"),
        });
        let bg_v = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.mid.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_v"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blur_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blur_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            // Horizontal: src -> mid
            pass.set_bind_group(0, &bg_h, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
            // Vertical: mid -> dst (flip direction flag)
            inner.queue.write_buffer(
                &bufs.params,
                0,
                bytemuck::bytes_of(&ParamsUniform {
                    width,
                    height,
                    radius,
                    mode: 2,
                    angle: 0.0,
                    brightness: 0.0,
                    contrast: 1.0,
                    saturation: 1.0,
                    hue_shift: 0.0,
                    param_f3: 0.5,
                    param_f4: 8.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
                }),
            );
            pass.set_bind_group(0, &bg_v, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

        // Block until done, then read back
        let (tx, rx) = std::sync::mpsc::channel();
        bufs.staging_out
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        inner.device.poll(wgpu::Maintain::Wait);
        rx.recv().ok()?.ok()?;
        {
            let data = bufs.staging_out.slice(..).get_mapped_range();
            if data.len() != pixels.len() {
                return None;
            }
            pixels.copy_from_slice(&data);
        }
        bufs.staging_out.unmap();
        record_timing(_t);
        Some(())
    }

    /// Directional (motion) blur: `taps` samples spread along `angle_deg`
    /// across a total length of `length_px`. Single dispatch.
    pub fn directional_blur(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        length_px: u32,
        angle_deg: f32,
    ) -> bool {
        if pixels.is_empty() || width == 0 || height == 0 || length_px < 2 {
            return false;
        }
        self.directional_blur_inner(pixels, width, height, length_px, angle_deg)
            .is_some()
    }

    fn directional_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        taps: u32,
        angle_deg: f32,
    ) -> Option<()> {
        use std::time::Instant;
        let t = Instant::now();
        let mut inner = self.inner.lock().ok()?;

        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }

        // Reuse the same cached buffer set (kernel unused in this mode)
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;

        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform {
                width,
                height,
                radius: taps.min(256),
                mode: 1,
                angle: angle_deg.to_radians(),
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.5,
                param_f4: 8.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            }),
        );

        let bg = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_dir"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("dir_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("dir_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

        let (tx, rx) = std::sync::mpsc::channel();
        bufs.staging_out
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        inner.device.poll(wgpu::Maintain::Wait);
        rx.recv().ok()?.ok()?;
        {
            let data = bufs.staging_out.slice(..).get_mapped_range();
            if data.len() != pixels.len() {
                return None;
            }
            pixels.copy_from_slice(&data);
        }
        bufs.staging_out.unmap();
        record_timing(t);
        Some(())
    }

    /// Radial zoom blur: `taps` samples along the ray from each pixel toward
    /// the frame center (zoom-blur look). Single dispatch.
    pub fn radial_blur(&self, pixels: &mut [u8], width: u32, height: u32, taps: u32) -> bool {
        if pixels.is_empty() || width == 0 || height == 0 || taps < 2 {
            return false;
        }
        self.radial_blur_inner(pixels, width, height, taps)
            .is_some()
    }

    fn radial_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        taps: u32,
    ) -> Option<()> {
        use std::time::Instant;
        let t = Instant::now();
        let mut inner = self.inner.lock().ok()?;

        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;

        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform {
                width,
                height,
                radius: taps.min(256),
                mode: 3,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.5,
                param_f4: 8.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            }),
        );

        let bg = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some("bg_radial"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("radial_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("radial_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

        let (tx, rx) = std::sync::mpsc::channel();
        bufs.staging_out
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                let _ = tx.send(res);
            });
        inner.device.poll(wgpu::Maintain::Wait);
        rx.recv().ok()?.ok()?;
        {
            let data = bufs.staging_out.slice(..).get_mapped_range();
            if data.len() != pixels.len() {
                return None;
            }
            pixels.copy_from_slice(&data);
        }
        bufs.staging_out.unmap();
        record_timing(t);
        Some(())
    }

    fn dispatch_fx(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        params: ParamsUniform,
        label: &str,
    ) -> bool {
        self.dispatch_fx_inner(pixels, width, height, params, label)
            .is_some()
    }

    fn dispatch_fx_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        params: ParamsUniform,
        label: &str,
    ) -> Option<()> {
        use std::time::Instant;
        let t = Instant::now();
        let mut inner = self.inner.lock().ok()?;
        let buf_len = (width as u64) * (height as u64) * 4;
        if buf_len > inner.device.limits().max_storage_buffer_binding_size as u64 {
            return None;
        }
        if inner.bufs.as_ref().is_none_or(|b| b.len != buf_len) {
            let fresh = {
                let dev = &inner.device;
                self.alloc_buf_set(dev, buf_len)
            };
            inner.bufs = Some(fresh);
        }
        let bufs = inner.bufs.as_ref()?;
        inner
            .queue
            .write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner
            .queue
            .write_buffer(&bufs.params, 0, bytemuck::bytes_of(&params));
        let bg = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs.params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bufs.src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bufs.dst.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bufs.kernel.as_entire_binding(),
                },
            ],
            label: Some(label),
        });
        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);
        let mut encoder = inner
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("fx_enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fx_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&inner.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));
        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(b) = inner.bufs.as_ref() {
            b.staging_out
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
        }
        inner.device.poll(wgpu::Maintain::Wait);
        let _ = rx.recv();
        if let Some(bufs) = inner.bufs.as_ref() {
            let data = bufs.staging_out.slice(..).get_mapped_range();
            if data.len() == pixels.len() {
                pixels.copy_from_slice(&data);
            }
            bufs.staging_out.unmap();
        }
        record_timing(t);
        Some(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn gpu_color_correct(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        br: f32,
        ct: f32,
        sat: f32,
        hue: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 4,
                angle: 0.0,
                brightness: br,
                contrast: ct,
                saturation: sat,
                hue_shift: hue,
                param_f3: 0.0,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_cc",
        )
    }
    pub fn gpu_sharpen(&self, pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 5,
                angle: 0.0,
                brightness: strength,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_sharp",
        )
    }
    pub fn gpu_threshold(&self, pixels: &mut [u8], w: u32, h: u32, cutoff: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 6,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: cutoff,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_thresh",
        )
    }
    pub fn gpu_emboss(&self, pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 7,
                angle: 0.0,
                brightness: strength,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_emboss",
        )
    }
    pub fn gpu_edge_detect(&self, pixels: &mut [u8], w: u32, h: u32, blend: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 8,
                angle: 0.0,
                brightness: blend,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_edge",
        )
    }
    pub fn gpu_invert(&self, pixels: &mut [u8], w: u32, h: u32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 9,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_inv",
        )
    }
    pub fn gpu_solarize(&self, pixels: &mut [u8], w: u32, h: u32, threshold: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 10,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: threshold,
                param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_solar",
        )
    }
    pub fn gpu_posterize(&self, pixels: &mut [u8], w: u32, h: u32, levels: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 11,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: levels, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_post",
        )
    }

    pub fn gpu_color_tint(&self, pixels: &mut [u8], w: u32, h: u32, tint_rgb: [f32; 3], intensity: f32) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 12,
                angle: 0.0, brightness: tint_rgb[0], contrast: tint_rgb[1],
                saturation: tint_rgb[2], hue_shift: intensity,
                param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_tint",
        )
    }
    pub fn gpu_drop_shadow(
        &self, pixels: &mut [u8], w: u32, h: u32,
        color: [f32; 4], distance: f32, angle: f32, blur_r: u32,
    ) -> bool {
        let rad = angle.to_radians();
        let dx = distance * rad.sin();
        let dy = -distance * rad.cos();
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: blur_r.min(32), mode: 13,
                angle: 0.0, brightness: color[0], contrast: color[1],
                saturation: color[2], hue_shift: color[3],
                param_f3: dx, param_f4: dy, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_shadow",
        )
    }
    pub fn gpu_glow(
        &self, pixels: &mut [u8], w: u32, h: u32,
        threshold: f32, radius: u32, intensity: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: radius.min(32), mode: 14,
                angle: 0.0, brightness: 0.0, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: threshold, param_f4: intensity, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_glow",
        )
    }
    pub fn gpu_levels(
        &self, pixels: &mut [u8], w: u32, h: u32,
        in_black: f32, in_white: f32, gamma: f32, out_black: f32, out_white: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 15,
                angle: 0.0, brightness: in_black / 255.0, contrast: in_white / 255.0,
                saturation: gamma, hue_shift: out_black / 255.0,
                param_f3: out_white / 255.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_levels",
        )
    }
    pub fn gpu_hue_saturation(
        &self, pixels: &mut [u8], w: u32, h: u32,
        hue_shift: f32, saturation: f32, lightness: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 16,
                angle: 0.0, brightness: hue_shift, contrast: saturation,
                saturation: lightness, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_hsl",
        )
    }
    pub fn gpu_offset(
        &self, pixels: &mut [u8], w: u32, h: u32, shift_x: i32, shift_y: i32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 17,
                angle: 0.0, brightness: 0.0, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: shift_x as f32, param_f4: shift_y as f32,
                param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_offset",
        )
    }
    pub fn gpu_twirl(
        &self, pixels: &mut [u8], w: u32, h: u32, angle: f32, cx: f32, cy: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 18,
                angle: 0.0, brightness: angle, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: cx, param_f6: cy, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_twirl",
        )
    }
    pub fn gpu_bulge(
        &self, pixels: &mut [u8], w: u32, h: u32, strength: f32, cx: f32, cy: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 19,
                angle: 0.0, brightness: strength, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: cx, param_f6: cy, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_bulge",
        )
    }
    pub fn gpu_spherize(
        &self, pixels: &mut [u8], w: u32, h: u32, strength: f32, cx: f32, cy: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 20,
                angle: 0.0, brightness: strength, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: cx, param_f6: cy, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_spherize",
        )
    }
    pub fn gpu_wave_warp(
        &self, pixels: &mut [u8], w: u32, h: u32,
        amplitude: f32, frequency: f32, phase: f32, direction: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 21,
                angle: phase, brightness: amplitude, contrast: frequency,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: direction, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_wave",
        )
    }
    pub fn gpu_turbulent_displace(
        &self, pixels: &mut [u8], w: u32, h: u32, amplitude: f32, scale: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 22,
                angle: 0.0, brightness: amplitude, contrast: scale,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_turb",
        )
    }
    pub fn gpu_chromatic_aberration(
        &self, pixels: &mut [u8], w: u32, h: u32, amount: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 23,
                angle: 0.0, brightness: amount, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_chroma",
        )
    }
    pub fn gpu_vignette(
        &self, pixels: &mut [u8], w: u32, h: u32, radius: f32, softness: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: 0, mode: 24,
                angle: 0.0, brightness: radius, contrast: softness,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_vign",
        )
    }
    pub fn gpu_minimax(
        &self, pixels: &mut [u8], w: u32, h: u32, radius: u32, maximize: bool,
    ) -> bool {
        self.dispatch_fx(
            pixels, w, h,
            ParamsUniform {
                width: w, height: h, radius: radius.min(16), mode: 25,
                angle: 0.0, brightness: 0.0, contrast: 1.0,
                saturation: 1.0, hue_shift: 0.0,
                param_f3: 0.0, param_f4: 0.0,
                param_f5: if maximize { 1.0 } else { 0.0 }, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0,
            },
            "gpu_mini",
        )
    }
    pub fn gpu_linear_wipe(&self, p: &mut [u8], w: u32, h: u32, completion: f32, angle: f32, feather: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 26, angle, brightness: completion, contrast: feather, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_lwipe")
    }
    pub fn gpu_shift_channels(&self, p: &mut [u8], w: u32, h: u32, r: u32, g: u32, b: u32, a: u32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 28, angle: 0.0, brightness: r as f32, contrast: g as f32, saturation: b as f32, hue_shift: a as f32, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_shift")
    }
    pub fn gpu_color_balance(&self, p: &mut [u8], w: u32, h: u32, shift: [f32; 3]) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 29, angle: 0.0, brightness: shift[0], contrast: shift[1], saturation: shift[2], hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_cb")
    }
    pub fn gpu_vibrance(&self, p: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 30, angle: 0.0, brightness: amount, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_vib")
    }
    pub fn gpu_white_balance(&self, p: &mut [u8], w: u32, h: u32, temp: f32, tint: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 31, angle: 0.0, brightness: temp, contrast: tint, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_wb")
    }
    pub fn gpu_hsl_adjust(&self, p: &mut [u8], w: u32, h: u32, hue: f32, sat: f32, light: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 32, angle: 0.0, brightness: hue, contrast: sat, saturation: light, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_hsl")
    }
    pub fn gpu_crt_scanlines(&self, p: &mut [u8], w: u32, h: u32, spacing: f32, intensity: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 33, angle: 0.0, brightness: spacing, contrast: intensity, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_crt")
    }
    pub fn gpu_alpha_from_luminance(&self, p: &mut [u8], w: u32, h: u32, invert: bool) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 34, angle: 0.0, brightness: if invert { 1.0 } else { 0.0 }, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_afl")
    }
    pub fn gpu_luma_key_range(&self, p: &mut [u8], w: u32, h: u32, low: f32, high: f32, invert: bool) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 35, angle: 0.0, brightness: low, contrast: high, saturation: if invert { 1.0 } else { 0.0 }, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_lk")
    }
    pub fn gpu_venetian_blinds(&self, p: &mut [u8], w: u32, h: u32, completion: f32, width: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 36, angle: 0.0, brightness: completion, contrast: width, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_vb")
    }
    pub fn gpu_tritone(&self, p: &mut [u8], w: u32, h: u32, shadow: [f32; 3], mid: [f32; 3], highlight: [f32; 3]) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 37, angle: 0.0, brightness: shadow[0], contrast: shadow[1], saturation: shadow[2], hue_shift: mid[0], param_f3: mid[1], param_f4: mid[2], param_f5: highlight[0], param_f6: highlight[1], param_f7: highlight[2], param_f8: 0.0, _pad: 0.0 }, "gpu_tri")
    }
    pub fn gpu_gradient_map(&self, p: &mut [u8], w: u32, h: u32, low: [f32; 3], mid: [f32; 3], high: [f32; 3]) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 38, angle: 0.0, brightness: low[0], contrast: low[1], saturation: low[2], hue_shift: mid[0], param_f3: mid[1], param_f4: mid[2], param_f5: high[0], param_f6: high[1], param_f7: high[2], param_f8: 0.0, _pad: 0.0 }, "gpu_gm")
    }
    pub fn gpu_letterbox(&self, p: &mut [u8], w: u32, h: u32, frac: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 39, angle: 0.0, brightness: frac, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_lb")
    }
    pub fn gpu_cc_lens(&self, p: &mut [u8], w: u32, h: u32, convergence: f32, zoom: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 40, angle: 0.0, brightness: convergence, contrast: zoom, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_lens")
    }
    pub fn gpu_polar_coords(&self, p: &mut [u8], w: u32, h: u32, to_polar: bool) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 41, angle: 0.0, brightness: if to_polar { 1.0 } else { 0.0 }, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_polar")
    }
    pub fn gpu_optics_comp(&self, p: &mut [u8], w: u32, h: u32, fov: f32, reverse: bool) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 42, angle: 0.0, brightness: fov, contrast: if reverse { 1.0 } else { 0.0 }, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_oc")
    }
    pub fn gpu_fisheye(&self, p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 43, angle: 0.0, brightness: strength, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_fish")
    }
    pub fn gpu_lens_correction(&self, p: &mut [u8], w: u32, h: u32, k1: f32, k2: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 44, angle: 0.0, brightness: k1, contrast: k2, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_lc")
    }
    pub fn gpu_vortex(&self, p: &mut [u8], w: u32, h: u32, angle: f32, radius: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 45, angle: 0.0, brightness: angle, contrast: radius, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_vtx")
    }
    pub fn gpu_pinch_punch(&self, p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 46, angle: 0.0, brightness: strength, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_pp")
    }
    pub fn gpu_refraction(&self, p: &mut [u8], w: u32, h: u32, ior: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 47, angle: 0.0, brightness: ior, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_refr")
    }
    pub fn gpu_bend_it(&self, p: &mut [u8], w: u32, h: u32, top: f32, bottom: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 48, angle: 0.0, brightness: top, contrast: bottom, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_bend")
    }
    pub fn gpu_tiler(&self, p: &mut [u8], w: u32, h: u32, scale: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 49, angle: 0.0, brightness: scale, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_tile")
    }
    pub fn gpu_directional_sharpen(&self, p: &mut [u8], w: u32, h: u32, angle: f32, strength: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 50, angle: 0.0, brightness: angle, contrast: strength, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_dsh")
    }
    pub fn gpu_halftone(&self, p: &mut [u8], w: u32, h: u32, cell_size: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 51, angle: 0.0, brightness: cell_size, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_ht")
    }
    pub fn gpu_mosaic(&self, p: &mut [u8], w: u32, h: u32, bw: f32, bh: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 52, angle: 0.0, brightness: bw, contrast: bh, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_mos")
    }
    pub fn gpu_crosshatch(&self, p: &mut [u8], w: u32, h: u32, gap: f32, threshold: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 53, angle: 0.0, brightness: gap, contrast: threshold, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_ch")
    }
    pub fn gpu_colorama(&self, p: &mut [u8], w: u32, h: u32, hue_offset: f32, sat: f32, light: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 54, angle: 0.0, brightness: hue_offset, contrast: sat, saturation: light, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_cma")
    }
    pub fn gpu_bevel_alpha(&self, p: &mut [u8], w: u32, h: u32, depth: f32, angle: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 55, angle: 0.0, brightness: depth, contrast: angle, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_bv")
    }
    pub fn gpu_chroma_key(&self, p: &mut [u8], w: u32, h: u32, key_rgb: [f32; 3], gain: f32, softness: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 56, angle: 0.0, brightness: key_rgb[0], contrast: key_rgb[1], saturation: key_rgb[2], hue_shift: gain, param_f3: softness, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_ck")
    }
    pub fn gpu_color_space_convert(&self, p: &mut [u8], w: u32, h: u32, mode: u32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 57, angle: 0.0, brightness: mode as f32, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_csc")
    }
    pub fn gpu_film_grain(&self, p: &mut [u8], w: u32, h: u32, intensity: f32, seed: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 58, angle: 0.0, brightness: intensity, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: seed, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_fg")
    }
    pub fn gpu_fractal_noise(&self, p: &mut [u8], w: u32, h: u32, scale: f32, evolution: f32, blend: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 59, angle: 0.0, brightness: scale, contrast: evolution, saturation: blend, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_fn")
    }
    pub fn gpu_glitch_displacement(&self, p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 60, angle: 0.0, brightness: amount, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: seed, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_gd")
    }
    pub fn gpu_scanline_glitch(&self, p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 61, angle: 0.0, brightness: amount, contrast: 1.0, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: seed, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_sg")
    }
    pub fn gpu_reflection_map(&self, p: &mut [u8], w: u32, h: u32, reflect_y: bool, fade: f32) -> bool {
        self.dispatch_fx(p, w, h, ParamsUniform { width: w, height: h, radius: 0, mode: 62, angle: 0.0, brightness: if reflect_y { 1.0 } else { 0.0 }, contrast: fade, saturation: 1.0, hue_shift: 0.0, param_f3: 0.0, param_f4: 0.0, param_f5: 0.0, param_f6: 0.0, param_f7: 0.0, param_f8: 0.0, _pad: 0.0 }, "gpu_rm")
    }

    fn alloc_buf_set(&self, dev: &wgpu::Device, buf_len: u64) -> BufSet {
        let any = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        BufSet {
            len: buf_len,
            src: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_src"),
                size: buf_len,
                usage: any,
                mapped_at_creation: false,
            }),
            mid: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_mid"),
                size: buf_len,
                usage: any,
                mapped_at_creation: false,
            }),
            dst: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_dst"),
                size: buf_len,
                usage: any,
                mapped_at_creation: false,
            }),
            kernel: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_kernel"),
                size: ((2 * MAX_BLUR_RADIUS + 1) * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            staging_out: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_staging"),
                size: buf_len,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            params: dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some("blur_params"),
                size: std::mem::size_of::<ParamsUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }
}

/// Minimal block-on executor (single future, parks the thread with a
/// proper waker so lost wakeups are impossible). Avoids pulling a full
/// async runtime into the core crate.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}

// ─── Global singleton ───────────────────────────────────────────────────────

static GLOBAL: OnceLock<Option<Arc<GpuComputeContext>>> = OnceLock::new();

/// Lazily initialize the global compute context. Returns None when the
/// machine has no usable adapter; the result is memoized either way.
pub fn global() -> Option<&'static Arc<GpuComputeContext>> {
    GLOBAL
        .get_or_init(|| GpuComputeContext::new().map(Arc::new))
        .as_ref()
}

/// Try running a GPU gaussian blur with the global context.
/// Returns false when disabled, unavailable, or unsupported size —
/// callers must fall back to the CPU implementation in that case.
pub fn try_gpu_gaussian_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    match global() {
        Some(ctx) => ctx.gaussian_blur(pixels, width, height, radius),
        None => false,
    }
}

/// Try running a GPU directional (motion) blur with the global context.
pub fn try_gpu_directional_blur(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    length_px: u32,
    angle_deg: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    match global() {
        Some(ctx) => ctx.directional_blur(pixels, width, height, length_px, angle_deg),
        None => false,
    }
}

/// Try running a GPU radial (zoom) blur with the global context.
pub fn try_gpu_radial_blur(pixels: &mut [u8], width: u32, height: u32, taps: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.radial_blur(pixels, width, height, taps))
        .unwrap_or(false)
}

pub fn try_gpu_color_correct(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    br: f32,
    ct: f32,
    sat: f32,
    hue: f32,
) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_color_correct(pixels, w, h, br, ct, sat, hue))
        .unwrap_or(false)
}
pub fn try_gpu_sharpen(pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_sharpen(pixels, w, h, strength))
        .unwrap_or(false)
}
pub fn try_gpu_threshold(pixels: &mut [u8], w: u32, h: u32, cutoff: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_threshold(pixels, w, h, cutoff))
        .unwrap_or(false)
}
pub fn try_gpu_emboss(pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_emboss(pixels, w, h, strength))
        .unwrap_or(false)
}
pub fn try_gpu_edge_detect(pixels: &mut [u8], w: u32, h: u32, blend: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_edge_detect(pixels, w, h, blend))
        .unwrap_or(false)
}
pub fn try_gpu_invert(pixels: &mut [u8], w: u32, h: u32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_invert(pixels, w, h))
        .unwrap_or(false)
}
pub fn try_gpu_solarize(pixels: &mut [u8], w: u32, h: u32, threshold: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_solarize(pixels, w, h, threshold))
        .unwrap_or(false)
}
pub fn try_gpu_posterize(pixels: &mut [u8], w: u32, h: u32, levels: f32) -> bool {
    if !gpu_effects_enabled() {
        return false;
    }
    global()
        .map(|c| c.gpu_posterize(pixels, w, h, levels))
        .unwrap_or(false)
}
pub fn try_gpu_color_tint(pixels: &mut [u8], w: u32, h: u32, tint_rgb: [f32; 3], intensity: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_color_tint(pixels, w, h, tint_rgb, intensity)).unwrap_or(false)
}
pub fn try_gpu_drop_shadow(
    pixels: &mut [u8], w: u32, h: u32,
    color: [f32; 4], distance: f32, angle: f32, blur_r: u32,
) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_drop_shadow(pixels, w, h, color, distance, angle, blur_r)).unwrap_or(false)
}
pub fn try_gpu_glow(
    pixels: &mut [u8], w: u32, h: u32,
    threshold: f32, radius: u32, intensity: f32,
) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_glow(pixels, w, h, threshold, radius, intensity)).unwrap_or(false)
}
pub fn try_gpu_levels(
    pixels: &mut [u8], w: u32, h: u32,
    in_black: f32, in_white: f32, gamma: f32, out_black: f32, out_white: f32,
) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_levels(pixels, w, h, in_black, in_white, gamma, out_black, out_white)).unwrap_or(false)
}
pub fn try_gpu_hue_saturation(
    pixels: &mut [u8], w: u32, h: u32,
    hue_shift: f32, saturation: f32, lightness: f32,
) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_hue_saturation(pixels, w, h, hue_shift, saturation, lightness)).unwrap_or(false)
}
pub fn try_gpu_offset(pixels: &mut [u8], w: u32, h: u32, shift_x: i32, shift_y: i32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_offset(pixels, w, h, shift_x, shift_y)).unwrap_or(false)
}
pub fn try_gpu_twirl(pixels: &mut [u8], w: u32, h: u32, angle: f32, cx: f32, cy: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_twirl(pixels, w, h, angle, cx, cy)).unwrap_or(false)
}
pub fn try_gpu_bulge(pixels: &mut [u8], w: u32, h: u32, strength: f32, cx: f32, cy: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_bulge(pixels, w, h, strength, cx, cy)).unwrap_or(false)
}
pub fn try_gpu_spherize(pixels: &mut [u8], w: u32, h: u32, strength: f32, cx: f32, cy: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_spherize(pixels, w, h, strength, cx, cy)).unwrap_or(false)
}
pub fn try_gpu_wave_warp(
    pixels: &mut [u8], w: u32, h: u32,
    amplitude: f32, frequency: f32, phase: f32, direction: f32,
) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_wave_warp(pixels, w, h, amplitude, frequency, phase, direction)).unwrap_or(false)
}
pub fn try_gpu_turbulent_displace(pixels: &mut [u8], w: u32, h: u32, amplitude: f32, scale: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_turbulent_displace(pixels, w, h, amplitude, scale)).unwrap_or(false)
}
pub fn try_gpu_chromatic_aberration(pixels: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_chromatic_aberration(pixels, w, h, amount)).unwrap_or(false)
}
pub fn try_gpu_vignette(pixels: &mut [u8], w: u32, h: u32, radius: f32, softness: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_vignette(pixels, w, h, radius, softness)).unwrap_or(false)
}
pub fn try_gpu_minimax(pixels: &mut [u8], w: u32, h: u32, radius: u32, maximize: bool) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_minimax(pixels, w, h, radius, maximize)).unwrap_or(false)
}
pub fn try_gpu_linear_wipe(p: &mut [u8], w: u32, h: u32, completion: f32, angle: f32, feather: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_linear_wipe(p, w, h, completion, angle, feather)).unwrap_or(false)
}
pub fn try_gpu_shift_channels(p: &mut [u8], w: u32, h: u32, r: u32, g: u32, b: u32, a: u32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_shift_channels(p, w, h, r, g, b, a)).unwrap_or(false)
}
pub fn try_gpu_color_balance(p: &mut [u8], w: u32, h: u32, shift: [f32; 3]) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_color_balance(p, w, h, shift)).unwrap_or(false)
}
pub fn try_gpu_vibrance(p: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_vibrance(p, w, h, amount)).unwrap_or(false)
}
pub fn try_gpu_white_balance(p: &mut [u8], w: u32, h: u32, temp: f32, tint: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_white_balance(p, w, h, temp, tint)).unwrap_or(false)
}
pub fn try_gpu_hsl_adjust(p: &mut [u8], w: u32, h: u32, hue: f32, sat: f32, light: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_hsl_adjust(p, w, h, hue, sat, light)).unwrap_or(false)
}
pub fn try_gpu_crt_scanlines(p: &mut [u8], w: u32, h: u32, spacing: f32, intensity: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_crt_scanlines(p, w, h, spacing, intensity)).unwrap_or(false)
}
pub fn try_gpu_alpha_from_luminance(p: &mut [u8], w: u32, h: u32, invert: bool) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_alpha_from_luminance(p, w, h, invert)).unwrap_or(false)
}
pub fn try_gpu_luma_key_range(p: &mut [u8], w: u32, h: u32, low: f32, high: f32, invert: bool) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_luma_key_range(p, w, h, low, high, invert)).unwrap_or(false)
}
pub fn try_gpu_venetian_blinds(p: &mut [u8], w: u32, h: u32, completion: f32, width: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_venetian_blinds(p, w, h, completion, width)).unwrap_or(false)
}
pub fn try_gpu_tritone(p: &mut [u8], w: u32, h: u32, shadow: [f32; 3], mid: [f32; 3], highlight: [f32; 3]) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_tritone(p, w, h, shadow, mid, highlight)).unwrap_or(false)
}
pub fn try_gpu_gradient_map(p: &mut [u8], w: u32, h: u32, low: [f32; 3], mid: [f32; 3], high: [f32; 3]) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_gradient_map(p, w, h, low, mid, high)).unwrap_or(false)
}
pub fn try_gpu_letterbox(p: &mut [u8], w: u32, h: u32, frac: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_letterbox(p, w, h, frac)).unwrap_or(false)
}
pub fn try_gpu_cc_lens(p: &mut [u8], w: u32, h: u32, convergence: f32, zoom: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_cc_lens(p, w, h, convergence, zoom)).unwrap_or(false)
}
pub fn try_gpu_polar_coords(p: &mut [u8], w: u32, h: u32, to_polar: bool) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_polar_coords(p, w, h, to_polar)).unwrap_or(false)
}
pub fn try_gpu_optics_comp(p: &mut [u8], w: u32, h: u32, fov: f32, reverse: bool) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_optics_comp(p, w, h, fov, reverse)).unwrap_or(false)
}
pub fn try_gpu_fisheye(p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_fisheye(p, w, h, strength)).unwrap_or(false)
}
pub fn try_gpu_lens_correction(p: &mut [u8], w: u32, h: u32, k1: f32, k2: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_lens_correction(p, w, h, k1, k2)).unwrap_or(false)
}
pub fn try_gpu_vortex(p: &mut [u8], w: u32, h: u32, angle: f32, radius: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_vortex(p, w, h, angle, radius)).unwrap_or(false)
}
pub fn try_gpu_pinch_punch(p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_pinch_punch(p, w, h, strength)).unwrap_or(false)
}
pub fn try_gpu_refraction(p: &mut [u8], w: u32, h: u32, ior: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_refraction(p, w, h, ior)).unwrap_or(false)
}
pub fn try_gpu_bend_it(p: &mut [u8], w: u32, h: u32, top: f32, bottom: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_bend_it(p, w, h, top, bottom)).unwrap_or(false)
}
pub fn try_gpu_tiler(p: &mut [u8], w: u32, h: u32, scale: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_tiler(p, w, h, scale)).unwrap_or(false)
}
pub fn try_gpu_directional_sharpen(p: &mut [u8], w: u32, h: u32, angle: f32, strength: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_directional_sharpen(p, w, h, angle, strength)).unwrap_or(false)
}
pub fn try_gpu_halftone(p: &mut [u8], w: u32, h: u32, cell_size: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_halftone(p, w, h, cell_size)).unwrap_or(false)
}
pub fn try_gpu_mosaic(p: &mut [u8], w: u32, h: u32, bw: f32, bh: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_mosaic(p, w, h, bw, bh)).unwrap_or(false)
}
pub fn try_gpu_crosshatch(p: &mut [u8], w: u32, h: u32, gap: f32, threshold: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_crosshatch(p, w, h, gap, threshold)).unwrap_or(false)
}
pub fn try_gpu_colorama(p: &mut [u8], w: u32, h: u32, hue_offset: f32, sat: f32, light: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_colorama(p, w, h, hue_offset, sat, light)).unwrap_or(false)
}
pub fn try_gpu_bevel_alpha(p: &mut [u8], w: u32, h: u32, depth: f32, angle: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_bevel_alpha(p, w, h, depth, angle)).unwrap_or(false)
}
pub fn try_gpu_chroma_key(p: &mut [u8], w: u32, h: u32, key_rgb: [f32; 3], gain: f32, softness: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_chroma_key(p, w, h, key_rgb, gain, softness)).unwrap_or(false)
}
pub fn try_gpu_color_space_convert(p: &mut [u8], w: u32, h: u32, mode: u32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_color_space_convert(p, w, h, mode)).unwrap_or(false)
}
pub fn try_gpu_film_grain(p: &mut [u8], w: u32, h: u32, intensity: f32, seed: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_film_grain(p, w, h, intensity, seed)).unwrap_or(false)
}
pub fn try_gpu_fractal_noise(p: &mut [u8], w: u32, h: u32, scale: f32, evolution: f32, blend: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_fractal_noise(p, w, h, scale, evolution, blend)).unwrap_or(false)
}
pub fn try_gpu_glitch_displacement(p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_glitch_displacement(p, w, h, amount, seed)).unwrap_or(false)
}
pub fn try_gpu_scanline_glitch(p: &mut [u8], w: u32, h: u32, amount: f32, seed: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_scanline_glitch(p, w, h, amount, seed)).unwrap_or(false)
}
pub fn try_gpu_reflection_map(p: &mut [u8], w: u32, h: u32, reflect_y: bool, fade: f32) -> bool {
    if !gpu_effects_enabled() { return false; }
    global().map(|c| c.gpu_reflection_map(p, w, h, reflect_y, fade)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The GPU enable flag is process-global; serialize flag-mutating tests.
    static FLAG_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_gpu_context_initializes_or_degrades_gracefully() {
        match GpuComputeContext::new() {
            Some(ctx) => {
                let label = ctx.backend_label();
                assert!(!label.is_empty());
            }
            None => {
                // Headless environments are valid; nothing to assert.
            }
        }
    }

    #[test]
    fn test_disabled_flag_short_circuits() {
        let _g = FLAG_LOCK.lock().unwrap();
        set_gpu_effects_enabled(false);
        let mut px = vec![128u8; 8 * 8 * 4];
        let before = px.clone();
        // Disabled flag gates the wrapper, not the context method.
        assert!(!try_gpu_gaussian_blur(&mut px, 8, 8, 2));
        assert_eq!(px, before, "disabled GPU must not touch the buffer");
        if global().is_some() {
            set_gpu_effects_enabled(true);
            let mut px2 = vec![128u8; 8 * 8 * 4];
            assert!(try_gpu_gaussian_blur(&mut px2, 8, 8, 2), "enabled must run");
            set_gpu_effects_enabled(false);
        }
    }

    #[test]
    fn test_blur_reduces_variance_and_preserves_size() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return, // headless: nothing to verify
        };
        set_gpu_effects_enabled(true);
        // Noise pattern
        let mut px: Vec<u8> = (0..16 * 16 * 4)
            .map(|i| {
                if i % 4 == 3 {
                    255
                } else {
                    ((i * 37) % 256) as u8
                }
            })
            .collect();
        let before_var = variance(&px);
        assert!(ctx.gaussian_blur(&mut px, 16, 16, 3));
        assert!(variance(&px) < before_var, "blur must smooth the image");
        set_gpu_effects_enabled(false);
    }

    #[test]
    fn test_blur_deterministic_same_hardware() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        let run = || {
            let mut px: Vec<u8> = (0..12 * 12 * 4)
                .map(|i| {
                    if i % 4 == 3 {
                        200
                    } else {
                        ((i * 91) % 256) as u8
                    }
                })
                .collect();
            assert!(ctx.gaussian_blur(&mut px, 12, 12, 2));
            px
        };
        assert_eq!(run(), run(), "GPU blur must be byte-stable across runs");
        set_gpu_effects_enabled(false);
    }

    #[test]
    fn test_directional_blur_smears_along_axis() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        // Single bright dot on transparent background; horizontal smear 8px
        let mut px = vec![0u8; 24 * 8 * 4];
        let dot = ((4 * 24 + 6) * 4) as usize;
        px[dot] = 255;
        px[dot + 1] = 255;
        px[dot + 2] = 255;
        px[dot + 3] = 255;
        assert!(ctx.directional_blur(&mut px, 24, 8, 9, 0.0));
        // Count lit pixels on the dot's row vs the row above
        let row_lit = (0..24)
            .filter(|x| px[((4 * 24 + x) * 4) as usize + 3] > 0)
            .count();
        let above_lit = (0..24)
            .filter(|x| px[((2 * 24 + x) * 4) as usize + 3] > 0)
            .count();
        assert!(
            row_lit >= 5,
            "horizontal smear must light multiple taps: {row_lit}"
        );
        assert!(above_lit <= row_lit / 2, "vertical bleed must stay small");
        set_gpu_effects_enabled(false);
    }

    #[test]
    fn test_timing_hud_reports_calls() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        let mut px = vec![100u8; 16 * 16 * 4];
        assert!(ctx.gaussian_blur(&mut px, 16, 16, 1));
        set_gpu_effects_enabled(false);
        let line = timing_hud_line();
        assert!(line.contains("GPU compute"), "hud: {line}");
        assert!(
            !line.contains("idle"),
            "after a call hud shows stats: {line}"
        );
    }

    #[test]
    fn test_radial_blur_preserves_center_and_smears_edge() {
        let _g = FLAG_LOCK.lock().unwrap();
        let ctx = match global() {
            Some(c) => c,
            None => return,
        };
        set_gpu_effects_enabled(true);
        // Center pixel stays put; an off-center dot smears toward the center
        let mut px = vec![0u8; 32 * 32 * 4];
        for (x, y) in [(16u32, 16u32), (28u32, 16u32)] {
            let i = ((y * 32 + x) * 4) as usize;
            px[i] = 255;
            px[i + 1] = 255;
            px[i + 2] = 255;
            px[i + 3] = 255;
        }
        assert!(ctx.radial_blur(&mut px, 32, 32, 8));
        // Center dot still present
        let center = ((16 * 32 + 16) * 4) as usize;
        assert!(px[center + 3] > 0, "center must remain");
        // Smear region between center and edge dot now lit
        let mid = ((16 * 32 + 22) * 4) as usize;
        assert!(px[mid + 3] > 0, "radial smear must fill toward center");
        set_gpu_effects_enabled(false);
    }

    fn variance(px: &[u8]) -> f64 {
        let vals: Vec<f64> = px.chunks_exact(4).map(|c| c[0] as f64).collect();
        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;
        vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n
    }
}
