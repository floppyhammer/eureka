use std::any::Any;
use cgmath::{Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use crate::{AsNode, Atlas, AtlasInstance, DynamicFont, InputEvent, RenderServer, Singletons, TextServer, Texture};
use crate::math::transform::Transform2d;
use crate::render::atlas::{AtlasMode, DrawAtlas};
use crate::resource::FONT_ATLAS_SIZE;
use crate::scene::{CameraInfo, InputServer, NodeType};

pub(crate) struct Label {
    text: String,

    pub(crate) transform: Transform2d,

    pub(crate) size: Vector2<f32>,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    single_line: bool,

    /// To draw grapheme sprites.
    atlas: Atlas,
}

impl Label {
    pub(crate) fn new(render_server: &RenderServer) -> Label {
        let size = Vector2::new(128.0_f32, 128.0);

        let mut atlas = Atlas::new(&render_server, Point2::new(FONT_ATLAS_SIZE, FONT_ATLAS_SIZE));
        atlas.set_mode(AtlasMode::Text);

        Self {
            text: "Label".to_string(),
            transform: Transform2d::default(),
            size,
            text_is_dirty: true,
            layout_is_dirty: true,
            single_line: false,
            atlas,
        }
    }

    pub fn set_text(
        &mut self,
        text: String,
    ) {
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

    fn update(&mut self, dt: f32, camera_info: &CameraInfo, singletons: &mut Singletons) {
        if self.text_is_dirty {
            let graphemes = singletons.text_server.font.get_graphemes(self.text.clone());

            // Update atlas data.
            let mut instances = vec![];
            let mut layout_pos = self.transform.position;

            for g in graphemes {
                let baseline_height = g.layout.y as f32;

                let instance = AtlasInstance {
                    position: Vector2::new(layout_pos.x, layout_pos.y + baseline_height),
                    size: Vector2::new((g.layout.z - g.layout.x) as f32, (g.layout.w - g.layout.y) as f32),
                    region: Vector4::new(g.region.x as f32 / FONT_ATLAS_SIZE as f32,
                                         g.region.y as f32 / FONT_ATLAS_SIZE as f32,
                                         g.region.z as f32 / FONT_ATLAS_SIZE as f32,
                                         g.region.w as f32 / FONT_ATLAS_SIZE as f32),
                    color: Vector4::new(1.0, 1.0, 1.0, 1.0),
                };
                instances.push(instance);

                // Update next grapheme's position.
                if g.text == " " {
                    layout_pos.x += singletons.text_server.font.size as f32 * 0.333;
                } else {
                    layout_pos.x += g.layout.z as f32 - g.layout.x as f32 + 1.0;
                }
            }

            self.atlas.set_instances(instances, &singletons.render_server);

            self.text_is_dirty = false;
        }
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_info: &'b CameraInfo,
        singletons: &'b Singletons,
    ) {
        self.atlas.draw(&singletons.text_server.font.atlas_bind_group, render_pass, camera_info, singletons);
    }
}
