use crate::render::render_backend::{PreparedFrame, RenderBackend};
use crate::render::render_graph::frame_context::FrameContext;
use crate::render::render_graph::resource_pool::ResourcePool;
use crate::render::render_graph::{
    standard_resources, BloomNode, ClearNode, CullingNode, FxaaNode, LightCullingNode, MeshNode,
    Node, PrepareInstancesNode, PrepareMaterialsNode, PrepareViewNode, ResourceDecl, ResourceId,
    ResourceKey, ResourceLifetime, ResourceSpec, ShadowNode, SkyboxNode, SpriteNode, SsaoNode,
    ToneMappingNode, TransparentMeshNode, VirtualResource, VolumetricApplyNode, VolumetricNode,
};
use crate::render::RenderContext;
use std::collections::{HashMap, VecDeque};

/// A Bevy-like Render Graph that manages rendering nodes and their execution order.
pub struct RenderGraph {
    nodes: HashMap<String, NodeState>,
    dependencies: HashMap<String, Vec<String>>,
    cached_execution_order: Option<Vec<String>>,
    pub pool: ResourcePool,
    frame_count: u64,
}

struct NodeState {
    node: Box<dyn Node>,
    name: String,
}

impl Default for RenderGraph {
    /// 默认实现保持轻量，不包含任何节点或资源。
    /// 这样在 std::mem::take 时非常快，且不会误重置状态。
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            dependencies: HashMap::new(),
            cached_execution_order: None,
            pool: ResourcePool::default(),
            frame_count: 0,
        }
    }
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// 初始化标准的渲染管线节点
    pub fn setup_standard_nodes(&mut self) {
        self.add_node("prepare_view", PrepareViewNode::default());
        self.add_node("prepare_materials", PrepareMaterialsNode::default());
        self.add_node("prepare_instances", PrepareInstancesNode::default());
        self.add_node("clear", ClearNode);
        self.add_node("culling", CullingNode::default());
        self.add_node("shadow", ShadowNode::default());
        self.add_node("light_culling", LightCullingNode::default());
        self.add_node("volumetric", VolumetricNode::default());
        self.add_node("volumetric_apply", VolumetricApplyNode::default());
        self.add_node("ssao", SsaoNode::default());
        self.add_node("skybox", SkyboxNode::default());
        self.add_node("mesh", MeshNode::default());
        self.add_node("transparent_mesh", TransparentMeshNode::default());
        self.add_node("bloom", BloomNode::default());
        self.add_node("tonemapping", ToneMappingNode::default());
        self.add_node("fxaa", FxaaNode::default());
        self.add_node("sprite", SpriteNode::default());

        self.add_node_edge("prepare_materials", "mesh");
        self.add_node_edge("prepare_materials", "ssao");
        self.add_node_edge("prepare_instances", "culling");
        self.add_node_edge("prepare_view", "culling");
        self.add_node_edge("prepare_view", "ssao");
        self.add_node_edge("prepare_view", "skybox");
        self.add_node_edge("prepare_view", "light_culling");
        self.add_node_edge("prepare_view", "volumetric");
        self.add_node_edge("shadow", "volumetric");
        self.add_node_edge("light_culling", "volumetric");
        self.add_node_edge("culling", "mesh");
        self.add_node_edge("culling", "ssao");
        self.add_node_edge("shadow", "mesh");
        self.add_node_edge("light_culling", "mesh");
        self.add_node_edge("volumetric", "volumetric_apply");
        self.add_node_edge("ssao", "mesh");
        self.add_node_edge("clear", "skybox");
        self.add_node_edge("skybox", "mesh");
        self.add_node_edge("mesh", "volumetric_apply");
        self.add_node_edge("volumetric_apply", "transparent_mesh");
        self.add_node_edge("transparent_mesh", "bloom");
        self.add_node_edge("bloom", "tonemapping");
        self.add_node_edge("tonemapping", "fxaa");
        self.add_node_edge("prepare_view", "sprite");
        self.add_node_edge("prepare_materials", "sprite");
        self.add_node_edge("fxaa", "sprite");
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
        backend: &mut RenderBackend,
        prepared: &PreparedFrame,
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
            self.log_structure(prepared, &order);
            self.cached_execution_order = Some(order);
        }
        let execution_order = self.cached_execution_order.as_ref().unwrap().clone();

        // 2. 预分析资源声明，进行合并和预分配
        let mut merged_decls: HashMap<ResourceId<()>, ResourceDecl> = HashMap::new();
        for node_name in &execution_order {
            if let Some(node_state) = self.nodes.get(node_name) {
                let resources = node_state.node.node_resources(prepared);
                for decl in resources
                    .inputs
                    .into_iter()
                    .chain(resources.outputs.into_iter())
                    .chain(resources.internals.into_iter())
                {
                    merged_decls
                        .entry(decl.id.clone())
                        .and_modify(|existing| {
                            existing.spec.merge(&decl.spec);
                            // 如果有一个节点要求持久，则整体持久
                            if decl.lifetime == ResourceLifetime::Persistent {
                                existing.lifetime = ResourceLifetime::Persistent;
                            }
                        })
                        .or_insert(decl);
                }
            }
        }

        let mut active_resources: HashMap<ResourceId<()>, (ResourceKey, VirtualResource)> =
            HashMap::new();

        for (id, decl) in &merged_decls {
            // 跳过内置的 final_output（一般为 Surface），它不由池管理
            if *id == standard_resources::final_output().erase() {
                continue;
            }

            let lifetime = decl.lifetime;
            match &decl.spec {
                ResourceSpec::Texture(key) => {
                    let mut key = *key;
                    if key.width == 0 {
                        key.width = render_context.surface_config.width;
                    }
                    if key.height == 0 {
                        key.height = render_context.surface_config.height;
                    }
                    if key.format.is_none() {
                        key.format = Some(render_context.surface_config.format);
                    }

                    let tex = match lifetime {
                        ResourceLifetime::Transient => {
                            self.pool.acquire_texture(&render_context.device, key)
                        }
                        ResourceLifetime::Persistent => self.pool.acquire_persistent_texture(
                            &render_context.device,
                            id.name(),
                            key,
                        ),
                    };
                    active_resources.insert(
                        id.clone(),
                        (ResourceKey::Texture(key), VirtualResource::Texture(tex)),
                    );
                }
                ResourceSpec::Buffer(key) => {
                    let key = *key;
                    if key.size == 0 {
                        continue;
                    }

                    let (buf, actual_key) = match lifetime {
                        ResourceLifetime::Transient => {
                            self.pool.acquire_buffer(&render_context.device, key)
                        }
                        ResourceLifetime::Persistent => {
                            let b = self.pool.acquire_persistent_buffer(
                                &render_context.device,
                                id.name(),
                                key,
                            );
                            (b, key)
                        }
                    };
                    active_resources.insert(
                        id.clone(),
                        (
                            ResourceKey::Buffer(actual_key),
                            VirtualResource::Buffer(buf),
                        ),
                    );
                }
                _ => {}
            }
        }

        // 验证资源依赖
        if let Err(err) = self.validate_resource_dependencies(prepared, &execution_order) {
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
                backend,
                pool: &mut self.pool,
                prepared,
                extracted: &prepared.extracted,
                encoder: &mut encoder,
                final_output_view,
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
        for (id, (key, resource)) in active_resources {
            // 获取该资源的原始声明以判断生命周期
            // 如果是持久资源，我们跳过 release_deferred，让它留在 persistent_resources 中
            let is_persistent = merged_decls
                .get(&id)
                .map_or(false, |d| d.lifetime == ResourceLifetime::Persistent);
            if is_persistent {
                continue;
            }

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

    fn validate_resource_dependencies(
        &self,
        render_prepared: &PreparedFrame,
        execution_order: &Vec<String>,
    ) -> Result<(), String> {
        let mut available_resources: HashMap<ResourceId<()>, ()> = HashMap::new();

        // 添加一些内置的初始资源（如最终输出视图）
        available_resources.insert(standard_resources::final_output().erase(), ());

        for node_name in execution_order {
            if let Some(node_state) = self.nodes.get(node_name.as_str()) {
                let resources = node_state.node.node_resources(render_prepared);

                // 检查输入资源是否可用
                for input in resources.inputs {
                    if !input.optional && !available_resources.contains_key(&input.id) {
                        return Err(format!(
                            "Node '{}' requires input resource '{}' which is not available",
                            node_name, input.id
                        ));
                    }
                }

                // 将输出和内部资源标记为可用
                for output in resources
                    .outputs
                    .into_iter()
                    .chain(resources.internals.into_iter())
                {
                    available_resources.insert(output.id, ());
                }
            }
        }

        Ok(())
    }

    fn log_structure(&self, render_prepared: &PreparedFrame, _order: &Vec<String>) {
        log::info!("RenderGraph Topology Updated (Mermaid):");

        let mut mermaid = String::from("\n```mermaid\ngraph TD\n");

        // 定义子图，每个节点作为一个子图，显示其 input/output
        for node_name in _order {
            if let Some(node_state) = self.nodes.get(node_name.as_str()) {
                let resources = node_state.node.node_resources(render_prepared);

                // 创建 mermaid 子图节点表示
                mermaid.push_str(&format!("    subgraph {}[\"{}\"]\n", node_name, node_name));

                // 收集 input 和 output 的名称
                if !resources.inputs.is_empty() {
                    let inputs: Vec<String> = resources
                        .inputs
                        .iter()
                        .map(|i| i.id.name().to_string())
                        .collect();
                    mermaid.push_str(&format!(
                        "        {}_inputs[\"Inputs:\\n{}\"]\n",
                        node_name,
                        inputs.join("\\n")
                    ));
                }

                if !resources.internals.is_empty() {
                    let internals: Vec<String> = resources
                        .internals
                        .iter()
                        .map(|i| i.id.name().to_string())
                        .collect();
                    mermaid.push_str(&format!(
                        "        {}_internals[\"Internals:\\n{}\"]\n",
                        node_name,
                        internals.join("\\n")
                    ));
                }

                if !resources.outputs.is_empty() {
                    let outputs: Vec<String> = resources
                        .outputs
                        .iter()
                        .map(|o| o.id.name().to_string())
                        .collect();
                    mermaid.push_str(&format!(
                        "        {}_outputs[\"Outputs:\\n{}\"]\n",
                        node_name,
                        outputs.join("\\n")
                    ));
                }

                mermaid.push_str("    end\n");
            }
        }

        // 添加依赖边
        for (to, froms) in &self.dependencies {
            for from in froms {
                // 直接连线节点（子图），表示节点间的逻辑依赖
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
