use crate::render::camera::CameraUniform;
use crate::render::light::{
    ClusterConfig, LightUniform, MAX_SHADOWED_POINT_LIGHTS,
};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{
    standard_resources, FrameContext, Node, ResourceSpec, SamplerKey, TextureKey,
};
use std::any::Any;

pub struct VolumetricNode {
    pipeline: Option<wgpu::ComputePipeline>,
}

impl Default for VolumetricNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

pub(crate) const VOLUMETRIC_RESOLUTION: [u32; 3] = [240, 135, 64];

impl Node for VolumetricNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::camera::CameraUniform;
        use crate::render::light::{
            ClusterConfig, LightUniform, PointLightUniform, MAX_SHADOWED_POINT_LIGHTS,
        };
        use crate::render::Texture;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::light_uniform_buffer(),
                ResourceSpec::buffer(
                    size_of::<LightUniform>() as u64,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            .input(
                standard_resources::point_light_storage_buffer(),
                ResourceSpec::buffer(
                    (size_of::<PointLightUniform>() * 1024) as u64,
                    wgpu::BufferUsages::STORAGE,
                ),
            )
            .input(
                standard_resources::cluster_config_buffer(),
                ResourceSpec::buffer(
                    size_of::<ClusterConfig>() as u64,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            .input(
                standard_resources::point_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 512,
                    height: 512,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: (MAX_SHADOWED_POINT_LIGHTS * 6) as u32,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .input(
                standard_resources::directional_shadow_map(),
                ResourceSpec::Texture(TextureKey {
                    width: 2048,
                    height: 2048,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 3,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .input(
                standard_resources::shadow_cascade_buffer(),
                ResourceSpec::buffer(
                    size_of::<crate::render::light::CascadeUniform>() as u64,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            .output(
                standard_resources::volumetric_lighting_texture(),
                ResourceSpec::Texture(TextureKey {
                    width: VOLUMETRIC_RESOLUTION[0],
                    height: VOLUMETRIC_RESOLUTION[1],
                    layers: VOLUMETRIC_RESOLUTION[2],
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    dimension: wgpu::TextureDimension::D3,
                    mip_levels: 1,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;

        if self.pipeline.is_none() {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Volumetric Layout"),
                bind_group_layouts: &[&device.create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: true,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 3,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 4,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::StorageTexture {
                                    access: wgpu::StorageTextureAccess::WriteOnly,
                                    format: wgpu::TextureFormat::Rgba16Float,
                                    view_dimension: wgpu::TextureViewDimension::D3,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 5,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::CubeArray,
                                    sample_type: wgpu::TextureSampleType::Depth,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 6,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Sampler(
                                    wgpu::SamplerBindingType::Comparison,
                                ),
                                count: None,
                            },
                            // Directional Shadow
                            wgpu::BindGroupLayoutEntry {
                                binding: 7,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2Array,
                                    sample_type: wgpu::TextureSampleType::Depth,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 8,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                        ],
                        label: None,
                    },
                )],
                push_constant_ranges: &[],
            });

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Volumetric Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/volumetric.wgsl").into(),
                ),
            });

            self.pipeline = Some(device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Volumetric Pipeline"),
                    layout: Some(&layout),
                    module: &shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                },
            ));
        }

        let camera_buffer = context.buffer(&standard_resources::camera_buffer());
        let light_uniform = context.buffer(&standard_resources::light_uniform_buffer());
        let point_lights = context.buffer(&standard_resources::point_light_storage_buffer());
        let config_buffer = context.buffer(&standard_resources::cluster_config_buffer());
        let shadow_map = context.texture(&standard_resources::point_shadow_map());
        let dir_shadow_map = context.texture(&standard_resources::directional_shadow_map());
        let cascade_buffer = context.buffer(&standard_resources::shadow_cascade_buffer());
        let volumetric_tex = context.texture(&standard_resources::volumetric_lighting_texture());

        let shadow_sampler = context.get_sampler(SamplerKey {
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shadow_view = shadow_map.get_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::CubeArray),
            array_layer_count: Some(MAX_SHADOWED_POINT_LIGHTS as u32 * 6),
            ..Default::default()
        });

        let dir_shadow_view = dir_shadow_map.get_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            array_layer_count: Some(3),
            ..Default::default()
        });

        let volumetric_view = volumetric_tex.get_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.pipeline.as_ref().unwrap().get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_buffer.buffer,
                        offset: 0,
                        size: Some(
                            wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_uniform.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: point_lights.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: config_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&volumetric_view.0),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&shadow_view.0),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&dir_shadow_view.0),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: cascade_buffer.buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        let mut cpass = context
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Volumetric Pass"),
                timestamp_writes: None,
            });
        cpass.set_pipeline(self.pipeline.as_ref().unwrap());
        cpass.set_bind_group(0, &bind_group, &[0]);
        // 提升分辨率后的 Dispatch
        cpass.dispatch_workgroups(
            (VOLUMETRIC_RESOLUTION[0] + 7) / 8,
            (VOLUMETRIC_RESOLUTION[1] + 7) / 8,
            1,
        );
    }
}

pub struct VolumetricApplyNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for VolumetricApplyNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for VolumetricApplyNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(
        &self,
        _prepared: &PreparedFrame,
    ) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::camera::CameraUniform;
        use crate::render::light::ClusterConfig;

        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        crate::render::render_graph::resource::NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::cluster_config_buffer(),
                ResourceSpec::buffer(
                    size_of::<ClusterConfig>() as u64,
                    wgpu::BufferUsages::UNIFORM,
                ),
            )
            .input(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::COPY_SRC,
                    ..TextureKey::default()
                }),
            )
            .input(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey::default()),
            )
            .input(
                standard_resources::volumetric_lighting_texture(),
                ResourceSpec::Texture(TextureKey::default()),
            )
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    ..TextureKey::default()
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;

        if self.pipeline.is_none() {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Volumetric Apply Layout"),
                bind_group_layouts: &[&device.create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: true,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 3,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Depth,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 4,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D3,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 5,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: None,
                    },
                )],
                push_constant_ranges: &[],
            });

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Volumetric Apply Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../shaders/volumetric_apply.wgsl").into(),
                ),
            });

            self.pipeline = Some(
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Volumetric Apply Pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            );
        }

        let camera_buffer = context.buffer(&standard_resources::camera_buffer());
        let config_buffer = context.buffer(&standard_resources::cluster_config_buffer());
        let main_color = context.texture(&standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());
        let volumetric_tex = context.texture(&standard_resources::volumetric_lighting_texture());

        // 创建一个新的临时纹理作为输入，因为我们要写入 main_color
        let main_color_input = context.get_texture(
            "volumetric_apply_input",
            TextureKey {
                width: context.render_context.surface_config.width,
                height: context.render_context.surface_config.height,
                format: Some(wgpu::TextureFormat::Rgba16Float),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                ..TextureKey::default()
            },
        );

        context.encoder.copy_texture_to_texture(
            main_color.texture.as_image_copy(),
            main_color_input.texture.as_image_copy(),
            wgpu::Extent3d {
                width: context.render_context.surface_config.width,
                height: context.render_context.surface_config.height,
                depth_or_array_layers: 1,
            },
        );

        let sampler = context.get_sampler(SamplerKey {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.pipeline.as_ref().unwrap().get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_buffer.buffer,
                        offset: 0,
                        size: Some(
                            wgpu::BufferSize::new(size_of::<CameraUniform>() as u64).unwrap(),
                        ),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: config_buffer.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&main_color_input.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&main_depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&volumetric_tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });

        let mut rpass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volumetric Apply Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_color.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        rpass.set_pipeline(self.pipeline.as_ref().unwrap());
        rpass.set_bind_group(0, &bind_group, &[0]);
        rpass.draw(0..3, 0..1);
    }
}
