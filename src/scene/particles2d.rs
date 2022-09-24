use crate::render::blit::BlitInstance;
use crate::Texture;

pub struct Particles2d {
    emitting: bool,
    amount: u32,

    lifetime: f32,

    pub texture: Option<Texture>,
    pub texture_bind_group: wgpu::BindGroup,

    instances: Vec<BlitInstance>,

    pub vertex_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}
