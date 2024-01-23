use crate::math::transform::Transform2d;
use crate::scene::{AsNode, NodeType};
use cgmath::{Point2, Vector2};
use std::any::Any;

pub struct Camera2d {
    pub transform: Transform2d,

    pub view_size: Vector2<u32>,

    /// Where to draw. None for screen.
    pub view: Option<u32>,
}

impl Camera2d {
    pub fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            view_size: Vector2::new(0, 0),
            view: None,
        }
    }

    pub fn when_view_size_changes(&mut self, new_size: Vector2<u32>) {
        self.view_size = new_size;
    }
}

impl AsNode for Camera2d {
    fn node_type(&self) -> NodeType {
        NodeType::Camera2d
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
