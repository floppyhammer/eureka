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

    /// To draw grapheme sprites.
    atlas: Atlas,
}

impl Label {
    pub(crate) fn new(render_server: &RenderServer, text_server: &TextServer) -> Label {
        let device = &render_server.device;

        let position = Vector2::new(0.0_f32, 0.0);
        let size = Vector2::new(128.0_f32, 128.0);

        let mut atlas = Atlas::new(&render_server);
        atlas.set_mode(AtlasMode::Text);

        Self {
            text: "Text".to_string(),
            position,
            size,
            atlas,
        }
    }

    pub fn set_text(
        &mut self,
        render_server: &RenderServer,
        text_server: &mut TextServer,
        text: String,
    ) {
        let graphemes = text_server.font.get_graphemes(text);

        // Set font atlas.
        self.atlas.set_texture(Texture::from_image(
            &render_server.device,
            &render_server.queue,
            &DynamicImage::ImageLuma8(text_server.font.atlas_image.clone()),
            None,
        ).unwrap(), &render_server);

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

        self.atlas.set_instances(instances, &render_server);
    }
}

impl AsNode for Label {
    fn node_type(&self) -> NodeType {
        NodeType::Sprite2d
    }

    fn input(&mut self, input: &InputEvent) {}

    fn update(&mut self, dt: f32, render_server: &RenderServer, singletons: Option<&Singletons>) {
        let camera = singletons.unwrap().camera2d.as_ref().unwrap();
    }

    fn draw<'a, 'b: 'a>(
        &'b self,
        render_pass: &mut wgpu::RenderPass<'a>,
        render_server: &'b RenderServer,
        singletons: &'b Singletons,
    ) {
        self.atlas.draw(render_pass, render_server, singletons);
    }
}
