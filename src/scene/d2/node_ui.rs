use crate::core::Singletons;
use crate::math::transform::Transform2d;
use crate::render::draw_command::DrawCommands;
use crate::scene::NodeType;
use crate::window::{InputEvent, InputServer};
use cgmath::Vector2;
use std::any::Any;

pub struct NodeUi {
    pub transform: Transform2d,

    pub size: Vector2<f32>,
}

impl Default for NodeUi {
    fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            size: Vector2::new(128.0_f32, 128.0),
        }
    }
}

pub trait AsNodeUi {
    fn get_size(&self) -> Vector2<f32>;

    fn set_size(&mut self, size: Vector2<f32>);

    fn get_position(&self) -> Vector2<f32>;

    fn set_position(&mut self, position: Vector2<f32>);

    fn get_rotation(&self) -> f32;

    fn set_rotation(&mut self, rotation: f32);
}
