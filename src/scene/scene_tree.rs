use cgmath::*;
use crate::scene::input_event::InputEvent;

pub trait Draw {
    fn draw(&self);
}

pub trait Update {
    fn update(&self, delta: f64);
}

pub trait WithInput {
    fn input(&mut self, input: InputEvent);
}

pub trait AsNode: Draw + Update + WithInput {}

pub struct SceneTree {
    // This vector is of type Box<dyn Draw>, which is a trait object;
    // it’s a stand-in for any type inside a Box that implements the Draw trait.
    pub nodes: Vec<Box<dyn AsNode>>,
}

impl SceneTree {
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
    pub label: String,
}

impl Draw for TextureRect {
    fn draw(&self) {
        // code to actually draw a button
    }
}

struct Model {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Vector3<f32>,
    pub scale: cgmath::Vector3<f32>,
    pub label: String,
}

impl Draw for Model {
    fn draw(&self) {
        // code to actually draw a select box
    }
}
