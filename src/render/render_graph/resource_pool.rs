use std::collections::HashMap;
use crate::render::{Texture, NEXT_TEXTURE_ID};
use std::sync::atomic::Ordering;
use super::resource::{TextureKey, BufferKey, PooledBuffer, BindGroupKey};

/// 瞬时资源池，用于在帧内复用纹理和缓冲区，并支持多帧并行下的延迟回收
#[derive(Default)]
pub struct ResourcePool {
    /// 纹理池
    textures: HashMap<TextureKey, Vec<Texture>>,
    pending_textures: Vec<(Texture, TextureKey, u64)>,

    /// 缓冲区池
    buffers: HashMap<BufferKey, Vec<PooledBuffer>>,
    pending_buffers: Vec<(PooledBuffer, BufferKey, u64)>,

    /// BindGroup 缓存
    bind_group_cache: HashMap<BindGroupKey, wgpu::BindGroup>,
}

impl ResourcePool {
    pub fn update(&mut self, current_frame: u64, frames_in_flight: u64) {
        // 1. 回收纹理
        let mut i = 0;
        while i < self.pending_textures.len() {
            if current_frame >= self.pending_textures[i].2 + frames_in_flight {
                let (texture, key, _) = self.pending_textures.remove(i);
                self.textures.entry(key).or_default().push(texture);
            } else {
                i += 1;
            }
        }

        // 2. 回收缓冲区
        let mut i = 0;
        while i < self.pending_buffers.len() {
            if current_frame >= self.pending_buffers[i].2 + frames_in_flight {
                let (buffer, key, _) = self.pending_buffers.remove(i);
                self.buffers.entry(key).or_default().push(buffer);
            } else {
                i += 1;
            }
        }
    }

    pub fn clear_bind_group_cache(&mut self) {
        self.bind_group_cache.clear();
    }

    // --- 纹理管理 ---

    pub fn acquire_texture(&mut self, device: &wgpu::Device, key: TextureKey) -> Texture {
        if let Some(textures) = self.textures.get_mut(&key) {
            if let Some(texture) = textures.pop() {
                return texture;
            }
        }

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("transient_texture"),
            size: wgpu::Extent3d {
                width: key.width,
                height: key.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: key.format,
            usage: key.usage,
            view_formats: &[],
        });

        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Texture {
            size: (key.width, key.height),
            texture: wgpu_texture,
            view,
            sampler,
            format: key.format,
            id: NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn release_texture_deferred(&mut self, key: TextureKey, texture: Texture, frame_id: u64) {
        self.pending_textures.push((texture, key, frame_id));
    }

    // --- 缓冲区管理 ---

    pub fn acquire_buffer(&mut self, device: &wgpu::Device, key: BufferKey) -> PooledBuffer {
        if let Some(buffers) = self.buffers.get_mut(&key) {
            if let Some(buffer) = buffers.pop() {
                return buffer;
            }
        }

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transient_buffer"),
            size: key.size,
            usage: key.usage | wgpu::BufferUsages::COPY_DST, // 强制包含 COPY_DST 方便写入
            mapped_at_creation: false,
        });

        PooledBuffer {
            buffer,
            id: NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn release_buffer_deferred(&mut self, key: BufferKey, buffer: PooledBuffer, frame_id: u64) {
        self.pending_buffers.push((buffer, key, frame_id));
    }

    // --- BindGroup 管理 ---

    pub fn get_or_create_bind_group<F>(
        &mut self,
        layout: &wgpu::BindGroupLayout,
        resource_ids: Vec<u64>,
        creator: F,
    ) -> wgpu::BindGroup
    where
        F: FnOnce() -> wgpu::BindGroup
    {
        let key = BindGroupKey {
            layout_ptr: layout as *const _ as usize,
            resource_ids,
        };
        self.bind_group_cache.entry(key).or_insert_with(creator).clone()
    }
}
