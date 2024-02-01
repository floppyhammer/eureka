use crate::math::alignup_u32;
use crate::render::shader_maker::ShaderMaker;
use crate::render::vertex::VertexBuffer;
use crate::render::{InstanceRaw, RenderServer, Texture, TextureCache, TextureId};
use cgmath::{Vector2, Vector4};
use std::collections::HashMap;
use std::mem;
use wgpu::{BufferAddress, DynamicOffset, RenderPass, SamplerBindingType};

pub struct AtlasRenderResources {
    // Use dynamic offset.
    params_bind_group_layout: wgpu::BindGroupLayout,
    params_bind_group: Option<wgpu::BindGroup>,
    params_buffer: Option<wgpu::Buffer>,
    params_buffer_capacity: usize,

    // Use range for different atlas.
    instance_buffer: Option<wgpu::Buffer>,
    instance_buffer_capacity: usize,

    pub(crate) texture_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) texture_bind_group_cache: HashMap<TextureId, wgpu::BindGroup>,

    pub(crate) pipeline_cache: HashMap<AtlasMode, wgpu::RenderPipeline>,
}

impl AtlasRenderResources {
    pub(crate) fn new(render_server: &RenderServer) -> Self {
        let params_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("atlas params bind group layout"),
                });

        let texture_bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                            ty: wgpu::BindingType::Sampler {
                                0: SamplerBindingType::Filtering,
                            },
                            count: None,
                        },
                    ],
                    label: Some("atlas texture bind group layout"),
                });

        Self {
            params_bind_group_layout,
            params_bind_group: None,
            params_buffer: None,
            params_buffer_capacity: 0,
            instance_buffer: None,
            texture_bind_group_layout,
            texture_bind_group_cache: HashMap::new(),
            instance_buffer_capacity: 0,
            pipeline_cache: Default::default(),
        }
    }

    fn create_pipeline(
        &mut self,
        mode: AtlasMode,
        render_server: &RenderServer,
        shader_maker: &mut ShaderMaker,
    ) {
        if self.pipeline_cache.get(&mode).is_some() {
            return;
        }

        let device = &render_server.device;

        let pipeline_label = "atlas pipeline";

        let pipeline = {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("atlas pipeline layout"),
                bind_group_layouts: &[
                    &self.params_bind_group_layout,
                    &self.texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            let defs = if mode == AtlasMode::Text {
                vec!["TEXT"]
            } else {
                vec![]
            };

            // Shader descriptor, not a shader module yet.
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("atlas shader"),
                source: shader_maker
                    .make_shader(include_str!("../shaders/atlas.wgsl"), defs.as_slice())
                    .unwrap(),
            };
            let shader_module = device.create_shader_module(shader);

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(pipeline_label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[AtlasInstanceRaw::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: render_server.surface_config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip, // Has to be triangle strip.
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        self.pipeline_cache.insert(mode, pipeline);
    }
}

/// CPU data for drawing multiple sprites with an instanced draw call.
#[derive(Clone)]
pub struct AtlasInstance {
    pub(crate) position: Vector2<f32>,
    pub(crate) size: Vector2<f32>,
    // Region in the atlas.
    pub(crate) region: Vector4<f32>,
    // Tint color.
    pub(crate) color: Vector4<f32>,
}

#[derive(Clone)]
pub struct ExtractedAtlas {
    pub(crate) atlas: Atlas,
    pub(crate) view_size: Vector2<u32>,
}

/// GPU data.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AtlasInstanceRaw {
    position: [f32; 2],
    size: [f32; 2],
    region: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AtlasParamsUniform {
    camera_view_size: [f32; 2],
    atlas_size: [f32; 2],
}

#[derive(Default, Copy, Clone, Eq, Hash, PartialEq)]
pub(crate) enum AtlasMode {
    #[default]
    Sprite = 0x1,
    Text = 0x2,
}

/// Parameters for atlas drawing control.
impl AtlasParamsUniform {
    pub(crate) fn new(atlas_size: Vector2<u32>, camera_view_size: Vector2<u32>) -> Self {
        Self {
            camera_view_size: [camera_view_size.x as f32, camera_view_size.y as f32],
            atlas_size: [atlas_size.x as f32, atlas_size.y as f32],
        }
    }

    pub(crate) fn default() -> Self {
        Self {
            camera_view_size: [0.0, 0.0],
            atlas_size: [0.0, 0.0],
        }
    }
}

impl AtlasInstance {
    // CPU data format -> GPU data format.
    fn to_raw(&self) -> AtlasInstanceRaw {
        AtlasInstanceRaw {
            position: self.position.into(),
            size: self.size.into(),
            region: self.region.into(),
            color: self.color.into(),
        }
    }
}

impl VertexBuffer for AtlasInstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<AtlasInstance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance.
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance.
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct Atlas {
    pub(crate) texture: Option<TextureId>,
    pub(crate) instances: Vec<AtlasInstance>,

    pub(crate) texture_size: (u32, u32),

    pub(crate) mode: AtlasMode,
}

impl Atlas {
    pub fn new(texture: TextureId, texture_cache: &TextureCache) -> Self {
        let texture_size = texture_cache.get(texture).unwrap().size;

        Self {
            texture: Some(texture),
            instances: vec![],
            texture_size,
            mode: AtlasMode::Sprite,
        }
    }

    pub fn empty(
        texture_cache: &mut TextureCache,
        render_server: &RenderServer,
        size: (u32, u32),
    ) -> Self {
        let texture = Texture::empty(
            &render_server.device,
            &render_server.queue,
            texture_cache,
            size,
        )
        .unwrap();

        Self {
            texture: Some(texture),
            instances: vec![],
            texture_size: size,
            mode: AtlasMode::Sprite,
        }
    }
}

pub fn prepare_atlas(
    extracted: &Vec<ExtractedAtlas>,
    render_resources: &mut AtlasRenderResources,
    render_server: &RenderServer,
    texture_cache: &TextureCache,
    shader_maker: &mut ShaderMaker,
) {
    let device = &render_server.device;

    let atlas_count = extracted.len();

    let mut all_instances = vec![];
    for e in extracted {
        all_instances.extend(e.atlas.instances.clone());
    }

    let instance_count = all_instances.len();

    // Prepare the instance buffer.
    {
        // Reallocate the instance buffer.
        if render_resources.instance_buffer_capacity < instance_count
            || render_resources.instance_buffer.is_none()
        {
            render_resources.instance_buffer_capacity = instance_count;

            let instance_buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("atlas instance buffer (unique)"),
                size: (mem::size_of::<InstanceRaw>() * instance_count) as BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            render_resources.instance_buffer = Some(instance_buffer);
        }

        // Convert to GPU raw data.
        let instance_data = all_instances
            .iter()
            .map(AtlasInstance::to_raw)
            .collect::<Vec<_>>();

        // Write the instance buffer.
        render_server.queue.write_buffer(
            render_resources.instance_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }

    // Prepare the params uniform buffer.
    {
        let offset_limit = wgpu::Limits::downlevel_defaults().min_uniform_buffer_offset_alignment;
        let offset =
            alignup_u32(mem::size_of::<AtlasParamsUniform>() as u32, offset_limit) * offset_limit;

        if render_resources.params_buffer_capacity < atlas_count {
            render_resources.params_buffer_capacity = atlas_count;

            let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("atlas params uniform buffer (unique)"),
                size: (offset * instance_count as u32) as BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = render_server
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &render_resources.params_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            // See DynamicUniformBufferOffset.
                            size: Some(
                                wgpu::BufferSize::new(mem::size_of::<AtlasParamsUniform>() as u64)
                                    .unwrap(),
                            ),
                        }),
                    }],
                    label: Some("atlas params uniform bind group"),
                });

            render_resources.params_buffer_capacity = atlas_count;
            render_resources.params_buffer = Some(buffer);
            render_resources.params_bind_group = Some(bind_group);
        }

        let mut uniforms = Vec::new();

        for e in extracted {
            let atlas_params = AtlasParamsUniform::new(e.atlas.texture_size.into(), e.view_size);

            uniforms.push(atlas_params);
        }

        if (render_resources.params_buffer.is_some()) {
            // Consider align-up.
            let mut aligned_up_data = vec![0u8; offset as usize * instance_count];

            for i in 0..uniforms.len() {
                let slice = bytemuck::cast_slice(&uniforms[i..i + 1]);

                for j in 0..slice.len() {
                    aligned_up_data[i * offset as usize + j] = slice[j];
                }
            }

            render_server.queue.write_buffer(
                render_resources.params_buffer.as_ref().unwrap(),
                0,
                &aligned_up_data[..],
            );
        }
    }

    // Prepare texture bind groups.
    {
        for e in extracted {
            let texture_id = e.atlas.texture.unwrap();
            let texture = texture_cache.get(texture_id).unwrap();

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &render_resources.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&(texture.view)),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: None,
            });

            render_resources
                .texture_bind_group_cache
                .insert(texture_id, bind_group);
        }
    }

    // Prepare pipelines.
    {
        for e in extracted {
            render_resources.create_pipeline(e.atlas.mode, render_server, shader_maker);
        }
    }
}

pub fn render_atlas<'a, 'b: 'a>(
    atlases: &'b Vec<ExtractedAtlas>,
    render_resources: &'b AtlasRenderResources,
    render_pass: &mut RenderPass<'a>,
) {
    let mut instance_offset = 0u32;

    for i in 0..atlases.len() {
        let a = &atlases[i].atlas;

        let pipeline = render_resources.pipeline_cache.get(&a.mode);
        let texture_bind_group = render_resources
            .texture_bind_group_cache
            .get(a.texture.as_ref().unwrap());

        render_pass.set_pipeline(pipeline.unwrap());

        // Set instance vertex buffer.
        render_pass.set_vertex_buffer(
            0,
            render_resources.instance_buffer.as_ref().unwrap().slice(..),
        );

        // Set bind groups.
        render_pass.set_bind_group(
            0,
            &render_resources.params_bind_group.as_ref().unwrap(),
            &[(i * mem::size_of::<AtlasParamsUniform>()) as DynamicOffset],
        );
        render_pass.set_bind_group(1, &texture_bind_group.unwrap(), &[]);

        render_pass.draw(0..4, instance_offset..a.instances.len() as u32);

        instance_offset += a.instances.len() as u32;
    }
}

pub trait DrawAtlas<'a> {
    fn draw_atlas(
        &mut self,
        pipeline: &'a wgpu::RenderPipeline,
        instance_buffer: &'a wgpu::Buffer,
        instance_count: u32,
        texture_bind_group: &'a wgpu::BindGroup,
        atlas_params_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawAtlas<'b> for wgpu::RenderPass<'a>
where
    'b: 'a, // This means 'b must outlive 'a.
{
    fn draw_atlas(
        &mut self,
        pipeline: &'b wgpu::RenderPipeline,
        instance_buffer: &'a wgpu::Buffer,
        instance_count: u32,
        texture_bind_group: &'a wgpu::BindGroup,
        atlas_params_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_pipeline(&pipeline);

        // Set instance vertex buffer.
        self.set_vertex_buffer(0, instance_buffer.slice(..));

        // Set bind groups.
        self.set_bind_group(0, &atlas_params_bind_group, &[]);
        self.set_bind_group(1, &texture_bind_group, &[]);

        self.draw(0..4, 0..instance_count);
    }
}
