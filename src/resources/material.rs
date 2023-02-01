use crate::resources::texture;

pub struct Material2d {
    // Material name for debugging reason. Not unique.
    pub name: String,
    pub texture: texture::Texture,
    // Bind group for the texture.
    pub bind_group: wgpu::BindGroup,
}

pub struct Material3d {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    // TODO: make this an option.
    pub normal_texture: texture::Texture,
    // Bind group for the textures.
    pub bind_group: wgpu::BindGroup,
}

pub struct MaterialSky {
    pub name: String,
    pub texture: texture::CubeTexture,
    pub bind_group: wgpu::BindGroup,
}
