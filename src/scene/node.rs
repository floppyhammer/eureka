use crate::render::draw_command::DrawCommands;
use crate::window::input_server::InputEvent;
use crate::{InputServer, Singletons};
use std::any::Any;

pub enum NodeType {
    // 2D
    Camera2d,
    Sprite2d,
    VectorSprite,
    Label,
    Button,

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
            NodeType::VectorSprite => write!(f, "VectorSprite"),
            NodeType::Label => write!(f, "Label"),
            NodeType::Button => write!(f, "Button"),
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

    // In a single frame, node functions will be called in the following order:
    // INPUT -> UPDATE -> DRAW

    fn input(&mut self, input_event: &mut InputEvent, input_server: &mut InputServer) {
        // Default implementation
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        // Default implementation
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        // Default implementation
    }
}
