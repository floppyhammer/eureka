use std::any::Any;
use anyhow::Context;
use anyhow::*;
use cgmath::InnerSpace;
use cgmath::*;
use std::error::Error;
use std::ops::Range;
use std::path::Path;
use std::time::Instant;
use wgpu::util::DeviceExt;

use crate::resource::CubemapTexture;
use crate::resource::{material, mesh, texture};
use crate::scene::{AsNode, CameraInfo, NodeType};
use crate::{InputEvent, RenderServer, Singletons};
use material::MaterialSky;
use mesh::Mesh;

pub struct Sky {
    pub rotation: cgmath::Quaternion<f32>,

    pub mesh: Mesh,

    pub material: MaterialSky,

    pub name: String,
}

impl Sky {
    pub fn new(render_server: &RenderServer, texture: CubemapTexture) -> Self {
        let now = Instant::now();

        let bind_group = render_server
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &render_server.skybox_texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: None,
            });

        let material = MaterialSky {
            name: "sky material".to_string(),
            texture,
            bind_group,
        };

        let mesh = Mesh::default_skybox(&render_server.device);

        let rotation = cgmath::Quaternion::new(0.0, 0.0, 0.0, 0.0);

        let elapsed_time = now.elapsed();
        log::info!("Sky setup took {} milliseconds", elapsed_time.as_millis());

        Self {
            rotation,
            mesh,
            material,
            name: "sky".to_string(),
        }
    }
}

impl AsNode for Sky {
    fn node_type(&self) -> NodeType {
        NodeType::Sky
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        match &camera_info.bind_group {
            Some(b) => {
                render_pass.set_pipeline(&singletons.render_server.skybox_pipeline);

                render_pass.draw_skybox(
                    &self.mesh,
                    &self.material,
                    b,
                );
            }
            None => {}
        }
    }
}

pub trait DrawSky<'a> {
    fn draw_skybox(
        &mut self,
        mesh: &'a Mesh,
        material: &'a MaterialSky,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSky<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{
    fn draw_skybox(
        &mut self,
        mesh: &'a Mesh,
        material: &'a MaterialSky,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set camera uniform.
        self.set_bind_group(0, camera_bind_group, &[]);

        // Set texture.
        self.set_bind_group(1, &material.bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
