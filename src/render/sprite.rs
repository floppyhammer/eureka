use crate::math::transform::Transform2d;
use crate::render::TextureId;
use glam::{Vec2, Vec4};

#[derive(Debug, Copy, Clone)]
pub struct ExtractedSprite2d {
    pub(crate) transform: Transform2d,
    pub(crate) color: [f32; 4],
    pub(crate) rect: Vec4, // [min_u, min_v, max_u, max_v]
    pub(crate) size: Vec2,
    pub(crate) texture_id: TextureId, // Bindless texture ID.
    pub(crate) centered: bool,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
    pub(crate) mode: u32,
}
