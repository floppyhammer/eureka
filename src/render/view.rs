use cgmath::Vector2;
use crate::render::TextureId;

enum RenderTarget {
    Window(u32),
    Image(TextureId),
}

pub struct ViewInfo {
    pub id: u32,

    pub view_size: Vector2<u32>,
}

impl Default for ViewInfo {
    fn default() -> Self {
        Self {
            id: 0,
            view_size: Vector2::new(0, 0),
        }
    }
}
