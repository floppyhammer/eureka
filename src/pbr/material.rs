use crate::render::texture::CubeTexture;
use crate::render::Texture;

pub struct Material2d {
    // Material name for debugging reason. Not unique.
    pub name: String,
    pub texture: Texture,
    // Bind group for the texture.
    pub bind_group: wgpu::BindGroup,
}

pub struct Material3d {
    pub name: String,
    pub diffuse_texture: Texture,
    // TODO: make this an option.
    pub normal_texture: Texture,
    // Bind group for the textures.
    pub bind_group: wgpu::BindGroup,
}

pub struct MaterialSky {
    pub name: String,
    pub texture: CubeTexture,
    pub bind_group: wgpu::BindGroup,
}
