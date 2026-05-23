use crate::core::singleton::Singletons;
use crate::math::color::ColorU;
use glam::{Quat, Vec3};
use std::any::Any;

use crate::render::draw_command::DrawCommands;
use crate::render::light::PointLightUniform;
use crate::scene::{AsNode, AsNode3d, Node3d, NodeType};
// use crate::scene::sprite3d::Sprite3d;
// use crate::scene::{AsNode, CameraInfo, NodeType};

pub struct PointLight {
    pub node_3d: Node3d,
    pub color: ColorU,
    pub strength: f32,
    // pub(crate) sprite: Sprite3d,
    pub custom_update: Option<fn(f32, &mut Self)>,
}

impl PointLight {
    pub fn new() -> Self {
        // let sprite_tex = Texture::load(&device, &queue, &mut render_server.texture_cache, icon_path).unwrap();
        // let sprite3d = Sprite3d::new(&render_server, sprite_tex);

        Self {
            node_3d: Node3d::default(),
            color: ColorU::white(),
            strength: 1.0,
            // sprite: sprite3d,
            custom_update: None,
        }
    }
}

impl AsNode for PointLight {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_type(&self) -> NodeType {
        NodeType::PointLight
    }

    fn as_node_3d(&self) -> Option<&dyn AsNode3d> {
        Some(self)
    }

    fn as_node_3d_mut(&mut self) -> Option<&mut dyn AsNode3d> {
        Some(self)
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        // let queue = &mut singletons.render_server.queue;

        // self.sprite.position = new_position;
        // self.sprite.update(dt, camera_info, singletons);

        if self.custom_update.is_some() {
            self.custom_update.unwrap()(dt, self);
        }
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let point_light = PointLightUniform {
            position: self.node_3d.global_transform.position.into(),
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

impl AsNode3d for PointLight {
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

    fn get_transform(&self) -> crate::math::transform::Transform3d {
        self.node_3d.transform
    }

    fn get_global_transform(&self) -> crate::math::transform::Transform3d {
        self.node_3d.global_transform
    }

    fn set_global_transform(&mut self, transform: crate::math::transform::Transform3d) {
        self.node_3d.global_transform = transform;
    }
}
