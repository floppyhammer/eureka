use std::collections::HashMap;
use crate::render::Texture;

/// 瞬时资源池，用于在帧内复用纹理，并支持多帧并行下的延迟回收
#[derive(Default)]
pub struct ResourcePool {
    /// 真正可以被立即领用的资源
    textures: HashMap<TextureKey, Vec<Texture>>,
    /// 处于“冷却期”的资源：(纹理, 它的Key, 释放时的帧号)
    pending: Vec<(Texture, TextureKey, u64)>,
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
            // 如果当前帧与释放帧的差距 >= FIF 数量，说明 GPU 已经处理完相关的旧帧指令
            if current_frame >= self.pending[i].2 + frames_in_flight {
                let (texture, key, _) = self.pending.remove(i);
                self.textures.entry(key).or_default().push(texture);
            } else {
                i += 1;
            }
        }
    }

    pub fn acquire(&mut self, device: &wgpu::Device, key: TextureKey) -> Texture {
        if let Some(textures) = self.textures.get_mut(&key) {
            if let Some(texture) = textures.pop() {
                return texture;
            }
        }

        // 如果池中没有，创建新的
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
        }
    }

    /// 延迟归还资源，记录当前释放时的帧号
    pub fn release_deferred(&mut self, key: TextureKey, texture: Texture, frame_id: u64) {
        self.pending.push((texture, key, frame_id));
    }
}
