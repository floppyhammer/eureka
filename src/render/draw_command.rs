use crate::render::render_world::Extracted;
use crate::render::view::ViewInfo;

#[derive(Default)]
pub struct DrawCommands {
    pub(crate) view_info: ViewInfo,
    pub(crate) extracted: Extracted,
}
