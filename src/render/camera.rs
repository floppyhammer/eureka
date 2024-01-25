use std::mem;
use cgmath::{Matrix4, ortho, perspective, Point2, Rad, Vector2};
use std::rc::Rc;
use wgpu::{BufferAddress, DynamicOffset};
use crate::math::alignup_u32;
use crate::render::RenderServer;
use crate::scene::OPENGL_TO_WGPU_MATRIX;

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

    pub(crate) fn get_uniform_offset_unit() -> u32 {
        let offset_limit = wgpu::Limits::downlevel_defaults().min_uniform_buffer_offset_alignment;
        let offset_factor =
            alignup_u32(mem::size_of::<CameraUniform>() as u32, offset_limit);

        return offset_factor * offset_limit;
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
    pub(crate) uniform_buffer: Option<wgpu::Buffer>,
    uniform_buffer_capacity: usize,

    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) bind_group: Option<wgpu::BindGroup>,
}

impl CameraRenderResources {
    pub fn new(render_server: &RenderServer) -> Self {
        let bind_group_layout =
            render_server
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("mesh camera bind group layout"),
                });

        Self {
            uniform_buffer: None,
            uniform_buffer_capacity: 0,
            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn prepare_cameras(&mut self, render_server: &RenderServer, cameras: &Vec<CameraUniform>) {
        if self.uniform_buffer_capacity < cameras.len() {
            let offset_unit = CameraUniform::get_uniform_offset_unit();
            let buffer_size = offset_unit * cameras.len() as u32;

            // Create a buffer for the camera uniform.
            let buffer = render_server.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("camera uniform buffer (unique)"),
                size: buffer_size as BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group =
                render_server
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &buffer,
                                offset: 0,
                                // See DynamicUniformBufferOffset.
                                size: Some(
                                    wgpu::BufferSize::new(mem::size_of::<CameraUniform>() as u64)
                                        .unwrap(),
                                ),
                            }),
                        }],
                        label: Some("camera bind group (unique)"),
                    });

            self.bind_group = Some(bind_group);
            self.uniform_buffer = Some(buffer);
            self.uniform_buffer_capacity = cameras.len();
        }

        let offset_unit = CameraUniform::get_uniform_offset_unit();

        // Write the camera buffer.
        if self.uniform_buffer.is_some() {
            // Consider align-up.
            let mut aligned_up_data = vec![0u8; offset_unit as usize * cameras.len()];

            for i in 0..cameras.len() {
                let slice = bytemuck::cast_slice(&cameras[i..i + 1]);

                for j in 0..slice.len() {
                    aligned_up_data[i * offset_unit as usize + j] = slice[j];
                }
            }

            render_server.queue.write_buffer(
                self.uniform_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(aligned_up_data.as_slice()),
            );
        }
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
    pub(crate) fn update(&mut self, width: f32, height: f32) {
        match self {
            Projection::Perspective(projection) => projection.update(width, height),
            Projection::Orthographic(projection) => projection.update(width, height),
        }
    }

    pub(crate) fn calc_matrix(&self) -> Matrix4<f32> {
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
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl PerspectiveProjection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn update(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }

    /// Get projection matrix.
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Clone)]
pub struct OrthographicProjection {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    znear: f32,
    zfar: f32,
}

impl OrthographicProjection {
    pub fn new(znear: f32, zfar: f32) -> Self {
        Self {
            left: 0f32,
            right: 1f32,
            bottom: 0f32,
            top: 1f32,
            znear,
            zfar,
        }
    }

    pub(crate) fn default() -> Self {
        OrthographicProjection {
            left: 0f32,
            right: 1f32,
            bottom: 0f32,
            top: 1f32,
            znear: 1000.0,
            zfar: 0.0,
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
    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * ortho(self.left, self.right, self.bottom, self.top, self.znear, self.zfar)
    }
}
