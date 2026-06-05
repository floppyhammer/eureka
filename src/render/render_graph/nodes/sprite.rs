use crate::render::atlas::render_atlas;
use crate::render::create_render_pipeline;
use crate::render::render_graph::{FrameContext, Node};
use crate::render::sprite::render_sprite;
use crate::render::vertex::{Vertex2d, VertexBuffer};

pub struct SpriteNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SpriteNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SpriteNode {
    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }

        let device = &context.render_context.device;
        let world = context.render_world;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite bindless layout"),
            bind_group_layouts: &[
                &world.camera_render_resources.bind_group_layout,
                &world.mesh_render_resources.bindless_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/sprite.wgsl").into()),
        };

        self.pipeline = Some(create_render_pipeline(
            device,
            &pipeline_layout,
            Some(context.render_context.surface_config.format),
            None,
            &[Vertex2d::desc()],
            shader,
            "sprite bindless",
            true,
            None,
        ));
    }

    fn run(&mut self, context: &mut FrameContext) {
        let world = context.render_world;
        if world.sprite_batches.is_empty() && world.extracted.atlases.is_empty() {
            return;
        }

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: context.final_output_view,
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

        if let (Some(cam_bg), Some(bindless_bg)) = (
            &world.camera_render_resources.bind_group,
            &world.mesh_render_resources.bindless_bind_group,
        ) {
            render_atlas(
                &world.extracted.atlases,
                &world.atlas_render_resources,
                &mut render_pass,
            );

            render_sprite(
                &world.sprite_batches,
                &world.sprite_render_resources,
                &mut render_pass,
                cam_bg,
                bindless_bg,
                self.pipeline.as_ref().unwrap(),
            );
        }
    }
}
