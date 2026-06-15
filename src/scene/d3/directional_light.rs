use crate::math::color::ColorU;

pub struct DirectionalLightComponent {
    pub color: ColorU,
    pub strength: f32,
    pub shadow_distance: f32,
}

impl Default for DirectionalLightComponent {
    fn default() -> Self {
        Self {
            color: ColorU::white(),
            strength: 1.0,
            shadow_distance: 20.0,
        }
    }
}
