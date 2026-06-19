use crate::math::transform::{Transform2d, Transform3d};
use glam::Mat4;
use hecs::Entity;

/// 3D 局部变换组件
pub struct CTransform3d(pub Transform3d);

/// 2D 局部变换组件
pub struct CTransform2d(pub Transform2d);

/// 全局变换组件
pub struct GlobalTransform(pub Mat4);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}

