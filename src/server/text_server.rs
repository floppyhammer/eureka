use std::path::Path;
use std::time::Instant;
use winit::event::VirtualKeyCode::P;
use crate::DynamicFont;

pub(crate) struct TextServer {
    pub(crate) font: DynamicFont,
}

impl TextServer {
    pub(crate) fn new<P: AsRef<Path>>(font_path: P) -> Self {
        let now = Instant::now();

        let mut font = DynamicFont::load(font_path);

        let elapsed_time = now.elapsed();
        log::info!("Text server setup took {} milliseconds", elapsed_time.as_millis());

        Self {
            font,
        }
    }
}
