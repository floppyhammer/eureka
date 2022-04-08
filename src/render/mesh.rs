use wgpu::util::DeviceExt;

/// Shared by 2D/3D meshes.
pub struct Mesh {
    pub name: String,
    // Mesh name for debugging reason.
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub material: usize, // id
}

// Every struct with this trait has to provide a desc() function.
pub trait Vertex {
    /// Vertex buffer layout provided to pipeline.
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

// Vertex data.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MeshVertex3d {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) normal: [f32; 3],

    pub(crate) tangent: [f32; 3],
    pub(crate) bi_tangent: [f32; 3],
}

impl Vertex for MeshVertex3d {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex3d>() as wgpu::BufferAddress,
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
pub(crate) struct MeshVertex2d {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
}

impl Vertex for MeshVertex2d {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex2d>() as wgpu::BufferAddress,
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

pub fn default_2d(device: &wgpu::Device) -> Mesh {
    let vertices = [
        MeshVertex2d { position: [0.0, 0.0, 0.0], uv: [1.0, 0.0] },
        MeshVertex2d { position: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
        MeshVertex2d { position: [1.0, 1.0, 0.0], uv: [0.0, 1.0] },
        MeshVertex2d { position: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
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
        num_indices: indices.len() as u32,
        material: 0,
    }
}
