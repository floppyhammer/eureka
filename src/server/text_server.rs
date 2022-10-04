use std::path::Path;
use winit::event::VirtualKeyCode::P;
use crate::DynamicFont;

pub(crate) struct TextServer {
    pub(crate) font: DynamicFont,
}

impl TextServer {
    pub(crate) fn new<P: AsRef<Path>>(font_path: P) -> Self {
        let mut font = DynamicFont::load(font_path);

        Self {
            font,
        }
    }
}
