use crate::math::transform::Transform2d;
use crate::scene::{AsNode, NodeType};
use crate::{InputEvent, RenderServer, Singletons};
use cgmath::{Point2, Vector2, Vector3};
use std::any::Any;
use wgpu::util::DeviceExt;

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera2dUniform {
    pos: [f32; 4],
    pub(crate) proj: [[f32; 4]; 4],
}

impl Camera2dUniform {
    pub(crate) fn default() -> Self {
        use cgmath::SquareMatrix;
        Self {
            pos: [0.0; 4],
            proj: cgmath::Matrix4::identity().into(),
        }
    }
}

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
