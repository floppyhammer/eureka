use crate::render::atlas::render_atlas;
use crate::render::create_render_pipeline;
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::sprite::render_sprite;
use crate::render::vertex::{Vertex2d, VertexBuffer};
use crate::render::Texture;
use std::any::Any;

pub struct SpriteNode {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl Default for SpriteNode {
    fn default() -> Self {
        Self { pipeline: None }
    }
}

impl Node for SpriteNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::standard_resources;
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::Texture;

        crate::render::render_graph::resource::NodeResources::new()
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Texture::DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::final_output(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    layers: 1,
                }),
            )
    }

    fn prepare(&mut self, context: &mut FrameContext) {
        if self.pipeline.is_some() {
            return;
        }

        let device = &context.render_context.device;
        let world = &*context.render_world;

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
            Some(Texture::DEPTH_FORMAT),
            &[Vertex2d::desc()],
            shader,
            "sprite bindless",
            true, // 重新开启深度测试
            None,
        ));
    }

    fn run(&mut self, context: &mut FrameContext) {
        let main_depth_key = TextureKey {
            width: context.render_context.surface_config.width,
            height: context.render_context.surface_config.height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let main_depth = context.get_texture("main_depth", main_depth_key);

        let world = &*context.render_world;
        if world.sprite_batches.is_empty() {
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0), // 关键：清除 3D 场景深度，开始 UI 深度测试
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        if let (Some(cam_bg), Some(bindless_bg)) = (
            &world.camera_render_resources.bind_group,
            &world.mesh_render_resources.bindless_bind_group,
        ) {
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
