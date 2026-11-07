use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::resource::{NodeResources, ResourceSpec, TextureKey};
use crate::render::render_graph::{standard_resources, FrameContext, Node};
use crate::render::Texture;
use std::any::Any;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DownsampleParams {
    src_width: u32,
    src_height: u32,
    depth_mode: u32, // 0: Standard Depth (0=Near, 1=Far), 1: Reverse-Z
    _pad: u32,
}

/// DepthPyramidNode 负责在 Compute Pass 中逐级下采样 Depth Prepass 深度图，
/// 构建带 Mipmap 链的 Hi-Z (Hierarchical Z-Buffer) 深度金字塔纹理 (R32Float)。
pub struct DepthPyramidNode {
    init_pipeline: Option<wgpu::ComputePipeline>,
    downsample_pipeline: Option<wgpu::ComputePipeline>,
    params_buffer: Option<wgpu::Buffer>,
}

impl Default for DepthPyramidNode {
    fn default() -> Self {
        Self {
            init_pipeline: None,
            downsample_pipeline: None,
            params_buffer: None,
        }
    }
}

impl Node for DepthPyramidNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self, _prepared: &PreparedFrame) -> NodeResources {
        NodeResources::new()
            .input(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                    mip_levels: 1,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
            .persistent_output(
                standard_resources::hiz_depth_pyramid(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::R32Float),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    layers: 1,
                    mip_levels: 12,
                    dimension: wgpu::TextureDimension::D2,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;
        let queue = &context.render_context.queue;

        let main_depth = context.texture(&standard_resources::main_depth());
        let hiz_depth = context.texture(&standard_resources::hiz_depth_pyramid());

        let width = hiz_depth.handle.size.0;
        let height = hiz_depth.handle.size.1;
        if width == 0 || height == 0 {
            return;
        }

        let max_dim = width.max(height);
        let total_mips = (max_dim as f32).log2().floor() as u32 + 1;
        let total_mips = total_mips.min(hiz_depth.texture.mip_level_count());

        // 初始化参数 Uniform Buffer
        if self.params_buffer.is_none() {
            self.params_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Depth Pyramid Downsample Params Buffer"),
                size: std::mem::size_of::<DownsampleParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        let shader_source = include_str!("../../../shaders/depth_pyramid.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Depth Pyramid Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // 1. 创建 Init Pipeline (Pass 0: Depth32Float -> R32Float Mip 0)
        if self.init_pipeline.is_none() {
            let init_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Hi-Z Init Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::R32Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Hi-Z Init Pipeline Layout"),
                bind_group_layouts: &[Some(&init_bind_group_layout)],
                immediate_size: 0,
            });

            self.init_pipeline = Some(device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Hi-Z Init Compute Pipeline"),
                    layout: Some(&layout),
                    module: &shader_module,
                    entry_point: Some("init_mip0"),
                    cache: None,
                    compilation_options: Default::default(),
                },
            ));
        }

        // 2. 创建 Downsample Pipeline (Pass 1..N: Mip k-1 -> Mip k)
        if self.downsample_pipeline.is_none() {
            let downsample_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Hi-Z Downsample Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::R32Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Hi-Z Downsample Pipeline Layout"),
                bind_group_layouts: &[Some(&downsample_bind_group_layout)],
                immediate_size: 0,
            });

            self.downsample_pipeline = Some(device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Hi-Z Downsample Compute Pipeline"),
                    layout: Some(&layout),
                    module: &shader_module,
                    entry_point: Some("downsample"),
                    cache: None,
                    compilation_options: Default::default(),
                },
            ));
        }

        // --- 执行 Pass 0: 复制 main_depth 到 hiz_depth_pyramid Mip 0 ---
        let mip0_view = hiz_depth
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("Hi-Z Mip 0 Storage View"),
                format: Some(wgpu::TextureFormat::R32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: None,
            });

        let main_depth_aspect_view = main_depth
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("Main Depth View For Hi-Z Init"),
                format: Some(Texture::DEPTH_FORMAT),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: None,
            });

        let init_pipeline = self.init_pipeline.as_ref().unwrap();
        let init_layout = init_pipeline.get_bind_group_layout(0);
        let init_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hi-Z Init Bind Group"),
            layout: &init_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&main_depth_aspect_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mip0_view),
                },
            ],
        });

        {
            let mut compute_pass =
                context
                    .encoder
                    .begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Hi-Z Init Mip 0 Pass"),
                        timestamp_writes: None,
                    });
            compute_pass.set_pipeline(init_pipeline);
            compute_pass.set_bind_group(0, &init_bind_group, &[]);
            compute_pass.dispatch_workgroups((width + 15) / 16, (height + 15) / 16, 1);
        }

        // --- 执行 Pass 1..N-1: 逐级下采样 Mip Level ---
        let downsample_pipeline = self.downsample_pipeline.as_ref().unwrap();
        let downsample_layout = downsample_pipeline.get_bind_group_layout(0);
        let params_buffer = self.params_buffer.as_ref().unwrap();

        for mip in 1..total_mips {
            let src_width = (width >> (mip - 1)).max(1);
            let src_height = (height >> (mip - 1)).max(1);
            let dst_width = (width >> mip).max(1);
            let dst_height = (height >> mip).max(1);

            let params = DownsampleParams {
                src_width,
                src_height,
                depth_mode: 0, // Standard Depth
                _pad: 0,
            };
            queue.write_buffer(params_buffer, 0, bytemuck::cast_slice(&[params]));

            let src_view = hiz_depth
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Hi-Z Src Mip View"),
                    format: Some(wgpu::TextureFormat::R32Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip - 1,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                    usage: None,
                });

            let dst_view = hiz_depth
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Hi-Z Dst Mip View"),
                    format: Some(wgpu::TextureFormat::R32Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                    usage: None,
                });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Hi-Z Downsample Bind Group"),
                layout: &downsample_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut compute_pass =
                context
                    .encoder
                    .begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Hi-Z Downsample Pass"),
                        timestamp_writes: None,
                    });
            compute_pass.set_pipeline(downsample_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((dst_width + 15) / 16, (dst_height + 15) / 16, 1);
        }
    }
}
