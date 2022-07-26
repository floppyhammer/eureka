use wgpu::Texture;
use crate::resource::{Material2d, Mesh};
use crate::scene::node::WithDraw;

pub struct Sprite {
    pub name: String,

    pub position: cgmath::Vector2<f32>,
    pub size: cgmath::Vector2<f32>,
    pub scale: cgmath::Vector2<f32>,

    pub texture: Option<Texture>,
}

impl WithDraw for Sprite {
    fn draw(&self) {
        // Code to actually draw.
    }
}

impl Sprite {
    fn new() -> Sprite {
        let position = cgmath::Vector2::new(0.0 as f32, 0.0);
        let size = cgmath::Vector2::new(128.0 as f32, 128.0);
        let scale = cgmath::Vector2::new(1.0 as f32, 1.0);

        Self {
            name: "".to_string(),
            position,
            size,
            scale,
            texture: None,
        }
    }
}

pub trait DrawSprite<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material2d,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSprite<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material2d,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        // Set vertex buffer for VertexInput.
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Set texture.
        self.set_bind_group(0, &material.bind_group, &[]);

        // Set camera uniform.
        self.set_bind_group(1, camera_bind_group, &[]);

        self.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
