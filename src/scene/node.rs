use crate::server::input_server::InputEvent;
use crate::server::MouseButton;
use crate::{Camera2d, RenderServer, Singletons};
use cgmath::*;

pub trait AsNode {
    fn input(&mut self, input: InputEvent);

    fn update(&mut self, queue: &wgpu::Queue, dt: f32, render_server: &RenderServer, singletons: Option<&Singletons>);

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
}

impl World {
    pub fn new() -> Self {
        let mut nodes: Vec<_> = Vec::new();

        Self { nodes }
    }

    pub fn add_node(&mut self, new_node: Box<dyn AsNode>) {
        self.nodes.push(new_node);
    }

    pub fn input(&mut self) {
        // Handle input.
        for node in self.nodes.iter_mut() {
            node.input(InputEvent::MouseButton(MouseButton::new()));
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue,
                  dt: f32,
                  render_server: &RenderServer,
                  singletons: Option<&Singletons>) {
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
    }
}
