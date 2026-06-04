use std::collections::{HashMap, VecDeque};
use crate::render::RenderServer;
use crate::render::render_world::RenderWorld;

pub mod node;
pub mod nodes;

pub use node::*;
pub use nodes::*;

/// A Bevy-like Render Graph that manages rendering nodes and their execution order.
#[derive(Default)]
pub struct RenderGraph {
    nodes: HashMap<String, NodeState>,
    // Adjacency list for dependencies
    dependencies: HashMap<String, Vec<String>>,
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
    }

    /// Adds a dependency edge between two nodes. Node `to` will run after node `from`.
    pub fn add_node_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        let from = from.into();
        let to = to.into();
        self.dependencies.entry(to).or_default().push(from);
    }

    pub fn run(
        &mut self,
        render_server: &RenderServer,
        render_world: &RenderWorld,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        let mut context = RenderContext {
            render_server,
            render_world,
            encoder,
            output_view,
        };

        // Simple topological sort for execution order
        let execution_order = self.topological_sort();

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
    }

    fn topological_sort(&self) -> Vec<String> {
        let mut in_degree = HashMap::new();
        for (node, deps) in &self.dependencies {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
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

pub struct RenderContext<'a> {
    pub render_server: &'a RenderServer<'a>,
    pub render_world: &'a RenderWorld,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub output_view: &'a wgpu::TextureView,
}
