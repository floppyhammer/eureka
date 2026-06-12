use crate::render::vertex::{Vertex3d, VertexSky};

pub struct MeshAllocator {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub sky_vertex_buffer: wgpu::Buffer,
    pub sky_index_buffer: wgpu::Buffer,

    pub vertex_count: u32,
    pub index_count: u32,
}

const DEFAULT_VERTEX_BUFFER_SIZE: u64 = 128 * 1024 * 1024; // 128MB
const DEFAULT_INDEX_BUFFER_SIZE: u64 = 64 * 1024 * 1024; // 64MB

impl MeshAllocator {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Vertex Buffer"),
            size: DEFAULT_VERTEX_BUFFER_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Index Buffer"),
            size: DEFAULT_INDEX_BUFFER_SIZE,
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

        let v_size = (v_offset as usize + vertices.len()) * size_of::<Vertex3d>();
        let i_size = (i_offset as usize + indices.len()) * size_of::<u32>();

        if v_size > self.vertex_buffer.size() as usize {
            panic!(
                "MeshAllocator: Vertex buffer overflow! Requested end: {} bytes, Buffer size: {} bytes. Consider increasing DEFAULT_VERTEX_BUFFER_SIZE.",
                v_size,
                self.vertex_buffer.size()
            );
        }

        if i_size > self.index_buffer.size() as usize {
            panic!(
                "MeshAllocator: Index buffer overflow! Requested end: {} bytes, Buffer size: {} bytes. Consider increasing DEFAULT_INDEX_BUFFER_SIZE.",
                i_size,
                self.index_buffer.size()
            );
        }

        queue.write_buffer(
            &self.vertex_buffer,
            (v_offset as usize * std::mem::size_of::<Vertex3d>()) as u64,
            bytemuck::cast_slice(vertices),
        );
        queue.write_buffer(
            &self.index_buffer,
            (i_offset as usize * std::mem::size_of::<u32>()) as u64,
            bytemuck::cast_slice(indices),
        );

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
        let v_size = (v_offset as usize + vertices.len()) * size_of::<Vertex3d>();
        let i_size = (i_offset as usize + indices.len()) * size_of::<u32>();

        if v_size > self.vertex_buffer.size() as usize {
            panic!("MeshAllocator: Vertex buffer overflow in update!");
        }
        if i_size > self.index_buffer.size() as usize {
            panic!("MeshAllocator: Index buffer overflow in update!");
        }

        queue.write_buffer(
            &self.vertex_buffer,
            (v_offset as usize * size_of::<Vertex3d>()) as u64,
            bytemuck::cast_slice(vertices),
        );
        queue.write_buffer(
            &self.index_buffer,
            (i_offset as usize * size_of::<u32>()) as u64,
            bytemuck::cast_slice(indices),
        );
    }

    pub(crate) fn setup_skybox(&self, queue: &wgpu::Queue, vertices: &[VertexSky], indices: &[u32]) {
        queue.write_buffer(&self.sky_vertex_buffer, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.sky_index_buffer, 0, bytemuck::cast_slice(indices));
    }
}
