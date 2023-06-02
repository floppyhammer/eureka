use cgmath::prelude::*;
use std::any::Any;
use std::ops::Range;
use std::path::Path;
use wgpu::util::DeviceExt;

use crate::resources::Mesh;
use crate::scene::{AsNode, CameraInfo, NodeType};
use crate::{InputEvent, Model, RenderServer, Singletons, Sprite3d, Texture};

pub struct Light {
    pub(crate) uniform: LightUniform,
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) sprite: Sprite3d,
}

impl Light {
    pub fn new<P: AsRef<Path>>(render_server: &RenderServer, icon_path: P) -> Self {
        let device = &render_server.device;
        let queue = &render_server.queue;

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

        let sprite_tex = Texture::load(&device, &queue, icon_path).unwrap();
        let sprite3d = Sprite3d::new(&render_server, sprite_tex);

        Self {
            uniform,
            buffer,
            bind_group,
            sprite: sprite3d,
        }
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

impl AsNode for Light {
    fn node_type(&self) -> NodeType {
        NodeType::Light
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        let queue = &mut singletons.render_server.queue;

        let old_position: cgmath::Vector3<_> = self.uniform.position.into();
        let new_position =
            cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt))
                * old_position;

        self.uniform.position = new_position.into();

        // Update buffer.
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));

        self.sprite.position = new_position;
        self.sprite.update(dt, camera_info, singletons);
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        self.sprite.draw(render_pass, camera_info, singletons);
    }
}
