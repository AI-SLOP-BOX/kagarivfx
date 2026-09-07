use eframe::{egui, egui_wgpu};
use kagari_vfx::{ui, KagariApp};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).expect("usage: ui_snapshot output.png [width height]");
    let width: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1440);
    let height: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(900);
    let ctx = egui::Context::default();
    ui::theme::configure_ae_theme(&ctx);
    ui::icons::init_image_loaders(&ctx);
    let mut app = KagariApp::default();
    ui::demo_scene::build(&mut app);
    app.playback.is_playing = false;
    app.show_welcome = false;
    let mut frame = 30;
    let duration = app.history.current().active_composition().duration_frames;
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("GPU adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).expect("GPU device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
    let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [width, height], pixels_per_point: 1.0 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI snapshot"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    for _ in 0..4 {
        let output = ctx.run(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width as f32, height as f32))),
            ..Default::default()
        }, |ctx| {
            ui::menu::draw(&mut app, ctx);
            ui::toolbar::draw(&mut app, ctx);
            ui::timeline::draw(&mut app, ctx, &mut frame, duration);
            ui::inspector::draw(&mut app, ctx, &mut frame);
            ui::effects_library::draw(&mut app, ctx, &mut frame);
            ui::viewport::draw(&mut app, ctx, frame);
        });
        for (id, delta) in &output.textures_delta.set { renderer.update_texture(&device, &queue, *id, delta); }
        let jobs = ctx.tessellate(output.shapes, 1.0);
        let mut encoder = device.create_command_encoder(&Default::default());
        let commands = renderer.update_buffers(&device, &queue, &mut encoder, &jobs, &screen);
        let view = texture.create_view(&Default::default());
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None, color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
            });
            renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        queue.submit(commands.into_iter().chain(std::iter::once(encoder.finish())));
        for id in &output.textures_delta.free { renderer.free_texture(id); }
    }
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: stride as u64 * height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(texture.as_image_copy(), wgpu::ImageCopyBuffer {
        buffer: &buffer, layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(stride), rows_per_image: Some(height) },
    }, wgpu::Extent3d { width, height, depth_or_array_layers: 1 });
    queue.submit(Some(encoder.finish()));
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range();
    let pixels: Vec<u8> = data.chunks(stride as usize).flat_map(|row| row[..width as usize * 4].iter().copied()).collect();
    image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8).unwrap();
    println!("Saved {path}");
}
