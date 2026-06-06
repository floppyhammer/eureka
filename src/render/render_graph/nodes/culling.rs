use crate::render::render_graph::{FrameContext, Node};

pub struct CullingNode {
    pipeline: Option<wgpu::ComputePipeline>,
}

impl Default for CullingNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for CullingNode {
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
        let world = &*context.render_world;
        let resources = &world.mesh_render_resources;

        if let Some(pipeline) = &self.pipeline {
            if let Some(bind_group) = &resources.cull_bind_group {
                let mut compute_pass =
                    context
                        .encoder
                        .begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Global Culling Pass"),
                            timestamp_writes: None,
                        });
                compute_pass.set_pipeline(pipeline);
                compute_pass.set_bind_group(0, bind_group, &[0]); // Camera offset

                let total_instances: u32 = world.extracted.meshes.len() as u32;
                if total_instances > 0 {
                    compute_pass.dispatch_workgroups((total_instances + 63) / 64, 1, 1);
                }
            }
        }
    }
}
