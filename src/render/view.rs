use crate::render::TextureId;
use glam::UVec2;

enum RenderTarget {
    Window(u32),
    Image(TextureId),
}

#[derive(Clone)]
pub struct ViewInfo {
    pub id: u32,
    pub view_size: UVec2,
}

impl Default for ViewInfo {
    fn default() -> Self {
        Self {
            id: 0,
            view_size: UVec2::new(0, 0),
        }
    }
}
