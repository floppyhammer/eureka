use crate::core::singleton::Singletons;
use crate::render::draw_command::DrawCommands;
use crate::scene::d2::AsNode2d;
use crate::scene::d3::AsNode3d;
use crate::window::input_server::InputEvent;
use crate::window::InputServer;
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
    PointLight,
    DirectionalLight,
    
    // Logic
    AnimationPlayer,
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
            NodeType::PointLight => write!(f, "PointLight"),
            NodeType::DirectionalLight => write!(f, "DirectionalLight"),
            NodeType::AnimationPlayer => write!(f, "AnimationPlayer"),
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
    // RECONCILE -> INPUT -> UPDATE -> DRAW

    /// Used for asset loading and resource finalization.
    fn reconcile(&mut self, _singletons: &mut Singletons, _render_world: &mut crate::render::render_world::RenderWorld) {
        // Default implementation
    }

    fn input(&mut self, input_event: &mut InputEvent, input_server: &mut InputServer) {
        // Default implementation
    }

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        // Default implementation
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        // Default implementation
    }

    fn as_node_2d(&self) -> Option<&dyn AsNode2d> {
        None
    }

    fn as_node_2d_mut(&mut self) -> Option<&mut dyn AsNode2d> {
        None
    }

    fn as_node_3d(&self) -> Option<&dyn AsNode3d> {
        None
    }

    fn as_node_3d_mut(&mut self) -> Option<&mut dyn AsNode3d> {
        None
    }

    fn as_property_provider_mut(&mut self) -> Option<&mut dyn crate::animation::property::PropertyProvider> {
        None
    }
}
