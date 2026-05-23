use crate::math::transform::Transform3d;
use glam::{Quat, Vec3};

pub struct Node3d {
    pub transform: Transform3d,
    pub global_transform: Transform3d,
}

impl Default for Node3d {
    fn default() -> Self {
        Self {
            transform: Transform3d::default(),
            global_transform: Transform3d::default(),
        }
    }
}

pub trait AsNode3d {
    fn get_position(&self) -> Vec3;

    fn set_position(&mut self, position: Vec3);

    fn get_rotation(&self) -> Quat;

    fn set_rotation(&mut self, rotation: Quat);

    fn get_scale(&self) -> Vec3;

    fn set_scale(&mut self, scale: Vec3);

    fn get_transform(&self) -> Transform3d;

    fn get_global_transform(&self) -> Transform3d;

    fn set_global_transform(&mut self, transform: Transform3d);
}
