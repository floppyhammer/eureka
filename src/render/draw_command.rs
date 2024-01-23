use crate::render::camera::{CameraUniform, ViewInfo};
use crate::render::sprite::ExtractedSprite2d;
use crate::render::ExtractedMesh;
use crate::scene::LightUniform;
use crate::render::atlas::ExtractedAtlas;
use crate::render::render_world::Extracted;

#[derive(Default)]
pub struct DrawCommands {
    pub(crate) view_info: ViewInfo,
    pub(crate) extracted: Extracted,
}
