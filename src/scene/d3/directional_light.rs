use cgmath::prelude::*;
use std::any::Any;
use std::ops::Range;
use std::path::Path;
use wgpu::util::DeviceExt;
use crate::math::color::ColorU;
use crate::math::transform::{Transform3d};

use crate::render::draw_command::DrawCommands;
use crate::render::{Mesh, RenderServer, Texture};
use crate::render::light::LightUniform;
use crate::scene::{AsNode, NodeType};
// use crate::scene::sprite3d::Sprite3d;
// use crate::scene::{AsNode, CameraInfo, NodeType};
use crate::Singletons;

pub struct Light {
    pub transform: Transform3d,
    pub color: ColorU,
    // pub(crate) sprite: Sprite3d,
}

impl Light {
    pub fn new() -> Self {
        // let sprite_tex = Texture::load(&device, &queue, &mut render_server.texture_cache, icon_path).unwrap();
        // let sprite3d = Sprite3d::new(&render_server, sprite_tex);

        Self {
            transform: Transform3d::default(),
            color: ColorU::white(),
            // sprite: sprite3d,
        }
    }
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


        // self.sprite.position = new_position;
        // self.sprite.update(dt, camera_info, singletons);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        // let old_position: cgmath::Vector3<_> = self.uniform.position.into();
        // let new_position =
        //     cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(60.0 * dt))
        //         * old_position;
        //
        // self.uniform.position = new_position.into();

        let mut uniform = LightUniform::default();
        uniform.position = self.transform.position.into();
        uniform.color = self.color.to_vec3().into();
        draw_cmds.extracted.lights.push(uniform);
    }
}
