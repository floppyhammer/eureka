use wgpu::util::DeviceExt;

// Every struct with this trait has to provide a desc() function.
pub trait Vertex {
    /// Vertex buffer layout provided to a pipeline.
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

// Vertex data.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex3d {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) normal: [f32; 3],

    pub(crate) tangent: [f32; 3],
    pub(crate) bi_tangent: [f32; 3],
}

impl Vertex for Vertex3d {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex3d>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { // Position.
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute { // UV.
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute { // Normal.
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Tangent and bi-tangent.
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex2d {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
}

impl Vertex for Vertex2d {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex2d>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { // Position.
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute { // UV.
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Shared by 2D/3D meshes.
pub struct Mesh {
    // Mesh name for debugging reason.
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    // A simple ID.
    pub material: usize,
}

pub fn default_2d(device: &wgpu::Device) -> Mesh {
    let vertices = [
        Vertex2d { position: [0.0, 0.0, 0.0], uv: [1.0, 0.0] },
        Vertex2d { position: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
        Vertex2d { position: [1.0, 1.0, 0.0], uv: [0.0, 1.0] },
        Vertex2d { position: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
    ];

    let indices = [
        0, 1, 2,
        2, 3, 0,
    ];

    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Default 2D Mesh Vertex Buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    );
    
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Default 2D Mesh Index Buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }
    );
    
    Mesh {
        name: "Default 2D Mesh".to_string(),
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
        material: 0,
    }
}
