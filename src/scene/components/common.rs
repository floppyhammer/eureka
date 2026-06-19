use glam::Vec2;
use hecs::Entity;

/// 节点名称
pub struct Name(pub String);

/// 父节点引用组件
pub struct Parent(pub Entity);

/// 节点大小 (2D)
pub struct Size(pub Vec2);

/// 标识当前激活的摄像机 (Tag 组件)
pub struct ActiveCamera;

/// 3D 模型组件 (Legacy or wrapper)
pub struct MeshComponent {
    pub model_path: String,
}
