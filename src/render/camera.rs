use crate::render::render_graph::BufferKey;
use glam::Mat4;

/// This only manages CPU resources.

#[derive(Clone, PartialEq)]
pub(crate) enum CameraType {
    D2,
    D3,
}

#[derive(Default, Clone)]
pub(crate) struct ExtractedCameras {
    pub(crate) types: Vec<CameraType>,
    pub(crate) uniforms: Vec<CameraUniform>,
}

impl ExtractedCameras {
    pub(crate) fn add(&mut self, camera_type: CameraType, uniform: CameraUniform) {
        self.types.push(camera_type);
        self.uniforms.push(uniform);
    }

    pub fn get_buffer_key(&self) -> BufferKey {
        let camera_count = self.uniforms.len();
        let offset_unit = CameraUniform::get_uniform_offset_unit();
        let buffer_size = offset_unit * camera_count as u32;

        BufferKey {
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    }
}

pub(crate) const CAMERA_STRUCT_WGSL: &str = include_str!("../shaders/camera.wgsl");

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub(crate) view_position: [f32; 4],
    pub(crate) view: [[f32; 4]; 4],
    pub(crate) proj: [[f32; 4]; 4],
    pub(crate) view_proj: [[f32; 4]; 4],
    pub(crate) unjittered_proj: [[f32; 4]; 4],     // 新增
    pub(crate) unjittered_view_proj: [[f32; 4]; 4],// 新增
    pub(crate) inv_proj: [[f32; 4]; 4],
    pub(crate) inv_view: [[f32; 4]; 4],
    pub(crate) inv_view_proj: [[f32; 4]; 4],
    pub(crate) inv_unjittered_view_proj: [[f32; 4]; 4], // 新增
    pub(crate) prev_view_proj: [[f32; 4]; 4],
    pub(crate) jitter: [f32; 4],
    pub(crate) ssao_enabled: u32,
    pub(crate) volumetric_enabled: u32,
    pub(crate) taa_enabled: u32,
    pub(crate) ssr_enabled: u32,
    pub(crate) ssgi_enabled: u32,
    pub(crate) frame_count: u32,
    pub(crate) _pad: [u32; 2],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view: Mat4::IDENTITY.to_cols_array_2d(),
            proj: Mat4::IDENTITY.to_cols_array_2d(),
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            unjittered_proj: Mat4::IDENTITY.to_cols_array_2d(),      // 新增
            unjittered_view_proj: Mat4::IDENTITY.to_cols_array_2d(), // 新增
            inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_view: Mat4::IDENTITY.to_cols_array_2d(),
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_unjittered_view_proj: Mat4::IDENTITY.to_cols_array_2d(), // 新增
            prev_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            jitter: [0.0; 4],
            ssao_enabled: 0,
            volumetric_enabled: 0,
            taa_enabled: 0,
            ssr_enabled: 0,
            ssgi_enabled: 0,
            frame_count: 0,
            _pad: [0; 2],
        }
    }
}

impl CameraUniform {
    pub(crate) fn get_uniform_offset_unit() -> u32 {
        let offset_alignment =
            wgpu::Limits::downlevel_defaults().min_uniform_buffer_offset_alignment;
        let size = size_of::<CameraUniform>() as u32;

        (size + offset_alignment - 1) & !(offset_alignment - 1)
    }
}


