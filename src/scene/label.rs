use crate::math::rect_to_vector4;
use crate::math::transform::Transform2d;
use crate::render::atlas::{AtlasMode, DrawAtlas};
use crate::resources::FONT_ATLAS_SIZE;
use crate::scene::{CameraInfo, InputServer, NodeType};
use crate::{
    AsNode, Atlas, AtlasInstance, DynamicFont, InputEvent, RenderServer, Singletons, TextServer,
    Texture,
};
use cgmath::{EuclideanSpace, Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use std::any::Any;

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
            let (glyphs, lines) = singletons.text_server.font.get_glyphs(self.text.as_str());

            let ascent = singletons.text_server.font.get_ascent();

            // Update atlas data.
            let mut instances = vec![];
            let origin = self.transform.position - Point2::new(0.0, ascent);

            let mut layout_pos = Point2::new(0.0, 0.0);

            for line in lines {
                for i in line {
                    let g = &glyphs[i];

                    let instance = AtlasInstance {
                        position: Vector2::new(
                            layout_pos.x + g.offset.x as f32,
                            layout_pos.y + g.offset.y as f32,
                        ) + origin,
                        size: Vector2::new(g.bitmap_size.x as f32, g.bitmap_size.y as f32),
                        region: rect_to_vector4(g.region.to_f32()) / FONT_ATLAS_SIZE as f32,
                        color: Vector4::new(1.0, 1.0, 1.0, 1.0),
                    };
                    instances.push(instance);

                    // Update next glyph's position.
                    layout_pos.x += g.x_adv as f32;
                }

                layout_pos.x = 0.0;
                layout_pos.y -= singletons.text_server.font.size as f32 + self.leading;
            }

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
            &singletons.text_server.font.atlas_bind_group,
            render_pass,
            camera_info,
            singletons,
        );
    }
}
