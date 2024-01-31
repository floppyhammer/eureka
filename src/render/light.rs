use crate::render::TextureId;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    pub(crate) position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding: u32,
    pub(crate) color: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    pub(crate) _padding2: u32,
}

struct LightRenderResources {
    shadow_map: Option<TextureId>,
    pipeline: Option<wgpu::RenderPipeline>,
    pub(crate) light_camera_bind_group: Option<wgpu::BindGroup>,
    pub(crate) light_camera_uniform_buffer: Option<wgpu::Buffer>,
}

pub(crate) fn render_shadow() {}
