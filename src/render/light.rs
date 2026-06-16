use glam::Vec3;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PointLightUniform {
    pub(crate) position: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) radius: f32,
    pub(crate) shadow_near: f32,
    pub(crate) shadow_far: f32,
    pub(crate) _pad: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct DirectionalLightUniform {
    pub(crate) direction: [f32; 3],
    pub(crate) strength: f32,
    pub(crate) color: [f32; 3],
    pub(crate) shadow_distance: f32,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ExtractedLights {
    pub(crate) point_lights: Vec<PointLightUniform>,
    pub(crate) directional_light: Option<DirectionalLightUniform>,
}

pub(crate) const NUM_CASCADES: usize = 3;

// Clustered Forward constants
pub(crate) const CLUSTER_GRID_SIZE: [u32; 3] = [16, 9, 24];
pub(crate) const MAX_LIGHTS_PER_CLUSTER: usize = 256;
pub(crate) const MAX_SHADOWED_POINT_LIGHTS: usize = 4;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Cluster {
    pub(crate) offset: u32,
    pub(crate) count: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ClusterConfig {
    pub(crate) screen_size: [f32; 2],
    pub(crate) _pad0: [f32; 2],
    pub(crate) grid_size: [u32; 3],
    pub(crate) num_lights: u32,
    pub(crate) z_near: f32,
    pub(crate) z_far: f32,
    pub(crate) _pad1: [f32; 2],
}

pub(crate) const POINT_SHADOW_FACES: [(Vec3, Vec3); 6] = [
    (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // +X
    (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // -X
    (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),  // +Y
    (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)), // -Y
    (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0)), // +Z
    (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)), // -Z
];

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LightUniform {
    pub(crate) ambient_color: [f32; 3],
    pub(crate) ambient_strength: f32,
    pub(crate) directional_light: DirectionalLightUniform,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CascadeUniform {
    pub(crate) view_proj: [[[f32; 4]; 4]; NUM_CASCADES],
    pub(crate) splits: [f32; 4],
}
