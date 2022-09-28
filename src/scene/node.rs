use crate::server::input_server::InputEvent;
use crate::{Camera2d, Gizmo, InputServer, RenderServer, Singletons};
use cgmath::*;
use indextree::{Arena, Descendants, NodeEdge, NodeId};

pub enum NodeType {
    // 2D
    Camera2d,
    Sprite2d,
    SpriteVector,
    Particles2d,

    // 3D
    Camera3d,
    Sprite3d,
    Model,
    Sky,
    Light,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NodeType::Camera2d => write!(f, "Camera2d"),
            NodeType::Sprite2d => write!(f, "Sprite2d"),
            NodeType::SpriteVector => write!(f, "SpriteVector"),
            NodeType::Particles2d => write!(f, "Particles2d"),
            NodeType::Camera3d => write!(f, "Camera3d"),
            NodeType::Sprite3d => write!(f, "Sprite3d"),
            NodeType::Model => write!(f, "Model"),
            NodeType::Sky => write!(f, "Sky"),
            NodeType::Light => write!(f, "Light"),
        }
    }
}

pub trait AsNode {
    fn node_type(&self) -> NodeType;

    fn input(&mut self, input: &InputEvent);

    fn update(&mut self, dt: f32, render_server: &RenderServer, singletons: Option<&Singletons>);

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    );
}

pub struct World {
    // Type Box<dyn AsNode> is a trait object;
    // it’s a stand-in for any type inside a Box that implements the AsNode trait.

    // Scene tree.
    pub arena: Arena<Box<dyn AsNode>>,

    root_node: Option<NodeId>,

    gizmo: Gizmo,
}

impl World {
    pub fn new() -> Self {
        let mut arena = Arena::new();

        let gizmo = Gizmo::new();

        Self {
            arena,
            root_node: None,
            gizmo,
        }
    }

    pub fn add_node(&mut self, new_node: Box<dyn AsNode>, parent: Option<NodeId>) -> NodeId {
        log::info!("Added node: {}", new_node.node_type().to_string());

        let id = self.arena.new_node(new_node);

        if self.root_node.is_none() {
            self.root_node = Some(id);
        } else {
            let parent = match parent {
                None => self.root_node.unwrap(),
                Some(p) => p,
            };

            parent.append(id, &mut self.arena);
        }

        id
    }

    fn traverse(&self) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = vec![];

        match self.root_node {
            None => {
                log::warn!("No root node in the scene tree.");
            }
            Some(root) => {
                // This must return `Some(_)` since the last item to be iterated
                // by `.traverse(...)` should be `NodeEdge::End(root_id)`.
                let mut next_id = root.traverse(&self.arena).find_map(|edge| match edge {
                    NodeEdge::Start(_) => None,
                    NodeEdge::End(id) => Some(id),
                });

                while let Some(current_id) = next_id {
                    next_id = if current_id == root {
                        // This will be the last node to iterate.
                        None
                    } else if let Some(next_sib_id) = self.arena[current_id].next_sibling() {
                        next_sib_id
                            .traverse(&self.arena)
                            .find_map(|edge| match edge {
                                NodeEdge::Start(_) => None,
                                NodeEdge::End(id) => Some(id),
                            })
                    } else {
                        // No more following siblings. Go to the parent node.
                        self.arena[current_id].parent()
                    };

                    ids.push(current_id);
                }
            }
        }

        ids
    }

    pub fn input(&mut self, input_event: &InputEvent) {
        for id in self.traverse() {
            self.arena[id].get_mut().input(input_event);
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        render_server: &RenderServer,
        singletons: Option<&Singletons>,
    ) {
        for id in self.traverse() {
            self.arena[id]
                .get_mut()
                .update(dt, &render_server, singletons);
        }
    }

    pub fn draw<'a, 'b: 'a>(
        &'b mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        match self.root_node {
            None => {
                log::warn!("No root node in the scene tree.");
            }
            Some(root) => {
                let iter = root.traverse(&self.arena).filter_map(|edge| match edge {
                    NodeEdge::Start(_) => None,
                    NodeEdge::End(id) => Some(id),
                });

                for id in iter {
                    self.arena[id]
                        .get()
                        .draw(render_pass, render_server, singletons);
                }
            }
        }

        self.gizmo.draw(render_pass, render_server, singletons);
    }
}
