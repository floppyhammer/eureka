use cgmath::prelude::*;
use std::any::Any;
use std::ops::Range;
use std::path::Path;
use wgpu::util::DeviceExt;

use crate::render::draw_command::DrawCommands;
use crate::render::{Mesh, RenderServer, Texture};
use crate::scene::{AsNode, NodeType};
// use crate::scene::sprite3d::Sprite3d;
// use crate::scene::{AsNode, CameraInfo, NodeType};
use crate::Singletons;

pub struct Light {
    pub(crate) uniform: LightUniform,
    // pub(crate) sprite: Sprite3d,
}

impl Light {
    pub fn new() -> Self {
        let uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        // let sprite_tex = Texture::load(&device, &queue, &mut render_server.texture_cache, icon_path).unwrap();
        // let sprite3d = Sprite3d::new(&render_server, sprite_tex);

        Self {
            uniform,
            // sprite: sprite3d,
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

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        // let queue = &mut singletons.render_server.queue;

        let old_position: cgmath::Vector3<_> = self.uniform.position.into();
        let new_position =
            cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt))
                * old_position;

        self.uniform.position = new_position.into();

        // self.sprite.position = new_position;
        // self.sprite.update(dt, camera_info, singletons);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        draw_cmds.extracted.lights.push(self.uniform);
    }
}
