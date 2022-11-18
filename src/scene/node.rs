use std::any::Any;
use crate::server::input_server::InputEvent;
use crate::{Camera2d, Gizmo, InputServer, RenderServer, Singletons};
use cgmath::*;

pub enum NodeType {
    // 2D
    Camera2d,
    Sprite2d,
    Label,
    SpriteVector,
    Particles2d,

    // 3D
    Camera3d,
    Sprite3d,
    Model,
    Sky,
    Light,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NodeType::Camera2d => write!(f, "Camera2d"),
            NodeType::Sprite2d => write!(f, "Sprite2d"),
            NodeType::Label => write!(f, "Label"),
            NodeType::SpriteVector => write!(f, "SpriteVector"),
            NodeType::Particles2d => write!(f, "Particles2d"),
            NodeType::Camera3d => write!(f, "Camera3d"),
            NodeType::Sprite3d => write!(f, "Sprite3d"),
            NodeType::Model => write!(f, "Model"),
            NodeType::Sky => write!(f, "Sky"),
            NodeType::Light => write!(f, "Light"),
        }
    }
}

pub trait AsNode {
    fn node_type(&self) -> NodeType;

    fn as_any(&self) -> &dyn Any;

    /// Called when being added to the scene tree.
    fn ready(&mut self) {
        // Default implementation
    }

    // INPUT -> UPDATE -> DRAW

    fn input(&mut self, input: &InputEvent) {
        // Default implementation
    }

    fn update(&mut self, dt: f32, singletons: Option<&Singletons>) {
        // Default implementation
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        // Default implementation
    }
}
