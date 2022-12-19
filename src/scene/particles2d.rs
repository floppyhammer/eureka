use crate::render::atlas::{AtlasInstance, DrawAtlas};
use crate::scene::{CameraInfo, NodeType};
use crate::{AsNode, Atlas, InputEvent, RenderServer, Singletons, Texture};
use cgmath::{Point2, Vector3, Vector4};
use std::any::Any;

pub struct ParticleMaterial {
    velocity: Vector3<f32>,
    force: Vector3<f32>,
}

pub struct Particles2d {
    emitting: bool,
    amount: u32,

    lifetime: f32,

    atlas: Atlas,
}

impl Particles2d {
    pub fn new(render_server: &RenderServer) -> Self {
        Self {
            emitting: true,
            amount: 8,
            lifetime: 1.0,
            atlas: Atlas::new(render_server, Point2::new(0, 0)),
        }
    }
}

impl AsNode for Particles2d {
    fn node_type(&self) -> NodeType {
        NodeType::Particles2d
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
