use crate::math::rect_to_vector4;
use crate::math::transform::Transform2d;
use crate::render::atlas::{AtlasMode, DrawAtlas};
use crate::scene::{CameraInfo, NodeType};
use crate::{
    AsNode, Atlas, AtlasInstance, RenderServer, Singletons, TextServer,
    Texture,
};
use cgmath::{EuclideanSpace, Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use std::any::Any;
use crate::text::FONT_ATLAS_SIZE;

pub struct Label {
    text: String,

    pub transform: Transform2d,

    pub(crate) size: Vector2<f32>,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    single_line: bool,

    /// To draw glyph sprites.
    atlas: Atlas,

    leading: f32,
    tracking: f32,
}

impl Label {
    pub fn new(render_server: &RenderServer) -> Label {
        let size = Vector2::new(128.0_f32, 128.0);

        let mut atlas = Atlas::new(
            &render_server,
            Point2::new(FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
        );
        atlas.set_mode(AtlasMode::Text);

        Self {
            text: "Label".to_string(),
            transform: Transform2d::default(),
            size,
            text_is_dirty: true,
            layout_is_dirty: true,
            single_line: false,
            atlas,
            leading: 20.0,
            tracking: 0.0,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.text_is_dirty = true;
    }
}

impl AsNode for Label {
    fn node_type(&self) -> NodeType {
        NodeType::Label
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        if self.text_is_dirty {
            let mut instances = singletons.text_server.get_instances(
                self.text.as_str(),
                None,
                self.transform,
                self.leading,
            );

            self.atlas
                .set_instances(instances, &singletons.render_server);

            self.text_is_dirty = false;
        }
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        self.atlas.draw(
            singletons.text_server.get_font_bind_group(None),
            render_pass,
            camera_info,
            singletons,
        );
    }
}
