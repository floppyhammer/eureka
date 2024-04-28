use crate::math::transform::{Transform2d, Transform3d};
use crate::scene::{AsNodeUi, Sprite2d};
use cgmath::{Quaternion, Vector2, Vector3};

pub struct Node3d {
    pub transform: Transform3d,
}

impl Default for Node3d {
    fn default() -> Self {
        Self {
            transform: Transform3d::default(),
        }
    }
}

pub trait AsNode3d {
    fn get_position(&self) -> Vector3<f32>;

    fn set_position(&mut self, position: Vector3<f32>);

    fn get_rotation(&self) -> Quaternion<f32>;

    fn set_rotation(&mut self, rotation: Quaternion<f32>);

    fn get_scale(&self) -> Vector3<f32>;

    fn set_scale(&mut self, scale: Vector3<f32>);
}
