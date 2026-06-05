use crate::core::singleton::Singletons;
use crate::math::transform::Transform2d;
use crate::render::camera::{CameraType, CameraUniform, OrthographicProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::scene::d2::AsNodeUi;
use crate::scene::{AsNode, NodeType};
use glam::{Mat4, UVec2, Vec2, Vec3};
use std::any::Any;

pub struct Camera2d {
    pub transform: Transform2d,
    pub global_transform: Transform2d,

    pub view_size: Vec2,

    /// Where to draw. None for screen.
    pub view: Option<u32>,

    projection: Projection,
}

impl Camera2d {
    pub fn default() -> Self {
        Self {
            transform: Transform2d::default(),
            global_transform: Transform2d::default(),
            view_size: Vec2::ZERO,
            view: None,
            projection: OrthographicProjection::default().into(),
        }
    }

    pub fn calc_view_matrix(&self) -> Mat4 {
        let rotation_mat = Mat4::from_rotation_z(-self.global_transform.rotation);
        let translation_mat = Mat4::from_translation(Vec3::new(
            -self.global_transform.position.x,
            -self.global_transform.position.y,
            0.0,
        ));

        rotation_mat * translation_mat
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

    fn as_node_ui(&self) -> Option<&dyn AsNodeUi> {
        Some(self)
    }

    fn as_node_ui_mut(&mut self) -> Option<&mut dyn AsNodeUi> {
        Some(self)
    }

    fn update(&mut self, _dt: f32, singletons: &mut Singletons) {
        self.projection.update(
            singletons.render_context.surface_config.width as f32,
            singletons.render_context.surface_config.height as f32,
        );
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix();
        let proj_mat = self.projection.calc_matrix();

        uniform.view_position[0] = self.global_transform.position.x;
        uniform.view_position[1] = self.global_transform.position.y;
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();

        draw_cmds.extracted.cameras.add(CameraType::D2, uniform);
    }
}

impl AsNodeUi for Camera2d {
    fn get_size(&self) -> Vec2 {
        self.view_size
    }

    fn set_size(&mut self, size: Vec2) {
        self.view_size = size;
    }

    fn get_position(&self) -> Vec2 {
        self.transform.position
    }

    fn set_position(&mut self, position: Vec2) {
        self.transform.position = position;
    }

    fn get_rotation(&self) -> f32 {
        self.transform.rotation
    }

    fn set_rotation(&mut self, rotation: f32) {
        self.transform.rotation = rotation;
    }

    fn get_transform(&self) -> Transform2d {
        self.transform
    }

    fn get_global_transform(&self) -> Transform2d {
        self.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform2d) {
        self.global_transform = transform;
    }
}
