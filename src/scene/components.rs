use glam::{Mat4, Vec3, Quat, Vec2};
use crate::math::transform::{Transform3d, Transform2d};
use hecs::Entity;

/// 3D 局部变换组件
pub struct Transform(pub Transform3d);

/// 2D 局部变换组件
pub struct Transform2dComponent(pub Transform2d);

/// 全局变换组件
pub struct GlobalTransform(pub Mat4);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Mat4::IDENTITY)
    }
}

/// 父节点引用组件
pub struct Parent(pub Entity);

/// 节点名称
pub struct Name(pub String);

/// 节点大小 (2D)
pub struct Size(pub Vec2);

/// 标识当前激活的摄像机 (Tag 组件)
pub struct ActiveCamera;

/// 3D 模型组件
pub struct MeshComponent {
    pub model_path: String,
}
