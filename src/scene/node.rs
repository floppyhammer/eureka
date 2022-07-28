use cgmath::*;
use crate::server::input_server::InputEvent;
use crate::server::MouseButton;

pub trait WithDraw {
    fn draw<'a, 'b: 'a>(&'b self, render_pass: &mut wgpu::RenderPass<'a>, camera_bind_group: &'a wgpu::BindGroup);
}

pub trait WithUpdate {
    fn update(&mut self, delta: f64);
}

pub trait WithInput {
    fn input(&mut self, input: InputEvent);
}

pub trait AsNode: WithDraw + WithUpdate + WithInput {}

pub struct World {
    // This vector is of type Box<dyn Draw>, which is a trait object;
    // it’s a stand-in for any type inside a Box that implements the AsNode trait.
    pub nodes: Vec<Box<dyn AsNode>>,
}

impl World {
    pub fn run(&mut self) {
        // Handle input.
        for node in self.nodes.iter_mut() {
            node.input(InputEvent::MouseButton(MouseButton::new()));
        }

        // Update nodes.
        for node in self.nodes.iter_mut() {
            node.update(0.001);
        }

        // Draw nodes.
        for node in self.nodes.iter() {
            //node.draw();
        }
    }
}
