use std::any::Any;
use cgmath::{Vector3, Vector4};
use crate::render::atlas::{AtlasInstance, DrawAtlas};
use crate::{AsNode, Atlas, InputEvent, RenderServer, Singletons, Texture};
use crate::scene::NodeType;

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
            atlas: Atlas::new(render_server),
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

    fn ready(&mut self) {}

    fn input(&mut self, input: &InputEvent) {}

    fn update(&mut self, dt: f32, render_server: &RenderServer, singletons: Option<&Singletons>) {}

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        // render_pass.draw_atlas(
        // );
    }
}
