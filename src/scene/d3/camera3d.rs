use crate::animation::property::PropertyProvider;
use crate::core::singleton::Singletons;
use crate::math::transform::Transform3d;
use crate::render::camera::{CameraType, CameraUniform};
use crate::render::draw_command::DrawCommands;
use crate::scene::{AsNode, AsNode3d, Node3d, NodeType};
use glam::{Mat4, Quat, UVec2, Vec3};
use std::any::Any;

const DEFAULT_FOV: f32 = 60.0_f32.to_radians();
const DEFAULT_NEAR: f32 = 0.1;
const DEFAULT_FAR: f32 = 100.0;

pub struct Camera3d {
    pub node_3d: Node3d,
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub ssao_enabled: bool,

    viewport_size: UVec2,
}

impl Camera3d {
    pub fn new() -> Self {
        Self {
            node_3d: Node3d {
                transform: Transform3d::default(),
                global_transform: Transform3d::default(),
            },
            fov: DEFAULT_FOV,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
            ssao_enabled: true,
            viewport_size: UVec2::new(1280, 720),
        }
    }

    /// Get view matrix.
    pub fn calc_view_matrix(&self) -> Mat4 {
        let forward = self.node_3d.global_transform.rotation * Vec3::NEG_Z;

        Mat4::look_to_rh(
            self.node_3d.global_transform.position,
            forward,
            Vec3::Y,
        )
    }

    pub fn calc_view_matrix_without_pos(&self) -> Mat4 {
        let forward = self.node_3d.global_transform.rotation * Vec3::NEG_Z;

        Mat4::look_to_rh(
            Vec3::ZERO,
            forward,
            Vec3::Y,
        )
    }

    pub fn when_view_size_changes(&mut self, new_size: UVec2) {
        self.viewport_size = new_size;
    }
}

impl AsNode for Camera3d {
    fn node_type(&self) -> NodeType {
        NodeType::Camera3d
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_node_3d(&self) -> Option<&dyn AsNode3d> {
        Some(self)
    }

    fn as_node_3d_mut(&mut self) -> Option<&mut dyn AsNode3d> {
        Some(self)
    }

    fn update(&mut self, _dt: f32, singletons: &mut Singletons) {
        // Update viewport size.
        self.viewport_size = UVec2::new(
            singletons.render_context.surface_config.width,
            singletons.render_context.surface_config.height,
        );

        self.node_3d.global_transform = self.node_3d.transform;
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let mut uniform = CameraUniform::default();

        let view_mat = self.calc_view_matrix();
        let aspect_ratio = self.viewport_size.x as f32 / self.viewport_size.y as f32;
        let proj_mat = Mat4::perspective_rh(self.fov, aspect_ratio, self.near, self.far);

        uniform.view_position = self.node_3d.global_transform.position.extend(1.0).to_array();
        uniform.view = view_mat.to_cols_array_2d();
        uniform.proj = proj_mat.to_cols_array_2d();
        uniform.view_proj = (proj_mat * view_mat).to_cols_array_2d();
        uniform.inv_proj = proj_mat.inverse().to_cols_array_2d();
        uniform.ssao_enabled = if self.ssao_enabled { 1 } else { 0 };

        draw_cmds.extracted.cameras.add(CameraType::D3, uniform);
    }

    fn as_property_provider_mut(&mut self) -> Option<&mut dyn PropertyProvider> {
        Some(&mut self.node_3d)
    }
}

impl AsNode3d for Camera3d {
    fn get_position(&self) -> Vec3 {
        self.node_3d.transform.position
    }

    fn set_position(&mut self, position: Vec3) {
        self.node_3d.transform.position = position;
    }

    fn get_rotation(&self) -> Quat {
        self.node_3d.transform.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.node_3d.transform.rotation = rotation;
    }

    fn get_scale(&self) -> Vec3 {
        self.node_3d.transform.scale
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.node_3d.transform.scale = scale;
    }

    fn get_transform(&self) -> Transform3d {
        self.node_3d.transform
    }

    fn get_global_transform(&self) -> Transform3d {
        self.node_3d.global_transform
    }

    fn set_global_transform(&mut self, transform: Transform3d) {
        self.node_3d.global_transform = transform;
    }
}
