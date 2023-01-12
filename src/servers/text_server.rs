use crate::resources::{RenderServer, Texture};
use crate::DynamicFont;
use std::path::Path;
use std::time::Instant;
use winit::event::VirtualKeyCode::P;

pub struct TextServer {
    pub(crate) font: DynamicFont,
}

impl TextServer {
    pub(crate) fn new<P: AsRef<Path>>(font_path: P, render_server: &RenderServer) -> Self {
        let now = Instant::now();

        let mut font = DynamicFont::load(font_path, render_server);

        let elapsed_time = now.elapsed();
        log::info!(
            "Text server setup took {} milliseconds",
            elapsed_time.as_millis()
        );

        Self { font }
    }

    pub(crate) fn update_gpu(&mut self, render_server: &RenderServer) {
        self.font.upload(render_server);
    }
}
