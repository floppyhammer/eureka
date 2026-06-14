use crate::animation::property::PropertyProvider;
use crate::core::singleton::Singletons;
use crate::math::transform::Transform2d;
use crate::render::camera::{CameraType, CameraUniform, OrthographicProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::scene::d2::node2d::{AsNode2d, Node2d};
use crate::scene::{AsNode, NodeType};
use glam::{Mat4, UVec2, Vec2, Vec3};
use std::any::Any;

pub struct Camera2d {
    pub node_2d: Node2d,

    pub view_size: Vec2,

    /// Where to draw. None for screen.
    pub view: Option<u32>,

    pub projection: Projection,
}

impl Camera2d {
    pub fn default() -> Self {
        Self {
            node_2d: Node2d::default(),
            view_size: Vec2::ZERO,
            view: None,
            projection: OrthographicProjection::default().into(),
        }
    }

    pub fn calc_view_matrix(&self) -> Mat4 {
        let rotation_mat = Mat4::from_rotation_z(-self.node_2d.global_transform.rotation);
        let translation_mat = Mat4::from_translation(Vec3::new(
            -self.node_2d.global_transform.position.x,
            -self.node_2d.global_transform.position.y,
            0.0,
        ));

        rotation_mat * translation_mat
    }

    pub fn when_view_size_changes(&mut self, new_size: UVec2) {
        self.projection.update(new_size.x as f32, new_size.y as f32);
    }
}

impl AsNode for Camera2d {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn node_type(&self) -> NodeType {
        NodeType::Camera2d
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

        uniform.view_position[0] = self.node_2d.global_transform.position.x;
        uniform.view_position[1] = self.node_2d.global_transform.position.y;
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();

        draw_cmds.extracted.cameras.add(CameraType::D2, uniform);
    }

    fn as_node_2d(&self) -> Option<&dyn AsNode2d> {
        Some(self)
    }

    fn as_node_2d_mut(&mut self) -> Option<&mut dyn AsNode2d> {
        Some(self)
    }

    fn as_property_provider_mut(&mut self) -> Option<&mut dyn PropertyProvider> {
        Some(&mut self.node_2d)
    }
}

impl AsNode2d for Camera2d {
    fn get_size(&self) -> Vec2 {
        self.view_size
    }

    fn set_size(&mut self, size: Vec2) {
        self.view_size = size;
    }

    fn get_position(&self) -> Vec2 {
        self.node_2d.transform.position
    }

    fn set_position(&mut self, position: Vec2) {
        self.node_2d.transform.position = position;
    }

    fn get_rotation(&self) -> f32 {
        self.node_2d.transform.rotation
    }

    fn set_rotation(&mut self, rotation: f32) {
        self.node_2d.transform.rotation = rotation;
    }

    fn get_transform(&self) -> Transform2d {
        self.node_2d.transform
    }

    fn get_global_transform(&self) -> Transform2d {
        self.node_2d.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform2d) {
        self.node_2d.global_transform = transform;
    }
}
