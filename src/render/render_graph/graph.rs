use crate::render::render_graph::frame_context::FrameContext;
use crate::render::render_graph::resource_pool::ResourcePool;
use crate::render::render_graph::{
    standard_resources, Node, ResourceId, ResourceKey, ResourceSpec, VirtualResource,
};
use crate::render::render_world::RenderWorld;
use crate::render::RenderContext;
use std::collections::{HashMap, VecDeque};

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
        self.nodes.insert(
            name.clone(),
            NodeState {
                node: Box::new(node),
                name: name.clone(),
            },
        );
        self.dependencies.entry(name).or_default();
        // Reset cache.
        self.cached_execution_order = None;
    }

    pub fn get_node_mut<T: Node>(&mut self, name: &str) -> Option<&mut T> {
        self.nodes
            .get_mut(name)
            .and_then(|s| s.node.as_any_mut().downcast_mut::<T>())
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
        final_output_view: &wgpu::TextureView,
    ) -> wgpu::CommandBuffer {
        // 1. 每帧开始时，尝试从冷却队列中回收旧资源
        self.pool
            .update(self.frame_count, render_context.frames_in_flight as u64);

        // 2. 关键防御：每帧清理 BindGroup 缓存。
        self.pool.clear_bind_group_cache();

        // Simple topological sort for execution order
        if self.cached_execution_order.is_none() {
            let order = self.topological_sort();
            self.log_structure(render_world, &order);
            self.cached_execution_order = Some(order);
        }
        let execution_order = self.cached_execution_order.as_ref().unwrap().clone();

        // 2. 预分析资源声明，进行合并和预分配
        let mut merged_specs: HashMap<ResourceId<()>, ResourceSpec> = HashMap::new();
        for node_name in &execution_order {
            if let Some(node_state) = self.nodes.get(node_name) {
                let resources = node_state.node.node_resources(render_world);
                for decl in resources
                    .inputs
                    .into_iter()
                    .chain(resources.outputs.into_iter())
                {
                    merged_specs
                        .entry(decl.id)
                        .and_modify(|s| s.merge(&decl.spec))
                        .or_insert(decl.spec);
                }
            }
        }

        let mut active_resources: HashMap<ResourceId<()>, (ResourceKey, VirtualResource)> =
            HashMap::new();

        for (id, spec) in merged_specs {
            // 跳过内置的 final_output（一般为 Surface），它不由池管理
            if id == standard_resources::final_output().erase() {
                continue;
            }

            match spec {
                ResourceSpec::Texture(mut key) => {
                    // 处理 0 尺寸继承（简单实现：使用当前 surface 尺寸）
                    if key.width == 0 {
                        key.width = render_context.surface_config.width;
                    }
                    if key.height == 0 {
                        key.height = render_context.surface_config.height;
                    }
                    // 未指定格式，使用当前 surface 格式
                    if key.format.is_none() {
                        key.format = Some(render_context.surface_config.format);
                    }

                    let tex = self.pool.acquire_texture(&render_context.device, key);
                    active_resources.insert(
                        id,
                        (ResourceKey::Texture(key), VirtualResource::Texture(tex)),
                    );
                }
                ResourceSpec::Buffer(key) => {
                    // Cannot allocate buffer with zero size.
                    if key.size == 0 {
                        continue;
                    }

                    let buf = self.pool.acquire_buffer(&render_context.device, key);
                    active_resources
                        .insert(id, (ResourceKey::Buffer(key), VirtualResource::Buffer(buf)));
                }
                _ => {} // 其他资源类型暂不预分配
            }
        }

        // 验证资源依赖
        if let Err(err) = self.validate_resource_dependencies(render_world, &execution_order) {
            log::error!("Resource dependency validation failed: {}", err);
            // 降级：返回一个空的完成编码器，而不是 panic
            return render_context
                .device
                .create_command_encoder(&Default::default())
                .finish();
        }

        // --- 核心重构：独立录制逻辑 ---
        let mut encoder =
            render_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Graph Encoder"),
                });

        {
            let mut context = FrameContext {
                render_context,
                render_world,
                encoder: &mut encoder,
                final_output_view,
                pool: &mut self.pool,
                active_resources: &mut active_resources,
            };

            // 执行所有节点
            for node_name in execution_order {
                if let Some(node_state) = self.nodes.get_mut(&node_name) {
                    node_state.node.run(&mut context);
                }
            }
        }

        // 帧结束，回收所有资源
        for (_, (key, resource)) in active_resources {
            match (key, resource) {
                (ResourceKey::Texture(k), VirtualResource::Texture(t)) => {
                    self.pool.release_texture_deferred(k, t, self.frame_count);
                }
                (ResourceKey::Buffer(k), VirtualResource::Buffer(b)) => {
                    self.pool.release_buffer_deferred(k, b, self.frame_count);
                }
                _ => panic!("Resource key and type mismatch during release"),
            }
        }

        self.frame_count += 1;

        encoder.finish()
    }

    fn validate_resource_dependencies(&self, render_world: &RenderWorld, execution_order: &[String]) -> Result<(), String> {
        let mut available_resources: HashMap<ResourceId<()>, ()> = HashMap::new();

        // 添加一些内置的初始资源（如最终输出视图）
        available_resources.insert(standard_resources::final_output().erase(), ());

        for node_name in execution_order {
            if let Some(node_state) = self.nodes.get(node_name) {
                let resources = node_state.node.node_resources(render_world);

                // 检查输入资源是否可用
                for input in resources.inputs {
                    if !input.optional && !available_resources.contains_key(&input.id) {
                        return Err(format!(
                            "Node '{}' requires input resource '{}' which is not available",
                            node_name, input.id
                        ));
                    }
                }

                // 将输出资源标记为可用
                for output in resources.outputs {
                    available_resources.insert(output.id, ());
                }
            }
        }

        Ok(())
    }

    fn log_structure(&self, render_world: &mut RenderWorld, _order: &[String]) {
        log::info!("RenderGraph Topology Updated (Mermaid):");

        let mut mermaid = String::from("\n```mermaid\ngraph TD\n");

        // 定义子图，每个节点作为一个子图，显示其 input/output
        for node_name in _order {
            if let Some(node_state) = self.nodes.get(node_name) {
                let resources = node_state.node.node_resources(render_world);

                // 收集 input 和 output 的名称
                let inputs: Vec<String> = resources
                    .inputs
                    .iter()
                    .map(|i| i.id.name().to_string())
                    .collect();
                let outputs: Vec<String> = resources
                    .outputs
                    .iter()
                    .map(|o| o.id.name().to_string())
                    .collect();

                // 创建 mermaid 子图节点表示
                mermaid.push_str(&format!("    subgraph {}[\"{}\"]\n", node_name, node_name));
                mermaid.push_str(&format!(
                    "        {}_inputs[\"Inputs:\\n{}\"]\n",
                    node_name,
                    if inputs.is_empty() {
                        "none".to_string()
                    } else {
                        inputs.join("\\n")
                    }
                ));
                mermaid.push_str(&format!(
                    "        {}_outputs[\"Outputs:\\n{}\"]\n",
                    node_name,
                    if outputs.is_empty() {
                        "none".to_string()
                    } else {
                        outputs.join("\\n")
                    }
                ));
                mermaid.push_str("    end\n");
            }
        }

        // 添加依赖边
        for (to, froms) in &self.dependencies {
            for from in froms {
                mermaid.push_str(&format!("    {}_outputs --> {}_inputs\n", from, to));
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
                enables
                    .entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(node.clone());
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
