use crate::math::transform::Transform2d;
use crate::scene::{AsNode, NodeType};
use cgmath::{Matrix4, Perspective, Point2, Vector2};
use std::any::Any;
use crate::render::camera::{CameraUniform, OrthographicProjection, Projection};
use crate::render::draw_command::DrawCommands;
use crate::Singletons;

pub struct Camera2d {
    pub transform: Transform2d,

    pub view_size: Vector2<u32>,

    /// Where to draw. None for screen.
    pub view: Option<u32>,

    pub projection: Projection,
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
        self.projection.update(singletons.render_server.surface_config.width as f32, singletons.render_server.surface_config.height as f32);
    }

    fn draw(&self, draw_cmds: &mut DrawCommands) {
        let mut uniform = CameraUniform::default();

        // We're using Vector4 because of the uniforms 16 byte spacing requirement.
        uniform.view_position[0] = self.transform.position.x;
        uniform.view_position[1] = self.transform.position.y;
        uniform.proj = self.projection.calc_matrix().into();

        draw_cmds.extracted.cameras.push(uniform);
    }
}
