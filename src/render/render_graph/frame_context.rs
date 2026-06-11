use crate::render::render_graph::resource_pool::ResourcePool;
use crate::render::render_graph::{
    resource, BufferId, BufferKey, PooledBuffer, ResolvedTransientTexture, ResourceId, ResourceKey,
    TextureId, TextureKey, VirtualResource,
};
use crate::render::render_world::RenderWorld;
use crate::render::RenderContext;
use std::collections::HashMap;

/// 单帧的渲染上下文
pub struct FrameContext<'a> {
    pub render_context: &'a RenderContext<'a>,
    pub render_world: &'a mut RenderWorld,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub final_output_view: &'a wgpu::TextureView,

    pub pool: &'a mut ResourcePool,
    pub(crate) active_resources: &'a mut HashMap<ResourceId<()>, (ResourceKey, VirtualResource)>,
}

impl<'a> FrameContext<'a> {
    /// 获取一个具名瞬时纹理。返回克隆的句柄以允许连续调用。
    pub fn get_texture(
        &mut self,
        name: impl Into<String>,
        key: TextureKey,
    ) -> ResolvedTransientTexture {
        let name = name.into();
        let id = TextureId::new(name);
        self.get_texture_by_id(&id, key)
    }

    /// 通过类型化资源ID获取纹理
    pub fn get_texture_by_id(
        &mut self,
        id: &TextureId,
        key: TextureKey,
    ) -> ResolvedTransientTexture {
        let res_id = id.clone().erase();
        let (_, resource) = self.active_resources.entry(res_id).or_insert_with(|| {
            let tex = self.pool.acquire_texture(&self.render_context.device, key);
            (ResourceKey::Texture(key), VirtualResource::Texture(tex))
        });

        if let VirtualResource::Texture(texture) = resource {
            ResolvedTransientTexture {
                texture: texture.texture.clone(),
                view: texture.view.clone(),
                id: texture.id,
                view_id: texture.view_id,
                handle: texture.clone(),
            }
        } else {
            panic!("Resource type mismatch: expected Texture");
        }
    }

    /// 获取一个瞬时采样器。返回克隆的句柄以允许连续调用。
    pub fn get_sampler(&mut self, key: resource::SamplerKey) -> wgpu::Sampler {
        self.pool.acquire_sampler(&self.render_context.device, key)
    }

    /// 获取一个瞬时缓冲区。返回其克隆句柄。
    pub fn get_buffer(&mut self, name: impl Into<String>, key: BufferKey) -> PooledBuffer {
        let name = name.into();
        let id = BufferId::new(name);
        self.get_buffer_by_id(&id, key)
    }

    /// 通过资源ID获取缓冲区
    pub fn get_buffer_by_id(&mut self, id: &BufferId, key: BufferKey) -> PooledBuffer {
        let res_id = id.clone().erase();
        let (_, resource) = self.active_resources.entry(res_id).or_insert_with(|| {
            let buf = self.pool.acquire_buffer(&self.render_context.device, key);
            (ResourceKey::Buffer(key), VirtualResource::Buffer(buf))
        });

        if let VirtualResource::Buffer(buffer) = resource {
            buffer.clone()
        } else {
            panic!("Resource type mismatch: expected Buffer");
        }
    }

    /// 获取一个已声明的纹理。如果资源未在 node_resources 中声明，则会 panic。
    pub fn texture(&self, id: &TextureId) -> ResolvedTransientTexture {
        let res_id = id.clone().erase();
        let resource = self
            .active_resources
            .get(&res_id)
            .map(|(_, res)| res)
            .unwrap_or_else(|| {
                panic!(
                    "Resource '{}' not found. Did you forget to declare it in node_resources()?",
                    id.name()
                );
            });

        if let VirtualResource::Texture(texture) = resource {
            ResolvedTransientTexture {
                texture: texture.texture.clone(),
                view: texture.view.clone(),
                id: texture.id,
                view_id: texture.view_id,
                handle: texture.clone(),
            }
        } else {
            panic!(
                "Resource type mismatch for '{}': expected Texture",
                id.name()
            );
        }
    }

    /// 获取一个已声明的缓冲区。如果资源未在 node_resources 中声明，则会 panic。
    pub fn buffer(&self, id: &BufferId) -> PooledBuffer {
        let res_id = id.clone().erase();
        let resource = self
            .active_resources
            .get(&res_id)
            .map(|(_, res)| res)
            .unwrap_or_else(|| {
                panic!(
                    "Resource '{}' not found. Did you forget to declare it in node_resources()?",
                    id.name()
                );
            });

        if let VirtualResource::Buffer(buffer) = resource {
            buffer.clone()
        } else {
            panic!(
                "Resource type mismatch for '{}': expected Buffer",
                id.name()
            );
        }
    }

    /// 检查资源是否存在
    pub fn has_resource<T>(&self, id: &ResourceId<T>) -> bool {
        let res_id = id.clone().erase();
        self.active_resources.contains_key(&res_id)
    }

    /// 获取或创建缓存的 BindGroup
    pub fn create_bind_group<F>(
        &mut self,
        layout: &wgpu::BindGroupLayout,
        resource_ids: Vec<u64>,
        creator: F,
    ) -> wgpu::BindGroup
    where
        F: FnOnce(&RenderContext) -> wgpu::BindGroup,
    {
        let render_context = self.render_context;
        self.pool
            .get_or_create_bind_group(layout, resource_ids, || creator(render_context))
    }
}
