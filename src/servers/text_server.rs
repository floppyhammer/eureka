use crate::math::rect_to_vector4;
use crate::math::transform::Transform2d;
use crate::render::atlas::AtlasInstance;
use crate::resources::{Glyph, RenderServer, Texture, FONT_ATLAS_SIZE};
use crate::DynamicFont;
use cgmath::{Point2, Vector2, Vector4};
use std::ops::Range;
use std::path::Path;
use std::time::Instant;
use winit::event::VirtualKeyCode::P;

pub struct TextServer {
    default_font: DynamicFont,
}

impl TextServer {
    pub(crate) fn new<P: AsRef<Path>>(font_path: P, render_server: &RenderServer) -> Self {
        let now = Instant::now();

        let mut font = DynamicFont::load(font_path, render_server);

        let elapsed_time = now.elapsed();
        log::info!(
            "Text server setup took {} milliseconds",
            elapsed_time.as_millis()
        );

        Self { default_font: font }
    }

    pub(crate) fn update_gpu(&mut self, render_server: &RenderServer) {
        self.default_font.upload(render_server);
    }

    pub(crate) fn get_default_font(&self) -> &DynamicFont {
        &self.default_font
    }

    pub(crate) fn get_instances(
        &mut self,
        text: &str,
        font_id: Option<String>,
        xform: Transform2d,
        leading: f32,
    ) -> Vec<AtlasInstance> {
        let font;

        if font_id.is_some() {
            font = &mut self.default_font;
        } else {
            font = &mut self.default_font;
        }

        let (glyphs, lines) = font.get_glyphs(text);

        let ascent = font.get_ascent();

        // Update atlas data.
        let mut instances = vec![];

        // Move origin from baseline to top-left.
        let origin = xform.position + Vector2::new(0.0, ascent);

        let mut layout_pos = Vector2::new(0.0, 0.0);

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
            layout_pos.y += font.size as f32 + leading;
        }

        instances
    }

    pub(crate) fn get_font_bind_group(&self, font_id: Option<String>) -> &wgpu::BindGroup {
        &self.default_font.atlas_bind_group
    }
}
