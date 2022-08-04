use crate::resource::Mesh;
use crate::scene::AsNode;
use crate::{InputEvent, Model, RenderServer, Singletons, Sprite3d, Texture};
use cgmath::prelude::*;
use std::ops::Range;
use wgpu::util::DeviceExt;

pub struct Light {
    pub(crate) uniform: LightUniform,
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) sprite: Sprite3d,
}

impl Light {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_server: &RenderServer,
    ) -> Self {
        let uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        // We'll want to update our lights position, so we use COPY_DST.
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light uniform buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_server.light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });

        let asset_dir = std::path::Path::new(env!("OUT_DIR")).join("assets");
        println!("Asset dir: {}", asset_dir.display());

        let sprite_tex = Texture::load(&device, &queue, asset_dir.join("light.png")).unwrap();
        let sprite3d = Sprite3d::new(&device, &queue, &render_server, sprite_tex);

        Self {
            uniform,
            buffer,
            bind_group,
            sprite: sprite3d,
        }
    }

    pub fn update(&mut self, dt: f32, queue: &wgpu::Queue, render_server: &RenderServer) {
        let old_position: cgmath::Vector3<_> = self.uniform.position.into();
        let new_position =
            cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt))
                * old_position;

        self.uniform.position = new_position.into();

        // Update buffer.
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));

        self.sprite.position = new_position;
        self.sprite.update(&queue, dt, &render_server, None);
    }

    pub fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        self.sprite.draw(render_pass, render_server, singletons);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    pub(crate) position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding: u32,
    pub(crate) color: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding2: u32,
}
