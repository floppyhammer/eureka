use crate::render::bind_group::{BindGroupCache, BindGroupId};
use crate::render::{Texture, TextureCache, TextureId};
use bitflags::bitflags;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MaterialStandard {
    pub name: String,
    pub color_texture: Option<TextureId>,
    pub normal_texture: Option<TextureId>,
    // Bind group for the textures.
    pub texture_bind_group: Option<BindGroupId>,
    pub transparent: bool,
}

bitflags! {
    pub struct MaterialFlags: u32 {
        const COLOR_TEXTURE = 1 << 0;
        const NORMAL_TEXTURE = 1 << 1;
        const TRANSPARENT = 1 << 2;
    }
}

impl MaterialStandard {
    pub fn get_flags(&self) -> u32 {
        let mut flags = 0;

        if (self.color_texture.is_some()) {
            flags = flags | MaterialFlags::COLOR_TEXTURE.bits();
        }

        if (self.normal_texture.is_some()) {
            flags = flags | MaterialFlags::NORMAL_TEXTURE.bits();
        }

        return flags;
    }

    pub fn get_shader_defs(&self) -> Vec<&str> {
        let mut shader_defs = vec![];

        if (self.color_texture.is_some()) {
            shader_defs.push("COLOR_MAP");
        }

        if (self.normal_texture.is_some()) {
            shader_defs.push("NORMAP_MAP");
        }

        return shader_defs;
    }

    pub fn get_bind_group_entries<'a>(
        &'a self,
        texture_cache: &'a TextureCache,
    ) -> Vec<wgpu::BindGroupEntry> {
        let mut bind_group_entries = vec![];
        let mut next_binding = 0;

        if self.color_texture.is_some() {
            let color_texture = texture_cache.get(self.color_texture.unwrap()).unwrap();

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: next_binding,
                resource: wgpu::BindingResource::TextureView(&color_texture.view),
            });
            next_binding += 1;

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: next_binding,
                resource: wgpu::BindingResource::Sampler(&color_texture.sampler),
            });
            next_binding += 1;
        }

        if self.normal_texture.is_some() {
            let normal_texture = texture_cache.get(self.normal_texture.unwrap()).unwrap();

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: next_binding,
                resource: wgpu::BindingResource::TextureView(&normal_texture.view),
            });
            next_binding += 1;

            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: next_binding,
                resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
            });
            next_binding += 1;
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
