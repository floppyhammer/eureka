use crate::math::transform::Transform2d;
use glam::Vec2;

pub struct NodeUi {
    pub transform: Transform2d,
    pub global_transform: Transform2d,

    pub size: Vec2,
}

impl Default for NodeUi {
    fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            global_transform: Transform2d::default(),
            size: Vec2::new(128.0_f32, 128.0),
        }
    }
}

pub trait AsNodeUi {
    fn get_size(&self) -> Vec2;

    fn set_size(&mut self, size: Vec2);

    fn get_position(&self) -> Vec2;

    fn set_position(&mut self, position: Vec2);

    fn get_rotation(&self) -> f32;

    fn set_rotation(&mut self, rotation: f32);

    fn get_transform(&self) -> Transform2d;

    fn get_global_transform(&self) -> Transform2d;

    fn set_global_transform(&mut self, transform: Transform2d);
}
