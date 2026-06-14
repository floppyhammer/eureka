use crate::animation::property::PropertyProvider;
use crate::core::singleton::Singletons;
use crate::math::color::ColorU;
use crate::math::transform::Transform3d;
use crate::render::draw_command::DrawCommands;
use crate::render::light::DirectionalLightUniform;
use crate::scene::{AsNode, AsNode3d, Node3d, NodeType};
use glam::{Quat, Vec3};
use std::any::Any;

pub struct DirectionalLight {
    pub node_3d: Node3d,
    pub color: ColorU,
    pub strength: f32,
    pub shadow_distance: f32,
}

impl DirectionalLight {
    pub fn new() -> Self {
        Self {
            node_3d: Node3d::default(),
            color: ColorU::white(),
            strength: 1.0,
            shadow_distance: 20.0,
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

    fn as_node_3d(&self) -> Option<&dyn AsNode3d> {
        Some(self)
    }

    fn as_node_3d_mut(&mut self) -> Option<&mut dyn AsNode3d> {
        Some(self)
    }

    fn update(&mut self, _dt: f32, _singletons: &mut Singletons) {}

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let direction = self.node_3d.global_transform.rotation * Vec3::NEG_Z;

        let directional_light = DirectionalLightUniform {
            direction: direction.to_array(),
            strength: self.strength,
            color: self.color.to_vec3().into(),
            shadow_distance: self.shadow_distance,
        };

        draw_cmds.extracted.lights.directional_light = Some(directional_light);
    }

    fn as_property_provider_mut(&mut self) -> Option<&mut dyn PropertyProvider> {
        Some(&mut self.node_3d)
    }
}

impl AsNode3d for DirectionalLight {
    fn get_position(&self) -> Vec3 {
        self.node_3d.transform.position
    }

    fn set_position(&mut self, position: Vec3) {
        self.node_3d.transform.position = position;
    }

    fn get_rotation(&self) -> Quat {
        self.node_3d.transform.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.node_3d.transform.rotation = rotation;
    }

    fn get_scale(&self) -> Vec3 {
        self.node_3d.transform.scale
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.node_3d.transform.scale = scale;
    }

    fn get_transform(&self) -> Transform3d {
        self.node_3d.transform
    }

    fn get_global_transform(&self) -> Transform3d {
        self.node_3d.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform3d) {
        self.node_3d.global_transform = transform;
    }
}
