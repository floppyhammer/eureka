use crate::core::singleton::Singletons;
use crate::render::atlas::{Atlas, ExtractedAtlas};
use crate::render::draw_command::DrawCommands;
use crate::scene::d2::node_ui::{AsNodeUi, NodeUi};
use crate::scene::{AsNode, NodeType};
use glam::Vec2;
use std::any::Any;
use crate::math::transform::Transform2d;

pub struct Label {
    node_ui: NodeUi,

    text: String,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    font_id: Option<String>,

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
            font_id: None,
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

    pub fn set_font(&mut self, font_id: String) {
        self.font_id = Some(font_id);
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

    fn as_node_ui(&self) -> Option<&dyn AsNodeUi> {
        Some(self)
    }

    fn as_node_ui_mut(&mut self) -> Option<&mut dyn AsNodeUi> {
        Some(self)
    }

    fn update(&mut self, _dt: f32, singletons: &mut Singletons) {
        if self.text_is_dirty || self.atlas.as_ref().map_or(true, |a| a.texture.is_none()) {
            let atlas = singletons.text_server.get_atlas(
                self.text.as_str(),
                self.font_id.clone(),
                self.node_ui.global_transform,
                self.leading,
            );

            if atlas.texture.is_some() {
                self.text_is_dirty = false;
            }
            self.atlas = Some(atlas);
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
    fn get_size(&self) -> Vec2 {
        self.node_ui.size
    }

    fn set_size(&mut self, size: Vec2) {
        self.node_ui.size = size;
    }

    fn get_position(&self) -> Vec2 {
        self.node_ui.transform.position
    }

    fn set_position(&mut self, position: Vec2) {
        self.node_ui.transform.position = position;
    }

    fn get_rotation(&self) -> f32 {
        self.node_ui.transform.rotation
    }

    fn set_rotation(&mut self, rotation: f32) {
        self.node_ui.transform.rotation = rotation;
    }

    fn get_transform(&self) -> Transform2d {
        self.node_ui.transform
    }

    fn get_global_transform(&self) -> Transform2d {
        self.node_ui.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform2d) {
        self.node_ui.global_transform = transform;
    }
}
