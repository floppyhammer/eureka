use std::collections::HashMap;
use crate::render::{Texture, NEXT_TEXTURE_ID};
use std::sync::atomic::Ordering;

/// 瞬时资源池，用于在帧内复用纹理，并支持多帧并行下的延迟回收
#[derive(Default)]
pub struct ResourcePool {
    /// 真正可以被立即领用的资源
    textures: HashMap<TextureKey, Vec<Texture>>,
    /// 处于“冷却期”的资源：(纹理, 它的Key, 释放时的帧号)
    pending: Vec<(Texture, TextureKey, u64)>,
    /// BindGroup 缓存：Key 是 (Layout地址, 资源ID列表)
    bind_group_cache: HashMap<BindGroupKey, wgpu::BindGroup>,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct BindGroupKey {
    pub layout_ptr: usize,
    pub resource_ids: Vec<u64>,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct TextureKey {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

impl ResourcePool {
    /// 每帧开始时调用，将已经度过冷却期的资源挪回可用池
    pub fn update(&mut self, current_frame: u64, frames_in_flight: u64) {
        let mut i = 0;
        while i < self.pending.len() {
            if current_frame >= self.pending[i].2 + frames_in_flight {
                let (texture, key, _) = self.pending.remove(i);
                self.textures.entry(key).or_default().push(texture);
            } else {
                i += 1;
            }
        }

        // 可选：如果缓存过大，可以在这里清理 bind_group_cache
        // 在实际项目中，当纹理被重建（如 Resize）时，通常需要清空此缓存
    }

    /// 清空 BindGroup 缓存（通常在窗口缩放或资源重建时调用）
    pub fn clear_bind_group_cache(&mut self) {
        self.bind_group_cache.clear();
    }

    pub fn acquire(&mut self, device: &wgpu::Device, key: TextureKey) -> Texture {
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

    pub fn release_deferred(&mut self, key: TextureKey, texture: Texture, frame_id: u64) {
        self.pending.push((texture, key, frame_id));
    }

    /// 获取或创建 BindGroup
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
