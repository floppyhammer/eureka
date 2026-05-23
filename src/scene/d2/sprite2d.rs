use crate::core::singleton::Singletons;
use crate::render::camera::CameraUniform;
use crate::render::draw_command::DrawCommands;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::view::ViewInfo;
use crate::render::{TextureCache, TextureId, RawTextureData, Texture, RenderServer};
use crate::scene::d2::node_ui::{AsNodeUi, NodeUi};
use crate::scene::{AsNode, NodeType};
use std::any::Any;
use std::path::{Path, PathBuf};
use glam::{Mat4, Vec2, Vec3, Vec4};

pub struct SpriteSheet {
    h_frames: u32,
    v_frames: u32,
    frame: u32,
}

pub struct Sprite2d {
    node_ui: NodeUi,
    use_original_size: bool,
    pub name: String,
    pub region: Vec4,
    pub sprite_sheet: SpriteSheet,
    pub texture: Option<TextureId>,
    pub centered: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub custom_update: Option<fn(f32, &mut Self)>,

    // Asynchronous loading
    pub asset_path: Option<PathBuf>,
}

impl Sprite2d {
    pub fn new(texture_cache: &TextureCache, texture_id: TextureId) -> Sprite2d {
        let texture = texture_cache.get(texture_id).unwrap();
        let size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);

        Self {
            node_ui: NodeUi {
                size,
                ..NodeUi::default()
            },
            use_original_size: true,
            name: "".to_string(),
            region: Vec4::new(0.0, 0.0, 1.0, 1.0),
            sprite_sheet: SpriteSheet { h_frames: 0, v_frames: 0, frame: 0 },
            texture: Some(texture_id),
            centered: false,
            flip_x: false,
            flip_y: false,
            custom_update: None,
            asset_path: None,
        }
    }

    pub fn at_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            node_ui: NodeUi::default(),
            use_original_size: true,
            name: path.as_ref().to_string_lossy().into_owned(),
            region: Vec4::new(0.0, 0.0, 1.0, 1.0),
            sprite_sheet: SpriteSheet { h_frames: 0, v_frames: 0, frame: 0 },
            texture: None,
            centered: false,
            flip_x: false,
            flip_y: false,
            custom_update: None,
            asset_path: Some(path.as_ref().to_path_buf()),
        }
    }

    pub fn finalize(
        &mut self,
        raw: RawTextureData,
        render_server: &RenderServer,
        texture_cache: &mut TextureCache,
    ) {
        let texture_id = Texture::from_raw(&render_server.device, &render_server.queue, texture_cache, raw);
        let texture = texture_cache.get(texture_id).unwrap();

        if self.use_original_size {
            self.node_ui.size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);
        }

        self.texture = Some(texture_id);
        self.asset_path = None;
    }

    pub fn set_texture(&mut self, texture_id: TextureId) {
        self.texture = Some(texture_id);
    }
}

impl AsNode for Sprite2d {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn node_type(&self) -> NodeType { NodeType::Sprite2d }
    fn ready(&mut self) {}

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        if self.custom_update.is_some() {
            self.custom_update.unwrap()(dt, self);
        }
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        if let Some(texture_id) = self.texture {
            let extracted = ExtractedSprite2d {
                transform: self.node_ui.transform,
                size: if self.use_original_size { None } else { Some(self.node_ui.size.into()) },
                texture_id,
                centered: self.centered,
                flip_x: self.flip_x,
                flip_y: self.flip_y,
            };
            draw_cmds.extracted.sprites.push(extracted);
        }
    }
}

impl AsNodeUi for Sprite2d {
    fn get_size(&self) -> Vec2 { self.node_ui.size }
    fn set_size(&mut self, size: Vec2) { self.node_ui.size = size; }
    fn get_position(&self) -> Vec2 { self.node_ui.transform.position }
    fn set_position(&mut self, position: Vec2) { self.node_ui.transform.position = position; }
    fn get_rotation(&self) -> f32 { self.node_ui.transform.rotation }
    fn set_rotation(&mut self, rotation: f32) { self.node_ui.transform.rotation = rotation; }
}
