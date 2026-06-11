use crate::render::camera::{CameraType, CameraUniform};
use crate::render::render_graph::{
    standard_resources, FrameContext, Node, NodeResources, ResourceSpec,
};
use std::any::Any;

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

    fn node_resources(&self) -> NodeResources {
        let camera_buffer_size = CameraUniform::get_uniform_offset_unit()
            * crate::render::render_graph::nodes::prepare_view::MAX_CAMERAS;

        NodeResources::new().input(
            standard_resources::camera_buffer(),
            ResourceSpec::buffer(camera_buffer_size as u64, wgpu::BufferUsages::UNIFORM),
        )
    }

    fn prepare(&mut self, context: &mut FrameContext) {}

    fn run(&mut self, context: &mut FrameContext) {
        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;
        let device = &context.render_context.device;

        let cull_bind_group_layout = context.pool.get_bind_group_layout("cull_bind_group_layout");
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
                    ],
                    label: Some("Cull Bind Group Layout"),
                });

            context
                .pool
                .add_bind_group_layout("cull_bind_group_layout", cull_bind_group_layout);
        }
        let cull_bind_group_layout = context
            .pool
            .get_bind_group_layout("cull_bind_group_layout")
            .unwrap()
            .clone();

        if self.pipeline.is_none() {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Cull Layout"),
                bind_group_layouts: &[&cull_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Cull Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/cull.wgsl").into()),
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
            .render_world
            .extracted
            .cameras
            .types
            .iter()
            .position(|t| *t == CameraType::D3)
        else {
            return;
        };

        let camera_offset = camera_index as u32 * CameraUniform::get_uniform_offset_unit();

        // 创建 Culling BindGroup（直接创建，不使用缓存）
        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;
        let total_instances: u32 = world.extracted.meshes.len() as u32;

        // Graph 外部的资源
        let Some(mesh_metadata_buffer) = resources.mesh_metadata_buffer.as_ref() else {
            return;
        };
        let Some(global_instance_buffer) = resources.global_instance_buffer.as_ref() else {
            return;
        };
        let Some(global_visible_instance_buffer) =
            resources.global_visible_instance_buffer.as_ref()
        else {
            return;
        };
        let Some(global_indirect_buffer) = resources.global_indirect_buffer.as_ref() else {
            return;
        };

        // todo: use cache
        let bind_group =
            context
                .render_context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
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
                            resource: mesh_metadata_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: global_instance_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: global_visible_instance_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: global_indirect_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("Cull Dynamic Bind Group"),
                });

        let mut compute_pass = context
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Global Culling Pass"),
                timestamp_writes: None,
            });

        compute_pass.set_pipeline(self.pipeline.as_ref().unwrap());
        compute_pass.set_bind_group(0, &bind_group, &[camera_offset]);

        if total_instances > 0 {
            compute_pass.dispatch_workgroups((total_instances + 63) / 64, 1, 1);
        }
    }
}
