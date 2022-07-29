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
    position: Point2<f32>,

    pub view_size: Point2<u32>,

    // If this camera is active.
    current: bool,

    pub(crate) proj: cgmath::Matrix4<f32>,
}

impl Camera2d {
    pub fn new(
        position: Point2<f32>,
        view_size: (u32, u32),
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> Self {
        let translation = cgmath::Matrix4::from_translation(Vector3::new(1.0, 1.0, 0.0));
        let scale = cgmath::Matrix4::from_nonuniform_scale(1.0 / view_size.0 as f32, 1.0 / view_size.1 as f32, 1.0);

        Self {
            position: position.into(),
            view_size: Point2::new(view_size.0, view_size.1),
            current: true,
            proj: scale * translation,
        }
    }

    pub fn update(&mut self, dt: f32, queue: &wgpu::Queue) {
    }

    pub fn when_view_size_changes(&mut self, new_width: u32, new_height: u32) {
        let translation = cgmath::Matrix4::from_translation(Vector3::new(1.0, 1.0, 0.0));
        let scale = cgmath::Matrix4::from_nonuniform_scale(1.0 / new_width as f32, 1.0 / new_height as f32, 1.0);

        self.proj = scale * translation;
    }
}
