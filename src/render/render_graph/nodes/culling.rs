use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_backend::PreparedFrame;
use crate::render::render_graph::{
    standard_resources, FrameContext, Node, NodeResources, ResourceSpec,
};
use crate::render::InstanceRaw;
use std::any::Any;
use wgpu::BufferAddress;

/// CullingNode 负责在 GPU 端实现高效的实例剔除 (Instance Culling)。
///
/// 它的主要职责是利用 Compute Shader 进行视锥体剔除 (Frustum Culling)，流程如下：
/// 1. **并行判定**: 每个 GPU 线程处理一个实例，将其 AABB 转换到裁剪空间，判断是否在视锥体内。
/// 2. **原子计数**: 使用原子操作统计每个 Mesh 的可见实例数量。
/// 3. **构建间接绘制指令**: 动态更新 `IndirectBuffer` 中的实例数量，使后续渲染节点能通过
///    `multi_draw_indexed_indirect` 一次性画出所有可见物体。
/// 4. **生成可见实例缓冲**: 将可见实例的数据紧凑地重新排列到 `cull_visible_instance_buffer` 中。
///
/// 通过该节点，CPU 仅需提交一次庞大的实例数据，后续每帧的剔除工作完全在 GPU 端异步完成，
/// 极大地减轻了 CPU 在 Draw Call 管理上的负担。
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
                standard_resources::cull_params_uniform(), // 临时资源
                ResourceSpec::buffer(
                    16,
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

        // 1. 获取逻辑上的实例总数
        let total_instances =
            (context.prepared.instance_buffer_size / size_of::<InstanceRaw>()) as u32;

        // 2. 获取参数 Buffer 并写入
        let cull_params_buffer = context.buffer(&standard_resources::cull_params_uniform());
        context.render_context.queue.write_buffer(
            &cull_params_buffer.buffer,
            0,
            bytemuck::cast_slice(&[total_instances]),
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

        // 找到第一个 D3 相机的索引
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

        let cull_bind_group = context.create_bind_group(
            "cull_bind_group_layout",
            vec![
                camera_buffer.id,
                mesh_metadata_buffer.id,
                global_instance_buffer.id,
                cull_visible_instance_buffer.id,
                cull_indirect_buffer.id,
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
