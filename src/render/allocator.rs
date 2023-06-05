// Refer to pathfinder/gpu/src/gpu/allocator.rs

//! GPU memory management.

use std::collections::{HashMap, VecDeque};
use std::default::Default;
use std::mem;
use std::time::Instant;
use wgpu;

// Everything above 16 MB is allocated exactly.
const MAX_BUFFER_SIZE_CLASS: u64 = 16 * 1024 * 1024;

// Number of seconds before unused memory is purged.
const DECAY_TIME: f32 = 0.250;

// Number of seconds before we can reuse an object buffer.
//
// This helps avoid stalls. This is admittedly a bit of a hack.
const REUSE_TIME: f32 = 0.015;

pub struct GpuMemoryAllocator {
    vertex_buffers_in_use: HashMap<VertexBufferId, BufferAllocation>,
    index_buffers_in_use: HashMap<IndexBufferId, BufferAllocation>,
    textures_in_use: HashMap<TextureId, TextureAllocation>,

    free_objects: VecDeque<FreeObject>,

    next_vertex_buffer_id: VertexBufferId,
    next_index_buffer_id: IndexBufferId,
    next_texture_id: TextureId,

    bytes_committed: u64,
    bytes_allocated: u64,
}

struct BufferAllocation {
    buffer: wgpu::Buffer,
    descriptor: BufferDescriptor,
    tag: BufferTag,
}

struct TextureAllocation {
    texture: wgpu::Texture,
    descriptor: TextureDescriptor,
    tag: TextureTag,
}

struct FreeObject {
    timestamp: Instant,
    kind: FreeObjectKind,
}

enum FreeObjectKind {
    VertexBuffer {
        id: VertexBufferId,
        allocation: BufferAllocation,
    },
    IndexBuffer {
        id: IndexBufferId,
        allocation: BufferAllocation,
    },
    Texture {
        id: TextureId,
        allocation: TextureAllocation,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferDescriptor {
    size: u64,
    usage: wgpu::BufferUsages,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureDescriptor {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexBufferId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexBufferId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u64);

// For debugging and profiling.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct BufferTag(pub &'static str);

// For debugging and profiling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureTag(pub &'static str);

impl GpuMemoryAllocator {
    pub fn new() -> GpuMemoryAllocator {
        GpuMemoryAllocator {
            vertex_buffers_in_use: HashMap::default(),
            index_buffers_in_use: HashMap::default(),
            textures_in_use: HashMap::default(),
            free_objects: VecDeque::new(),
            next_vertex_buffer_id: VertexBufferId(0),
            next_index_buffer_id: IndexBufferId(0),
            next_texture_id: TextureId(0),
            bytes_committed: 0,
            bytes_allocated: 0,
        }
    }

    pub fn allocate_vertex_buffer<T>(
        &mut self,
        device: &wgpu::Device,
        size: u64,
        tag: BufferTag,
    ) -> VertexBufferId {
        let mut byte_size = size * mem::size_of::<T>() as u64;
        if byte_size < MAX_BUFFER_SIZE_CLASS {
            byte_size = byte_size.next_power_of_two();
        }

        let now = Instant::now();

        for free_object_index in 0..self.free_objects.len() {
            match self.free_objects[free_object_index] {
                FreeObject {
                    ref timestamp,
                    kind: FreeObjectKind::VertexBuffer { ref allocation, .. },
                } if allocation.descriptor.size == byte_size
                    && (now - *timestamp).as_secs_f32() >= REUSE_TIME => {}
                _ => continue,
            }

            let (id, mut allocation) = match self.free_objects.remove(free_object_index) {
                Some(FreeObject {
                    kind: FreeObjectKind::VertexBuffer { id, allocation },
                    ..
                }) => (id, allocation),
                _ => unreachable!(),
            };

            allocation.tag = tag;
            self.bytes_committed += allocation.descriptor.size;
            self.vertex_buffers_in_use.insert(id, allocation);
            return id;
        }

        let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(tag.0),
            size: byte_size as wgpu::BufferAddress,
            usage,
            mapped_at_creation: false,
        });

        let id = self.next_vertex_buffer_id;
        self.next_vertex_buffer_id.0 += 1;

        let descriptor = BufferDescriptor {
            size: byte_size,
            usage,
        };

        self.vertex_buffers_in_use.insert(
            id,
            BufferAllocation {
                buffer,
                descriptor,
                tag,
            },
        );
        self.bytes_allocated += byte_size;
        self.bytes_committed += byte_size;

        id
    }

    pub fn allocate_index_buffer<T>(
        &mut self,
        device: &wgpu::Device,
        size: u64,
        tag: BufferTag,
    ) -> IndexBufferId {
        let mut byte_size = size * mem::size_of::<T>() as u64;
        if byte_size < MAX_BUFFER_SIZE_CLASS {
            byte_size = byte_size.next_power_of_two();
        }

        let now = Instant::now();

        for free_object_index in 0..self.free_objects.len() {
            match self.free_objects[free_object_index] {
                FreeObject {
                    ref timestamp,
                    kind: FreeObjectKind::IndexBuffer { ref allocation, .. },
                } if allocation.descriptor.size == byte_size
                    && (now - *timestamp).as_secs_f32() >= REUSE_TIME => {}
                _ => continue,
            }

            let (id, mut allocation) = match self.free_objects.remove(free_object_index) {
                Some(FreeObject {
                    kind: FreeObjectKind::IndexBuffer { id, allocation },
                    ..
                }) => (id, allocation),
                _ => unreachable!(),
            };

            allocation.tag = tag;
            self.bytes_committed += allocation.descriptor.size;
            self.index_buffers_in_use.insert(id, allocation);
            return id;
        }

        let usage = wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(tag.0),
            size: byte_size as wgpu::BufferAddress,
            usage,
            mapped_at_creation: false,
        });

        let id = self.next_index_buffer_id;
        self.next_index_buffer_id.0 += 1;

        let descriptor = BufferDescriptor {
            size: byte_size,
            usage,
        };

        self.index_buffers_in_use.insert(
            id,
            BufferAllocation {
                buffer,
                descriptor,
                tag,
            },
        );
        self.bytes_allocated += byte_size;
        self.bytes_committed += byte_size;

        id
    }

    pub fn allocate_texture(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
        format: wgpu::TextureFormat,
        tag: TextureTag,
    ) -> TextureId {
        let descriptor = TextureDescriptor {
            width: size.0,
            height: size.1,
            format,
        };
        let byte_size = descriptor.byte_size();

        for free_object_index in 0..self.free_objects.len() {
            match self.free_objects[free_object_index] {
                FreeObject {
                    kind: FreeObjectKind::Texture { ref allocation, .. },
                    ..
                } if allocation.descriptor == descriptor => {}
                _ => continue,
            }

            let (id, mut allocation) = match self.free_objects.remove(free_object_index) {
                Some(FreeObject {
                    kind: FreeObjectKind::Texture { id, allocation },
                    ..
                }) => (id, allocation),
                _ => unreachable!(),
            };

            allocation.tag = tag;
            self.bytes_committed += allocation.descriptor.byte_size();
            self.textures_in_use.insert(id, allocation);
            return id;
        }

        let size = wgpu::Extent3d {
            width: descriptor.width,
            height: descriptor.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(tag.0),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);
        let id = self.next_texture_id;
        self.next_texture_id.0 += 1;

        self.textures_in_use.insert(
            id,
            TextureAllocation {
                texture,
                descriptor,
                tag,
            },
        );

        self.bytes_allocated += byte_size;
        self.bytes_committed += byte_size;

        id
    }

    pub fn purge_if_needed(&mut self) {
        let now = Instant::now();
        loop {
            match self.free_objects.front() {
                Some(FreeObject { timestamp, .. })
                    if (now - *timestamp).as_secs_f32() >= DECAY_TIME => {}
                _ => break,
            }
            match self.free_objects.pop_front() {
                None => break,
                Some(FreeObject {
                    kind: FreeObjectKind::VertexBuffer { allocation, .. },
                    ..
                }) => {
                    log::debug!("purging vertex buffer: {}", allocation.descriptor.size);
                    self.bytes_allocated -= allocation.descriptor.size;
                }
                Some(FreeObject {
                    kind: FreeObjectKind::IndexBuffer { allocation, .. },
                    ..
                }) => {
                    log::debug!("purging index buffer: {}", allocation.descriptor.size);
                    self.bytes_allocated -= allocation.descriptor.size;
                }
                Some(FreeObject {
                    kind: FreeObjectKind::Texture { allocation, .. },
                    ..
                }) => {
                    log::debug!("purging texture: {:?}", allocation.descriptor);
                    self.bytes_allocated -= allocation.descriptor.byte_size();
                }
            }
        }
    }

    pub fn free_vertex_buffer(&mut self, id: VertexBufferId) {
        let allocation = self
            .vertex_buffers_in_use
            .remove(&id)
            .expect("Attempted to free unallocated buffer!");
        self.bytes_committed -= allocation.descriptor.size;
        self.free_objects.push_back(FreeObject {
            timestamp: Instant::now(),
            kind: FreeObjectKind::VertexBuffer { id, allocation },
        });
    }

    pub fn free_index_buffer(&mut self, id: IndexBufferId) {
        let allocation = self
            .index_buffers_in_use
            .remove(&id)
            .expect("Attempted to free unallocated index buffer!");
        self.bytes_committed -= allocation.descriptor.size;
        self.free_objects.push_back(FreeObject {
            timestamp: Instant::now(),
            kind: FreeObjectKind::IndexBuffer { id, allocation },
        });
    }

    pub fn free_texture(&mut self, id: TextureId) {
        let allocation = self
            .textures_in_use
            .remove(&id)
            .expect("Attempted to free unallocated texture!");
        let byte_size = allocation.descriptor.byte_size();
        self.bytes_committed -= byte_size;
        self.free_objects.push_back(FreeObject {
            timestamp: Instant::now(),
            kind: FreeObjectKind::Texture { id, allocation },
        });
    }

    pub fn get_vertex_buffer(&self, id: VertexBufferId) -> &wgpu::Buffer {
        &self.vertex_buffers_in_use[&id].buffer
    }

    pub fn get_index_buffer(&self, id: IndexBufferId) -> &wgpu::Buffer {
        &self.index_buffers_in_use[&id].buffer
    }

    pub fn get_texture(&self, id: TextureId) -> &wgpu::Texture {
        &self.textures_in_use[&id].texture
    }

    #[inline]
    pub fn bytes_allocated(&self) -> u64 {
        self.bytes_allocated
    }

    #[inline]
    pub fn bytes_committed(&self) -> u64 {
        self.bytes_committed
    }

    #[allow(dead_code)]
    pub fn dump(&self) {
        println!("GPU memory dump");
        println!("---------------");

        println!("Vertex Buffers:");
        let mut ids: Vec<VertexBufferId> = self.vertex_buffers_in_use.keys().cloned().collect();
        // ids.sort();
        for id in ids {
            let allocation = &self.vertex_buffers_in_use[&id];
            println!(
                "id {:?}: {:?} ({:?} B)",
                id, allocation.tag, allocation.descriptor.size
            );
        }

        println!("Index Buffers:");
        let mut ids: Vec<IndexBufferId> = self.index_buffers_in_use.keys().cloned().collect();
        // ids.sort();
        for id in ids {
            let allocation = &self.index_buffers_in_use[&id];
            println!(
                "id {:?}: {:?} ({:?} B)",
                id, allocation.tag, allocation.descriptor.size
            );
        }

        println!("Textures:");
        let mut ids: Vec<TextureId> = self.textures_in_use.keys().cloned().collect();
        // ids.sort();
        for id in ids {
            let allocation = &self.textures_in_use[&id];
            println!(
                "id {:?}: {:?} {:?}x{:?} {:?} ({:?} B)",
                id,
                allocation.tag,
                allocation.descriptor.width,
                allocation.descriptor.height,
                allocation.descriptor.format,
                allocation.descriptor.byte_size()
            );
        }
    }
}

impl TextureDescriptor {
    fn byte_size(&self) -> u64 {
        self.width as u64
            * self.height as u64
            * bytes_per_pixel_of_texture_format(self.format) as u64
    }
}

fn bytes_per_pixel_of_texture_format(format: wgpu::TextureFormat) -> usize {
    match format {
        wgpu::TextureFormat::Rgba8Unorm => 4,
        _ => {
            panic!()
        }
    }
}
