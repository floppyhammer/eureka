use crate::core::singleton::Singletons;
use crate::render::atlas::Atlas;
use crate::render::draw_command::DrawCommands;
use crate::scene::d2::node2d::{AsNode2d, Node2d};
use crate::scene::{AsNode, NodeType};
use crate::animation::property::PropertyProvider;
use glam::Vec2;
use std::any::Any;
use crate::math::transform::Transform2d;

use crate::render::sprite::ExtractedSprite2d;

pub struct Label {
    node_2d: Node2d,

    text: String,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    font_id: Option<String>,

    single_line: bool,

    leading: f32,
    tracking: f32,

    /// For rendering glyph sprites.
    atlas: Option<Atlas>,

    last_global_transform: Transform2d,
}

impl Label {
    pub fn new(text: &str) -> Label {
        Self {
            node_2d: Node2d::default(),
            text: text.to_string(),
            text_is_dirty: true,
            layout_is_dirty: true,
            font_id: None,
            single_line: false,
            leading: 20.0,
            tracking: 0.0,
            atlas: None,
            last_global_transform: Transform2d::default(),
        }
    }

    pub fn default() -> Label {
        Self {
            node_2d: Node2d::default(),
            text: "Label".to_string(),
            text_is_dirty: true,
            layout_is_dirty: true,
            font_id: None,
            single_line: false,
            leading: 20.0,
            tracking: 0.0,
            atlas: None,
            last_global_transform: Transform2d::default(),
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

    fn update(&mut self, _dt: f32, singletons: &mut Singletons) {
        let transform_changed = (self.node_2d.global_transform.position - self.last_global_transform.position).length_squared() > 0.0001
            || (self.node_2d.global_transform.rotation - self.last_global_transform.rotation).abs() > 0.0001;

        if self.text_is_dirty || self.atlas.as_ref().map_or(true, |a| a.texture.is_none()) || transform_changed {
            let atlas = singletons.text_server.get_atlas(
                self.text.as_str(),
                self.font_id.clone(),
                self.node_2d.global_transform,
                self.leading,
            );

            if atlas.texture.is_some() {
                self.text_is_dirty = false;
                self.last_global_transform = self.node_2d.global_transform;
            }
            self.atlas = Some(atlas);
        }
    }

    fn draw(&self, draw_commands: &mut DrawCommands) {
        if let Some(atlas) = &self.atlas {
            if let Some(texture_id) = atlas.texture {
                for instance in &atlas.instances {
                    // instance.position is Bottom-Left in Y-down.
                    // ExtractedSprite2d with centered=false expects Top-Left.
                    let tl_pos = Vec2::new(instance.position.x, instance.position.y - instance.size.y);
                    draw_commands.extracted.sprites.push(ExtractedSprite2d {
                        transform: Transform2d {
                            position: tl_pos,
                            rotation: 0.0,
                            scale: Vec2::ONE,
                        },
                        color: instance.color.into(),
                        rect: instance.region,
                        size: instance.size,
                        texture_id,
                        centered: false,
                        flip_x: false,
                        flip_y: false,
                        mode: 1,
                    });
                }
            }
        }
    }

    fn as_node_2d(&self) -> Option<&dyn AsNode2d> {
        Some(self)
    }

    fn as_node_2d_mut(&mut self) -> Option<&mut dyn AsNode2d> {
        Some(self)
    }

    fn as_property_provider_mut(&mut self) -> Option<&mut dyn PropertyProvider> {
        Some(&mut self.node_2d)
    }
}

impl AsNode2d for Label {
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
