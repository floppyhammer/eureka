use crate::render::vertex::Vertex3d;

pub struct MeshAllocator {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

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
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Index Buffer"),
            size: DEFAULT_INDEX_BUFFER_SIZE,
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            vertex_count: 0,
            index_count: 0,
        }
    }

    fn ensure_vertex_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        required_bytes: u64,
    ) {
        let current_size = self.vertex_buffer.size();
        if required_bytes <= current_size {
            return;
        }

        let new_size = ((current_size * 2).max(required_bytes) + 65535) & !65535;

        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Vertex Buffer"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let existing_bytes = (self.vertex_count as usize * size_of::<Vertex3d>()) as u64;
        if existing_bytes > 0 {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MeshAllocator Vertex Buffer Growth Encoder"),
            });
            encoder.copy_buffer_to_buffer(&self.vertex_buffer, 0, &new_buffer, 0, existing_bytes);
            queue.submit(Some(encoder.finish()));
        }

        log::info!(
            "MeshAllocator: Vertex buffer dynamically grown from {:.2} MB to {:.2} MB",
            current_size as f64 / 1024.0 / 1024.0,
            new_size as f64 / 1024.0 / 1024.0
        );

        self.vertex_buffer = new_buffer;
    }

    fn ensure_index_capacity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        required_bytes: u64,
    ) {
        let current_size = self.index_buffer.size();
        if required_bytes <= current_size {
            return;
        }

        let new_size = ((current_size * 2).max(required_bytes) + 65535) & !65535;

        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Global Index Buffer"),
            size: new_size,
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let existing_bytes = (self.index_count as usize * size_of::<u32>()) as u64;
        if existing_bytes > 0 {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MeshAllocator Index Buffer Growth Encoder"),
            });
            encoder.copy_buffer_to_buffer(&self.index_buffer, 0, &new_buffer, 0, existing_bytes);
            queue.submit(Some(encoder.finish()));
        }

        log::info!(
            "MeshAllocator: Index buffer dynamically grown from {:.2} MB to {:.2} MB",
            current_size as f64 / 1024.0 / 1024.0,
            new_size as f64 / 1024.0 / 1024.0
        );

        self.index_buffer = new_buffer;
    }

    pub fn allocate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex3d],
        indices: &[u32],
    ) -> (u32, u32) {
        let v_offset = self.vertex_count;
        let i_offset = self.index_count;

        let v_bytes = (v_offset as usize + vertices.len()) * size_of::<Vertex3d>();
        let i_bytes = (i_offset as usize + indices.len()) * size_of::<u32>();

        self.ensure_vertex_capacity(device, queue, v_bytes as u64);
        self.ensure_index_capacity(device, queue, i_bytes as u64);

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

        self.vertex_count += vertices.len() as u32;
        self.index_count += indices.len() as u32;

        (v_offset, i_offset)
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        v_offset: u32,
        i_offset: u32,
        vertices: &[Vertex3d],
        indices: &[u32],
    ) {
        let v_bytes = (v_offset as usize + vertices.len()) * size_of::<Vertex3d>();
        let i_bytes = (i_offset as usize + indices.len()) * size_of::<u32>();

        self.ensure_vertex_capacity(device, queue, v_bytes as u64);
        self.ensure_index_capacity(device, queue, i_bytes as u64);

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
}
