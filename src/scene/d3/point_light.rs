use crate::core::singleton::Singletons;
use crate::math::color::ColorU;
use crate::math::transform::Transform3d;
use cgmath::prelude::*;
use std::any::Any;
use std::ops::Range;
use std::path::Path;

use crate::render::draw_command::DrawCommands;
use crate::render::light::{LightUniform, PointLightUniform};
use crate::render::{Mesh, RenderServer, Texture};
use crate::scene::{AsNode, NodeType};
// use crate::scene::sprite3d::Sprite3d;
// use crate::scene::{AsNode, CameraInfo, NodeType};

pub struct PointLight {
    pub transform: Transform3d,
    pub color: ColorU,
    pub strength: f32,
    // pub(crate) sprite: Sprite3d,
}

impl PointLight {
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

impl AsNode for PointLight {
    fn node_type(&self) -> NodeType {
        NodeType::PointLight
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

        let point_light = PointLightUniform {
            position: self.transform.position.into(),
            strength: self.strength,
            color: self.color.to_vec3().into(),
            constant: 1.0,
            linear: 0.09,
            quadratic: 0.032,
            ..Default::default()
        };

        draw_cmds.extracted.lights.point_lights.push(point_light);
    }
}
