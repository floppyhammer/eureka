use crate::render::{RawCubeTextureData, RenderContext, Texture, TextureCache, TextureId};
use glam::Quat;
use std::path::PathBuf;

pub struct SkyComponent {
    pub rotation: Quat,
    pub texture: Option<TextureId>,
}

pub struct SkyAssetPending(pub PathBuf);

impl SkyComponent {
    pub fn new(texture: TextureId) -> Self {
        Self {
            rotation: Quat::IDENTITY,
            texture: Some(texture),
        }
    }

    pub fn empty() -> Self {
        Self {
            rotation: Quat::IDENTITY,
            texture: None,
        }
    }

    pub fn finalize(
        &mut self,
        raw: RawCubeTextureData,
        render_server: &RenderContext,
        imported_texture_cache: &mut TextureCache,
    ) {
        let texture_id = Texture::from_raw_cube(
            &render_server.device,
            &render_server.queue,
            imported_texture_cache,
            raw,
        );
        self.texture = Some(texture_id);
    }
}
