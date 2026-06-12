use crate::render::render_world::Extracted;
use crate::render::view::ViewInfo;

#[derive(Clone, Default)]
pub struct DrawCommands {
    pub extracted: Extracted,
}
