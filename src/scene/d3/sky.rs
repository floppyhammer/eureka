use anyhow::Context;
use anyhow::*;
use cgmath::InnerSpace;
use cgmath::*;
use std::any::Any;
use std::error::Error;
use std::ops::Range;
use std::path::Path;
use std::time::Instant;
use wgpu::util::DeviceExt;

use crate::render::{Mesh, Texture, TextureId};
use crate::scene::{AsNode, NodeType};
use crate::{RenderServer, Singletons};
use crate::render::draw_command::DrawCommands;
use crate::render::material::MaterialId;
use crate::render::sky::ExtractedSky;

pub struct Sky {
    // TODO
    pub rotation: Quaternion<f32>,

    pub texture: TextureId,
}

impl Sky {
    pub fn new(texture: TextureId) -> Self {
        let rotation = cgmath::Quaternion::new(0.0, 0.0, 0.0, 0.0);

        Self {
            rotation,
            texture,
        }
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

    fn draw(
        &self, draw_commands: &mut DrawCommands,
    ) {
        draw_commands.extracted.sky = Some(ExtractedSky {
            texture: self.texture,
        });
    }
}
