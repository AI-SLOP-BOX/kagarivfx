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

use std::sync::atomic::{AtomicBool, Ordering};
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
    horizontal: u32,
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
        let (device, queue) =
            block_on(adapter.request_device(
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
            inner: Mutex::new(Inner { device, queue, pipeline, bufs: None }),
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
        self.gaussian_blur_inner(pixels, width, height, radius).is_some()
    }

    fn gaussian_blur_inner(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        radius_in: u32,
    ) -> Option<()> {
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
            let any = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC;
            inner.bufs = Some(BufSet {
                len: buf_len,
                src: inner.device.create_buffer(&wgpu::BufferDescriptor { label: Some("blur_src"), size: buf_len, usage: any, mapped_at_creation: false }),
                mid: inner.device.create_buffer(&wgpu::BufferDescriptor { label: Some("blur_mid"), size: buf_len, usage: any, mapped_at_creation: false }),
                dst: inner.device.create_buffer(&wgpu::BufferDescriptor { label: Some("blur_dst"), size: buf_len, usage: any, mapped_at_creation: false }),
                kernel: inner.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("blur_kernel"),
                    size: ((2 * MAX_BLUR_RADIUS + 1) * 4) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                staging_out: inner.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("blur_staging"),
                    size: buf_len,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                params: inner.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("blur_params"),
                    size: std::mem::size_of::<ParamsUniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            });
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
        inner.queue.write_buffer(&bufs.src, 0, bytemuck::cast_slice(pixels));
        inner.queue.write_buffer(&bufs.kernel, 0, bytemuck::cast_slice(&weights));
        inner.queue.write_buffer(
            &bufs.params,
            0,
            bytemuck::bytes_of(&ParamsUniform { width, height, radius, horizontal: 1 }),
        );

        let bg_h = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bufs.params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: bufs.src.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: bufs.mid.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: bufs.kernel.as_entire_binding() },
            ],
            label: Some("bg_h"),
        });
        let bg_v = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &inner.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bufs.params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: bufs.mid.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: bufs.dst.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: bufs.kernel.as_entire_binding() },
            ],
            label: Some("bg_v"),
        });

        let wg_x = width.div_ceil(8);
        let wg_y = height.div_ceil(8);

        let mut encoder = inner.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("blur_enc") });
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
                bytemuck::bytes_of(&ParamsUniform { width, height, radius, horizontal: 0 }),
            );
            pass.set_bind_group(0, &bg_v, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        encoder.copy_buffer_to_buffer(&bufs.dst, 0, &bufs.staging_out, 0, buf_len);
        inner.queue.submit(Some(encoder.finish()));

        // Block until done, then read back
        let (tx, rx) = std::sync::mpsc::channel();
        bufs.staging_out.slice(..).map_async(wgpu::MapMode::Read, move |res| {
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
        Some(())
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
    GLOBAL.get_or_init(|| GpuComputeContext::new().map(Arc::new)).as_ref()
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
            .map(|i| if i % 4 == 3 { 255 } else { ((i * 37) % 256) as u8 })
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
                .map(|i| if i % 4 == 3 { 200 } else { ((i * 91) % 256) as u8 })
                .collect();
            assert!(ctx.gaussian_blur(&mut px, 12, 12, 2));
            px
        };
        assert_eq!(run(), run(), "GPU blur must be byte-stable across runs");
        set_gpu_effects_enabled(false);
    }

    fn variance(px: &[u8]) -> f64 {
        let vals: Vec<f64> = px.chunks_exact(4).map(|c| c[0] as f64).collect();
        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;
        vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n
    }
}
