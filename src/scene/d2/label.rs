use crate::core::singleton::Singletons;
use crate::math::rect_to_vector4;
use crate::math::transform::Transform2d;
use crate::render::atlas::{Atlas, AtlasMode, DrawAtlas, ExtractedAtlas};
use crate::render::draw_command::DrawCommands;
use crate::render::{RenderServer, TextureCache};
use crate::scene::d2::node_ui::{AsNodeUi, NodeUi};
use crate::scene::{AsNode, NodeType};
use crate::text::FONT_ATLAS_SIZE;
use cgmath::{EuclideanSpace, Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use std::any::Any;
use usvg::Node;

pub struct Label {
    node_ui: NodeUi,

    text: String,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    single_line: bool,

    leading: f32,
    tracking: f32,

    /// For rendering glyph sprites.
    atlas: Option<Atlas>,
}

impl Label {
    pub fn default() -> Label {
        Self {
            node_ui: NodeUi::default(),
            text: "Label".to_string(),
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
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_type(&self) -> NodeType {
        NodeType::Label
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        if self.text_is_dirty {
            self.atlas = Some(singletons.text_server.get_atlas(
                self.text.as_str(),
                None,
                self.node_ui.transform,
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

impl AsNodeUi for Label {
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
