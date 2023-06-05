use crate::math::transform::Transform2d;
use crate::scene::{AsNode, NodeType};
use crate::{Singletons};
use cgmath::{Point2, Vector2, Vector3};
use std::any::Any;
use wgpu::util::DeviceExt;

pub struct Camera2d {
    pub transform: Transform2d,

    pub view_size: Point2<u32>,
}

impl Camera2d {
    pub fn new() -> Self {
        Self {
            transform: Transform2d::default(),
            view_size: Point2::new(0, 0),
        }
    }

    pub fn when_view_size_changes(&mut self, new_size: Point2<u32>) {
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
