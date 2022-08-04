use crate::scene::AsNode;
use crate::{InputEvent, RenderServer};
use cgmath::{Point2, Vector2, Vector3};
use wgpu::util::DeviceExt;

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera2dUniform {
    view_position: [f32; 4],
    pub(crate) proj: [[f32; 4]; 4],
}

impl Camera2dUniform {
    pub(crate) fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 4],
            proj: cgmath::Matrix4::identity().into(),
        }
    }
}

pub struct Camera2d {
    pub position: Point2<f32>,

    pub view_size: Point2<u32>,

    // If this camera is active.
    current: bool,
}

impl Camera2d {
    pub fn new(
        position: Point2<f32>,
        view_size: (u32, u32),
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> Self {
        Self {
            position: position.into(),
            view_size: Point2::new(view_size.0, view_size.1),
            current: true,
        }
    }

    pub fn update(&mut self, dt: f32, queue: &wgpu::Queue) {}

    pub fn when_view_size_changes(&mut self, new_width: u32, new_height: u32) {
        self.view_size.x = new_width;
        self.view_size.x = new_height;
    }
}
