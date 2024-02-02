use crate::core::singleton::Singletons;
use crate::math::transform::Transform2d;
use crate::render::camera::CameraUniform;
use crate::render::draw_command::DrawCommands;
use crate::render::sprite::ExtractedSprite2d;
use crate::render::view::ViewInfo;
use crate::render::{Mesh, Texture, TextureCache, TextureId};
use crate::scene::d2::node_ui::{AsNodeUi, NodeUi};
use crate::scene::{AsNode, Label, NodeType};
use cgmath::{Vector2, Vector3, Vector4};
use std::any::Any;

pub struct SpriteSheet {
    h_frames: u32,
    v_frames: u32,
    frame: u32,
}

pub struct Sprite2dRenderResources {
    // A big buffer for all camera uniforms. Use offset to use different part of it.
    pub camera_buffer: wgpu::Buffer,
}

pub struct Sprite2d {
    node_ui: NodeUi,

    use_original_size: bool,

    pub name: String,

    // A portion of the texture to draw.
    pub region: Vector4<f32>,

    pub sprite_sheet: SpriteSheet,

    pub texture: Option<TextureId>,

    // pub camera_uniform: CameraUniform,
    pub centered: bool,

    pub flip_x: bool,
    pub flip_y: bool,

    pub custom_update: Option<fn(f32, &mut Self)>,
}

impl Sprite2d {
    pub fn new(texture_cache: &TextureCache, texture_id: TextureId) -> Sprite2d {
        let texture = texture_cache.get(texture_id).unwrap();

        let size = Vector2::new(texture.size.0 as f32, texture.size.1 as f32);

        let region = Vector4::new(0.0, 0.0, 1.0, 1.0);

        Self {
            node_ui: NodeUi::default(),
            use_original_size: true,
            name: "".to_string(),
            region,
            sprite_sheet: SpriteSheet {
                h_frames: 0,
                v_frames: 0,
                frame: 0,
            },
            texture: Some(texture_id),
            centered: false,
            flip_x: false,
            flip_y: false,
            custom_update: None,
        }
    }

    pub fn set_texture(&mut self, texture_id: TextureId) {
        self.texture = Some(texture_id);
    }

    pub fn calc_render_params(&self, view_info: &ViewInfo) -> CameraUniform {
        let mut camera_uniform = CameraUniform::default();

        let transform = self.node_ui.transform;

        let scaled_width = transform.scale.x * self.node_ui.size.x;
        let scaled_height = transform.scale.y * self.node_ui.size.y;

        let view_size = view_info.view_size;

        let translation = if self.centered {
            cgmath::Matrix4::from_translation(Vector3::new(
                (transform.position.x / view_size.x as f32 - scaled_width * 0.5)
                    / view_size.x as f32
                    * 2.0
                    - 1.0,
                (transform.position.y / view_size.y as f32 - scaled_height * 0.5)
                    / view_size.y as f32
                    * 2.0
                    - 1.0,
                0.0,
            ))
        } else {
            cgmath::Matrix4::from_translation(Vector3::new(
                transform.position.x / view_size.x as f32 * 2.0 - 1.0,
                transform.position.y / view_size.y as f32 * 2.0 + 1.0,
                0.0,
            ))
        };

        let scale = cgmath::Matrix4::from_nonuniform_scale(
            scaled_width / view_size.x as f32 * 2.0,
            scaled_height / view_size.y as f32 * 2.0,
            1.0,
        );

        // Note the multiplication direction (left multiplication).
        // So, scale first, translation second.
        camera_uniform.proj = (translation * scale).into();

        camera_uniform
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

    fn ready(&mut self) {}

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        if self.custom_update.is_some() {
            self.custom_update.unwrap()(dt, self);
        }
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        if (self.texture.is_none()) {
            return;
        }

        let extracted = ExtractedSprite2d {
            transform: self.node_ui.transform,
            size: if self.use_original_size {
                None
            } else {
                Some(self.node_ui.size.into())
            },
            texture_id: self.texture.unwrap(),
            centered: self.centered,
            flip_x: self.flip_x,
            flip_y: self.flip_y,
        };

        draw_cmds.extracted.sprites.push(extracted);
    }
}

impl AsNodeUi for Sprite2d {
    fn get_size(&self) -> Vector2<f32> {
        self.node_ui.size
    }

    fn set_size(&mut self, size: Vector2<f32>) {
        self.node_ui.size = size;
    }

    fn get_position(&self) -> Vector2<f32> {
        self.node_ui.transform.position
    }

    fn set_position(&mut self, position: Vector2<f32>) {
        self.node_ui.transform.position = position;
    }

    fn get_rotation(&self) -> f32 {
        self.node_ui.transform.rotation
    }

    fn set_rotation(&mut self, rotation: f32) {
        self.node_ui.transform.rotation = rotation;
    }
}
