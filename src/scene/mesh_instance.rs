use crate::resource::{texture, mesh, material};
use crate::scene::node::WithDraw;

struct MeshInstance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Vector3<f32>,
    pub scale: cgmath::Vector3<f32>,
    pub name: String,
}

impl WithDraw for MeshInstance {
    fn draw(&self) {
        // Code to actually draw.
    }
}
