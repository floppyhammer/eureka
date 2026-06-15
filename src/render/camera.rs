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

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We're using Vector4 because of the uniforms 16 byte spacing requirement.
    pub(crate) view_position: [f32; 4],
    /// Multiplication of the view and projection matrices.
    pub(crate) view: [[f32; 4]; 4],
    pub(crate) proj: [[f32; 4]; 4],
    pub(crate) view_proj: [[f32; 4]; 4],
    pub(crate) inv_proj: [[f32; 4]; 4],
    pub(crate) ssao_enabled: u32,
    pub(crate) _pad: [u32; 3],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view: Mat4::IDENTITY.to_cols_array_2d(),
            proj: Mat4::IDENTITY.to_cols_array_2d(),
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
            ssao_enabled: 0,
            _pad: [0; 3],
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

#[derive(Clone)]
pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl From<PerspectiveProjection> for Projection {
    fn from(p: PerspectiveProjection) -> Self {
        Self::Perspective(p)
    }
}

impl From<OrthographicProjection> for Projection {
    fn from(p: OrthographicProjection) -> Self {
        Self::Orthographic(p)
    }
}

impl Projection {
    pub fn update(&mut self, width: f32, height: f32) {
        match self {
            Projection::Perspective(projection) => projection.update(width, height),
            Projection::Orthographic(projection) => projection.update(width, height),
        }
    }

    pub(crate) fn calc_matrix(&self) -> Mat4 {
        match self {
            Projection::Perspective(projection) => projection.calc_matrix(),
            Projection::Orthographic(projection) => projection.calc_matrix(),
        }
    }
}

#[derive(Clone)]
/// The projection needs to change if the window (or render target) resizes.
pub struct PerspectiveProjection {
    aspect: f32,
    fovy: f32,
    // Note : near and far are always positive.
    near: f32,
    far: f32,
}

impl PerspectiveProjection {
    pub fn new(width: u32, height: u32, fovy_radians: f32, near: f32, far: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy_radians,
            near,
            far,
        }
    }

    pub fn update(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }

    /// 设置垂直视野角度（弧度）
    pub fn set_fovy(&mut self, fovy_radians: f32) {
        self.fovy = fovy_radians;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect, self.near, self.far)
    }
}

#[derive(Clone)]
pub struct OrthographicProjection {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
}

impl OrthographicProjection {
    pub fn new(near: f32, far: f32) -> Self {
        Self {
            left: 0f32,
            right: 1f32,
            bottom: 0f32,
            top: 1f32,
            near,
            far,
        }
    }

    pub(crate) fn default() -> Self {
        OrthographicProjection {
            left: 0f32,
            right: 1f32,
            bottom: 0f32,
            top: 1.0,
            near: -100.0,
            far: 100.0,
        }
    }

    pub fn update(&mut self, width: f32, height: f32) {
        let origin_x = 0f32;
        let origin_y = 0f32;

        self.left = -origin_x;
        self.top = -origin_y;
        self.right = width - origin_x;
        self.bottom = height - origin_y;
    }

    /// Get projection matrix.
    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        )
    }
}
