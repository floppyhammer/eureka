use crate::core::singleton::Singletons;
use crate::math::rect_to_vector4;
use crate::math::transform::Transform2d;
use crate::render::atlas::{Atlas, AtlasMode, DrawAtlas, ExtractedAtlas};
use crate::render::draw_command::DrawCommands;
use crate::render::{RenderServer, TextureCache};
use crate::scene::{AsNode, NodeType};
use crate::text::FONT_ATLAS_SIZE;
use cgmath::{EuclideanSpace, Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use std::any::Any;

pub struct Label {
    text: String,

    pub transform: Transform2d,

    pub(crate) size: Vector2<f32>,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    single_line: bool,

    leading: f32,
    tracking: f32,

    /// For rendering glyph sprites.
    atlas: Option<Atlas>,
}

impl Label {
    pub fn new(texture_cache: &mut TextureCache, render_server: &RenderServer) -> Label {
        let size = Vector2::new(128.0_f32, 128.0);

        Self {
            text: "Label".to_string(),
            transform: Transform2d::default(),
            size,
            text_is_dirty: true,
            layout_is_dirty: true,
            single_line: false,
            leading: 20.0,
            tracking: 0.0,
            atlas: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.text_is_dirty = true;
    }
}

impl AsNode for Label {
    fn node_type(&self) -> NodeType {
        NodeType::Label
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        if self.text_is_dirty {
            self.atlas = Some(singletons.text_server.get_atlas(
                self.text.as_str(),
                None,
                self.transform,
                self.leading,
            ));

            self.text_is_dirty = false;
        }
    }

    fn draw(&self, draw_commands: &mut DrawCommands) {
        draw_commands.extracted.atlases.push(ExtractedAtlas {
            atlas: self.atlas.clone().unwrap(),
            view_size: draw_commands.view_info.view_size.into(),
        });
    }
}
