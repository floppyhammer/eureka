use crate::render::{RawTextureData, RenderContext, Texture, TextureCache, TextureId};
use glam::{Vec2, Vec4};
use std::path::{Path, PathBuf};

pub struct SpriteComponent {
    pub use_original_size: bool,
    pub region: Vec4,
    pub centered: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub texture: Option<TextureId>,
    pub color: [f32; 4],
}

pub struct SpriteAssetPending(pub PathBuf);

impl SpriteComponent {
    pub fn empty() -> Self {
        Self {
            use_original_size: true,
            region: Vec4::new(0.0, 0.0, 1.0, 1.0),
            centered: false,
            flip_x: false,
            flip_y: false,
            texture: None,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn finalize(
        &mut self,
        raw: RawTextureData,
        render_server: &RenderContext,
        imported_texture_cache: &mut TextureCache,
    ) -> Vec2 {
        let texture_id = Texture::from_raw(
            &render_server.device,
            &render_server.queue,
            imported_texture_cache,
            raw,
        );
        let texture = imported_texture_cache.get(texture_id).unwrap();
        self.texture = Some(texture_id);

        Vec2::new(texture.size.0 as f32, texture.size.1 as f32)
    }
}
