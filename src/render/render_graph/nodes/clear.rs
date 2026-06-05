use crate::render::camera::CameraType;
use crate::render::render_graph::{FrameContext, Node};

pub struct ClearNode;

impl Node for ClearNode {
    fn run(&mut self, context: &mut FrameContext) {
        let world = context.render_world;
        let depth_texture = world
            .texture_cache
            .get(world.surface_depth_texture)
            .unwrap();

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
                    view: context.output_view,
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
                    view: &depth_texture.view,
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
