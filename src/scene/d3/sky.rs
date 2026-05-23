use glam::Quat;
use std::any::Any;
use std::path::{Path, PathBuf};

use crate::render::draw_command::DrawCommands;
use crate::render::sky::ExtractedSky;
use crate::render::{RawCubeTextureData, RenderServer, Texture, TextureCache, TextureId};
use crate::scene::{AsNode, NodeType};

pub struct Sky {
    pub rotation: Quat,
    pub texture: Option<TextureId>,
    pub asset_path: Option<PathBuf>,
}

impl Sky {
    pub fn new(texture: TextureId) -> Self {
        Self {
            rotation: Quat::IDENTITY,
            texture: Some(texture),
            asset_path: None,
        }
    }

    pub fn at_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            rotation: Quat::IDENTITY,
            texture: None,
            asset_path: Some(path.as_ref().to_path_buf()),
        }
    }

    pub fn finalize(
        &mut self,
        raw: RawCubeTextureData,
        render_server: &RenderServer,
        texture_cache: &mut TextureCache,
    ) {
        let texture_id = Texture::from_raw_cube(
            &render_server.device,
            &render_server.queue,
            texture_cache,
            raw,
        );
        self.texture = Some(texture_id);
        self.asset_path = None;
    }
}

impl AsNode for Sky {
    fn node_type(&self) -> NodeType {
        NodeType::Sky
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn draw(&self, draw_commands: &mut DrawCommands) {
        if let Some(texture) = self.texture {
            draw_commands.extracted.sky = Some(ExtractedSky { texture });
        }
    }
}
