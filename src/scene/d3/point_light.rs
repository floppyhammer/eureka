use crate::math::color::ColorU;
use crate::scene::components::*;
use crate::math::transform::Transform3d;
use hecs::Entity;

/// 点光源组件
pub struct PointLightComponent {
    pub color: ColorU,
    pub strength: f32,
    pub shadow_near: f32,
    pub shadow_far: f32,
}

impl Default for PointLightComponent {
    fn default() -> Self {
        Self {
            color: ColorU::white(),
            strength: 1.0,
            shadow_near: 0.1,
            shadow_far: 100.0,
        }
    }
}
