use std::any::Any;
use cgmath::{Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use crate::{AsNode, Atlas, AtlasInstance, DynamicFont, InputEvent, RenderServer, Singletons, TextServer, Texture};
use crate::render::atlas::{AtlasMode, DrawAtlas};
use crate::resource::FONT_ATLAS_SIZE;
use crate::scene::{CameraInfo, InputServer, Label, NodeType};

pub(crate) struct Button {
    label: Label,

    pub(crate) position: Vector2<f32>,

    pub(crate) size: Vector2<f32>,
}

impl Button {
    pub(crate) fn new(render_server: &RenderServer) -> Button {
        let position = Vector2::new(0.0_f32, 0.0);
        let size = Vector2::new(128.0_f32, 128.0);

        Self {
            label: Label::new(render_server),
            position,
            size,
        }
    }

    pub fn set_text(
        &mut self,
        text: String,
    ) {
        self.label.set_text(text);
    }
}

impl AsNode for Button {
    fn node_type(&self) -> NodeType {
        NodeType::Label
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn ready(&mut self) {}

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        self.label.update(dt, camera_info, singletons);
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        self.label.draw(render_pass, camera_info, singletons);
    }
}
