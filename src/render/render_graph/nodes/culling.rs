use crate::render::camera::CameraUniform;
use crate::render::render_graph::{standard_resources, BufferKey, FrameContext, Node, ResourceId};
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

    fn input_resources(&self) -> Vec<ResourceId<()>> {
        vec![standard_resources::camera_buffer()]
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }

        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;
        let device = &context.render_context.device;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cull layout"),
            bind_group_layouts: &[&resources.cull_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cull shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/cull.wgsl").into()),
        });

        self.pipeline = Some(
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("cull pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            }),
        );
    }

    fn run(&mut self, context: &mut FrameContext) {
        // 获取相机 Buffer (自动参与 FIF 同步)
        let camera_buffer_key = BufferKey {
            size: (CameraUniform::get_uniform_offset_unit()
                * context.render_world.extracted.cameras.uniforms.len() as u32)
                as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        };
        let camera_buffer =
            context.get_buffer_by_id(&standard_resources::camera_buffer(), camera_buffer_key);

        if let Some(pipeline) = &self.pipeline {
            // 创建 Culling BindGroup（直接创建，不使用缓存）
            let world = &*context.render_world;
            let resources = &world.mesh_render_resources;
            let total_instances: u32 = world.extracted.meshes.len() as u32;

            // 检查所有必要的 buffer 是否存在
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

            let bind_group =
                context
                    .render_context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &resources.cull_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                    buffer: &camera_buffer.buffer,
                                    offset: 0,
                                    size: Some(
                                        wgpu::BufferSize::new(
                                            std::mem::size_of::<CameraUniform>() as u64
                                        )
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
                        label: Some("global cull dynamic bind group"),
                    });

            let mut compute_pass =
                context
                    .encoder
                    .begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Global Culling Pass"),
                        timestamp_writes: None,
                    });
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[0]); // Camera offset

            if total_instances > 0 {
                compute_pass.dispatch_workgroups((total_instances + 63) / 64, 1, 1);
            }
        }
    }
}
