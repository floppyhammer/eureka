use crate::render::atlas::ExtractedAtlas;
use crate::render::camera::CameraUniform;
use crate::render::render_world::Extracted;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::view::ViewInfo;
use crate::render::ExtractedMesh;

#[derive(Default)]
pub struct DrawCommands {
    pub(crate) view_info: ViewInfo,
    pub(crate) extracted: Extracted,
}
