use cgmath::{Point2, Vector2};
use std::rc::Rc;

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub(crate) view_position: [f32; 4],
    /// Multiplication of the view and projection matrices.
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array.
    pub(crate) view: [[f32; 4]; 4],
    pub(crate) proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub(crate) fn default() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 4],
            view: cgmath::Matrix4::identity().into(),
            proj: cgmath::Matrix4::identity().into(),
        }
    }
}

pub struct ViewInfo {
    pub id: u32,

    pub view_size: Vector2<u32>,
}

impl Default for ViewInfo {
    fn default() -> Self {
        Self {
            id: 0,
            view_size: Vector2::new(0, 0),
        }
    }
}

pub(crate) struct CameraRenderResources {
    /// A big buffer for all 3d camera uniforms. Allows using uniform buffer offset.
    pub(crate) buffer: wgpu::Buffer,
    buffer_capacity: u32,

    pub(crate) camera_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) camera_uniform_buffer: Option<wgpu::Buffer>,
}
