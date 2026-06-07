use std::collections::{HashMap, VecDeque};
use crate::render::RenderContext;
use crate::render::render_world::RenderWorld;
use crate::render::Texture;

pub mod node;
pub mod nodes;

pub use node::*;
pub use nodes::*;

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

/// A Bevy-like Render Graph that manages rendering nodes and their execution order.
pub struct RenderGraph {
    nodes: HashMap<String, NodeState>,
    dependencies: HashMap<String, Vec<String>>,
    cached_execution_order: Option<Vec<String>>,
    pool: ResourcePool,
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
            self.cached_execution_order = None;
        }
    }

    pub fn remove_node_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        let from = from.into();
        let to = to.into();
        if let Some(deps) = self.dependencies.get_mut(&to) {
            if let Some(pos) = deps.iter().position(|x| *x == from) {
                deps.remove(pos);
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
        let mut active_resources: HashMap<String, (TextureKey, Texture)> = HashMap::new();

        // Simple topological sort for execution order
        if self.cached_execution_order.is_none() {
            self.cached_execution_order = Some(self.topological_sort());
        }
        let execution_order = self.cached_execution_order.as_ref().unwrap().clone();

        // 临时包装，方便 Node 使用
        let mut context = FrameContext {
            render_context,
            render_world,
            encoder,
            final_output_view,
            pool: &mut self.pool,
            active_resources: &mut active_resources,
        };

        for node_name in &execution_order {
            if let Some(node_state) = self.nodes.get_mut(node_name) {
                node_state.node.prepare(&mut context);
            }
        }

        for node_name in execution_order {
            if let Some(node_state) = self.nodes.get_mut(&node_name) {
                node_state.node.run(&mut context);
            }
        }

        // 帧结束，回收所有申请的资源回池中
        for (_, (key, texture)) in active_resources {
            self.pool.release(key, texture);
        }
    }

    fn topological_sort(&self) -> Vec<String> {
        let mut in_degree = HashMap::new();
        for (node, deps) in &self.dependencies {
            in_degree.entry(node.clone()).or_insert(0);
            for _dep in deps {
                *in_degree.entry(node.clone()).or_insert(0) += 1;
            }
        }

        // Reverse dependencies to find what each node enables
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

        // If result.len() != self.nodes.len(), there is a cycle, but we'll ignore it for now or just return what we have
        result
    }
}

pub struct FrameContext<'a> {
    pub render_context: &'a RenderContext<'a>,
    pub render_world: &'a mut RenderWorld,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub final_output_view: &'a wgpu::TextureView,

    pool: &'a mut ResourcePool,
    active_resources: &'a mut HashMap<String, (TextureKey, Texture)>,
}

/// 包含克隆后的句柄，不绑定生命周期
pub struct ResolvedTransientTexture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl<'a> FrameContext<'a> {
    /// 获取一个具名瞬时纹理。返回克隆的句柄以允许连续调用。
    pub fn get_texture(&mut self, name: impl Into<String>, key: TextureKey) -> ResolvedTransientTexture {
        let name = name.into();
        let (_, texture) = self.active_resources.entry(name).or_insert_with(|| {
            let tex = self.pool.acquire(&self.render_context.device, key);
            (key, tex)
        });

        ResolvedTransientTexture {
            view: texture.view.clone(),
            sampler: texture.sampler.clone(),
        }
    }
}
