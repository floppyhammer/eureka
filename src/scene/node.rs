use cgmath::*;
use crate::scene::input_event::InputEvent;

pub trait WithDraw {
    fn draw(&self);
}

pub trait WithUpdate {
    fn update(&self, delta: f64);
}

pub trait WithInput {
    fn input(&mut self, input: InputEvent);
}

pub trait AsNode: WithDraw + WithUpdate + WithInput {}

pub struct World {
    // This vector is of type Box<dyn Draw>, which is a trait object;
    // it’s a stand-in for any type inside a Box that implements the Draw trait.
    pub nodes: Vec<Box<dyn AsNode>>,
}

impl World {
    pub fn run(&self) {
        // First we update nodes.
        for node in self.nodes.iter() {
            node.update(0.001);
        }

        // Then we draw them.
        for node in self.nodes.iter() {
            node.draw();
        }
    }
}

pub struct TextureRect {
    pub rect_position: cgmath::Vector2<f32>,
    pub rect_size: cgmath::Vector2<f32>,
    pub rect_scale: cgmath::Vector2<f32>,
    pub name: String,
}

impl WithDraw for TextureRect {
    fn draw(&self) {
        // Code to actually draw.
    }
}

struct Model {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Vector3<f32>,
    pub scale: cgmath::Vector3<f32>,
    pub name: String,
}

impl WithDraw for Model {
    fn draw(&self) {
        // Code to actually draw.
    }
}
