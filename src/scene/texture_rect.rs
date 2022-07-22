use crate::resource::texture;
use crate::scene::node::WithDraw;

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
