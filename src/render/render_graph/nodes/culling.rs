use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{
    standard_resources, FrameContext, Node, NodeResources, ResourceSpec, SamplerKey,
};
use crate::render::InstanceRaw;
use std::any::Any;
use wgpu::BufferAddress;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CullParams {
    pub total_instances: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub max_mip_level: u32,
    pub depth_mode: u32, // 0: Standard Depth, 1: Reverse-Z
    pub use_hiz: u32,    // 0: Disable Hi-Z test, 1: Enable Hi-Z test
    pub _pad0: u32,
    pub _pad1: u32,
}

/// CullingNode 负责在 GPU 端实现高效的实例剔除 (Instance Culling)。
///
/// 它的主要职责是利用 Compute Shader 进行统一短路剔除流水线：
/// 1. **视锥体剔除 (Frustum Culling)**: 每个 GPU 线程处理一个实例，进行 Early Exit。
/// 2. **法线锥背面剔除 (Normal Cone Culling)**: 对 Sub-mesh / Meshlet 包围体做背面剔除。
/// 3. **Hi-Z 遮挡剔除 (Hi-Z Occlusion Culling)**: 将 AABB 投影到屏幕 UV 空间，在上一帧的 Hi-Z 金字塔上做采样比对。
/// 4. **原子计数 & 生成可见缓冲**: 将通过测试的可见实例紧凑写入 `cull_visible_instance_buffer`。
pub struct CullingNode {
    pipeline: Option<wgpu::ComputePipeline>,
}

impl Default for CullingNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for CullingNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self, prepared: &PreparedFrame) -> NodeResources {
        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        let metadata_buffer_size = (prepared.mesh_metadatas.len()
            * size_of::<crate::render::mesh::MeshMetadata>())
            as BufferAddress;

        NodeResources::new()
            .input(
                standard_resources::camera_buffer(),
                ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
            )
            .input(
                standard_resources::global_instance_buffer(), // 未经过 culling 的 instance 数据
                ResourceSpec::buffer(
                    prepared.instance_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .input(
                standard_resources::mesh_metadata_buffer(),
                ResourceSpec::buffer(
                    metadata_buffer_size as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .optional_input(
                standard_resources::hiz_depth_pyramid(),
                ResourceSpec::Texture(crate::render::render_graph::resource::TextureKey {
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
            .output(
                standard_resources::cull_visible_instance_buffer(),
                ResourceSpec::buffer(
                    prepared.instance_buffer_size as BufferAddress,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .output(
                standard_resources::cull_indirect_buffer(),
                ResourceSpec::buffer(
                    prepared.indirect_buffer_size as BufferAddress,
                    wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::INDIRECT
                        | wgpu::BufferUsages::COPY_DST,
                ),
            )
            .internal(
                standard_resources::cull_params_uniform(),
                ResourceSpec::buffer(
                    std::mem::size_of::<CullParams>() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                ),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let device = &context.render_context.device;

        if context.prepared.instance_buffer_size == 0 {
            return;
        }

        let cull_indirect_buffer = context.buffer(&standard_resources::cull_indirect_buffer());

        context.render_context.queue.write_buffer(
            &cull_indirect_buffer.buffer,
            0,
            bytemuck::cast_slice(&context.prepared.indirect_commands),
        );

        let total_instances =
            (context.prepared.instance_buffer_size / size_of::<InstanceRaw>()) as u32;

        let has_hiz = context.has_resource(&standard_resources::hiz_depth_pyramid());
        let use_hiz = if context.frame_count() == 0 || !has_hiz { 0 } else { 1 };

        let (screen_width, screen_height, hiz_view_id, hiz_view) = if use_hiz == 1 {
            let tex = context.texture(&standard_resources::hiz_depth_pyramid());
            (tex.handle.size.0, tex.handle.size.1, tex.id, tex.view)
        } else {
            (
                context.render_context.surface_config.width,
                context.render_context.surface_config.height,
                0u64,
                context.backend.dummy_2d_view.clone(),
            )
        };

        let max_dim = screen_width.max(screen_height);
        let max_mip_level = if max_dim > 0 {
            (max_dim as f32).log2().floor() as u32
        } else {
            0
        };

        // 2. 获取参数 Buffer 并写入
        let cull_params_buffer = context.buffer(&standard_resources::cull_params_uniform());
        let cull_params = CullParams {
            total_instances,
            screen_width,
            screen_height,
            max_mip_level,
            depth_mode: 0, // Standard Depth
            use_hiz,
            _pad0: 0,
            _pad1: 0,
        };

        context.render_context.queue.write_buffer(
            &cull_params_buffer.buffer,
            0,
            bytemuck::cast_slice(&[cull_params]),
        );

        let cull_bind_group_layout = context
            .backend
            .get_bind_group_layout("cull_bind_group_layout");

        if cull_bind_group_layout.is_none() {
            let cull_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                            count: None,
                        },
                    ],
                    label: Some("Cull Bind Group Layout"),
                });

            context
                .backend
                .add_bind_group_layout("cull_bind_group_layout", cull_bind_group_layout);
        }

        let cull_bind_group_layout = context
            .backend
            .get_bind_group_layout("cull_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cull Pipeline Layout"),
                bind_group_layouts: &[Some(&cull_bind_group_layout)],
                immediate_size: 0,
            });

            let source = include_str!("../../../shaders/cull.wgsl")
                .replace("#import eureka::camera::Camera", crate::render::camera::CAMERA_STRUCT_WGSL);

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Cull Shader"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

            self.pipeline = Some(device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("Cull Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options: Default::default(),
                },
            ));
        }

        let camera_buffer = context.buffer(&standard_resources::camera_buffer());

        let Some(camera_index) = context
            .extracted
            .cameras
            .types
            .iter()
            .position(|t| *t == CameraType::D3)
        else {
            return;
        };

        let camera_offset = camera_index as u32 * CameraUniform::get_uniform_offset_unit();

        let global_instance_buffer = context.buffer(&standard_resources::global_instance_buffer());
        let mesh_metadata_buffer = context.buffer(&standard_resources::mesh_metadata_buffer());
        let total_instances = context.prepared.instance_buffer_size / size_of::<InstanceRaw>();

        let cull_visible_instance_buffer =
            context.buffer(&standard_resources::cull_visible_instance_buffer());

        let hiz_sampler = context.get_sampler(SamplerKey::nearest_clamp());

        let cull_bind_group = context.create_bind_group(
            "cull_bind_group_layout",
            vec![
                camera_buffer.id,
                mesh_metadata_buffer.id,
                global_instance_buffer.id,
                cull_visible_instance_buffer.id,
                cull_indirect_buffer.id,
                hiz_view_id,
            ],
            |ctx| {
                ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &cull_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &camera_buffer.buffer,
                                offset: 0,
                                size: Some(
                                    wgpu::BufferSize::new(size_of::<CameraUniform>() as u64)
                                        .unwrap(),
                                ),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: mesh_metadata_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: global_instance_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: cull_visible_instance_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: cull_indirect_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: cull_params_buffer.buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 6,
                            resource: wgpu::BindingResource::TextureView(&hiz_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::Sampler(&hiz_sampler),
                        },
                    ],
                    label: Some("Cull Bind Group"),
                })
            },
        );

        let mut compute_pass = context
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cull Compute Pass"),
                timestamp_writes: None,
            });

        compute_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        compute_pass.set_bind_group(0, &cull_bind_group, &[camera_offset]);

        if total_instances > 0 {
            compute_pass.dispatch_workgroups(((total_instances + 63) / 64) as u32, 1, 1);
        }
    }
}
