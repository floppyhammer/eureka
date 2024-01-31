use crate::math::transform::Transform2d;
use crate::render::camera::{CameraType, CameraUniform, OrthographicProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::scene::{AsNode, NodeType};
use crate::Singletons;
use cgmath::{Angle, InnerSpace, Matrix4, Perspective, Point2, Point3, Vector2, Vector3};
use std::any::Any;

pub struct Camera2d {
    pub transform: Transform2d,

    pub view_size: Vector2<u32>,

    /// Where to draw. None for screen.
    pub view: Option<u32>,

    projection: Projection,
}

impl Camera2d {
    pub fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            view_size: Vector2::new(0, 0),
            view: None,
            projection: OrthographicProjection::default().into(),
        }
    }

    pub fn calc_view_matrix(&self) -> Matrix4<f32> {
        let rotation_mat = Matrix4::from_angle_z(-cgmath::Deg(self.transform.rotation));
        let translation_mat = Matrix4::from_translation(Vector3::new(
            self.transform.position.x,
            self.transform.position.y,
            0.0,
        ));

        translation_mat * rotation_mat
    }

    pub fn when_view_size_changes(&mut self, new_size: Vector2<u32>) {
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

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
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
        uniform.view = view_mat.into();
        uniform.proj = proj_mat.into();
        uniform.view_proj = (proj_mat * view_mat).into();

        draw_cmds.extracted.cameras.add(CameraType::D2, uniform);
    }
}
