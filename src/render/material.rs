use crate::render::texture;

pub struct Material3d {
    pub name: String, // Material name for debugging reason.
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup, // Bind group for the textures.
}

pub struct Material2d {
    pub name: String, // Material name for debugging reason.
    pub texture: texture::Texture,
    pub bind_group: wgpu::BindGroup, // Bind group for the textures.
}
