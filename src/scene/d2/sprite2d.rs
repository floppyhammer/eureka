use crate::animation::property::PropertyProvider;
use crate::core::singleton::Singletons;
use crate::math::transform::Transform2d;
use crate::render::draw_command::DrawCommands;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::{RawTextureData, RenderContext, Texture, TextureCache, TextureId};
use crate::scene::d2::node2d::{AsNode2d, Node2d};
use crate::scene::{AsNode, NodeType};
use glam::{Vec2, Vec4};
use std::any::Any;
use std::path::{Path, PathBuf};

pub struct Sprite2d {
    node_2d: Node2d,
    use_original_size: bool,
    pub name: String,
    pub region: Vec4,
    pub centered: bool,
    pub flip_x: bool,
    pub flip_y: bool,

    pub texture: Option<TextureId>,
    // Asynchronous loading
    pub asset_path: Option<PathBuf>,
}

impl Sprite2d {
    pub fn new(imported_texture_cache: &TextureCache, texture_id: TextureId) -> Sprite2d {
        let texture = imported_texture_cache.get(texture_id).unwrap();
        let size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);

        Self {
            node_2d: Node2d {
                size,
                ..Node2d::default()
            },
            use_original_size: true,
            name: "".to_string(),
            region: Vec4::new(0.0, 0.0, 1.0, 1.0),
            texture: Some(texture_id),
            centered: false,
            flip_x: false,
            flip_y: false,
            asset_path: None,
        }
    }

    pub fn at_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            node_2d: Node2d::default(),
            use_original_size: true,
            name: path.as_ref().to_string_lossy().into_owned(),
            region: Vec4::new(0.0, 0.0, 1.0, 1.0),
            texture: None,
            centered: false,
            flip_x: false,
            flip_y: false,
            asset_path: Some(path.as_ref().to_path_buf()),
        }
    }

    pub fn finalize(
        &mut self,
        raw: RawTextureData,
        render_server: &RenderContext,
        imported_texture_cache: &mut TextureCache,
    ) {
        let texture_id = Texture::from_raw(
            &render_server.device,
            &render_server.queue,
            imported_texture_cache,
            raw,
        );
        let texture = imported_texture_cache.get(texture_id).unwrap();

        if self.use_original_size {
            self.node_2d.size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);
        }

        self.texture = Some(texture_id);
        self.asset_path = None;
    }

    pub fn set_texture(&mut self, texture_id: TextureId) {
        self.texture = Some(texture_id);
    }
}

impl AsNode for Sprite2d {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn node_type(&self) -> NodeType {
        NodeType::Sprite2d
    }

    fn as_node_2d(&self) -> Option<&dyn AsNode2d> {
        Some(self)
    }

    fn as_node_2d_mut(&mut self) -> Option<&mut dyn AsNode2d> {
        Some(self)
    }

    fn reconcile(
        &mut self,
        singletons: &mut Singletons,
        render_world: &mut crate::render::render_world::RenderWorld,
    ) {
        if let Some(path) = &self.asset_path {
            singletons.asset_server.request_texture(path);
            if let Some(raw) = singletons.asset_server.loaded_raw_textures.get(path) {
                let raw = raw.clone();
                self.finalize(
                    raw,
                    &singletons.render_context,
                    &mut render_world.imported_texture_cache.write().unwrap(),
                );
            }
        }
    }

    fn ready(&mut self) {}

    fn update(&mut self, _dt: f32, _singletons: &mut Singletons) {}

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        if let Some(texture_id) = self.texture {
            let extracted = ExtractedSprite2d {
                transform: self.node_2d.global_transform,
                color: [1.0, 1.0, 1.0, 1.0],
                rect: self.region,
                size: self.node_2d.size,
                texture_id,
                centered: self.centered,
                flip_x: self.flip_x,
                flip_y: self.flip_y,
                mode: 0,
            };
            draw_cmds.extracted.sprites.push(extracted);
        }
    }

    fn as_property_provider_mut(&mut self) -> Option<&mut dyn PropertyProvider> {
        Some(&mut self.node_2d)
    }
}

impl AsNode2d for Sprite2d {
    fn get_size(&self) -> Vec2 {
        self.node_2d.size
    }
    fn set_size(&mut self, size: Vec2) {
        self.node_2d.size = size;
    }
    fn get_position(&self) -> Vec2 {
        self.node_2d.transform.position
    }
    fn set_position(&mut self, position: Vec2) {
        self.node_2d.transform.position = position;
    }
    fn get_rotation(&self) -> f32 {
        self.node_2d.transform.rotation
    }
    fn set_rotation(&mut self, rotation: f32) {
        self.node_2d.transform.rotation = rotation;
    }

    fn get_transform(&self) -> Transform2d {
        self.node_2d.transform
    }

    fn get_global_transform(&self) -> Transform2d {
        self.node_2d.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform2d) {
        self.node_2d.global_transform = transform;
    }
}
