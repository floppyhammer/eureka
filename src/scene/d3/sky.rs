use crate::render::{RawTextureData, RenderContext, Texture, TextureCache, TextureId};
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
        raw: RawTextureData,
        render_server: &RenderContext,
        imported_texture_cache: &mut TextureCache,
        path: Option<PathBuf>,
    ) {
        // 1. 创建 Panorama 2D 纹理
        let panorama_id = Texture::from_raw(
            &render_server.device,
            &render_server.queue,
            imported_texture_cache,
            raw,
        );

        // 2. 将 Panorama 转换为 Cubemap
        let cubemap_id = Texture::from_panorama(
            &render_server.device,
            &render_server.queue,
            imported_texture_cache,
            panorama_id,
            Some("Skybox Cubemap"),
        ).expect("Failed to convert panorama to cubemap");

        if let Some(p) = path {
            imported_texture_cache.set_path(cubemap_id, p);
        }
        self.texture = Some(cubemap_id);
    }

    pub fn finalize_with_id(&mut self, texture_id: TextureId) {
        self.texture = Some(texture_id);
    }
}
