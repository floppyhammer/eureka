use cgmath::{Point2, Vector2, Vector3};
use wgpu::util::DeviceExt;

// We need this for Rust to store our data correctly for the shaders.
#[repr(C)]
// This is so we can store this in a buffer.
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera2dUniform {
    view_position: [f32; 4],
    proj: [[f32; 4]; 4],
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

    // CPU data
    uniform: Camera2dUniform,

    // GPU data
    buffer: wgpu::Buffer,

    pub(crate) bind_group_layout: wgpu::BindGroupLayout,

    pub(crate) bind_group: wgpu::BindGroup,
}

impl Camera2d {
    pub fn new(
        position: Point2<f32>,
        view_size: (u32, u32),
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> Self {
        // This will be used in the model shader.
        let mut uniform = Camera2dUniform::new();
        let translation = cgmath::Matrix4::from_translation(Vector3::new(1.0, 1.0, 0.0));
        let scale = cgmath::Matrix4::from_nonuniform_scale(1.0 / view_size.0 as f32, 1.0 / view_size.1 as f32, 1.0);
        uniform.proj = (scale * translation).into();

        // Create a buffer for the camera uniform.
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        // Create a bind group layout for the camera buffer.
        // Bind group layout is used to create actual bind groups.
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
                label: Some("camera_bind_group_layout"),
            });

        // Create the actual bind group.
        // A bind group describes a set of resources and how they can be accessed by a shader.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }
                ],
                label: Some("camera_bind_group"),
            });
        // ----------------------------

        Self {
            position: position.into(),
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn update(&mut self, dt: f32, queue: &wgpu::Queue) {
        // Update camera buffer.
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn when_view_size_changes(&mut self, new_width: u32, new_height: u32) {
        let translation = cgmath::Matrix4::from_translation(Vector3::new(1.0, 1.0, 0.0));
        let scale = cgmath::Matrix4::from_nonuniform_scale(1.0 / new_width as f32, 1.0 / new_height as f32, 1.0);

        self.uniform.proj = (scale * translation).into();
    }
}
