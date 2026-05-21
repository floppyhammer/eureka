use std::any::Any;
use glam::Quat;

use crate::render::draw_command::DrawCommands;
use crate::render::sky::ExtractedSky;
use crate::render::TextureId;
use crate::scene::{AsNode, NodeType};

pub struct Sky {
    // TODO
    pub rotation: Quat,

    pub texture: TextureId,
}

impl Sky {
    pub fn new(texture: TextureId) -> Self {
        let rotation = Quat::IDENTITY;

        Self { rotation, texture }
    }
}

impl AsNode for Sky {
    fn node_type(&self) -> NodeType {
        NodeType::Sky
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn draw(&self, draw_commands: &mut DrawCommands) {
        draw_commands.extracted.sky = Some(ExtractedSky {
            texture: self.texture,
        });
    }
}
