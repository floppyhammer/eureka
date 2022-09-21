use crate::server::input_server::InputEvent;
use crate::{Camera2d, Gizmo, InputServer, RenderServer, Singletons};
use cgmath::*;
use indextree::{Arena, NodeId, Descendants, NodeEdge};
use crate::scene::scene_tree::Node;

pub trait AsNode {
    fn input(&mut self, input: &InputEvent);

    fn update(
        &mut self,
        queue: &wgpu::Queue,
        dt: f32,
        render_server: &RenderServer,
        singletons: Option<&Singletons>,
    );

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
        let id = self.arena.new_node(new_node);

        if self.root_node.is_none() {
            self.root_node = Some(id);
        } else {
            let parent = match parent {
                None => {
                    self.root_node.unwrap()
                }
                Some(p) => {
                    p
                }
            };

            parent.append(id, &mut self.arena);
        }

        id
    }

    pub fn input(&mut self, input_event: &InputEvent) {
        // Handle input.
        for node in self.arena.iter_mut() {
            node.get_mut().input(input_event);
        }
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        dt: f32,
        render_server: &RenderServer,
        singletons: Option<&Singletons>,
    ) {
        // Update nodes.
        for node in self.arena.iter_mut() {
            node.get_mut().update(&queue, dt, &render_server, singletons);
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
                let iter = root
                    .traverse(&self.arena)
                    .filter_map(|ev| match ev {
                        NodeEdge::Start(_) => None,
                        NodeEdge::End(id) => Some(id),
                    });

                for id in iter {
                    self.arena.get(id).unwrap().get().draw(render_pass, render_server, singletons);
                }
            }
        }

        self.gizmo.draw(render_pass, render_server, singletons);
    }
}
