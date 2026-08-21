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
    exposure_ev: f32,
    lut_mode: u32,
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

    // Levels Adjustment
    levels_enabled: u32,
    levels_in_black: f32,
    levels_in_white: f32,
    levels_gamma: f32,
    levels_out_black: f32,
    levels_out_white: f32,

    // Hue / Saturation
    huesat_enabled: u32,
    huesat_hue: f32,
    huesat_sat: f32,
    huesat_light: f32,

    // Glow / Bloom
    glow_enabled: u32,
    glow_threshold: f32,
    glow_radius: f32,
    glow_intensity: f32,
    glow_color: [f32; 4],

    // Physical Film Grain
    grain_enabled: u32,
    grain_intensity: f32,
    grain_size: f32,

    // Track Matte System
    track_matte_mode: u32,

    // Shape params: x = polygon/star point count, y = rectangle corner radius (px)
    shape_params: [f32; 4],

    // Mesh Warp / Corner Pin
    meshwarp_enabled: u32,
    corner_top_left: [f32; 2],
    corner_top_right: [f32; 2],
    corner_bottom_left: [f32; 2],
    corner_bottom_right: [f32; 2],

    _padding_align: [[f32; 4]; 10], // Align to 512 bytes (multiple of 256 for WGPU dynamic uniform offsets)
}

/// Bakes a text stroke into a rasterized text bitmap: dilates the fill alpha by the
/// stroke radius, colors it with stroke_color, and composites it behind the fill.
/// Returns padded (width, height, pixels) so the stroke is not clipped.
fn bake_text_stroke(
    pixels: &[u8],
    width: u32,
    height: u32,
    stroke_color: [f32; 4],
    stroke_width: f32,
) -> (u32, u32, Vec<u8>) {
    let radius = (stroke_width * 0.5).ceil().max(1.0) as i32;
    let pad = radius + 1;
    let (nw, nh) = (width + (pad * 2) as u32, height + (pad * 2) as u32);
    let mut out = vec![0u8; (nw * nh * 4) as usize];

    let sr = (stroke_color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let sg = (stroke_color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let sb = (stroke_color[2].clamp(0.0, 1.0) * 255.0) as u8;
    let stroke_a = stroke_color[3];

    let w = width as i32;
    let h = height as i32;
    for py in 0..nh as i32 {
        for px in 0..nw as i32 {
            let tx = px - pad;
            let ty = py - pad;
            let oidx = ((py as u32 * nw + px as u32) * 4) as usize;

            // Fill sample (offset back by pad)
            let fill_alpha = if tx >= 0 && ty >= 0 && tx < w && ty < h {
                let idx = ((ty * w + tx) * 4) as usize;
                pixels[idx + 3] as f32 / 255.0
            } else {
                0.0
            };

            // Stroke: max over neighbors within radius of fill alpha, feathered by distance
            let mut stroke_alpha = 0.0f32;
            if fill_alpha < 0.999 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = tx + dx;
                        let ny = ty + dy;
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                            if dist <= stroke_width * 0.5 {
                                let nidx = ((ny * w + nx) * 4) as usize;
                                let n_alpha = pixels[nidx + 3] as f32 / 255.0;
                                if n_alpha > 0.001 {
                                    let edge = (stroke_width * 0.5 - dist) / (stroke_width * 0.25).max(0.5);
                                    stroke_alpha = stroke_alpha.max(edge.clamp(0.0, 1.0));
                                }
                            }
                        }
                    }
                }
            }

            // Composite stroke behind fill (premultiplied-style over)
            let stroke_a_px = stroke_alpha * stroke_a;
            let out_a = fill_alpha + stroke_a_px * (1.0 - fill_alpha);
            if out_a > 0.001 {
                let fr = pixels.get(((ty.max(0) * w + tx.max(0)) * 4) as usize).copied().unwrap_or(0);
                let fg = pixels.get(((ty.max(0) * w + tx.max(0)) * 4 + 1) as usize).copied().unwrap_or(0);
                let fb = pixels.get(((ty.max(0) * w + tx.max(0)) * 4 + 2) as usize).copied().unwrap_or(0);
                // Stroke color behind fill color
                let mix_r = (sr as f32 * (1.0 - fill_alpha) + fr as f32 * fill_alpha) as u8;
                let mix_g = (sg as f32 * (1.0 - fill_alpha) + fg as f32 * fill_alpha) as u8;
                let mix_b = (sb as f32 * (1.0 - fill_alpha) + fb as f32 * fill_alpha) as u8;
                out[oidx] = mix_r;
                out[oidx + 1] = mix_g;
                out[oidx + 2] = mix_b;
                out[oidx + 3] = (out_a * 255.0) as u8;
            }
        }
    }
    (nw, nh, out)
}

struct TextRasterParams {
    text: String,
    font_size: u32,
    color: [f32; 4],
    font_family: String,
    tracking: f32,
    leading: f32,
    align: usize,
    stroke_color: [f32; 4],
    stroke_width: f32,
}

type RenderKey = (u64, u32, u32, u32, (u32, u32));
type TextTextureKey = (String, String, u32, [u32; 4], u32);
type TextTextureCache = std::collections::HashMap<TextTextureKey, (wgpu::Texture, std::sync::Arc<wgpu::BindGroup>, u32, u32)>;

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

    // Snapshot target offscreen texture
    pub snapshot_texture: Option<wgpu::Texture>,
    pub snapshot_view: Option<wgpu::TextureView>,

    // Dirty-checking: skip re-render when inputs are unchanged.
    // Keyed by (version, frame, exposure bits, lut, target size) per target type.
    last_main_key: Option<RenderKey>,
    last_snapshot_key: Option<RenderKey>,

    /// Optional cap on preview render width (px). When set, large compositions
    /// are rendered at a downscaled resolution — the viewport samples the
    /// texture at display size anyway, so this is visually near-free and can
    /// cut fill-rate by 4-16x on 4K comps.
    preview_max_width: Option<u32>,

    // GPU text rendering: cache of CPU-rasterized text textures keyed by (layer_id, text, font_size)
    text_texture_cache: std::cell::RefCell<TextTextureCache>,
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
            snapshot_texture: None,
            snapshot_view: None,
            text_texture_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
            last_main_key: None,
            last_snapshot_key: None,
            preview_max_width: None,
        }
    }

    /// Prepares/resizes the offscreen target texture if needed.
    /// Returns true if the texture was recreated.
    /// Rasterizes text on CPU and uploads it as a GPU texture, cached by (layer_id, text, font_size).
    /// Returns (width, height, bind_group) for the text texture, or None if rasterization fails.
    fn get_or_create_text_texture(
        &self,
        layer_id: &str,
        params: &TextRasterParams,
    ) -> Option<(u32, u32, std::sync::Arc<wgpu::BindGroup>)> {
        let (text, font_size, color, font_family, tracking, leading, align) =
            (params.text.as_str(), params.font_size, params.color, params.font_family.as_str(), params.tracking, params.leading, params.align);
        let (stroke_color, stroke_width) = (params.stroke_color, params.stroke_width);
        // Floats hashed via bit patterns (f32 is not Hash)
        let key = (
            layer_id.to_string(),
            text.to_string(),
            font_size,
            [stroke_color[0].to_bits(), stroke_color[1].to_bits(), stroke_color[2].to_bits(), stroke_color[3].to_bits()],
            stroke_width.to_bits(),
        );
        // Cached: return stored dimensions — no CPU rasterization on hits
        if let Some(bind_group) = self
            .text_texture_cache
            .borrow()
            .get(&key)
            .map(|(_, bg, w, h)| (bg.clone(), *w, *h))
        {
            let (bind_group, w, h) = bind_group;
            return Some((w, h, bind_group));
        }

        let alignment = match align {
            1 => crate::core::text_layout::TextAlign::Center,
            2 => crate::core::text_layout::TextAlign::Right,
            _ => crate::core::text_layout::TextAlign::Left,
        };
        let rasterized = crate::core::font_rasterizer::with_font_rasterizer(|r| {
            let family = r.resolve_family(font_family);
            r.rasterize_text_formatted(&family, text, font_size as f32, color, tracking, leading, 0.0, alignment)
        })?;
        if rasterized.0 == 0 || rasterized.1 == 0 || rasterized.2.is_empty() {
            return None;
        }
        // Bake stroke (if any) into the bitmap with padding
        let (tw, th, pixels) = if stroke_width > 0.1 {
            bake_text_stroke(&rasterized.2, rasterized.0, rasterized.1, stroke_color, stroke_width)
        } else {
            rasterized
        };

        let size = wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Layer Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(tw * 4),
                rows_per_image: Some(th),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
            label: Some("text_texture_bind_group"),
        });
        let bind_group = std::sync::Arc::new(bind_group);
        self.text_texture_cache
            .borrow_mut()
            .insert(key, (texture, bind_group.clone(), tw, th));
        Some((tw, th, bind_group))
    }

    pub fn ensure_target_size(&mut self, width: u32, height: u32) -> bool {        if self.target_size == (width, height) && self.target_texture.is_some() {
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

        let snap_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Renderer Snapshot Target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let snap_view = snap_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.target_texture = Some(texture);
        self.target_view = Some(view);
        self.snapshot_texture = Some(snap_texture);
        self.snapshot_view = Some(snap_view);
        self.target_size = (width, height);
        true
    }

    /// Internal core rendering implementation for both primary preview and snapshot target views.
    fn render_internal(&mut self, comp: &Composition, frame: u32, exposure_ev: f32, lut_mode: u32, target_snapshot: bool) -> bool {
        // Dirty-checking: the viewport calls render() at display refresh rate even
        // when nothing changed. Skip the full encode/upload/draw pass when the
        // project version, frame, exposure, LUT, and target size are unchanged.
        // Effective preview resolution: downscale large comps to the viewport cap.
        let (eff_w, eff_h) = match self.preview_max_width {
            Some(cap) if comp.width > cap => {
                let s = cap as f32 / comp.width as f32;
                (
                    cap.max(1),
                    ((comp.height as f32 * s) as u32).max(1),
                )
            }
            _ => (comp.width, comp.height),
        };

        let render_key: RenderKey = (
            crate::core::frame_cache::current_version(),
            frame,
            exposure_ev.to_bits(),
            lut_mode,
            (eff_w, eff_h),
        );
        let last_key = if target_snapshot { &self.last_snapshot_key } else { &self.last_main_key };
        if *last_key == Some(render_key) {
            return false; // nothing changed — reuse the existing target texture
        }

        // Clamp to both our sanity limit and the device's texture limit —
        // oversized textures would trip wgpu validation and abort the process.
        let max_dim = self.device.limits().max_texture_dimension_2d.min(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        let width = eff_w.clamp(1, max_dim);
        let height = eff_h.clamp(1, max_dim);
        let recreated = self.ensure_target_size(width, height);

        // Update Globals Uniform
        let globals = GlobalsUniform {
            viewport_size: [width as f32, height as f32],
            exposure_ev,
            lut_mode,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Per-layer text texture bind groups (declared before the render pass so they outlive it)
        let mut layer_textures: Vec<Option<std::sync::Arc<wgpu::BindGroup>>> = Vec::new();

        // Create Command Encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(if target_snapshot { "Snapshot Render Encoder" } else { "Render Encoder" }),
            });

        {
            let target_view = if target_snapshot {
                match &self.snapshot_view {
                    Some(view) => view,
                    None => return false,
                }
            } else {
                match &self.target_view {
                    Some(view) => view,
                    None => return false,
                }
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if target_snapshot { "Snapshot Render Pass" } else { "Render Pass" }),
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

                // Retrieve transform values at the current frame.
                // Parented layers and expression-driven layers resolve through the full
                // composition-aware path (position/scale/rotation/opacity inherit).
                let layer_has_exprs = layer.transform.position_expression.is_some()
                    || layer.transform.rotation_expression.is_some()
                    || layer.transform.scale_expression.is_some()
                    || layer.transform.opacity_expression.is_some();
                let (pos, scale, rotation, opacity) = if layer.parent_id.is_some() || layer_has_exprs {
                    let (p, s, r, o) = comp.resolve_world_transform(layer, frame);
                    (p, s, r, o / 100.0)
                } else {
                    (
                        layer.transform.position.evaluate(frame),
                        layer.transform.scale.evaluate(frame),
                        layer.transform.rotation.evaluate(frame),
                        layer.transform.opacity.evaluate(frame),
                    )
                };

                // Default layer dimensions (solid size or fallback)
                let (mut layer_w, mut layer_h) = match &layer.layer_type {
                    LayerType::Solid { .. } => (1.0, 1.0),
                    LayerType::Image { .. } => (1.0, 1.0),
                    LayerType::Text { font_size, .. } => (1.0, *font_size as f32 * 10.0), // Overridden below if text texture rasterization succeeds
                    LayerType::Shape { .. } => (1.0, 1.0),
                    LayerType::Null => (0.0, 0.0),
                    LayerType::PreComp { .. } => (comp.width as f32, comp.height as f32),
                    LayerType::AdjustmentLayer => (comp.width as f32, comp.height as f32),
                    LayerType::Audio { .. } => (0.0, 0.0),
                    LayerType::Particle { .. } => (comp.width as f32, comp.height as f32),
                };

                // GPU text rendering: rasterize text to a cached texture; if successful,
                // size the quad to the text bitmap and render via the image sampling path.
                let mut text_bind_group: Option<std::sync::Arc<wgpu::BindGroup>> = None;
                let mut is_textured_text = false;
                if let LayerType::Text { text, font_size, color, font_family, tracking, leading, align, stroke_color, stroke_width, .. } = &layer.layer_type {
                    let params = TextRasterParams {
                        text: text.clone(), font_size: *font_size, color: *color,
                        font_family: font_family.clone(), tracking: *tracking, leading: *leading, align: *align,
                        stroke_color: *stroke_color, stroke_width: *stroke_width,
                    };
                    if let Some((tw, th, bg)) = self.get_or_create_text_texture(&layer.id, &params) {
                        layer_w = tw as f32;
                        layer_h = th as f32;
                        text_bind_group = Some(bg);
                        is_textured_text = true;
                    }
                }
                layer_textures.push(text_bind_group);


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
                let transform_matrix = if layer.is_3d {
                    comp.resolve_world_transform_3d(layer, frame)
                } else {
                    mat4_mul(m_proj, m_model)
                };

                // Prepare Layer Uniform details
                let (mut layer_type, shape_type, mut color) = match &layer.layer_type {
                    LayerType::Solid { color } => (0u32, 0u32, *color),
                    LayerType::Image { .. } => (1u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::Shape { shape_type, color, .. } => {
                        let st = match shape_type {
                            ShapeType::Rectangle { .. } => 0u32,
                            ShapeType::Ellipse { .. } => 1u32,
                            ShapeType::Star { .. } => 2u32,
                            ShapeType::Polygon { .. } => 3u32,
                        };
                        (2u32, st, *color)
                    }
                    LayerType::Text { color, .. } => (3u32, 0u32, *color),                    LayerType::Null => (4u32, 0u32, [0.0, 0.0, 0.0, 0.0]),
                    LayerType::PreComp { .. } => (5u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::AdjustmentLayer => (7u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                    LayerType::Audio { .. } => (6u32, 0u32, [0.0, 0.0, 0.0, 0.0]),
                    LayerType::Particle { .. } => (8u32, 0u32, [1.0, 1.0, 1.0, 1.0]),
                };

                // Textured text uses the image sampling path with unmodified texture colors
                if is_textured_text {
                    layer_type = 1u32;
                    color = [1.0, 1.0, 1.0, 1.0];
                }


                let ep = evaluate_effects(&layer.effects, frame);

                // Shape parameters for GPU SDFs: polygon/star point count, rectangle corner radius
                let shape_params_eval: [f32; 4] = match &layer.layer_type {
                    LayerType::Shape { shape_type, .. } => match shape_type {
                        ShapeType::Polygon { sides, .. } => [sides.evaluate(frame), 0.0, 0.0, 0.0],
                        ShapeType::Star { points, .. } => [points.evaluate(frame), 0.0, 0.0, 0.0],
                        ShapeType::Rectangle { corner_radius, width, height, .. } => {
                            let cr = corner_radius.evaluate(frame);
                            let w = width.evaluate(frame).max(1.0);
                            let h = height.evaluate(frame).max(1.0);
                            // Normalize corner radius to 0..0.5 of the smaller half-size
                            [0.0, (cr / w.min(h)).clamp(0.0, 0.5), 0.0, 0.0]
                        }
                        _ => [0.0; 4],
                    },
                    _ => [0.0; 4],
                };

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
                        crate::core::timeline::BlendMode::SoftLight => 7,
                        crate::core::timeline::BlendMode::HardLight => 8,
                        crate::core::timeline::BlendMode::Difference => 9,
                        crate::core::timeline::BlendMode::Exclusion => 10,
                        crate::core::timeline::BlendMode::Divide => 11,
                        crate::core::timeline::BlendMode::Subtract => 12,
                    },
                    levels_enabled: ep.levels_enabled,
                    levels_in_black: ep.levels_in_black,
                    levels_in_white: ep.levels_in_white,
                    levels_gamma: ep.levels_gamma,
                    levels_out_black: ep.levels_out_black,
                    levels_out_white: ep.levels_out_white,
                    huesat_enabled: ep.huesat_enabled,
                    huesat_hue: ep.huesat_hue,
                    huesat_sat: ep.huesat_sat,
                    huesat_light: ep.huesat_light,
                    glow_enabled: ep.glow_enabled,
                    glow_threshold: ep.glow_threshold,
                    glow_radius: ep.glow_radius,
                    glow_intensity: ep.glow_intensity,
                    glow_color: ep.glow_color,
                    grain_enabled: ep.grain_enabled,
                    grain_intensity: ep.grain_intensity,
                    grain_size: ep.grain_size,
                    shape_params: shape_params_eval,
                    track_matte_mode: match layer.track_matte {
                        crate::core::timeline::TrackMatteMode::None => 0,
                        crate::core::timeline::TrackMatteMode::AlphaMatte => 1,
                        crate::core::timeline::TrackMatteMode::AlphaMatteInverted => 2,
                        crate::core::timeline::TrackMatteMode::LumaMatte => 3,
                        crate::core::timeline::TrackMatteMode::LumaMatteInverted => 4,
                    },
                    meshwarp_enabled: ep.meshwarp_enabled,
                    corner_top_left: ep.corner_top_left,
                    corner_top_right: ep.corner_top_right,
                    corner_bottom_left: ep.corner_bottom_left,
                    corner_bottom_right: ep.corner_bottom_right,
                    _padding_align: [[0.0; 4]; 10],
                };

                uniforms.push(layer_uniform);
                active_layers.push(layer);
            }

            // Step 2: Upload all Layer Uniforms in a single GPU command write
            if !uniforms.is_empty() {
                if uniforms.len() > 256 {
                    log::warn!(
                        "[WgpuRenderer] Active layer count ({}) exceeds 256 layer limit; extra layers will be truncated",
                        uniforms.len()
                    );
                }
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

                // Texture binding (use per-layer text texture when available, dummy for solid/SDF shapes)
                let tex_bg: &wgpu::BindGroup = match layer_textures.get(i) {
                    Some(Some(bg)) => bg,
                    _ => &self.dummy_texture_bind_group,
                };
                render_pass.set_bind_group(2, tex_bg, &[]);

                // Draw!
                render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Remember inputs so redundant renders can be skipped
        if target_snapshot {
            self.last_snapshot_key = Some(render_key);
        } else {
            self.last_main_key = Some(render_key);
        }
        recreated
    }

    /// Caps preview render width (px). `None` renders at composition resolution.
    pub fn set_preview_max_width(&mut self, cap: Option<u32>) {
        if self.preview_max_width != cap {
            self.preview_max_width = cap;
            // Resolution change must invalidate the dirty-check keys
            self.last_main_key = None;
            self.last_snapshot_key = None;
        }
    }

    /// Renders the given composition at the specified frame, returning the texture view.
    pub fn render(&mut self, comp: &Composition, frame: u32, exposure_ev: f32, lut_mode: u32) -> (&wgpu::TextureView, bool) {
        let recreated = self.render_internal(comp, frame, exposure_ev, lut_mode, false);
        if self.target_view.is_none() {
            log::error!("[WgpuRenderer] render(): target view missing; using fallback view");
            self.dummy_view_or_create(false);
        }
        (
            self.target_view.as_ref().expect("fallback view just created"),
            recreated,
        )
    }

    /// Renders the given composition at the specified frame to the snapshot target, returning the snapshot texture view.
    pub fn render_snapshot_frame(&mut self, comp: &Composition, frame: u32, exposure_ev: f32, lut_mode: u32) -> (&wgpu::TextureView, bool) {
        let recreated = self.render_internal(comp, frame, exposure_ev, lut_mode, true);
        if self.snapshot_view.is_none() {
            log::error!("[WgpuRenderer] render_snapshot_frame(): snapshot view missing; using fallback view");
            self.dummy_view_or_create(true);
        }
        (
            self.snapshot_view.as_ref().expect("fallback view just created"),
            recreated,
        )
    }

    /// Last-resort 1x1 fallback view so a missing target can never panic the UI thread.
    fn dummy_view_or_create(&mut self, snapshot: bool) -> &wgpu::TextureView {
        let slot = if snapshot { &mut self.snapshot_view } else { &mut self.target_view };
        if slot.is_none() {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Fallback 1x1 Target"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            *slot = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        slot.as_ref().unwrap()
    }
}

/// Helper to align dynamic uniform buffer byte offsets against WGPU hardware limits.
/// Dynamically adapts to device.limits().min_uniform_buffer_offset_alignment (e.g. 64B, 256B, 512B).
#[allow(dead_code)]
pub fn align_uniform_buffer_offset(offset: u64, alignment: u32) -> u64 {
    let align = alignment as u64;
    if align == 0 {
        return offset;
    }
    (offset + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_uniform_memory_alignment() {
        let size = std::mem::size_of::<LayerUniform>();
        assert_eq!(
            size % 256,
            0,
            "LayerUniform size ({} bytes) must be a multiple of 256 for WGPU dynamic uniform offset alignment",
            size
        );
        for align in [64, 256, 512] {
            assert_eq!(align_uniform_buffer_offset(size as u64, align) % align as u64, 0);
        }
    }
}
