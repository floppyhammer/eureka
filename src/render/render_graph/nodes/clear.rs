use crate::render::camera::CameraType;
use crate::render::render_graph::{FrameContext, Node, TextureKey};
use crate::render::Texture;

use std::any::Any;

pub struct ClearNode;

impl Node for ClearNode {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_resources(&self) -> crate::render::render_graph::resource::NodeResources {
        use crate::render::render_graph::standard_resources;
        use crate::render::render_graph::resource::{ResourceSpec, TextureKey};
        use crate::render::Texture;

        crate::render::render_graph::resource::NodeResources::new()
            .output(
                standard_resources::main_color(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
            .output(
                standard_resources::main_depth(),
                ResourceSpec::Texture(TextureKey {
                    width: 0,
                    height: 0,
                    format: Texture::DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    layers: 1,
                }),
            )
    }

    fn run(&mut self, context: &mut FrameContext) {
        let width = context.render_context.surface_config.width;
        let height = context.render_context.surface_config.height;
        let format = context.render_context.surface_config.format;

        // 先定义所有的 Key，不要在使用过程中从 context 拿数据
        let main_color_key = TextureKey {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };
        let main_depth_key = TextureKey {
            width,
            height,
            format: Texture::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        };

        // 按顺序获取纹理句柄（现在返回的是克隆后的句柄，不会互相冲突）
        let main_color = context.get_texture("main_color", main_color_key);
        let main_depth = context.get_texture("main_depth", main_depth_key);

        let world = &*context.render_world;

        let ssao_ran = {
            let mut wants_ssao = false;
            for (i, cam_type) in world.extracted.cameras.types.iter().enumerate() {
                if *cam_type == CameraType::D3 {
                    if world.extracted.cameras.uniforms[i].ssao_enabled == 1 {
                        wants_ssao = true;
                        break;
                    }
                }
            }
            wants_ssao && !world.extracted.meshes.is_empty()
        };

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
                        load: if ssao_ran {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(1.0)
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
    }
}
