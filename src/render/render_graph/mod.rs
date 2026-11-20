pub mod builder;
pub mod config;
mod frame_context;
mod graph;
pub mod node;
pub mod nodes;
pub mod resource;
mod resource_pool;

pub use builder::*;
pub use config::*;
pub use frame_context::*;
pub use graph::*;
pub use node::*;
pub use nodes::*;
pub use resource::*;

/// 判断当前帧提取的摄像机列表中是否存在至少一个 3D 摄像机。
/// 供所有纯 3D 渲染节点的 `is_enabled` 实现使用。
#[inline]
pub(crate) fn has_3d_camera(prepared: &crate::render::render_backend::PreparedFrame) -> bool {
    prepared
        .extracted
        .cameras
        .types
        .iter()
        .any(|t| *t == crate::render::camera::CameraType::D3)
}
