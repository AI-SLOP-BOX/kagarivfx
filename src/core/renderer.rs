use crate::core::effect_plugin::evaluate_effects;
use crate::core::timeline::{Composition, LayerType, ShapeType};

use std::sync::Arc;

// Helper matrix functions
#[allow(dead_code)]
fn mat4_identity() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[allow(dead_code)]
fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] =
                a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
        }
    }
    out
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
    }, // Top-Left
    Vertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
    }, // Bottom-Left
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
    }, // Bottom-Right
    Vertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
    }, // Top-Right
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsUniform {
    viewport_size: [f32; 2],
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LayerUniform {
    transform_matrix: [[f32; 4]; 4],
    color: [f32; 4],
    opacity: f32,
    layer_type: u32,
    shape_type: u32,

    effect_tint_enabled: u32,
    effect_tint_color: [f32; 4],
    effect_tint_intensity: f32,
    effect_blur_enabled: u32,
    effect_blur_radius: f32,

    effect_shadow_enabled: u32,
    effect_shadow_color: [f32; 4],
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
    effect_vignette_color: [f32; 4],
    blend_mode: u32,
    _padding_align: [f32; 11], // Align to 256 bytes
}

#[allow(dead_code)]
pub struct WgpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,

    layer_buffer: wgpu::Buffer,
    layer_bind_group: wgpu::BindGroup,

    texture_bind_group_layout: wgpu::BindGroupLayout,
    dummy_texture_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,

    // Target offscreen texture
    pub target_texture: Option<wgpu::Texture>,
    pub target_view: Option<wgpu::TextureView>,
    pub target_size: (u32, u32),
}

impl WgpuRenderer {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Shaders compile
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Buffers
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals Buffer"),
            size: std::mem::size_of::<GlobalsUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layer_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Layer Buffer"),
            size: (std::mem::size_of::<LayerUniform>() * 256) as u64, // Pre-allocate up to 256 layers
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind Group Layouts
        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("globals_bind_group_layout"),
            });

        let layer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true, // Enable dynamic uniform offsets
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(std::mem::size_of::<LayerUniform>() as u64)
                                .unwrap(),
                        ),
                    },
                    count: None,
                }],
                label: Some("layer_bind_group_layout"),
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
            label: Some("globals_bind_group"),
        });

        let layer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &layer_buffer,
                    offset: 0,
                    size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<LayerUniform>() as u64)
                            .unwrap(),
                    ),
                }),
            }],
            label: Some("layer_bind_group"),
        });

        // Create dummy texture for default binds
        let dummy_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dummy Texture"),
            size: dummy_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &dummy_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            dummy_size,
        );
        let dummy_texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let dummy_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&dummy_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("dummy_texture_bind_group"),
        });

        // Pipeline Layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &globals_bind_group_layout,
                    &layer_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            device,
            queue,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            globals_buffer,
            globals_bind_group,
            layer_buffer,
            layer_bind_group,
            texture_bind_group_layout,
            dummy_texture_bind_group,
            sampler,
            target_texture: None,
            target_view: None,
            target_size: (0, 0),
        }
    }

    /// Prepares/resizes the offscreen target texture if needed.
    /// Returns true if the texture was recreated.
    pub fn ensure_target_size(&mut self, width: u32, height: u32) -> bool {
        if self.target_size == (width, height) && self.target_texture.is_some() {
            return false;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Offscreen Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.target_texture = Some(texture);
        self.target_view = Some(view);
        self.target_size = (width, height);
        true
    }

    /// Renders the given composition at the specified frame, returning the texture view.
    pub fn render(&mut self, comp: &Composition, frame: u32) -> (&wgpu::TextureView, bool) {
        let (width, height) = (comp.width, comp.height);
        let recreated = self.ensure_target_size(width, height);

        // Update Globals Uniform
        let globals = GlobalsUniform {
            viewport_size: [width as f32, height as f32],
            _padding: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Create Command Encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let target_view = self.target_view.as_ref().unwrap();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, &self.globals_bind_group, &[]);

            // Viewport projection matrix:
            // Maps [0, width] to [-1, 1] on X, and [0, height] to [1, -1] on Y.
            let m_proj = [
                [2.0 / width as f32, 0.0, 0.0, 0.0],
                [0.0, -2.0 / height as f32, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0], // Column-major translation: -1 on x, 1 on y
            ];

            // Step 1: Pre-evaluate active layer transform matrices and effect properties
            let mut active_layers = Vec::new();
            let mut uniforms = Vec::new();

            for layer in &comp.layers {
                if !layer.is_active(frame) {
                    continue;
                }

                // Retrieve transform values at the current frame
                let pos = layer.transform.position.evaluate(frame);
                let scale = layer.transform.scale.evaluate(frame);
                let rotation = layer.transform.rotation.evaluate(frame);
                let opacity = layer.transform.opacity.evaluate(frame);

                // Default layer dimensions (solid size or fallback)
                let (layer_w, layer_h) = match &layer.layer_type {
                    LayerType::Solid { .. } => (1.0, 1.0),
                    LayerType::Image { .. } => (1.0, 1.0),
                    LayerType::Text { font_size, .. } => (1.0, *font_size as f32 * 10.0),
                    LayerType::Shape { .. } => (1.0, 1.0),
                    LayerType::Null => (0.0, 0.0),
                    LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
                    LayerType::Audio { .. } => (0.0, 0.0),
                };

                let anc = layer.transform.anchor_point.evaluate(frame);

                // Compute layer-to-world transformation matrix
                let m_size = [
                    [layer_w, 0.0, 0.0, 0.0],
                    [0.0, layer_h, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [layer_w * 0.5, layer_h * 0.5, 0.0, 1.0],
                ];

                let m_anc = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [-anc[0], -anc[1], 0.0, 1.0],
                ];

                let m_scale = [
                    [scale[0] / 100.0, 0.0, 0.0, 0.0],
                    [0.0, scale[1] / 100.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];

                let rad = rotation.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let m_rot = [
                    [cos_r, sin_r, 0.0, 0.0],
                    [-sin_r, cos_r, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ];

                let m_pos = [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [pos[0], pos[1], 0.0, 1.0],
                ];

                let m_model = mat4_mul(
                    m_pos,
                    mat4_mul(m_rot, mat4_mul(m_scale, mat4_mul(m_anc, m_size))),
                );

                // Total projection * model matrix
                let transform_matrix = mat4_mul(m_proj, m_model);

                // Prepare Layer Uniform details
                let (layer_type, shape_type, color) = match &layer.layer_type {
                    LayerType::Solid { color } => (0u32, 0u32, *color),
                    LayerType::Image { .. } => (1u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::Shape { shape_type, color } => {
                        let st = match shape_type {
                            ShapeType::Rectangle => 0u32,
                            ShapeType::Ellipse => 1u32,
                            ShapeType::Star => 2u32,
                            ShapeType::Polygon => 3u32,
                        };
                        (2u32, st, *color)
                    }
                    LayerType::Text { color, .. } => (3u32, 0u32, *color),
                    LayerType::Null => (4u32, 0u32, [0.0, 0.0, 0.0, 0.0]),
                    LayerType::PreComp { .. } => (5u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::Audio { .. } => (6u32, 0u32, [0.0, 0.0, 0.0, 0.0]),
                };

                // ── OBS-style plugin evaluation ──────────────────────────
                // Each effect is dispatched through the RenderEffectPlugin trait.
                // Adding a new effect type requires zero changes here.
                let ep = evaluate_effects(&layer.effects, frame);

                let layer_uniform = LayerUniform {
                    transform_matrix,
                    color,
                    opacity,
                    layer_type,
                    shape_type,
                    effect_tint_enabled: ep.tint_enabled,
                    effect_tint_color: ep.tint_color,
                    effect_tint_intensity: ep.tint_intensity,
                    effect_blur_enabled: ep.blur_enabled,
                    effect_blur_radius: ep.blur_radius,
                    effect_shadow_enabled: ep.shadow_enabled,
                    effect_shadow_color: ep.shadow_color,
                    effect_shadow_opacity: ep.shadow_opacity,
                    effect_shadow_direction: ep.shadow_direction,
                    effect_shadow_distance: ep.shadow_distance,
                    effect_shadow_softness: ep.shadow_softness,
                    effect_ca_enabled: ep.chromatic_enabled,
                    effect_ca_shift_r: ep.chromatic_shift_r,
                    effect_ca_shift_b: ep.chromatic_shift_b,
                    effect_ca_edge_falloff: ep.chromatic_edge_falloff,
                    effect_vignette_enabled: ep.vignette_enabled,
                    effect_vignette_intensity: ep.vignette_intensity,
                    effect_vignette_roundness: ep.vignette_roundness,
                    effect_vignette_feather: ep.vignette_feather,
                    effect_vignette_color: ep.vignette_color,
                    blend_mode: match layer.blend_mode {
                        crate::core::timeline::BlendMode::Normal => 0,
                        crate::core::timeline::BlendMode::Multiply => 1,
                        crate::core::timeline::BlendMode::Screen => 2,
                        crate::core::timeline::BlendMode::Overlay => 3,
                        crate::core::timeline::BlendMode::Add => 4,
                        crate::core::timeline::BlendMode::Darken => 5,
                        crate::core::timeline::BlendMode::Lighten => 6,
                    },
                    _padding_align: [0.0; 11],
                };


                uniforms.push(layer_uniform);
                active_layers.push(layer);
            }

            // Step 2: Upload all Layer Uniforms in a single GPU command write
            if !uniforms.is_empty() {
                let upload_len = uniforms.len().min(256);
                self.queue.write_buffer(
                    &self.layer_buffer,
                    0,
                    bytemuck::cast_slice(&uniforms[0..upload_len]),
                );
            }

            // Step 3: Draw active layers using dynamic offsets without CPU-GPU sync blockers
            for (i, _layer) in active_layers.iter().enumerate() {
                if i >= 256 {
                    break;
                }

                // Bind resources using dynamic uniform offset
                let dynamic_offset = (i * std::mem::size_of::<LayerUniform>()) as u32;
                render_pass.set_bind_group(1, &self.layer_bind_group, &[dynamic_offset]);

                // Texture binding (use dummy for solid/SDF shapes)
                render_pass.set_bind_group(2, &self.dummy_texture_bind_group, &[]);

                // Draw!
                render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        (self.target_view.as_ref().unwrap(), recreated)
    }
}
