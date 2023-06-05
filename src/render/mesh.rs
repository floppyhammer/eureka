use crate::render::vertex::{Vertex2d, Vertex3d, VertexSky};
use wgpu::util::DeviceExt;

/// Shared by 2D/3D meshes.
pub struct Mesh {
    // Mesh name for debugging reason. Not unique.
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    // Optional, a simple ID.
    pub material: usize,
}

impl Mesh {
    pub fn default_2d(device: &wgpu::Device) -> Mesh {
        let vertices = [
            Vertex2d {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0],
            },
            Vertex2d {
                position: [0.0, -1.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0],
            },
            Vertex2d {
                position: [1.0, -1.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0],
            },
            Vertex2d {
                position: [1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0],
            },
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 2d mesh's vertex buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 2d mesh's index buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            name: "default 2d mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material: 0,
        }
    }

    pub fn default_3d(device: &wgpu::Device) -> Mesh {
        let vertices = [
            Vertex3d {
                position: [0.0, 0.0, 0.0],
                uv: [0.0, 1.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
            Vertex3d {
                position: [1.0, 0.0, 0.0],
                uv: [1.0, 1.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
            Vertex3d {
                position: [1.0, 1.0, 0.0],
                uv: [1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
            Vertex3d {
                position: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
                tangent: [0.0, 0.0, 0.0],
                bi_tangent: [0.0, 0.0, 0.0],
            },
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 3d mesh's vertex buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default 3d mesh's index buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            name: "default 3d mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material: 0,
        }
    }

    pub fn default_skybox(device: &wgpu::Device) -> Mesh {
        let vertices = [
            VertexSky {
                position: [-1.0, -1.0, -1.0],
            },
            VertexSky {
                position: [1.0, -1.0, -1.0],
            },
            VertexSky {
                position: [1.0, 1.0, -1.0],
            },
            VertexSky {
                position: [-1.0, 1.0, -1.0],
            },
            VertexSky {
                position: [-1.0, -1.0, 1.0],
            },
            VertexSky {
                position: [1.0, -1.0, 1.0],
            },
            VertexSky {
                position: [1.0, 1.0, 1.0],
            },
            VertexSky {
                position: [-1.0, 1.0, 1.0],
            },
        ];

        let indices = [
            0, 1, 2, 2, 3, 0, 4, 6, 5, 6, 4, 7, 2, 6, 7, 2, 7, 3, 1, 5, 6, 1, 6, 2, 3, 7, 0, 4, 0,
            7, 5, 1, 4, 4, 1, 0,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default skybox mesh's vertex buffer")),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("default skybox mesh's index buffer")),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            name: "default skybox mesh".to_string(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material: 0,
        }
    }
}
