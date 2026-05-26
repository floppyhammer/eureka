use crate::render::bind_group::BindGroupId;
use crate::render::{TextureCache, TextureId};
use bitflags::bitflags;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MaterialStandard {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub color_texture: Option<TextureId>,
    pub normal_texture: Option<TextureId>,
    pub metallic_roughness_texture: Option<TextureId>,
    // Bind group for the textures.
    pub texture_bind_group: Option<BindGroupId>,
    pub transparent: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl MaterialStandard {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.5,
            roughness: 0.5,
            color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            texture_bind_group: None,
            transparent: false,
        }
    }

    pub fn to_uniform(&self) -> MaterialUniform {
        MaterialUniform {
            base_color: self.base_color,
            metallic: self.metallic,
            roughness: self.roughness,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

bitflags! {
    pub struct MaterialFlags: u32 {
        const COLOR_TEXTURE = 1 << 0;
        const NORMAL_TEXTURE = 1 << 1;
        const METALLIC_ROUGHNESS_TEXTURE = 1 << 2;
        const TRANSPARENT = 1 << 3;
    }
}

impl MaterialStandard {
    pub fn get_flags(&self) -> u32 {
        let mut flags = 0;

        if self.color_texture.is_some() {
            flags = flags | MaterialFlags::COLOR_TEXTURE.bits();
        }

        if self.normal_texture.is_some() {
            flags = flags | MaterialFlags::NORMAL_TEXTURE.bits();
        }

        if self.metallic_roughness_texture.is_some() {
            flags = flags | MaterialFlags::METALLIC_ROUGHNESS_TEXTURE.bits();
        }

        flags
    }

    pub fn get_shader_defs(&self) -> Vec<&str> {
        let mut shader_defs = vec![];

        if self.color_texture.is_some() {
            shader_defs.push("COLOR_MAP");
        }

        if self.normal_texture.is_some() {
            shader_defs.push("NORMAL_MAP");
        }

        if self.metallic_roughness_texture.is_some() {
            shader_defs.push("METALLIC_ROUGHNESS_MAP");
        }

        shader_defs
    }

    pub fn get_bind_group_entries<'a>(
        &'a self,
        texture_cache: &'a TextureCache,
    ) -> Vec<wgpu::BindGroupEntry<'a>> {
        let mut bind_group_entries = vec![];

        if self.color_texture.is_some() {
            let color_texture = texture_cache.get(self.color_texture.unwrap()).unwrap();

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 0, // 纹理逻辑内使用 0, 1，外面会偏移
                resource: wgpu::BindingResource::TextureView(&color_texture.view),
            });
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&color_texture.sampler),
            });
        }

        if self.normal_texture.is_some() {
            let normal_texture = texture_cache.get(self.normal_texture.unwrap()).unwrap();

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&normal_texture.view),
            });
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
            });
        }

        if self.metallic_roughness_texture.is_some() {
            let mr_texture = texture_cache
                .get(self.metallic_roughness_texture.unwrap())
                .unwrap();

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&mr_texture.view),
            });
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(&mr_texture.sampler),
            });
        }

        bind_group_entries
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(uuid::Uuid);

pub struct MaterialCache {
    pub storage: HashMap<MaterialId, MaterialStandard>,
}

impl MaterialCache {
    pub(crate) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(crate) fn add(&mut self, material: MaterialStandard) -> MaterialId {
        let id = MaterialId(uuid::Uuid::new_v4());
        self.storage.insert(id, material);
        id
    }

    pub(crate) fn get(&self, material_id: &MaterialId) -> Option<&MaterialStandard> {
        self.storage.get(material_id)
    }
}
