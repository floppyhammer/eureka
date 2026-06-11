use crate::render::render_graph::{standard_resources, FrameContext, Node};
use std::any::Any;
use crate::render::render_world::RenderWorld;

pub struct ClearNode;

impl Node for ClearNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self, world: &RenderWorld) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::render_graph::standard_resources;
        use crate::render::Texture;

        crate::render::render_graph::resource::NodeResources::new()
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Some(Texture::DEPTH_FORMAT),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let main_color = context.texture(&standard_resources::main_color());
        let main_depth = context.texture(&standard_resources::main_depth());

        let _render_pass = context
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_color.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0), // 永远清空，确保深度缓冲区干净
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
    }
}
