use crate::render::camera::{CameraUniform, OrthographicProjection};
use glam::{Mat4, UVec2};

pub struct Camera2dComponent {
    pub(crate) viewport_size: UVec2,
    pub view: Option<u32>,
}

impl Camera2dComponent {
    pub fn default() -> Self {
        Self {
            viewport_size: UVec2::new(1280, 720),
            view: None,
        }
    }

    pub fn calc_view_matrix(&self, global_transform: &Mat4) -> Mat4 {
        // 对于 2D 摄像机，视图矩阵应该是全局矩阵的逆
        global_transform.inverse()
    }

    pub fn build_uniform(&self, global_transform: &Mat4) -> CameraUniform {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix(global_transform);

        let mut projection = OrthographicProjection::default();
        projection.update(self.viewport_size.x as f32, self.viewport_size.y as f32);
        let proj_mat = projection.calc_matrix();

        let (_, _, translation) = global_transform.to_scale_rotation_translation();

        uniform.view_position[0] = translation.x;
        uniform.view_position[1] = translation.y;
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();

        uniform
    }
}
