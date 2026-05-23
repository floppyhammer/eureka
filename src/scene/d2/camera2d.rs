use crate::core::singleton::Singletons;
use crate::math::transform::Transform2d;
use crate::render::camera::{CameraType, CameraUniform, OrthographicProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::scene::{AsNode, NodeType};
use glam::{Mat4, UVec2, Vec2, Vec3};
use std::any::Any;

pub struct Camera2d {
    pub transform: Transform2d,

    pub view_size: Vec2,

    /// Where to draw. None for screen.
    pub view: Option<u32>,

    projection: Projection,
}

impl Camera2d {
    pub fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            view_size: Vec2::ZERO,
            view: None,
            projection: OrthographicProjection::default().into(),
        }
    }

    pub fn calc_view_matrix(&self) -> Mat4 {
        let rotation_mat = Mat4::from_rotation_z(-self.transform.rotation);
        let translation_mat = Mat4::from_translation(Vec3::new(
            self.transform.position.x,
            self.transform.position.y,
            0.0,
        ));

        translation_mat * rotation_mat
    }

    pub fn when_view_size_changes(&mut self, new_size: UVec2) {
        self.projection.update(new_size.x as f32, new_size.y as f32);
    }
}

impl AsNode for Camera2d {
    fn node_type(&self) -> NodeType {
        NodeType::Camera2d
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn update(&mut self, _dt: f32, singletons: &mut Singletons) {
        self.projection.update(
            singletons.render_server.surface_config.width as f32,
            singletons.render_server.surface_config.height as f32,
        );
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix();
        let proj_mat = self.projection.calc_matrix();

        uniform.view_position[0] = self.transform.position.x;
        uniform.view_position[1] = self.transform.position.y;
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();

        draw_cmds.extracted.cameras.add(CameraType::D2, uniform);
    }
}
