use crate::server::input_server::InputEvent;
use crate::{Camera2d, Gizmo, InputServer, RenderServer, Singletons};
use cgmath::*;
use indextree::{Arena, NodeId};

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
    // This vector is of type Box<dyn AsNode>, which is a trait object;
    // it’s a stand-in for any type inside a Box that implements the AsNode trait.
    pub nodes: Vec<Box<dyn AsNode>>,

    // Scene tree.
    pub arena: Arena<u64>,

    gizmo: Gizmo,
}

impl World {
    pub fn new() -> Self {
        let mut nodes: Vec<_> = Vec::new();

        let mut arena = Arena::new();

        let gizmo = Gizmo::new();

        Self {
            nodes,
            arena,
            gizmo,
        }
    }

    pub fn add_node(&mut self, new_node: Box<dyn AsNode>) -> NodeId {
        let id = self.arena.new_node(self.nodes.len() as u64);
        self.nodes.push(new_node);

        id
    }

    pub fn input(&mut self, input_event: &InputEvent) {
        // Handle input.
        for node in self.nodes.iter_mut() {
            node.input(input_event);
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
        for node in self.nodes.iter_mut() {
            node.update(&queue, dt, &render_server, singletons);
        }
    }

    pub fn draw<'a, 'b: 'a>(
        &'b mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        // Draw nodes.
        for node in self.nodes.iter() {
            node.draw(render_pass, render_server, singletons);
        }

        self.gizmo.draw(render_pass, render_server, singletons);
    }
}
