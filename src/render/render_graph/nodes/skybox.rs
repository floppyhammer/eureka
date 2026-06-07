use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::sky::render_sky;
use crate::render::vertex::{VertexBuffer, VertexSky};
use crate::render::{create_render_pipeline, Texture};
use std::any::Any;

pub struct SkyboxNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SkyboxNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SkyboxNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }
        let device = &context.render_context.device;
        let world = &*context.render_world;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skybox pipeline layout"),
            bind_group_layouts: &[
                &world.camera_render_resources.bind_group_layout,
                &world.sky_render_resources.texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("skybox shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/skybox.wgsl").into()),
        };

        self.pipeline = Some(create_render_pipeline(
            device,
            &pipeline_layout,
            Some(context.render_context.surface_config.format),
            Some(Texture::DEPTH_FORMAT),
            &[VertexSky::desc()],
            shader,
            "skybox pipeline",
            false,
            Some(wgpu::Face::Back),
        ));
    }

    fn run(&mut self, context: &mut FrameContext) {
        let width = context.render_context.surface_config.width;
        let height = context.render_context.surface_config.height;
        let format = context.render_context.surface_config.format;

        let main_color_key = TextureKey {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        let main_color = context.get_texture("main_color", main_color_key);
        let main_depth = context.get_texture("main_depth", main_depth_key);

        let world = &*context.render_world;
        if world.extracted.sky.is_none() {
            return;
        }

        let mut render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("skybox render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_color.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        if let Some(camera_bind_group) = &world.camera_render_resources.bind_group {
            render_sky(
                camera_bind_group,
                &world.sky_render_resources,
                &mut render_pass,
                &world.mesh_render_resources.mesh_allocator,
                self.pipeline.as_ref().unwrap(),
            );
        }
    }
}
