use crate::math::rect_to_vector4;
use crate::math::transform::Transform2d;
// use crate::render::atlas::AtlasInstance;
use crate::render::atlas::{Atlas, AtlasInstance, AtlasMode};
use crate::render::{RenderServer, Texture, TextureCache};
use crate::text::{DynamicFont, Glyph, FONT_ATLAS_SIZE};
use cgmath::{Point2, Vector2, Vector4};
use font_kit::source::SystemSource;
use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;
use std::time::Instant;
use winit::keyboard::Key::Named;

pub struct TextServer {
    fonts: HashMap<&'static str, DynamicFont>,
}

impl TextServer {
    pub(crate) fn new(render_server: &RenderServer, texture_cache: &mut TextureCache) -> Self {
        let now = Instant::now();

        let default_font_data = find_system_font("arial");

        let font =
            DynamicFont::load_from_memory(default_font_data.unwrap(), render_server, texture_cache);

        let elapsed_time = now.elapsed();
        log::info!(
            "Text server setup took {} milliseconds",
            elapsed_time.as_millis()
        );

        let mut fonts = HashMap::new();
        fonts.insert("default", font);

        Self { fonts }
    }

    /// Load a new font from disk.
    pub fn load_font(
        &mut self,
        font_path: &'static str,
        render_server: &RenderServer,
        texture_cache: &mut TextureCache,
    ) {
        let font = DynamicFont::load_from_file(font_path, render_server, texture_cache);
        self.fonts.insert(font_path, font);
    }

    pub(crate) fn prepare(
        &mut self,
        render_server: &RenderServer,
        texture_cache: &mut TextureCache,
    ) {
        for (key, font) in &mut self.fonts {
            font.upload(render_server, texture_cache);
        }
    }

    pub(crate) fn get_default_font(&self) -> &DynamicFont {
        self.fonts.get("default").unwrap()
    }

    pub(crate) fn get_atlas(
        &mut self,
        text: &str,
        font_id: Option<String>,
        xform: Transform2d,
        leading: f32,
    ) -> Atlas {
        let font;

        if font_id.is_some() {
            font = self.fonts.get_mut(&*font_id.unwrap()).unwrap();
        } else {
            font = self.fonts.get_mut("default").unwrap();
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

        Atlas {
            texture: Some(font.atlas_texture),
            instances,
            texture_size: (FONT_ATLAS_SIZE, FONT_ATLAS_SIZE),
            mode: AtlasMode::Text,
        }
    }
}

fn find_system_font(font_name: &str) -> Option<Vec<u8>> {
    let result = std::panic::catch_unwind(|| {
        let mut font = None;

        if !font_name.is_empty() {
            let res = SystemSource::new().select_by_postscript_name(font_name);

            if res.is_ok() {
                font = Some(res.unwrap().load().unwrap());
            }
        }

        if (font.is_none()) {
            let family_names = [font_kit::family_name::FamilyName::Serif];
            let properties = font_kit::properties::Properties::default();

            let res = SystemSource::new().select_best_match(&family_names, &properties);

            if res.is_ok() {
                font = Some(res.unwrap().load().unwrap());
            }
        }

        if (font.is_none()) {
            let handle = SystemSource::new()
                .all_fonts()
                .unwrap()
                .first()
                .unwrap()
                .clone();

            font = Some(handle.load().unwrap());
        }

        let font_data = font
            .take()
            .expect("Font fallback failed!")
            .copy_font_data()
            .unwrap();
        let font_data = (*font_data).clone();

        Some(font_data)
    });
    if result.is_err() {
        eprintln!("ERROR: failed to find font: {}", font_name);
        return None;
    }

    result.unwrap()
}
