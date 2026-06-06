use crate::render::vertex::{Vertex3d, VertexSky, Vertex2d};
use wgpu::util::DeviceExt;

pub struct MeshAllocator {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub sky_vertex_buffer: wgpu::Buffer,
    pub sky_index_buffer: wgpu::Buffer,

    pub vertex_count: u32,
    pub index_count: u32,
}

impl MeshAllocator {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Vertex Buffer"),
            size: 16 * 1024 * 1024, // 16MB
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Index Buffer"),
            size: 8 * 1024 * 1024, // 8MB
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Dedicated buffers for skybox (small and unique layout)
        let sky_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skybox Vertex Buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sky_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skybox Index Buffer"),
            size: 1024,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            sky_vertex_buffer,
            sky_index_buffer,
            vertex_count: 0,
            index_count: 0,
        }
    }

    pub fn allocate(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex3d],
        indices: &[u32],
    ) -> (u32, u32) {
        let v_offset = self.vertex_count;
        let i_offset = self.index_count;

        queue.write_buffer(&self.vertex_buffer, (v_offset as usize * std::mem::size_of::<Vertex3d>()) as u64, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.index_buffer, (i_offset * 4) as u64, bytemuck::cast_slice(indices));

        self.vertex_count += vertices.len() as u32;
        self.index_count += indices.len() as u32;

        (v_offset, i_offset)
    }

    pub fn update(
        &self,
        queue: &wgpu::Queue,
        v_offset: u32,
        i_offset: u32,
        vertices: &[Vertex3d],
        indices: &[u32],
    ) {
        queue.write_buffer(
            &self.vertex_buffer,
            (v_offset as usize * size_of::<Vertex3d>()) as u64,
            bytemuck::cast_slice(vertices),
        );
        queue.write_buffer(
            &self.index_buffer,
            (i_offset * 4) as u64,
            bytemuck::cast_slice(indices),
        );
    }

    pub fn setup_skybox(&self, queue: &wgpu::Queue, vertices: &[VertexSky], indices: &[u32]) {
        queue.write_buffer(&self.sky_vertex_buffer, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.sky_index_buffer, 0, bytemuck::cast_slice(indices));
    }
}
