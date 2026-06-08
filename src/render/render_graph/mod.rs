use std::collections::{HashMap, VecDeque};
use crate::render::RenderContext;
use crate::render::render_world::RenderWorld;
use crate::render::Texture;

pub mod node;
pub mod nodes;
mod resource_pool;
pub mod resource;

pub use node::*;
pub use nodes::*;
pub use resource::*;
use crate::render::render_graph::resource_pool::{ResourcePool, TextureKey, BufferKey};

/// A Bevy-like Render Graph that manages rendering nodes and their execution order.
pub struct RenderGraph {
    nodes: HashMap<String, NodeState>,
    dependencies: HashMap<String, Vec<String>>,
    cached_execution_order: Option<Vec<String>>,
    pool: ResourcePool,
    frame_count: u64,
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

struct NodeState {
    node: Box<dyn Node>,
    name: String,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            dependencies: HashMap::new(),
            cached_execution_order: None,
            pool: ResourcePool::default(),
            frame_count: 0,
        }
    }

    /// Adds a new node to the graph.
    pub fn add_node<T: Node>(&mut self, name: impl Into<String>, node: T) {
        let name = name.into();
        self.nodes.insert(name.clone(), NodeState {
            node: Box::new(node),
            name: name.clone(),
        });
        self.dependencies.entry(name).or_default();
        // Reset cache.
        self.cached_execution_order = None;
    }

    pub fn get_node_mut<T: Node>(&mut self, name: &str) -> Option<&mut T> {
        self.nodes.get_mut(name).and_then(|s| s.node.as_any_mut().downcast_mut::<T>())
    }

    /// Adds a dependency edge between two nodes. Node `to` will run after node `from`.
    pub fn add_node_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        let from = from.into();
        let to = to.into();
        let deps = self.dependencies.entry(to).or_default();
        if !deps.contains(&from) {
            deps.push(from);
            // Reset cache.
            self.cached_execution_order = None;
        }
    }

    pub fn remove_node_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        let from = from.into();
        let to = to.into();
        if let Some(deps) = self.dependencies.get_mut(&to) {
            if let Some(pos) = deps.iter().position(|x| *x == from) {
                deps.remove(pos);
                // Reset cache.
                self.cached_execution_order = None;
            }
        }
    }

    pub fn run(
        &mut self,
        render_context: &RenderContext,
        render_world: &mut RenderWorld,
        encoder: &mut wgpu::CommandEncoder,
        final_output_view: &wgpu::TextureView,
    ) {
        // 1. 每帧开始时，尝试从冷却队列中回收旧资源
        self.pool.update(self.frame_count, render_context.frames_in_flight as u64);

        let mut active_textures: HashMap<ResourceId<()>, (TextureKey, Texture)> = HashMap::new();
        let mut active_buffers: HashMap<ResourceId<()>, (BufferKey, wgpu::Buffer)> = HashMap::new();

        // Simple topological sort for execution order
        if self.cached_execution_order.is_none() {
            let order = self.topological_sort();
            self.log_structure(&order);
            self.cached_execution_order = Some(order);
        }
        let execution_order = self.cached_execution_order.as_ref().unwrap().clone();

        // 验证资源依赖
        if let Err(err) = self.validate_resource_dependencies(&execution_order) {
            log::error!("Resource dependency validation failed: {}", err);
            return;
        }

        // 临时包装，方便 Node 使用
        let mut context = FrameContext {
            render_context,
            render_world,
            encoder,
            final_output_view,
            pool: &mut self.pool,
            active_textures: &mut active_textures,
            active_buffers: &mut active_buffers,
        };

        // 准备所有节点
        for node_name in &execution_order {
            if let Some(node_state) = self.nodes.get_mut(node_name) {
                node_state.node.prepare(&mut context);
            }
        }

        // 执行所有节点
        for node_name in execution_order {
            if let Some(node_state) = self.nodes.get_mut(&node_name) {
                node_state.node.run(&mut context);
            }
        }

        // 帧结束，回收所有资源
        for (_, (key, texture)) in active_textures {
            self.pool.release_texture_deferred(key, texture, self.frame_count);
        }
        for (_, (key, buffer)) in active_buffers {
            self.pool.release_buffer_deferred(key, buffer, self.frame_count);
        }

        self.frame_count += 1;
    }

    fn validate_resource_dependencies(&self, execution_order: &[String]) -> Result<(), String> {
        let mut available_resources: HashMap<ResourceId<()>, ()> = HashMap::new();

        // 添加一些内置的初始资源（如最终输出视图）
        available_resources.insert(ResourceId::new("final_output"), ());

        for node_name in execution_order {
            if let Some(node_state) = self.nodes.get(node_name) {
                // 检查输入资源是否可用
                for input_id in node_state.node.input_resources() {
                    if !available_resources.contains_key(&input_id) {
                        return Err(format!(
                            "Node '{}' requires input resource '{}' which is not available",
                            node_name, input_id
                        ));
                    }
                }

                // 将输出资源标记为可用
                for output_id in node_state.node.output_resources() {
                    available_resources.insert(output_id, ());
                }
            }
        }

        Ok(())
    }

    fn log_structure(&self, _order: &[String]) {
        log::info!("RenderGraph Topology Updated (Mermaid):");

        let mut mermaid = String::from("\n```mermaid\ngraph TD\n");
        for name in _order {
            mermaid.push_str(&format!("    {}\n", name));
        }

        for (to, froms) in &self.dependencies {
            for from in froms {
                mermaid.push_str(&format!("    {} --> {}\n", from, to));
            }
        }
        mermaid.push_str("```\n");

        log::info!("{}", mermaid);
    }

    /// Sort node running order.
    fn topological_sort(&self) -> Vec<String> {
        let mut in_degree = HashMap::new();
        for (node, deps) in &self.dependencies {
            in_degree.entry(node.clone()).or_insert(0);
            for _dep in deps {
                *in_degree.entry(node.clone()).or_insert(0) += 1;
            }
        }

        let mut enables = HashMap::new();
        for (node, deps) in &self.dependencies {
            for dep in deps {
                enables.entry(dep.clone()).or_insert_with(Vec::new).push(node.clone());
            }
        }

        let mut queue = VecDeque::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }

        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            if let Some(next_nodes) = enables.get(&node) {
                for next in next_nodes {
                    let count = in_degree.get_mut(next).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(next.clone());
                    }
                }
            }
        }

        result
    }
}

pub struct FrameContext<'a> {
    pub render_context: &'a RenderContext<'a>,
    pub render_world: &'a mut RenderWorld,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub final_output_view: &'a wgpu::TextureView,

    pool: &'a mut ResourcePool,
    active_textures: &'a mut HashMap<ResourceId<()>, (TextureKey, Texture)>,
    active_buffers: &'a mut HashMap<ResourceId<()>, (BufferKey, wgpu::Buffer)>,
}

/// 包含克隆后的句柄，不绑定生命周期
pub struct ResolvedTransientTexture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub id: u64,
}

impl<'a> FrameContext<'a> {
    /// 获取一个具名瞬时纹理。返回克隆的句柄以允许连续调用。
    pub fn get_texture(&mut self, name: impl Into<String>, key: TextureKey) -> ResolvedTransientTexture {
        let name = name.into();
        let res_id = ResourceId::new(name);
        let (_, texture) = self.active_textures.entry(res_id).or_insert_with(|| {
            let tex = self.pool.acquire_texture(&self.render_context.device, key);
            (key, tex)
        });

        ResolvedTransientTexture {
            view: texture.view.clone(),
            sampler: texture.sampler.clone(),
            id: texture.id,
        }
    }

    /// 通过类型化资源ID获取纹理
    pub fn get_texture_by_id(&mut self, id: &ResourceId<()>, key: TextureKey) -> ResolvedTransientTexture {
        let (_, texture) = self.active_textures.entry(id.clone()).or_insert_with(|| {
            let tex = self.pool.acquire_texture(&self.render_context.device, key);
            (key, tex)
        });

        ResolvedTransientTexture {
            view: texture.view.clone(),
            sampler: texture.sampler.clone(),
            id: texture.id,
        }
    }

    /// 获取一个瞬时缓冲区。返回其克隆句柄。
    pub fn get_buffer(&mut self, name: impl Into<String>, key: BufferKey) -> wgpu::Buffer {
        let name = name.into();
        let res_id = ResourceId::new(name);
        let (_, buffer) = self.active_buffers.entry(res_id).or_insert_with(|| {
            let buf = self.pool.acquire_buffer(&self.render_context.device, key);
            (key, buf)
        });

        buffer.clone()
    }

    /// 通过资源ID获取缓冲区
    pub fn get_buffer_by_id(&mut self, id: &ResourceId<()>, key: BufferKey) -> wgpu::Buffer {
        let (_, buffer) = self.active_buffers.entry(id.clone()).or_insert_with(|| {
            let buf = self.pool.acquire_buffer(&self.render_context.device, key);
            (key, buf)
        });

        buffer.clone()
    }

    /// 检查资源是否存在
    pub fn has_resource(&self, id: &ResourceId<()>) -> bool {
        self.active_textures.contains_key(id) || self.active_buffers.contains_key(id)
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
        self.pool.get_or_create_bind_group(layout, resource_ids, || creator(render_context))
    }
}
