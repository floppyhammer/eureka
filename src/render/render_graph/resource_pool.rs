use std::collections::HashMap;
use crate::render::Texture;

/// 瞬时资源池，用于在帧内复用纹理
#[derive(Default)]
pub struct ResourcePool {
    textures: HashMap<TextureKey, Vec<Texture>>,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct TextureKey {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

impl ResourcePool {
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

    pub fn release(&mut self, key: TextureKey, texture: Texture) {
        self.textures.entry(key).or_default().push(texture);
    }
}
