use crate::scene::{Camera3dUniform, CameraInfo};
use crate::server::input_server::InputEvent;
use crate::{Camera2d, Gizmo, InputServer, RenderServer, Singletons};
use cgmath::*;
use std::any::Any;

pub enum NodeType {
    // 2D
    Camera2d,
    Sprite2d,
    SpriteV,
    Label,
    Button,
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
            NodeType::SpriteV => write!(f, "SpriteV"),
            NodeType::Label => write!(f, "Label"),
            NodeType::Button => write!(f, "Button"),
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
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn node_type(&self) -> NodeType;

    // TODO: add node retrieval by path.
    // fn get_name(&self) -> String;

    /// Called when being added to the scene tree.
    fn ready(&mut self) {
        // Default implementation
    }

    // INPUT -> UPDATE -> DRAW

    fn input(&mut self, input_event: &mut InputEvent, input_server: &mut InputServer) {
        // Default implementation
    }

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        // Default implementation
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        // Default implementation
    }
}
