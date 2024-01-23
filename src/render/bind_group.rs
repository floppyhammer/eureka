use crate::render::{Texture, TextureCache, TextureId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupId(uuid::Uuid);

pub struct BindGroupCache {
    storage: HashMap<BindGroupId, wgpu::BindGroup>,
}

impl BindGroupCache {
    pub(crate) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(crate) fn add(&mut self, bind_group: wgpu::BindGroup) -> BindGroupId {
        let id = BindGroupId(uuid::Uuid::new_v4());
        self.storage.insert(id, bind_group);
        id
    }

    pub(crate) fn get(&self, bind_group_id: BindGroupId) -> Option<&wgpu::BindGroup> {
        self.storage.get(&bind_group_id)
    }
}
