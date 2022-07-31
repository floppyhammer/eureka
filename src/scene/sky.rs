use std::error::Error;
use std::ops::Range;
use std::path::Path;
use anyhow::Context;
use cgmath::InnerSpace;
use wgpu::util::DeviceExt;
use anyhow::*;
use cgmath::*;

use crate::resource::{texture, mesh, material};
use crate::resource::CubemapTexture;
use mesh::{Mesh, VertexSky};
use material::MaterialSky;
use crate::{InputEvent, RenderServer};
use crate::scene::AsNode;

pub struct Sky {
    pub rotation: cgmath::Quaternion<f32>,

    pub mesh: Mesh,

    pub material: MaterialSky,

    pub name: String,
}

impl Sky {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_server: &RenderServer,
        texture: CubemapTexture,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        let mesh = Mesh::default_skybox(device);

        let rotation = cgmath::Quaternion::new(0.0, 0.0, 0.0, 0.0);

        Self { rotation, mesh, material, name: "sky".to_string() }
    }
}

impl AsNode for Sky {
    fn input(&mut self, input: InputEvent) {}

    fn update(&mut self, queue: &wgpu::Queue, dt: f32, render_server: &RenderServer) {}

    fn draw<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>, render_server: &'b RenderServer) {
        render_pass.set_pipeline(&render_server.skybox_pipeline);

        render_pass.draw_skybox(
            &self.mesh,
            &self.material,
            &render_server.camera3d.as_ref().unwrap().bind_group,
        );
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
    where 'b: 'a {
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
