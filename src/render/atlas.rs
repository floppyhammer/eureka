use crate::render::TextureId;
use glam::{UVec2, Vec2, Vec4};

#[derive(Clone)]
pub struct AtlasInstance {
    pub(crate) position: Vec2,
    pub(crate) size: Vec2,
    pub(crate) region: Vec4,
    pub(crate) color: Vec4,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AtlasInstanceRaw {
    position: [f32; 2],
    size: [f32; 2],
    region: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AtlasParamsUniform {
    camera_view_size: [f32; 2],
    atlas_size: [f32; 2],
}
impl AtlasParamsUniform {
    pub(crate) fn get_uniform_offset_unit() -> u32 {
        let offset_alignment =
            wgpu::Limits::downlevel_defaults().min_uniform_buffer_offset_alignment;
        let size = size_of::<AtlasParamsUniform>() as u32;
        (size + offset_alignment - 1) & !(offset_alignment - 1)
    }

    pub(crate) fn new(atlas_size: UVec2, camera_view_size: UVec2) -> Self {
        Self {
            camera_view_size: [camera_view_size.x as f32, camera_view_size.y as f32],
            atlas_size: [atlas_size.x as f32, atlas_size.y as f32],
        }
    }
}

impl AtlasInstance {
    fn to_raw(&self) -> AtlasInstanceRaw {
        AtlasInstanceRaw {
            position: self.position.into(),
            size: self.size.into(),
            region: self.region.into(),
            color: self.color.into(),
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct Atlas {
    pub(crate) texture: Option<TextureId>,
    pub(crate) instances: Vec<AtlasInstance>,
    pub(crate) texture_size: (u32, u32),
}
