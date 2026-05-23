use crate::core::singleton::Singletons;
use crate::math::color::ColorU;
use crate::math::transform::Transform3d;
use glam::Vec3;
use std::any::Any;

use crate::render::draw_command::DrawCommands;
use crate::render::light::DirectionalLightUniform;
use crate::scene::{AsNode, NodeType};

pub struct DirectionalLight {
    pub transform: Transform3d,
    pub color: ColorU,
    pub strength: f32,
}

impl DirectionalLight {
    pub fn new() -> Self {
        Self {
            transform: Transform3d::default(),
            color: ColorU::white(),
            strength: 1.0,
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

    fn update(&mut self, _dt: f32, _singletons: &mut Singletons) {}

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let direction = self.transform.rotation * Vec3::NEG_Z;

        let directional_light = DirectionalLightUniform {
            direction: direction.to_array(),
            strength: self.strength,
            color: self.color.to_vec3().into(),
            distance: 20.0, // Default distance for shadow mapping
        };

        draw_cmds.extracted.lights.directional_light = Some(directional_light);
    }
}
