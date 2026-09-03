use std::sync::Arc;

use aftereffects_oss::core::renderer::WgpuRenderer;
use aftereffects_oss::core::timeline::{Composition, Layer, LayerType, TrackMatteMode};

fn request_gpu() -> Option<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("gpu-layer-capacity-test"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 6,
                ..wgpu::Limits::default()
            },
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .ok()?;
    Some((Arc::new(device), Arc::new(queue)))
}

#[test]
fn gpu_renderer_expands_layer_buffer_past_legacy_limit() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU capacity test: no adapter available");
        return;
    };

    let mut comp = Composition::new("gpu-capacity".into(), "GPU Capacity".into(), 16, 16, 30, 1);
    for index in 0..257 {
        comp.layers.push(Layer::new(
            format!("layer-{index}"),
            format!("Layer {index}"),
            LayerType::Solid {
                color: [0.1, 0.2, 0.3, 1.0],
            },
            1,
        ));
    }

    let mut renderer = WgpuRenderer::new(device, queue);
    let (_view, _recreated) = renderer.render(&comp, 0, 0.0, 0);
}

#[test]
fn gpu_track_matte_fallback_reuses_same_frame_cache() {
    let Some((device, queue)) = request_gpu() else {
        eprintln!("skipping GPU matte cache test: no adapter available");
        return;
    };

    let mut comp = Composition::new(
        "gpu-matte-cache".into(),
        "GPU Matte Cache".into(),
        16,
        16,
        30,
        1,
    );
    comp.layers.push(Layer::new(
        "matte".into(),
        "Matte".into(),
        LayerType::Solid { color: [1.0; 4] },
        1,
    ));
    let mut content = Layer::new(
        "content".into(),
        "Content".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        1,
    );
    content.track_matte = TrackMatteMode::AlphaMatte;
    comp.layers.push(content);

    let mut renderer = WgpuRenderer::new(device, queue);
    let (_first_view, first_recreated) = renderer.render(&comp, 0, 0.0, 0);
    let (_second_view, second_recreated) = renderer.render(&comp, 0, 0.0, 0);
    assert!(first_recreated);
    assert!(!second_recreated);
}
