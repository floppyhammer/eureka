use crate::math::color::ColorU;

/// 点光源组件
pub struct PointLightComponent {
    pub color: ColorU,
    pub strength: f32,
    pub radius: f32, // 新增：影响半径
    pub shadow_near: f32,
    pub shadow_far: f32,
}

impl Default for PointLightComponent {
    fn default() -> Self {
        Self {
            color: ColorU::white(),
            strength: 1.0,
            radius: 10.0, // 默认影响 10 米
            shadow_near: 0.1,
            shadow_far: 100.0,
        }
    }
}
