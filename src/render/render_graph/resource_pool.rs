use super::resource::{BindGroupKey, BufferKey, PooledBuffer, SamplerKey, TextureKey};
use crate::render::{Texture, NEXT_TEXTURE_ID, NEXT_VIEW_ID};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 瞬时资源池，用于在帧内复用纹理和缓冲区，并支持多帧并行下的延迟回收
#[derive(Default)]
pub struct ResourcePool {
    /// 跨帧纹理池（支持 FIF 数据隔离）
    textures: HashMap<TextureKey, Vec<Texture>>, // “就绪”池
    pending_textures: Vec<(Texture, TextureKey, u64)>, // “待回收”队列

    /// 跨帧缓冲区池（支持 FIF 数据隔离）
    buffers: HashMap<BufferKey, Vec<PooledBuffer>>,
    pending_buffers: Vec<(PooledBuffer, BufferKey, u64)>,

    /// 跨帧采样器池（支持 FIF 数据隔离）
    samplers: HashMap<SamplerKey, Vec<wgpu::Sampler>>,
    pending_samplers: Vec<(wgpu::Sampler, SamplerKey, u64)>,

    /// 帧内 BindGroup 缓存，每帧清空
    frame_bind_group_cache: HashMap<BindGroupKey, wgpu::BindGroup>,

    /// 持久化存在
    bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
    pipeline_layouts: HashMap<String, wgpu::PipelineLayout>,
}

impl ResourcePool {
    pub fn update(&mut self, current_frame: u64, frames_in_flight: u64) {
        // 1. 回收纹理
        /*
        1. 检查 pending_textures：遍历这个队列。
        2. 判断安全期：如果某个纹理的 frame_count 已经距离现在超过了 MAX_FRAMES_IN_FLIGHT（通常是 2 或 3 帧），说明 GPU 肯定已经用完它了。
        3. 转移：将这个纹理从 pending_textures 移动到 textures 池中。
        4. 真正可用：从此，这个纹理才能再次被 acquire_texture 捡走。
        */
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

        // 3. 回收采样器
        let mut i = 0;
        while i < self.pending_samplers.len() {
            if current_frame >= self.pending_samplers[i].2 + frames_in_flight {
                let (sampler, key, _) = self.pending_samplers.remove(i);
                self.samplers.entry(key).or_default().push(sampler);
            } else {
                i += 1;
            }
        }
    }

    pub fn clear_bind_group_cache(&mut self) {
        self.frame_bind_group_cache.clear();
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
                depth_or_array_layers: key.layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: key.format.unwrap(),
            usage: key.usage,
            view_formats: &[],
        });

        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: if key.layers > 1 {
                Some(wgpu::TextureViewDimension::D2Array)
            } else {
                Some(wgpu::TextureViewDimension::D2)
            },
            ..Default::default()
        });

        Texture {
            size: (key.width, key.height),
            texture: wgpu_texture,
            view,
            format: key.format.unwrap(),
            id: NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed),
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            view_cache: Arc::new(RefCell::new(HashMap::new())),
        }
    }

    /// 调用后，资源并不会立即进入 textures 池。相反，它会被塞进 pending_textures，
    /// 并打上一个“时间戳”（当前的 frame_count）
    pub fn release_texture_deferred(&mut self, key: TextureKey, texture: Texture, frame_id: u64) {
        self.pending_textures.push((texture, key, frame_id));
    }

    // --- 缓冲区管理 ---

    pub fn acquire_buffer(&mut self, device: &wgpu::Device, key: BufferKey) -> PooledBuffer {
        // 尝试寻找一个大小足够且用法兼容的现有缓冲区
        let mut found_key = None;
        for (pool_key, buffers) in &mut self.buffers {
            if !buffers.is_empty()
                && pool_key.size >= key.size
                && pool_key.usage.contains(key.usage)
            {
                found_key = Some(*pool_key);
                break;
            }
        }

        if let Some(k) = found_key {
            return self.buffers.get_mut(&k).unwrap().pop().unwrap();
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

    // --- 采样器管理 ---

    pub fn acquire_sampler(&mut self, device: &wgpu::Device, key: SamplerKey) -> wgpu::Sampler {
        if let Some(samplers) = self.samplers.get_mut(&key) {
            if let Some(sampler) = samplers.pop() {
                return sampler;
            }
        }

        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: key.address_mode_u,
            address_mode_v: key.address_mode_v,
            address_mode_w: key.address_mode_w,
            mag_filter: key.mag_filter,
            min_filter: key.min_filter,
            mipmap_filter: key.mipmap_filter,
            compare: key.compare,
            lod_min_clamp: key.lod_min_clamp,
            lod_max_clamp: key.lod_max_clamp,
            ..Default::default()
        })
    }

    pub fn release_sampler_deferred(
        &mut self,
        key: SamplerKey,
        sampler: wgpu::Sampler,
        frame_id: u64,
    ) {
        self.pending_samplers.push((sampler, key, frame_id));
    }

    // --- BindGroup 管理 ---

    pub fn get_or_create_bind_group<F>(
        &mut self,
        layout: &wgpu::BindGroupLayout,
        resource_ids: Vec<u64>,
        creator: F,
    ) -> wgpu::BindGroup
    where
        F: FnOnce() -> wgpu::BindGroup,
    {
        let key = BindGroupKey {
            layout_ptr: layout as *const _ as usize,
            resource_ids,
        };
        self.frame_bind_group_cache
            .entry(key)
            .or_insert_with(creator)
            .clone()
    }

    // 固定资源存取

    pub fn add_bind_group_layout(
        &mut self,
        name: impl Into<String>,
        layout: wgpu::BindGroupLayout,
    ) {
        self.bind_group_layouts.insert(name.into(), layout);
    }

    pub fn get_bind_group_layout(&self, name: &str) -> Option<&wgpu::BindGroupLayout> {
        self.bind_group_layouts.get(name)
    }

    pub fn add_pipeline_layout(&mut self, name: impl Into<String>, layout: wgpu::PipelineLayout) {
        self.pipeline_layouts.insert(name.into(), layout);
    }

    pub fn get_pipeline_layout(&self, name: &str) -> Option<&wgpu::PipelineLayout> {
        self.pipeline_layouts.get(name)
    }
}
