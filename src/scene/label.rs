use std::any::Any;
use cgmath::{Point2, Vector2, Vector3, Vector4};
use image::DynamicImage;
use crate::{AsNode, Atlas, AtlasInstance, DynamicFont, InputEvent, RenderServer, Singletons, TextServer, Texture};
use crate::render::atlas::{AtlasMode, DrawAtlas};
use crate::resource::FONT_ATLAS_SIZE;
use crate::scene::NodeType;

pub(crate) struct Label {
    text: String,

    pub(crate) position: Vector2<f32>,

    pub(crate) size: Vector2<f32>,

    text_is_dirty: bool,
    layout_is_dirty: bool,

    /// To draw grapheme sprites.
    atlas: Atlas,
}

impl Label {
    pub(crate) fn new(render_server: &RenderServer) -> Label {
        let position = Vector2::new(0.0_f32, 0.0);
        let size = Vector2::new(128.0_f32, 128.0);

        let mut atlas = Atlas::new(&render_server);
        atlas.set_mode(AtlasMode::Text);

        Self {
            text: "Label".to_string(),
            position,
            size,
            text_is_dirty: true,
            layout_is_dirty: true,
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

    fn ready(&mut self) {}

    fn input(&mut self, input: &InputEvent) {}

    fn update(&mut self, dt: f32, singletons: &mut Singletons) {
        if self.text_is_dirty {
            let graphemes = singletons.text_server.font.get_graphemes(self.text.clone());

            // Set font atlas.
            self.atlas.set_texture(Texture::from_image(
                &singletons.render_server.device,
                &singletons.render_server.queue,
                &DynamicImage::ImageLuma8(singletons.text_server.font.atlas_image.clone()),
                None,
            ).unwrap(), &singletons.render_server);

            let mut instances = vec![];
            let mut layout_pos = Point2::new(self.position.x, self.position.y);

            for g in graphemes {
                let instance = AtlasInstance {
                    position: Vector2::new(layout_pos.x, layout_pos.y),
                    size: Vector2::new((g.layout.z - g.layout.x) as f32, (g.layout.w - g.layout.y) as f32),
                    region: Vector4::new(g.region.x as f32 / FONT_ATLAS_SIZE as f32,
                                         g.region.y as f32 / FONT_ATLAS_SIZE as f32,
                                         g.region.z as f32 / FONT_ATLAS_SIZE as f32,
                                         g.region.w as f32 / FONT_ATLAS_SIZE as f32),
                    color: Vector4::new(1.0, 1.0, 1.0, 1.0),
                };
                instances.push(instance);
                layout_pos.x += g.layout.z as f32 - g.layout.x as f32;
            }

            self.atlas.set_instances(instances, &singletons.render_server);

            self.text_is_dirty = false;
        }
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        singletons: &'b Singletons,
    ) {
        self.atlas.draw(render_pass, singletons);
    }
}
