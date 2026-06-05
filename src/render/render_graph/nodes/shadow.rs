use crate::render::light::render_shadow;
use crate::render::render_graph::{FrameContext, Node};
use crate::render::vertex::{Vertex3d, VertexBuffer};
use crate::render::{create_render_pipeline, InstanceRaw, Texture};

pub struct ShadowNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for ShadowNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for ShadowNode {
    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }

        let device = &context.render_context.device;
        let camera_resources = &context.render_world.camera_render_resources;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[&camera_resources.bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("shadow shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/shadow.wgsl").into()),
        };

        let pipeline = create_render_pipeline(
            device,
            &pipeline_layout,
            None,
            Some(Texture::DEPTH_FORMAT),
            &[Vertex3d::desc(), InstanceRaw::desc()],
            shader,
            "shadow pipeline",
            false,
            Some(wgpu::Face::Front),
        );

        self.pipeline = Some(pipeline);
    }

    fn run(&mut self, context: &mut FrameContext) {
        let world = context.render_world;
        if let Some(pipeline) = &self.pipeline {
            render_shadow(
                context.encoder,
                &world.texture_cache,
                &world.light_render_resources,
                &world.extracted.lights,
                &world.extracted.meshes,
                &world.mesh_cache,
                &world.mesh_render_resources,
                &world.extracted.bvh,
                pipeline,
            );
        }
    }
}
