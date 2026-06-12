use glam::Vec3;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointLightUniform {
    pub(crate) position: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) constant: f32,
    pub(crate) linear: f32,
    pub(crate) quadratic: f32,
    pub(crate) shadow_near: f32,
    pub(crate) shadow_far: f32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct DirectionalLightUniform {
    pub(crate) direction: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) distance: f32,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ExtractedLights {
    pub(crate) point_lights: Vec<PointLightUniform>,
    pub(crate) directional_light: Option<DirectionalLightUniform>,
}

pub(crate) const MAX_POINT_LIGHTS: usize = 4;
pub(crate) const NUM_CASCADES: usize = 3;

pub(crate) const POINT_SHADOW_FACES: [(Vec3, Vec3); 6] = [
    (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)),  // +X
    (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // -X
    (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),   // +Y
    (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)), // -Y
    (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0)),  // +Z
    (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)), // -Z
];

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    pub(crate) ambient_color: [f32; 3],
    pub(crate) ambient_strength: f32,
    pub(crate) directional_light: DirectionalLightUniform,
    pub(crate) point_lights: [PointLightUniform; MAX_POINT_LIGHTS],
    pub(crate) point_light_count: u32,
    pub(crate) _pad: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CascadeUniform {
    pub(crate) view_proj: [[[f32; 4]; 4]; NUM_CASCADES],
    pub(crate) splits: [f32; 4],
}
