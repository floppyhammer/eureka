use crate::core::singleton::Singletons;
use crate::math::color::ColorU;
use crate::math::transform::Transform3d;
use cgmath::prelude::*;
use naga::SwitchValue::Default;
use std::any::Any;
use std::ops::Range;
use std::path::Path;
use wgpu::util::DeviceExt;

use crate::render::draw_command::DrawCommands;
use crate::render::light::{DirectionalLightUniform, LightUniform, PointLightUniform};
use crate::render::{Mesh, RenderServer, Texture};
use crate::scene::{AsNode, NodeType};
// use crate::scene::sprite3d::Sprite3d;
// use crate::scene::{AsNode, CameraInfo, NodeType};

pub struct DirectionalLight {
    pub transform: Transform3d,
    pub color: ColorU,
    pub strength: f32,
    // pub(crate) sprite: Sprite3d,
}

impl DirectionalLight {
    pub fn new() -> Self {
        // let sprite_tex = Texture::load(&device, &queue, &mut render_server.texture_cache, icon_path).unwrap();
        // let sprite3d = Sprite3d::new(&render_server, sprite_tex);

        Self {
            transform: Transform3d::default(),
            color: ColorU::white(),
            strength: 1.0,
            // sprite: sprite3d,
        }
    }
}

impl AsNode for DirectionalLight {
    fn node_type(&self) -> NodeType {
        NodeType::DirectionalLight
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

        let directional_light = DirectionalLightUniform {
            direction: self.transform.position.into(),
            strength: self.strength,
            color: self.color.to_vec3().into(),
            ..std::default::Default::default()
        };

        draw_cmds.extracted.lights.directional_light = Some(directional_light);
    }
}
